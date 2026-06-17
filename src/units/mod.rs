//! 量纲系统 + 量纲一致性检查（科学正确性护栏）。
//!
//! 见 `docs/spec-units.md`。本期（Phase 4a）只做**量纲检查**：
//! - [`Dimension`]：7 个 SI 基本量的指数向量；全 0 表示无量纲。
//! - [`parse_dimension`]：把单位字符串（如 `kPa`、`umol/m2/s`、`mol/mol`、`degC`）
//!   解析成量纲；无法识别返回 `None`（跳过、不误报）。
//! - [`check_expr`] / [`check_equation_file`]：在表达式上传播量纲，抓出
//!   加减/比较两侧量纲不一致、超越函数参数非无量纲、方程右侧与声明输出量纲不符等错误。
//!
//! 暂不做（Phase 4b）：单位换算（比例因子、°C↔K 偏移）、耦合接口自动转换。

use crate::ast::Expr;
use std::collections::HashMap;
use std::fmt;

/// 7 个 SI 基本量的量纲指数。全 0 表示无量纲。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimension {
    pub mass: i8,
    pub length: i8,
    pub time: i8,
    pub temperature: i8,
    pub amount: i8,
    pub current: i8,
    pub luminous: i8,
}

impl Dimension {
    pub const DIMENSIONLESS: Dimension = Dimension {
        mass: 0,
        length: 0,
        time: 0,
        temperature: 0,
        amount: 0,
        current: 0,
        luminous: 0,
    };

    pub fn is_dimensionless(&self) -> bool {
        *self == Self::DIMENSIONLESS
    }

    fn map(&self, f: impl Fn(i8) -> i8) -> Dimension {
        Dimension {
            mass: f(self.mass),
            length: f(self.length),
            time: f(self.time),
            temperature: f(self.temperature),
            amount: f(self.amount),
            current: f(self.current),
            luminous: f(self.luminous),
        }
    }

    fn zip(&self, o: &Dimension, f: impl Fn(i8, i8) -> i8) -> Dimension {
        Dimension {
            mass: f(self.mass, o.mass),
            length: f(self.length, o.length),
            time: f(self.time, o.time),
            temperature: f(self.temperature, o.temperature),
            amount: f(self.amount, o.amount),
            current: f(self.current, o.current),
            luminous: f(self.luminous, o.luminous),
        }
    }

    /// 乘法：量纲指数相加。
    pub fn mul(&self, o: &Dimension) -> Dimension {
        self.zip(o, |a, b| a + b)
    }

    /// 除法：量纲指数相减。
    pub fn div(&self, o: &Dimension) -> Dimension {
        self.zip(o, |a, b| a - b)
    }

    /// 整数次幂：量纲指数乘以 n。
    pub fn powi(&self, n: i8) -> Dimension {
        self.map(|a| a * n)
    }

    /// 平方根：所有指数为偶数时减半，否则 None（无法用整数指数表示）。
    pub fn sqrt(&self) -> Option<Dimension> {
        self.nth_root(2)
    }

    /// 立方根。
    pub fn cbrt(&self) -> Option<Dimension> {
        self.nth_root(3)
    }

    fn nth_root(&self, n: i8) -> Option<Dimension> {
        let ok = [
            self.mass,
            self.length,
            self.time,
            self.temperature,
            self.amount,
            self.current,
            self.luminous,
        ]
        .iter()
        .all(|e| e % n == 0);
        if ok {
            Some(self.map(|a| a / n))
        } else {
            None
        }
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return write!(f, "无量纲");
        }
        let parts = [
            ("M", self.mass),
            ("L", self.length),
            ("T", self.time),
            ("Θ", self.temperature),
            ("N", self.amount),
            ("I", self.current),
            ("J", self.luminous),
        ];
        let mut first = true;
        for (sym, e) in parts {
            if e == 0 {
                continue;
            }
            if !first {
                write!(f, "·")?;
            }
            first = false;
            if e == 1 {
                write!(f, "{sym}")?;
            } else {
                write!(f, "{sym}^{e}")?;
            }
        }
        Ok(())
    }
}

// ============================================
// 单位字符串 -> 量纲 解析
// ============================================

/// 基本/常见单位符号 -> 量纲（不含词头）。
fn base_unit(sym: &str) -> Option<Dimension> {
    use Dimension as D;
    let d = |mass, length, time, temperature, amount| Dimension {
        mass,
        length,
        time,
        temperature,
        amount,
        current: 0,
        luminous: 0,
    };
    Some(match sym {
        // 无量纲
        "1" | "-" | "rad" | "sr" | "%" | "percent" | "ratio" | "frac" | "count" => D::DIMENSIONLESS,
        // SI 基本量
        "g" => d(1, 0, 0, 0, 0),          // 质量（以克为基，词头另算；kg 也映射到 M）
        "m" => d(0, 1, 0, 0, 0),          // 长度
        "s" | "sec" => d(0, 0, 1, 0, 0),  // 时间
        "K" | "degC" | "°C" | "C" => d(0, 0, 0, 1, 0), // 温度（量纲层面统一为 Θ）
        "mol" => d(0, 0, 0, 0, 1),        // 物质的量
        // 常见时间单位
        "min" => d(0, 0, 1, 0, 0),
        "h" | "hr" | "hour" => d(0, 0, 1, 0, 0),
        "d" | "day" => d(0, 0, 1, 0, 0),
        "yr" | "year" => d(0, 0, 1, 0, 0),
        // 常见长度/面积/体积
        "ha" => d(0, 2, 0, 0, 0),
        "L" | "l" | "litre" | "liter" => d(0, 3, 0, 0, 0),
        // 导出单位
        "N" => d(1, 1, -2, 0, 0),  // 力
        "Pa" => d(1, -1, -2, 0, 0), // 压强
        "J" => d(1, 2, -2, 0, 0),  // 能量
        "W" => d(1, 2, -3, 0, 0),  // 功率
        "Hz" => d(0, 0, -1, 0, 0), // 频率
        _ => return None,
    })
}

/// SI 词头 -> 不影响量纲（仅影响比例，量纲检查中忽略）。
fn known_prefix(c: char) -> bool {
    matches!(
        c,
        'Y' | 'Z' | 'E' | 'P' | 'T' | 'G' | 'M' | 'k' | 'h' | 'd' | 'c' | 'm' | 'u' | 'µ' | 'n'
            | 'p' | 'f' | 'a'
    )
}

/// 解析单个带词头与指数的单位记号，如 `kPa`、`umol`、`m2`、`s-1`。
fn parse_token(tok: &str) -> Option<(Dimension, i8)> {
    let tok = tok.trim();
    if tok.is_empty() {
        return Some((Dimension::DIMENSIONLESS, 1));
    }
    // 分离尾部指数：支持 `m2`、`m^2`、`s-1`、`m^-2`
    let (sym_part, exp) = split_exponent(tok);
    // 先整体匹配；不行再尝试「词头 + 单位」
    let dim = if let Some(d) = base_unit(sym_part) {
        d
    } else {
        let mut chars = sym_part.chars();
        let first = chars.next()?;
        let rest: String = chars.collect();
        if known_prefix(first) && !rest.is_empty() {
            base_unit(&rest)?
        } else {
            return None;
        }
    };
    Some((dim, exp))
}

/// 从记号尾部分离指数。返回 (符号部分, 指数)。默认指数 1。
fn split_exponent(tok: &str) -> (&str, i8) {
    // 形如 sym^n
    if let Some(idx) = tok.find('^') {
        let (sym, e) = tok.split_at(idx);
        if let Ok(n) = e[1..].parse::<i8>() {
            return (sym, n);
        }
    }
    // 形如 sym<digits>（可带负号）：从末尾向前取数字
    let bytes = tok.as_bytes();
    let mut i = bytes.len();
    while i > 0 && (bytes[i - 1].is_ascii_digit() || (i == bytes.len() && bytes[i - 1] == b'-')) {
        i -= 1;
    }
    // 处理可能的负号
    let mut start = i;
    if start > 0 && bytes[start - 1] == b'-' && start < bytes.len() {
        start -= 1;
    }
    if start < tok.len() && start > 0 {
        if let Ok(n) = tok[start..].parse::<i8>() {
            return (&tok[..start], n);
        }
    }
    (tok, 1)
}

/// 把单位字符串解析为量纲。支持复合：`/`（除）、`*` 或 `·`（乘）、尾部指数。
/// 无法识别任一记号则返回 `None`（跳过、不误报）。
pub fn parse_dimension(unit: &str) -> Option<Dimension> {
    let unit = unit.trim();
    if unit.is_empty() {
        return Some(Dimension::DIMENSIONLESS);
    }
    let mut result = Dimension::DIMENSIONLESS;
    // 以 '/' 切分：第一段为分子，其余为分母
    for (gi, group) in unit.split('/').enumerate() {
        let sign: i8 = if gi == 0 { 1 } else { -1 };
        for tok in group.split(['*', '·']) {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            let (dim, exp) = parse_token(tok)?;
            result = result.mul(&dim.powi(exp * sign));
        }
    }
    Some(result)
}

// ============================================
// 量纲检查
// ============================================

/// 量纲错误。
#[derive(Debug, Clone, PartialEq)]
pub enum DimError {
    /// 两侧量纲应相同但不同（加减、比较、分支、聚合等）。
    Mismatch {
        context: String,
        left: Dimension,
        right: Dimension,
    },
    /// 某算子要求参数无量纲（超越函数、逻辑运算等），但参数有量纲。
    NonDimensionless { op: String, got: Dimension },
}

impl fmt::Display for DimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DimError::Mismatch { context, left, right } => {
                write!(f, "{context}：量纲不一致（{left} vs {right}）")
            }
            DimError::NonDimensionless { op, got } => {
                write!(f, "{op} 的参数应为无量纲，实际为 {got}")
            }
        }
    }
}

/// 要求两个（可能未知的）量纲相同；不同则记错误。返回用于继续传播的量纲。
fn require_same(
    a: Option<Dimension>,
    b: Option<Dimension>,
    context: &str,
    errs: &mut Vec<DimError>,
) -> Option<Dimension> {
    match (a, b) {
        (Some(x), Some(y)) => {
            if x != y {
                errs.push(DimError::Mismatch {
                    context: context.to_string(),
                    left: x,
                    right: y,
                });
            }
            Some(x)
        }
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// 要求参数无量纲；有量纲则记错误。
fn require_dimensionless(d: Option<Dimension>, op: &str, errs: &mut Vec<DimError>) {
    if let Some(dim) = d {
        if !dim.is_dimensionless() {
            errs.push(DimError::NonDimensionless {
                op: op.to_string(),
                got: dim,
            });
        }
    }
}

/// 推断表达式量纲。`env`: 变量/参数名 -> 量纲。
/// 返回 `None` 表示量纲未知（缺少单位声明或暂不支持的算子），此时跳过、不误报。
/// 检查中发现的错误推入 `errs`。
fn infer(expr: &Expr, env: &HashMap<String, Dimension>, errs: &mut Vec<DimError>) -> Option<Dimension> {
    // 叶子
    match expr {
        Expr::Const(_) | Expr::Pi | Expr::E => return Some(Dimension::DIMENSIONLESS),
        Expr::Var(n) | Expr::Param(n) => return env.get(n).copied(),
        _ => {}
    }

    // 注册表算子：按名称分类应用量纲规则
    if let Some((name, args)) = crate::ops::as_operator(expr) {
        let ds: Vec<Option<Dimension>> = args.iter().map(|a| infer(a, env, errs)).collect();
        return apply_op_dim(name, &ds, &args, errs);
    }

    // 非注册表：聚合与特殊形式
    match expr {
        Expr::Max(xs) | Expr::Min(xs) => {
            let mut acc: Option<Dimension> = None;
            for x in xs {
                let d = infer(x, env, errs);
                acc = require_same(acc, d, "max/min 各参数", errs).or(acc).or(d);
            }
            acc
        }
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = infer(cond, env, errs);
            require_dimensionless(c, "if 条件", errs);
            let t = infer(then_branch, env, errs);
            let e = infer(else_branch, env, errs);
            require_same(t, e, "if 的两个分支", errs)
        }
        Expr::Piecewise { pieces, otherwise } => {
            let mut acc = infer(otherwise, env, errs);
            for (cond, val) in pieces {
                let c = infer(cond, env, errs);
                require_dimensionless(c, "piecewise 条件", errs);
                let v = infer(val, env, errs);
                acc = require_same(acc, v, "piecewise 各分支", errs).or(acc).or(v);
            }
            acc
        }
        Expr::Sum { lower, upper, body, .. } => {
            require_dimensionless(infer(lower, env, errs), "sum 下界", errs);
            require_dimensionless(infer(upper, env, errs), "sum 上界", errs);
            // 求和保持被加项量纲
            infer(body, env, errs)
        }
        Expr::Product { lower, upper, body, .. } => {
            require_dimensionless(infer(lower, env, errs), "product 下界", errs);
            require_dimensionless(infer(upper, env, errs), "product 上界", errs);
            // 连乘量纲依赖项数，静态不确定；仅当被乘项无量纲时结果无量纲
            match infer(body, env, errs) {
                Some(d) if d.is_dimensionless() => Some(Dimension::DIMENSIONLESS),
                _ => None,
            }
        }
        // 其它（特殊函数、向量/矩阵、lambda 等）暂不支持 -> 未知
        _ => None,
    }
}

/// 注册表算子的量纲规则。
fn apply_op_dim(
    name: &str,
    ds: &[Option<Dimension>],
    args: &[&Expr],
    errs: &mut Vec<DimError>,
) -> Option<Dimension> {
    match name {
        // 加减：两侧同量纲，结果同量纲
        "add" | "sub" => require_same(ds[0], ds[1], &format!("{name} 两侧"), errs),
        // 乘除：量纲相乘/相除
        "mul" => match (ds[0], ds[1]) {
            (Some(a), Some(b)) => Some(a.mul(&b)),
            _ => None,
        },
        "div" => match (ds[0], ds[1]) {
            (Some(a), Some(b)) => Some(a.div(&b)),
            _ => None,
        },
        // 保持量纲的一元/同量纲算子
        "neg" | "abs" | "ceil" | "floor" | "round" | "trunc" => ds[0],
        "mod" => require_same(ds[0], ds[1], "mod 两侧", errs),
        "copysign" => ds[0], // 取 arg0 的量纲，符号来自 arg1
        "clamp" => {
            let lo = require_same(ds[0], ds[1], "clamp 值与下界", errs);
            require_same(lo, ds[2], "clamp 值与上界", errs)
        }
        "hypot" => require_same(ds[0], ds[1], "hypot 两参数", errs),
        "hypot3" => {
            let ab = require_same(ds[0], ds[1], "hypot3 参数", errs);
            require_same(ab, ds[2], "hypot3 参数", errs)
        }
        // fma(a,b,c) = a*b + c：dim(a*b) 必须等于 dim(c)
        "fma" => {
            let ab = match (ds[0], ds[1]) {
                (Some(a), Some(b)) => Some(a.mul(&b)),
                _ => None,
            };
            require_same(ab, ds[2], "fma 的 a*b 与 c", errs)
        }
        // 幂：底无量纲 -> 无量纲；指数为整数常量 -> 量纲按整数缩放；否则未知
        "pow" => match ds[0] {
            Some(b) if b.is_dimensionless() => Some(Dimension::DIMENSIONLESS),
            Some(b) => const_int(args[1]).map(|n| b.powi(n)),
            None => None,
        },
        "sqrt" => match ds[0] {
            Some(d) => d.sqrt(),
            None => None,
        },
        "cbrt" => match ds[0] {
            Some(d) => d.cbrt(),
            None => None,
        },
        // atan2(y,x)：两参数同量纲，结果为角度（无量纲）
        "atan2" => {
            require_same(ds[0], ds[1], "atan2 两参数", errs);
            Some(Dimension::DIMENSIONLESS)
        }
        // sign：参数任意，结果无量纲
        "sign" => Some(Dimension::DIMENSIONLESS),
        // 比较：两侧同量纲，结果无量纲（布尔）
        "eq" | "lt" | "gt" | "leq" | "geq" | "neq" => {
            require_same(ds[0], ds[1], &format!("{name} 两侧"), errs);
            Some(Dimension::DIMENSIONLESS)
        }
        // 逻辑：参数应无量纲（布尔），结果无量纲
        "and" | "or" => {
            require_dimensionless(ds[0], name, errs);
            require_dimensionless(ds[1], name, errs);
            Some(Dimension::DIMENSIONLESS)
        }
        "not" => {
            require_dimensionless(ds[0], name, errs);
            Some(Dimension::DIMENSIONLESS)
        }
        // 超越函数（exp/ln/log/三角/双曲/反三角/反双曲/倒数三角双曲）：参数无量纲，结果无量纲
        _ => {
            for d in ds {
                require_dimensionless(*d, name, errs);
            }
            Some(Dimension::DIMENSIONLESS)
        }
    }
}

/// 若表达式是整数常量则返回其整数值（用于 pow 的整数指数）。
fn const_int(e: &Expr) -> Option<i8> {
    match e {
        Expr::Const(v) if v.fract() == 0.0 && v.abs() < 127.0 => Some(*v as i8),
        Expr::Neg(inner) => const_int(inner).map(|n| -n),
        _ => None,
    }
}

/// 检查单个表达式的量纲。返回 (推断量纲, 错误列表)。
pub fn check_expr(
    expr: &Expr,
    env: &HashMap<String, Dimension>,
) -> (Option<Dimension>, Vec<DimError>) {
    let mut errs = Vec::new();
    let dim = infer(expr, env, &mut errs);
    (dim, errs)
}

// ============================================
// 在 EquationFile 上检查（需要 schema，仅 cli 特性）
// ============================================

/// 一条量纲诊断（绑定到具体方程）。
#[cfg(feature = "cli")]
#[derive(Debug, Clone)]
pub struct DimDiagnostic {
    pub equation_id: String,
    pub message: String,
}

/// 对整个方程文件做量纲检查：
/// - 由 parameters/variables 的 `unit` 字段建立量纲环境（无法解析的跳过）；
/// - 对每条方程推断右侧量纲、收集错误；
/// - 若输出变量声明了量纲，检查其与右侧推断量纲是否一致。
#[cfg(feature = "cli")]
pub fn check_equation_file(file: &crate::schema::EquationFile) -> Vec<DimDiagnostic> {
    let mut env: HashMap<String, Dimension> = HashMap::new();
    for (name, p) in &file.parameters {
        if let Some(u) = &p.unit {
            if let Some(d) = parse_dimension(u) {
                env.insert(name.clone(), d);
            }
        }
    }
    for (name, v) in &file.variables {
        if let Some(u) = &v.unit {
            if let Some(d) = parse_dimension(u) {
                env.insert(name.clone(), d);
            }
        }
    }

    let mut out = Vec::new();
    for eq in &file.equations {
        let (rhs_dim, errs) = check_expr(&eq.expression, &env);
        for e in errs {
            out.push(DimDiagnostic {
                equation_id: eq.id.clone(),
                message: e.to_string(),
            });
        }
        // 输出变量声明量纲 vs 右侧推断量纲
        if let (Some(rhs), Some(decl)) = (rhs_dim, env.get(&eq.output).copied()) {
            if rhs != decl {
                out.push(DimDiagnostic {
                    equation_id: eq.id.clone(),
                    message: format!(
                        "输出 {} 声明量纲 {} 与方程右侧推断量纲 {} 不一致",
                        eq.output, decl, rhs
                    ),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_to_expr;

    fn dim(m: i8, l: i8, t: i8, th: i8, n: i8) -> Dimension {
        Dimension { mass: m, length: l, time: t, temperature: th, amount: n, current: 0, luminous: 0 }
    }

    #[test]
    fn test_dimension_algebra() {
        let length = dim(0, 1, 0, 0, 0);
        let time = dim(0, 0, 1, 0, 0);
        // 速度 = 长度/时间 = L·T^-1
        let speed = length.div(&time);
        assert_eq!(speed, dim(0, 1, -1, 0, 0));
        // 面积 = 长度^2
        assert_eq!(length.powi(2), dim(0, 2, 0, 0, 0));
        // sqrt(面积) = 长度
        assert_eq!(length.powi(2).sqrt(), Some(length));
        // sqrt(长度) 无法用整数指数表示
        assert_eq!(length.sqrt(), None);
    }

    #[test]
    fn test_parse_units() {
        // 基本
        assert_eq!(parse_dimension("m"), Some(dim(0, 1, 0, 0, 0)));
        assert_eq!(parse_dimension("s"), Some(dim(0, 0, 1, 0, 0)));
        assert_eq!(parse_dimension("degC"), Some(dim(0, 0, 0, 1, 0)));
        // 词头：umol 与 mol 同量纲
        assert_eq!(parse_dimension("umol"), Some(dim(0, 0, 0, 0, 1)));
        assert_eq!(parse_dimension("kPa"), Some(dim(1, -1, -2, 0, 0)));
        // 复合
        assert_eq!(parse_dimension("umol/m2/s"), Some(dim(0, -2, -1, 0, 1)));
        assert_eq!(parse_dimension("m/s"), Some(dim(0, 1, -1, 0, 0)));
        // 无量纲
        assert_eq!(parse_dimension("mol/mol"), Some(Dimension::DIMENSIONLESS));
        assert_eq!(parse_dimension("percent"), Some(Dimension::DIMENSIONLESS));
        // 未知 -> None（跳过）
        assert_eq!(parse_dimension("flibbertigibbet"), None);
    }

    #[test]
    fn test_check_correct_expression() {
        // v = 距离 / 时间 ：无错误，结果 L·T^-1
        let mut env = HashMap::new();
        env.insert("dist".to_string(), dim(0, 1, 0, 0, 0));
        env.insert("t".to_string(), dim(0, 0, 1, 0, 0));
        let (d, errs) = check_expr(&parse_to_expr("(div dist t)").unwrap(), &env);
        assert!(errs.is_empty());
        assert_eq!(d, Some(dim(0, 1, -1, 0, 0)));
    }

    #[test]
    fn test_check_catches_add_mismatch() {
        // 长度 + 时间 ：量纲不一致
        let mut env = HashMap::new();
        env.insert("len".to_string(), dim(0, 1, 0, 0, 0));
        env.insert("t".to_string(), dim(0, 0, 1, 0, 0));
        let (_d, errs) = check_expr(&parse_to_expr("(add len t)").unwrap(), &env);
        assert!(errs.iter().any(|e| matches!(e, DimError::Mismatch { .. })), "应抓到量纲不一致: {errs:?}");
    }

    #[test]
    fn test_check_catches_transcendental_with_units() {
        // exp(温度) ：超越函数参数必须无量纲
        let mut env = HashMap::new();
        env.insert("T".to_string(), dim(0, 0, 0, 1, 0));
        let (_d, errs) = check_expr(&parse_to_expr("(exp T)").unwrap(), &env);
        assert!(
            errs.iter().any(|e| matches!(e, DimError::NonDimensionless { .. })),
            "应抓到超越函数非无量纲参数: {errs:?}"
        );
    }

    #[test]
    fn test_dimensionless_transcendental_ok() {
        // exp(无量纲比值) ：无错误
        let mut env = HashMap::new();
        env.insert("r".to_string(), Dimension::DIMENSIONLESS);
        let (d, errs) = check_expr(&parse_to_expr("(exp r)").unwrap(), &env);
        assert!(errs.is_empty());
        assert_eq!(d, Some(Dimension::DIMENSIONLESS));
    }

    #[test]
    fn test_unknown_units_are_skipped() {
        // 缺少单位声明的变量 -> 量纲未知 -> 不报错
        let env = HashMap::new();
        let (_d, errs) = check_expr(&parse_to_expr("(add a b)").unwrap(), &env);
        assert!(errs.is_empty(), "未知量纲不应误报: {errs:?}");
    }

    // 端到端：在带单位的方程文件上检查（cli 路径）。
    #[cfg(feature = "cli")]
    #[test]
    fn test_check_equation_file_end_to_end() {
        use crate::schema::{
            DataType, Equation, EquationFile, Metadata, Variable, VariableType,
        };

        fn var(unit: &str) -> Variable {
            Variable {
                var_type: VariableType::Intermediate,
                dtype: DataType::Float,
                unit: Some(unit.to_string()),
                description: None,
                source: None,
            }
        }
        fn eq(id: &str, output: &str, expr: &str) -> Equation {
            Equation {
                id: id.to_string(),
                name: id.to_string(),
                output: output.to_string(),
                expression: parse_to_expr(expr).unwrap(),
                formula_display: None,
                reference: None,
            }
        }

        let mut variables = indexmap::IndexMap::new();
        variables.insert("Tmax".to_string(), var("degC"));
        variables.insert("Tmin".to_string(), var("degC"));
        variables.insert("Tmean".to_string(), var("degC"));
        variables.insert("Vlen".to_string(), var("m")); // 长度
        variables.insert("Bad".to_string(), var("degC"));

        let file = EquationFile {
            meta: Metadata {
                id: "TEST".into(),
                model: "Test".into(),
                name_cn: "测试".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
            },
            parameters: Default::default(),
            variables,
            equations: vec![
                // 正确：日均温 (degC) = (Tmax+Tmin)/2
                eq("OK", "Tmean", "(div (add Tmax Tmin) 2)"),
                // 错误1：输出声明为长度 m，但右侧推断为温度 Θ
                eq("BAD_OUT", "Vlen", "(add Tmax Tmin)"),
                // 错误2：超越函数参数带量纲（exp(温度)）
                eq("BAD_EXP", "Bad", "(exp Tmax)"),
            ],
        };

        let diags = check_equation_file(&file);
        // OK 方程不应有诊断
        assert!(!diags.iter().any(|d| d.equation_id == "OK"), "正确方程不应报错: {diags:?}");
        // BAD_OUT 应报「声明量纲与右侧不一致」
        assert!(
            diags.iter().any(|d| d.equation_id == "BAD_OUT" && d.message.contains("不一致")),
            "应抓到输出量纲不一致: {diags:?}"
        );
        // BAD_EXP 应报「参数应为无量纲」
        assert!(
            diags.iter().any(|d| d.equation_id == "BAD_EXP" && d.message.contains("无量纲")),
            "应抓到超越函数带量纲: {diags:?}"
        );
    }
}
