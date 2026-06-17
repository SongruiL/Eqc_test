//! 求值器：对 `Expr` AST 直接做数值求值（树遍历解释器）。
//!
//! 设计见 `docs/spec-operator-registry-and-evaluator.md`。
//!
//! 核心思路：算子的**语义**集中在 [`apply_scalar`]（一张 `名称 -> 计算` 的数据表），
//! [`Expr::eval`] 只负责遍历 AST、求出子节点的值、再查表求值。这样算子语义是
//! 单一真相源，将来代码生成（Phase 2）也可以从同一组名称派生。
//!
//! Phase 1 覆盖标准库 `f64` 能直接算的核心算子；特殊函数（gamma/bessel/erf…）、
//! 向量/矩阵、积分/导数等暂未实现，会显式返回 [`EvalError::Unsupported`]。
//!
//! 语义规范（spec §4）：
//! - `sign(0) = 0`（数学符号函数，区别于 Rust `f64::signum` 在 0 处返回 ±1）
//! - `mod` 为数学取模（floored，结果符号随除数）
//! - 严格模式下，任何非有限结果（NaN/Inf，含除零、`ln` 负数等）一律报 [`EvalError::NonFinite`]

use crate::ast::Expr;
use std::collections::HashMap;
use std::fmt;

/// 求值错误
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// 变量/参数未在环境中定义
    UndefinedVar(String),
    /// 参数个数错误
    ArityMismatch {
        op: String,
        expected: usize,
        found: usize,
    },
    /// 算子尚未在求值器中实现（如特殊函数、向量/矩阵、积分等）
    Unsupported(String),
    /// 结果非有限（NaN/Inf）——严格模式下产生。涵盖除零、`ln` 负数、定义域外等。
    NonFinite { op: String },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::UndefinedVar(name) => write!(f, "未定义的变量/参数: {name}"),
            EvalError::ArityMismatch { op, expected, found } => {
                write!(f, "算子 {op} 参数个数错误：期望 {expected}，实际 {found}")
            }
            EvalError::Unsupported(op) => write!(f, "求值器尚未实现该算子: {op}"),
            EvalError::NonFinite { op } => write!(f, "算子 {op} 求值结果非有限（NaN/Inf）"),
        }
    }
}

impl std::error::Error for EvalError {}

/// 求值环境：变量/参数名 -> 值，外加用于 `sum`/`product` 绑定循环变量的作用域栈。
#[derive(Debug, Clone, Default)]
pub struct Env {
    vars: HashMap<String, f64>,
    /// 内层作用域绑定（如 sum 的循环变量），查找时优先于 `vars`。
    scopes: Vec<(String, f64)>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }

    /// 链式设置一个变量值。
    pub fn with(mut self, name: impl Into<String>, val: f64) -> Self {
        self.vars.insert(name.into(), val);
        self
    }

    /// 设置/覆盖一个变量值。
    pub fn set(&mut self, name: impl Into<String>, val: f64) -> &mut Self {
        self.vars.insert(name.into(), val);
        self
    }

    /// 从 (名称, 值) 列表构造环境。
    pub fn from_pairs(pairs: &[(&str, f64)]) -> Self {
        let mut env = Self::new();
        for (k, v) in pairs {
            env.vars.insert((*k).to_string(), *v);
        }
        env
    }

    /// 查找变量值：先查内层作用域（后进先出），再查全局变量。
    pub fn get(&self, name: &str) -> Option<f64> {
        for (n, v) in self.scopes.iter().rev() {
            if n == name {
                return Some(*v);
            }
        }
        self.vars.get(name).copied()
    }

    fn push_scope(&mut self, name: String, val: f64) {
        self.scopes.push((name, val));
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

/// 求值模式。
#[derive(Debug, Clone, Copy)]
pub struct EvalMode {
    /// 严格模式：非有限结果（NaN/Inf）报错。默认开启；
    /// 将来 GP 阶段可关闭，让 NaN 传播为「惩罚」。
    pub strict: bool,
}

impl Default for EvalMode {
    fn default() -> Self {
        Self { strict: true }
    }
}

impl Expr {
    /// 以默认（严格）模式对表达式求值。
    pub fn eval(&self, env: &Env) -> Result<f64, EvalError> {
        self.eval_with(env, EvalMode::default())
    }

    /// 以指定模式对表达式求值。
    pub fn eval_with(&self, env: &Env, mode: EvalMode) -> Result<f64, EvalError> {
        let mut work = env.clone();
        ev(self, &mut work, mode)
    }
}

/// 递归求值。
fn ev(expr: &Expr, env: &mut Env, mode: EvalMode) -> Result<f64, EvalError> {
    // 注册表快路径：所有纯函数式标量算子从 `ops` 注册表求值（语义单一真相源）。
    if let Some((name, args)) = crate::ops::as_operator(expr) {
        if let Some(s) = crate::ops::spec(name) {
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(ev(a, env, mode)?);
            }
            return chk(name, (s.eval)(&vals), mode);
        }
    }

    match expr {
        // === 叶子 ===
        Expr::Const(c) => Ok(*c),
        Expr::Pi => Ok(std::f64::consts::PI),
        Expr::E => Ok(std::f64::consts::E),
        Expr::Var(n) | Expr::Param(n) => env.get(n).ok_or_else(|| EvalError::UndefinedVar(n.clone())),

        // 所有纯函数式标量算子（算术/三角/双曲/比较/逻辑/hypot/clamp/fma 等）已迁移至 ops 注册表，
        // 由上方注册表快路径处理；此处仅保留叶子、聚合与特殊形式。

        // === 聚合（可变参数，非纯函数式）===
        Expr::Max(xs) => fold_nary("max", xs, f64::NEG_INFINITY, f64::max, env, mode),
        Expr::Min(xs) => fold_nary("min", xs, f64::INFINITY, f64::min, env, mode),

        // === 特殊形式 ===
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            if ev(cond, env, mode)? != 0.0 {
                ev(then_branch, env, mode)
            } else {
                ev(else_branch, env, mode)
            }
        }
        Expr::Piecewise { pieces, otherwise } => {
            for (cond, val) in pieces {
                if ev(cond, env, mode)? != 0.0 {
                    return ev(val, env, mode);
                }
            }
            ev(otherwise, env, mode)
        }
        Expr::Sum { index, lower, upper, body } => {
            let lo = ev(lower, env, mode)?.round() as i64;
            let hi = ev(upper, env, mode)?.round() as i64;
            let mut acc = 0.0_f64;
            for i in lo..=hi {
                env.push_scope(index.clone(), i as f64);
                let r = ev(body, env, mode);
                env.pop_scope();
                acc += r?;
            }
            chk("sum", acc, mode)
        }
        Expr::Product { index, lower, upper, body } => {
            let lo = ev(lower, env, mode)?.round() as i64;
            let hi = ev(upper, env, mode)?.round() as i64;
            let mut acc = 1.0_f64;
            for i in lo..=hi {
                env.push_scope(index.clone(), i as f64);
                let r = ev(body, env, mode);
                env.pop_scope();
                acc *= r?;
            }
            chk("product", acc, mode)
        }

        // === 暂未实现的算子（特殊函数、向量/矩阵、积分/导数等）===
        other => Err(EvalError::Unsupported(variant_tag(other))),
    }
}

/// 聚合（max/min）：对所有子节点求值并折叠。
fn fold_nary(
    name: &str,
    xs: &[Expr],
    init: f64,
    f: fn(f64, f64) -> f64,
    env: &mut Env,
    mode: EvalMode,
) -> Result<f64, EvalError> {
    if xs.is_empty() {
        return Err(EvalError::ArityMismatch {
            op: name.to_string(),
            expected: 1,
            found: 0,
        });
    }
    let mut acc = init;
    for x in xs {
        acc = f(acc, ev(x, env, mode)?);
    }
    chk(name, acc, mode)
}

/// 严格模式下：非有限结果报错。
fn chk(op: &str, v: f64, mode: EvalMode) -> Result<f64, EvalError> {
    if mode.strict && !v.is_finite() {
        Err(EvalError::NonFinite { op: op.to_string() })
    } else {
        Ok(v)
    }
}

/// 从未实现的变体取一个可读的算子名（用于 Unsupported 错误）。
fn variant_tag(e: &Expr) -> String {
    let dbg = format!("{e:?}");
    dbg.split(|c| c == '(' || c == '{' || c == ' ')
        .next()
        .unwrap_or("?")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_to_expr;

    const EPS: f64 = 1e-9;

    fn eval_str(s: &str, env: &Env) -> Result<f64, EvalError> {
        parse_to_expr(s).expect("parse").eval(env)
    }

    #[test]
    fn test_basic_arithmetic() {
        let env = Env::new();
        assert!((eval_str("(add 1 2)", &env).unwrap() - 3.0).abs() < EPS);
        assert!((eval_str("(mul (add 1 2) 4)", &env).unwrap() - 12.0).abs() < EPS);
        assert!((eval_str("(div 7 2)", &env).unwrap() - 3.5).abs() < EPS);
    }

    #[test]
    fn test_variables_and_env() {
        let env = Env::from_pairs(&[("x", 3.0), ("y", 4.0)]);
        // hypot(x, y) = 5
        assert!((eval_str("(sqrt (add (mul x x) (mul y y)))", &env).unwrap() - 5.0).abs() < EPS);
    }

    #[test]
    fn test_undefined_var() {
        let env = Env::new();
        assert_eq!(
            parse_to_expr("(add x 1)").unwrap().eval(&env),
            Err(EvalError::UndefinedVar("x".to_string()))
        );
    }

    #[test]
    fn test_transcendental_and_trig() {
        let env = Env::new();
        assert!((eval_str("(exp 0)", &env).unwrap() - 1.0).abs() < EPS);
        assert!((eval_str("(ln e)", &env).unwrap() - 1.0).abs() < EPS);
        assert!(eval_str("(sin pi)", &env).unwrap().abs() < 1e-9);
        assert!((eval_str("(cos 0)", &env).unwrap() - 1.0).abs() < EPS);
    }

    // ---- 钉死语义（spec §4）----

    #[test]
    fn test_sign_zero_is_zero() {
        let env = Env::new();
        // sgn(0) = 0（区别于 Rust f64::signum 在 0 处返回 +1）
        assert_eq!(Expr::sign(Expr::Const(0.0)).eval(&env).unwrap(), 0.0);
        assert_eq!(Expr::sign(Expr::Const(-3.0)).eval(&env).unwrap(), -1.0);
        assert_eq!(Expr::sign(Expr::Const(2.5)).eval(&env).unwrap(), 1.0);
    }

    #[test]
    fn test_mod_is_mathematical_floored() {
        let env = Env::new();
        // 数学取模：(-7) mod 3 = 2（结果符号随除数），区别于 Rust `%` 的 -1
        let m = Expr::modulo(Expr::Const(-7.0), Expr::Const(3.0));
        assert!((m.eval(&env).unwrap() - 2.0).abs() < EPS);
    }

    #[test]
    fn test_strict_div_by_zero_errors() {
        let env = Env::new();
        let expr = parse_to_expr("(div 1 0)").unwrap();
        // 严格模式（默认）：除零 -> NonFinite
        assert!(matches!(expr.eval(&env), Err(EvalError::NonFinite { .. })));
        // 非严格模式：允许 inf 传播
        let v = expr.eval_with(&env, EvalMode { strict: false }).unwrap();
        assert!(v.is_infinite());
    }

    #[test]
    fn test_unsupported_special_function() {
        let env = Env::from_pairs(&[("x", 5.0)]);
        // gamma 尚未在求值器中实现 -> Unsupported
        let r = parse_to_expr("(gamma x)").unwrap().eval(&env);
        assert!(matches!(r, Err(EvalError::Unsupported(_))));
    }

    #[test]
    fn test_special_forms() {
        let env = Env::from_pairs(&[("x", -2.0)]);
        // if (gt x 0) x (neg x) = |x| = 2
        assert!((eval_str("(if (gt x 0) x (neg x))", &env).unwrap() - 2.0).abs() < EPS);
        // sum i 1 3 i = 6
        assert!((eval_str("(sum i 1 3 i)", &env).unwrap() - 6.0).abs() < EPS);
        // product i 1 4 i = 24
        assert!((eval_str("(product i 1 4 i)", &env).unwrap() - 24.0).abs() < EPS);
        // max/min
        assert!((eval_str("(max 1 5 3)", &env).unwrap() - 5.0).abs() < EPS);
        assert!((eval_str("(min 1 5 3)", &env).unwrap() - 1.0).abs() < EPS);
    }

    // ---- greenhouse.sexpr 验收：每个算子能求值，且与内联 Rust 参考一致 ----

    #[test]
    fn test_greenhouse_acceptance() {
        // 输入
        let (tmax, tmin, tbase, rh, light, alpha, pmax, rd) =
            (30.0, 10.0, 10.0, 60.0_f64, 800.0, 0.05, 25.0, 1.5);

        // tmean = (Tmax + Tmin)/2
        let env = Env::from_pairs(&[("Tmax", tmax), ("Tmin", tmin)]);
        let tmean = eval_str("(div (add Tmax Tmin) 2)", &env).unwrap();
        assert!((tmean - (tmax + tmin) / 2.0).abs() < EPS);

        // gdd = max(0, Tmean - Tbase)
        let env = Env::from_pairs(&[("Tmean", tmean), ("Tbase", tbase)]);
        let gdd = eval_str("(max 0 (sub Tmean Tbase))", &env).unwrap();
        assert!((gdd - (0.0_f64).max(tmean - tbase)).abs() < EPS);

        // es = 0.6108 * exp(17.27*Tmean/(Tmean+237.3))
        let env = Env::from_pairs(&[("Tmean", tmean)]);
        let es = eval_str(
            "(mul 0.6108 (exp (div (mul 17.27 Tmean) (add Tmean 237.3))))",
            &env,
        )
        .unwrap();
        let es_ref = 0.6108 * (17.27 * tmean / (tmean + 237.3)).exp();
        assert!((es - es_ref).abs() < 1e-9);

        // vpd = es * (1 - RH/100)
        let env = Env::from_pairs(&[("es", es), ("RH", rh)]);
        let vpd = eval_str("(mul es (sub 1 (div RH 100)))", &env).unwrap();
        assert!((vpd - es * (1.0 - rh / 100.0)).abs() < 1e-9);

        // pn = (alpha*I*Pmax)/(alpha*I + Pmax) - Rd
        let env = Env::from_pairs(&[
            ("I", light),
            ("alpha", alpha),
            ("Pmax", pmax),
            ("Rd", rd),
        ]);
        let pn = eval_str(
            "(sub (div (mul (mul alpha I) Pmax) (add (mul alpha I) Pmax)) Rd)",
            &env,
        )
        .unwrap();
        let ai = alpha * light;
        let pn_ref = (ai * pmax) / (ai + pmax) - rd;
        assert!((pn - pn_ref).abs() < 1e-9);
    }
}
