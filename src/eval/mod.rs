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

#[cfg(feature = "advanced_math")]
use statrs::distribution::{Continuous, ContinuousCDF};

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
    /// 算子尚未在求值器中实现（如特殊函数、矩阵运算、积分等）
    Unsupported(String),
    /// 结果非有限（NaN/Inf）——严格模式下产生。涵盖除零、`ln` 负数、定义域外等。
    NonFinite { op: String },
    /// 期望标量却得到向量/矩阵（如把向量当 if 条件、求和上限）。
    NotScalar,
    /// 形状不匹配（广播失败：非标量参数形状不一致）。
    ShapeMismatch { op: String },
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
            EvalError::NotScalar => write!(f, "期望标量，却得到向量/矩阵"),
            EvalError::ShapeMismatch { op } => write!(f, "算子 {op} 形状不匹配（广播失败）"),
        }
    }
}

impl std::error::Error for EvalError {}

/// 求值结果的值：标量 / 向量（1D）/ 矩阵（2D，行主序）。
///
/// 标量行为与从前完全一致（[`Value::Scalar`]）。向量/矩阵让 cohort（同期群）等
/// 「按索引的量」能作为一等值参与运算。详见 `docs/spec-vector-matrix.md`。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(f64),
    Vector(Vec<f64>),
    Matrix { rows: usize, cols: usize, data: Vec<f64> },
}

/// 值的形状（广播判定用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Scalar,
    Vec(usize),
    Mat(usize, usize),
}

impl Value {
    /// 取标量；非标量报 [`EvalError::NotScalar`]。
    pub fn as_scalar(&self) -> Result<f64, EvalError> {
        match self {
            Value::Scalar(x) => Ok(*x),
            _ => Err(EvalError::NotScalar),
        }
    }

    /// 取向量切片（标量视为长度 1；矩阵报错另议——本期仅 Vector 调用）。
    pub fn as_vector(&self) -> Option<&[f64]> {
        match self {
            Value::Vector(v) => Some(v),
            _ => None,
        }
    }

    /// 形状。
    pub fn shape(&self) -> Shape {
        match self {
            Value::Scalar(_) => Shape::Scalar,
            Value::Vector(v) => Shape::Vec(v.len()),
            Value::Matrix { rows, cols, .. } => Shape::Mat(*rows, *cols),
        }
    }

    /// 线性索引取元素（标量广播：忽略 i 返回自身值）。
    fn elem(&self, i: usize) -> f64 {
        match self {
            Value::Scalar(x) => *x,
            Value::Vector(v) => v[i],
            Value::Matrix { data, .. } => data[i],
        }
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Self {
        Value::Scalar(x)
    }
}
impl From<Vec<f64>> for Value {
    fn from(v: Vec<f64>) -> Self {
        Value::Vector(v)
    }
}

/// 求值环境：变量/参数名 -> [`Value`]，外加用于 `sum`/`product` 绑定循环变量的作用域栈。
#[derive(Debug, Clone, Default)]
pub struct Env {
    vars: HashMap<String, Value>,
    /// 内层作用域绑定（如 sum 的循环变量，恒为标量），查找时优先于 `vars`。
    scopes: Vec<(String, Value)>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }

    /// 链式设置一个变量值（标量传 f64、向量传 Vec<f64> 均可）。
    pub fn with(mut self, name: impl Into<String>, val: impl Into<Value>) -> Self {
        self.vars.insert(name.into(), val.into());
        self
    }

    /// 设置/覆盖一个变量值。
    pub fn set(&mut self, name: impl Into<String>, val: impl Into<Value>) -> &mut Self {
        self.vars.insert(name.into(), val.into());
        self
    }

    /// 从 (名称, 标量值) 列表构造环境。
    pub fn from_pairs(pairs: &[(&str, f64)]) -> Self {
        let mut env = Self::new();
        for (k, v) in pairs {
            env.vars.insert((*k).to_string(), Value::Scalar(*v));
        }
        env
    }

    /// 查找变量值：先查内层作用域（后进先出），再查全局变量。
    pub fn get(&self, name: &str) -> Option<Value> {
        for (n, v) in self.scopes.iter().rev() {
            if n == name {
                return Some(v.clone());
            }
        }
        self.vars.get(name).cloned()
    }

    /// 查找并要求标量（向量/矩阵返回 None）。
    pub fn get_scalar(&self, name: &str) -> Option<f64> {
        self.get(name).and_then(|v| v.as_scalar().ok())
    }

    fn push_scope(&mut self, name: String, val: f64) {
        self.scopes.push((name, Value::Scalar(val)));
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
    /// 以默认（严格）模式对表达式求值，返回 [`Value`]（可能是标量/向量/矩阵）。
    pub fn eval(&self, env: &Env) -> Result<Value, EvalError> {
        self.eval_with(env, EvalMode::default())
    }

    /// 以指定模式对表达式求值。
    pub fn eval_with(&self, env: &Env, mode: EvalMode) -> Result<Value, EvalError> {
        let mut work = env.clone();
        ev(self, &mut work, mode)
    }

    /// 求值并要求结果为标量（垫片：标量调用点用它，语义与从前一致）。
    pub fn eval_scalar(&self, env: &Env) -> Result<f64, EvalError> {
        self.eval(env)?.as_scalar()
    }

    /// 指定模式求值并要求标量。
    pub fn eval_scalar_with(&self, env: &Env, mode: EvalMode) -> Result<f64, EvalError> {
        self.eval_with(env, mode)?.as_scalar()
    }
}

/// 递归求值，返回 [`Value`]（标量/向量/矩阵）。
fn ev(expr: &Expr, env: &mut Env, mode: EvalMode) -> Result<Value, EvalError> {
    // 注册表快路径：纯函数式标量算子 + **广播**——标量算子自动逐元素作用于向量/矩阵。
    if let Some((name, args)) = crate::ops::as_operator(expr) {
        if let Some(s) = crate::ops::spec(name) {
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(ev(a, env, mode)?);
            }
            return broadcast_apply(name, s.eval, &vals, mode);
        }
    }

    match expr {
        // === 叶子 ===
        Expr::Const(c) => Ok(Value::Scalar(*c)),
        Expr::Pi => Ok(Value::Scalar(std::f64::consts::PI)),
        Expr::E => Ok(Value::Scalar(std::f64::consts::E)),
        Expr::Var(n) | Expr::Param(n) => env.get(n).ok_or_else(|| EvalError::UndefinedVar(n.clone())),

        // === 聚合（可变参数）：广播后逐元素折叠 ===
        Expr::Max(xs) => nary("max", xs, f64::NEG_INFINITY, f64::max, env, mode),
        Expr::Min(xs) => nary("min", xs, f64::INFINITY, f64::min, env, mode),

        // === 向量/矩阵字面量（V0：元素须为标量）===
        Expr::VectorLit(elems) => {
            let mut data = Vec::with_capacity(elems.len());
            for e in elems {
                data.push(ev(e, env, mode)?.as_scalar()?);
            }
            Ok(Value::Vector(data))
        }
        Expr::MatrixLit(rows) => {
            let r = rows.len();
            let c = rows.first().map_or(0, |row| row.len());
            let mut data = Vec::with_capacity(r * c);
            for row in rows {
                if row.len() != c {
                    return Err(EvalError::ShapeMismatch { op: "matrix".into() });
                }
                for e in row {
                    data.push(ev(e, env, mode)?.as_scalar()?);
                }
            }
            Ok(Value::Matrix { rows: r, cols: c, data })
        }

        // === 向量算子（V1：AST 已有，此处补求值）===
        Expr::Reduce { kind, arg } => {
            let v = ev(arg, env, mode)?;
            let r = reduce_value(*kind, &v)?;
            Ok(Value::Scalar(chk(kind.name(), r, mode)?))
        }
        Expr::Dot(a, b) => {
            let (da, db) = (ev(a, env, mode)?, ev(b, env, mode)?);
            let (u, v) = (require_vec(&da, "dot")?, require_vec(&db, "dot")?);
            if u.len() != v.len() {
                return Err(EvalError::ShapeMismatch { op: "dot".into() });
            }
            let r: f64 = u.iter().zip(v).map(|(x, y)| x * y).sum();
            Ok(Value::Scalar(chk("dot", r, mode)?))
        }
        Expr::Cross(a, b) => {
            let (da, db) = (ev(a, env, mode)?, ev(b, env, mode)?);
            let (u, v) = (require_vec(&da, "cross")?, require_vec(&db, "cross")?);
            if u.len() != 3 || v.len() != 3 {
                return Err(EvalError::ShapeMismatch { op: "cross".into() });
            }
            let out = vec![
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            for &x in &out {
                chk("cross", x, mode)?;
            }
            Ok(Value::Vector(out))
        }
        Expr::VecNorm(a) => {
            let da = ev(a, env, mode)?;
            let u = require_vec(&da, "norm")?;
            let r = u.iter().map(|x| x * x).sum::<f64>().sqrt();
            Ok(Value::Scalar(chk("norm", r, mode)?))
        }
        Expr::VecNormalize(a) => {
            let da = ev(a, env, mode)?;
            let u = require_vec(&da, "normalize")?;
            let n = u.iter().map(|x| x * x).sum::<f64>().sqrt();
            let mut out = Vec::with_capacity(u.len());
            for &x in u {
                out.push(chk("normalize", x / n, mode)?);
            }
            Ok(Value::Vector(out))
        }

        // === 特殊形式（条件 / 上下限须为标量）===
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            if ev(cond, env, mode)?.as_scalar()? != 0.0 {
                ev(then_branch, env, mode)
            } else {
                ev(else_branch, env, mode)
            }
        }
        Expr::Piecewise { pieces, otherwise } => {
            for (cond, val) in pieces {
                if ev(cond, env, mode)?.as_scalar()? != 0.0 {
                    return ev(val, env, mode);
                }
            }
            ev(otherwise, env, mode)
        }
        Expr::Sum { index, lower, upper, body } => {
            let lo = ev(lower, env, mode)?.as_scalar()?.round() as i64;
            let hi = ev(upper, env, mode)?.as_scalar()?.round() as i64;
            let mut acc = 0.0_f64;
            for i in lo..=hi {
                env.push_scope(index.clone(), i as f64);
                let r = ev(body, env, mode).and_then(|v| v.as_scalar());
                env.pop_scope();
                acc += r?;
            }
            Ok(Value::Scalar(chk("sum", acc, mode)?))
        }
        Expr::Product { index, lower, upper, body } => {
            let lo = ev(lower, env, mode)?.as_scalar()?.round() as i64;
            let hi = ev(upper, env, mode)?.as_scalar()?.round() as i64;
            let mut acc = 1.0_f64;
            for i in lo..=hi {
                env.push_scope(index.clone(), i as f64);
                let r = ev(body, env, mode).and_then(|v| v.as_scalar());
                env.pop_scope();
                acc *= r?;
            }
            Ok(Value::Scalar(chk("product", acc, mode)?))
        }

        // === 特殊函数（标量；部分需 advanced_math）；其余 -> Unsupported ===
        other => match eval_special(other, env, mode) {
            Some(r) => r.map(Value::Scalar),
            None => Err(EvalError::Unsupported(variant_tag(other))),
        },
    }
}

/// 特殊函数求值。返回 `None` 表示非本函数处理的算子（交由调用方报 Unsupported）；
/// `Some(Ok/Err)` 表示已处理。部分函数需 `advanced_math` 特性（statrs/puruspe）。
fn eval_special(expr: &Expr, env: &mut Env, mode: EvalMode) -> Option<Result<f64, EvalError>> {
    // 特殊函数均为标量函数：先把参数求值并要求标量。
    macro_rules! e1 {
        ($name:expr, $a:expr, $f:expr) => {
            match ev($a, env, mode).and_then(|v| v.as_scalar()) {
                Ok(x) => Some(chk($name, ($f)(x), mode)),
                Err(e) => Some(Err(e)),
            }
        };
    }
    #[cfg(feature = "advanced_math")]
    macro_rules! e2 {
        ($name:expr, $a:expr, $b:expr, $f:expr) => {
            match (
                ev($a, env, mode).and_then(|v| v.as_scalar()),
                ev($b, env, mode).and_then(|v| v.as_scalar()),
            ) {
                (Ok(x), Ok(y)) => Some(chk($name, ($f)(x, y), mode)),
                (Err(e), _) | (_, Err(e)) => Some(Err(e)),
            }
        };
    }
    #[cfg(feature = "advanced_math")]
    macro_rules! e3 {
        ($name:expr, $a:expr, $b:expr, $c:expr, $f:expr) => {
            match (
                ev($a, env, mode).and_then(|v| v.as_scalar()),
                ev($b, env, mode).and_then(|v| v.as_scalar()),
                ev($c, env, mode).and_then(|v| v.as_scalar()),
            ) {
                (Ok(x), Ok(y), Ok(z)) => Some(chk($name, ($f)(x, y, z), mode)),
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Some(Err(e)),
            }
        };
    }

    match expr {
        // === 纯函数（无需 advanced_math）===
        Expr::Factorial(n) => e1!("factorial", n, |x: f64| {
            if x < 0.0 {
                f64::NAN
            } else {
                (1..=(x as u64)).product::<u64>() as f64
            }
        }),
        Expr::Logit(x) => e1!("logit", x, |v: f64| v.ln() - (1.0 - v).ln()),
        Expr::Expit(x) => e1!("expit", x, |v: f64| 1.0 / (1.0 + (-v).exp())),

        // === 需要 advanced_math 的特殊函数（与 to_rust 的库调用一致）===
        #[cfg(feature = "advanced_math")]
        Expr::Gamma(x) => e1!("gamma", x, |v: f64| puruspe::gamma(v)),
        #[cfg(feature = "advanced_math")]
        Expr::Lgamma(x) => e1!("lgamma", x, |v: f64| statrs::function::gamma::ln_gamma(v)),
        #[cfg(feature = "advanced_math")]
        Expr::Digamma(x) => e1!("digamma", x, |v: f64| statrs::function::gamma::digamma(v)),
        #[cfg(feature = "advanced_math")]
        Expr::Beta(a, b) => e2!("beta", a, b, |x: f64, y: f64| puruspe::beta(x, y)),
        #[cfg(feature = "advanced_math")]
        Expr::Lbeta(a, b) => e2!("lbeta", a, b, |x: f64, y: f64| statrs::function::beta::ln_beta(x, y)),
        #[cfg(feature = "advanced_math")]
        Expr::Erf(x) => e1!("erf", x, |v: f64| puruspe::erf(v)),
        #[cfg(feature = "advanced_math")]
        Expr::Erfc(x) => e1!("erfc", x, |v: f64| puruspe::erfc(v)),
        #[cfg(feature = "advanced_math")]
        Expr::Erfinv(x) => e1!("erfinv", x, |v: f64| statrs::function::erf::erf_inv(v)),
        #[cfg(feature = "advanced_math")]
        Expr::NormPdf(x, mu, sig) => e3!("norm_pdf", x, mu, sig, |xx: f64, m: f64, s: f64| {
            statrs::distribution::Normal::new(m, s).map(|d| d.pdf(xx)).unwrap_or(f64::NAN)
        }),
        #[cfg(feature = "advanced_math")]
        Expr::NormCdf(x, mu, sig) => e3!("norm_cdf", x, mu, sig, |xx: f64, m: f64, s: f64| {
            statrs::distribution::Normal::new(m, s).map(|d| d.cdf(xx)).unwrap_or(f64::NAN)
        }),
        #[cfg(feature = "advanced_math")]
        Expr::NormPpf(p, mu, sig) => e3!("norm_ppf", p, mu, sig, |pp: f64, m: f64, s: f64| {
            statrs::distribution::Normal::new(m, s).map(|d| d.inverse_cdf(pp)).unwrap_or(f64::NAN)
        }),

        _ => None,
    }
}

/// 形状广播判定：标量可广播到任意形状；非标量须同形状，否则 [`EvalError::ShapeMismatch`]。
fn broadcast_shape(name: &str, args: &[Value]) -> Result<Shape, EvalError> {
    let mut shape = Shape::Scalar;
    for v in args {
        let s = v.shape();
        if s == Shape::Scalar {
            continue;
        }
        if shape == Shape::Scalar {
            shape = s;
        } else if shape != s {
            return Err(EvalError::ShapeMismatch { op: name.to_string() });
        }
    }
    Ok(shape)
}

/// 把标量算子 `f` 逐元素作用于（广播后的）各参数。标量参数广播；同形状非标量逐位计算。
fn broadcast_apply(
    name: &str,
    f: fn(&[f64]) -> f64,
    args: &[Value],
    mode: EvalMode,
) -> Result<Value, EvalError> {
    match broadcast_shape(name, args)? {
        Shape::Scalar => {
            let xs: Vec<f64> = args.iter().map(|v| v.as_scalar()).collect::<Result<_, _>>()?;
            Ok(Value::Scalar(chk(name, f(&xs), mode)?))
        }
        Shape::Vec(n) => {
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let xs: Vec<f64> = args.iter().map(|v| v.elem(i)).collect();
                out.push(chk(name, f(&xs), mode)?);
            }
            Ok(Value::Vector(out))
        }
        Shape::Mat(r, c) => {
            let mut out = Vec::with_capacity(r * c);
            for i in 0..(r * c) {
                let xs: Vec<f64> = args.iter().map(|v| v.elem(i)).collect();
                out.push(chk(name, f(&xs), mode)?);
            }
            Ok(Value::Matrix { rows: r, cols: c, data: out })
        }
    }
}

/// 可变参数聚合（max/min）：广播后逐元素折叠。
fn nary(
    name: &str,
    xs: &[Expr],
    init: f64,
    fold: fn(f64, f64) -> f64,
    env: &mut Env,
    mode: EvalMode,
) -> Result<Value, EvalError> {
    if xs.is_empty() {
        return Err(EvalError::ArityMismatch { op: name.to_string(), expected: 1, found: 0 });
    }
    let vals: Vec<Value> = xs.iter().map(|x| ev(x, env, mode)).collect::<Result<_, _>>()?;
    match broadcast_shape(name, &vals)? {
        Shape::Scalar => {
            let mut acc = init;
            for v in &vals {
                acc = fold(acc, v.as_scalar()?);
            }
            Ok(Value::Scalar(chk(name, acc, mode)?))
        }
        Shape::Vec(n) => {
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let mut acc = init;
                for v in &vals {
                    acc = fold(acc, v.elem(i));
                }
                out.push(chk(name, acc, mode)?);
            }
            Ok(Value::Vector(out))
        }
        Shape::Mat(r, c) => {
            let mut out = Vec::with_capacity(r * c);
            for i in 0..(r * c) {
                let mut acc = init;
                for v in &vals {
                    acc = fold(acc, v.elem(i));
                }
                out.push(chk(name, acc, mode)?);
            }
            Ok(Value::Matrix { rows: r, cols: c, data: out })
        }
    }
}

/// 要求是向量（取切片）；否则 [`EvalError::ShapeMismatch`]。
fn require_vec<'a>(v: &'a Value, op: &str) -> Result<&'a [f64], EvalError> {
    match v {
        Value::Vector(d) => Ok(d),
        _ => Err(EvalError::ShapeMismatch { op: op.to_string() }),
    }
}

/// 向量归约（标量视为自身；矩阵对全部元素归约）。
fn reduce_value(kind: crate::ast::ReduceKind, v: &Value) -> Result<f64, EvalError> {
    use crate::ast::ReduceKind::*;
    let d: &[f64] = match v {
        Value::Scalar(x) => return Ok(*x),
        Value::Vector(d) => d,
        Value::Matrix { data, .. } => data,
    };
    Ok(match kind {
        Sum => d.iter().sum(),
        Prod => d.iter().product(),
        Mean => {
            if d.is_empty() {
                f64::NAN
            } else {
                d.iter().sum::<f64>() / d.len() as f64
            }
        }
        Min => d.iter().cloned().fold(f64::INFINITY, f64::min),
        Max => d.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    })
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
        parse_to_expr(s).expect("parse").eval_scalar(env)
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
        assert_eq!(Expr::sign(Expr::Const(0.0)).eval_scalar(&env).unwrap(), 0.0);
        assert_eq!(Expr::sign(Expr::Const(-3.0)).eval_scalar(&env).unwrap(), -1.0);
        assert_eq!(Expr::sign(Expr::Const(2.5)).eval_scalar(&env).unwrap(), 1.0);
    }

    #[test]
    fn test_mod_is_mathematical_floored() {
        let env = Env::new();
        // 数学取模：(-7) mod 3 = 2（结果符号随除数），区别于 Rust `%` 的 -1
        let m = Expr::modulo(Expr::Const(-7.0), Expr::Const(3.0));
        assert!((m.eval_scalar(&env).unwrap() - 2.0).abs() < EPS);
    }

    #[test]
    fn test_strict_div_by_zero_errors() {
        let env = Env::new();
        let expr = parse_to_expr("(div 1 0)").unwrap();
        // 严格模式（默认）：除零 -> NonFinite
        assert!(matches!(expr.eval(&env), Err(EvalError::NonFinite { .. })));
        // 非严格模式：允许 inf 传播
        let v = expr.eval_scalar_with(&env, EvalMode { strict: false }).unwrap();
        assert!(v.is_infinite());
    }

    // 未开启 advanced_math 时，gamma 等特殊函数未实现 -> Unsupported
    #[cfg(not(feature = "advanced_math"))]
    #[test]
    fn test_unsupported_special_function() {
        let env = Env::from_pairs(&[("x", 5.0)]);
        let r = parse_to_expr("(gamma x)").unwrap().eval(&env);
        assert!(matches!(r, Err(EvalError::Unsupported(_))));
    }

    // 纯函数特殊算子（无需 advanced_math）
    #[test]
    fn test_pure_special_functions() {
        let env = Env::new();
        // 5! = 120
        assert!((eval_str("(factorial 5)", &env).unwrap() - 120.0).abs() < EPS);
        // expit(0) = 0.5
        assert!((eval_str("(expit 0)", &env).unwrap() - 0.5).abs() < EPS);
        // logit(0.5) = 0
        assert!(eval_str("(logit 0.5)", &env).unwrap().abs() < EPS);
        // logit(expit(x)) = x（往返）
        let v = eval_str("(logit (expit 1.3))", &env).unwrap();
        assert!((v - 1.3).abs() < 1e-9);
    }

    // 需要 advanced_math 的特殊函数
    #[cfg(feature = "advanced_math")]
    #[test]
    fn test_advanced_special_functions() {
        let env = Env::new();
        // Γ(5) = 4! = 24
        assert!((eval_str("(gamma 5)", &env).unwrap() - 24.0).abs() < 1e-6);
        // lgamma(5) = ln(24)
        assert!((eval_str("(lgamma 5)", &env).unwrap() - 24.0_f64.ln()).abs() < 1e-6);
        // erf(0) = 0, erfc(0) = 1
        assert!(eval_str("(erf 0)", &env).unwrap().abs() < 1e-9);
        assert!((eval_str("(erfc 0)", &env).unwrap() - 1.0).abs() < 1e-9);
        // erfinv(erf(0.4)) = 0.4（往返）
        assert!((eval_str("(erfinv (erf 0.4))", &env).unwrap() - 0.4).abs() < 1e-6);
        // 标准正态 CDF(0) = 0.5，PDF(0) = 1/sqrt(2π)
        assert!((eval_str("(norm_cdf 0 0 1)", &env).unwrap() - 0.5).abs() < 1e-9);
        assert!(
            (eval_str("(norm_pdf 0 0 1)", &env).unwrap()
                - 1.0 / (2.0 * std::f64::consts::PI).sqrt())
            .abs()
                < 1e-9
        );
        // PPF(0.5) = 0（中位数）
        assert!(eval_str("(norm_ppf 0.5 0 1)", &env).unwrap().abs() < 1e-9);
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

    // ---- V0：向量值 + 广播 ----

    #[test]
    fn test_vector_broadcast_v0() {
        let env = Env::new();
        let v = Expr::vector_lit(vec![Expr::Const(1.0), Expr::Const(2.0), Expr::Const(3.0)]);
        // 向量字面量
        assert_eq!(v.eval(&env).unwrap(), Value::Vector(vec![1.0, 2.0, 3.0]));
        // 标量广播：2 * [1,2,3] = [2,4,6]
        let scaled = Expr::mul(Expr::Const(2.0), v.clone());
        assert_eq!(scaled.eval(&env).unwrap(), Value::Vector(vec![2.0, 4.0, 6.0]));
        // 同形状逐元素：[1,2,3] + [10,20,30]
        let v2 = Expr::vector_lit(vec![Expr::Const(10.0), Expr::Const(20.0), Expr::Const(30.0)]);
        assert_eq!(
            Expr::add(v.clone(), v2).eval(&env).unwrap(),
            Value::Vector(vec![11.0, 22.0, 33.0])
        );
        // 算子逐元素（注册表算子免费支持）：exp([0,1]) = [1, e]
        let env2 = Env::new().with("x", vec![0.0, 1.0]);
        match parse_to_expr("(exp x)").unwrap().eval(&env2).unwrap() {
            Value::Vector(d) => {
                assert!((d[0] - 1.0).abs() < EPS && (d[1] - std::f64::consts::E).abs() < 1e-9);
            }
            other => panic!("应为向量: {other:?}"),
        }
        // 形状不匹配 -> ShapeMismatch
        let bad = Expr::add(
            Expr::vector_lit(vec![Expr::Const(1.0), Expr::Const(2.0), Expr::Const(3.0)]),
            Expr::vector_lit(vec![Expr::Const(1.0), Expr::Const(2.0)]),
        );
        assert!(matches!(bad.eval(&env), Err(EvalError::ShapeMismatch { .. })));
        // 向量当标量用 -> NotScalar
        assert!(matches!(v.eval(&env).unwrap().as_scalar(), Err(EvalError::NotScalar)));
        // max 逐元素：max([1,5],[3,2]) = [3,5]
        let mx = Expr::Max(vec![
            Expr::vector_lit(vec![Expr::Const(1.0), Expr::Const(5.0)]),
            Expr::vector_lit(vec![Expr::Const(3.0), Expr::Const(2.0)]),
        ]);
        assert_eq!(mx.eval(&env).unwrap(), Value::Vector(vec![3.0, 5.0]));
    }

    // ---- V1：向量算子（归约 / 点积 / 范数 / 归一化 / 叉积）----

    #[test]
    fn test_vector_ops_v1() {
        use crate::ast::ReduceKind;
        let env = Env::new().with("v", vec![3.0, 4.0]).with("u", vec![1.0, 2.0, 3.0]);

        // 归约
        assert_eq!(Expr::reduce(ReduceKind::Sum, Expr::var("u")).eval_scalar(&env).unwrap(), 6.0);
        assert!((Expr::reduce(ReduceKind::Mean, Expr::var("u")).eval_scalar(&env).unwrap() - 2.0).abs() < EPS);
        assert_eq!(Expr::reduce(ReduceKind::Max, Expr::var("u")).eval_scalar(&env).unwrap(), 3.0);
        assert_eq!(Expr::reduce(ReduceKind::Min, Expr::var("u")).eval_scalar(&env).unwrap(), 1.0);
        assert_eq!(Expr::reduce(ReduceKind::Prod, Expr::var("u")).eval_scalar(&env).unwrap(), 6.0);

        // 点积 [3,4]·[3,4]=25；范数 ‖[3,4]‖=5；归一化=[0.6,0.8]
        assert_eq!(
            Expr::Dot(Box::new(Expr::var("v")), Box::new(Expr::var("v"))).eval_scalar(&env).unwrap(),
            25.0
        );
        assert!((Expr::vec_norm(Expr::var("v")).eval_scalar(&env).unwrap() - 5.0).abs() < EPS);
        match Expr::vec_normalize(Expr::var("v")).eval(&env).unwrap() {
            Value::Vector(d) => assert!((d[0] - 0.6).abs() < EPS && (d[1] - 0.8).abs() < EPS),
            o => panic!("应为向量: {o:?}"),
        }

        // 叉积 [1,0,0]×[0,1,0]=[0,0,1]
        let cr = Expr::Cross(
            Box::new(Expr::vector_lit(vec![Expr::Const(1.0), Expr::Const(0.0), Expr::Const(0.0)])),
            Box::new(Expr::vector_lit(vec![Expr::Const(0.0), Expr::Const(1.0), Expr::Const(0.0)])),
        );
        assert_eq!(cr.eval(&env).unwrap(), Value::Vector(vec![0.0, 0.0, 1.0]));

        // 点积长度不一致 -> ShapeMismatch
        assert!(matches!(
            Expr::Dot(Box::new(Expr::var("v")), Box::new(Expr::var("u"))).eval(&env),
            Err(EvalError::ShapeMismatch { .. })
        ));

        // cohort 写法：gs = 0.24 * DRFG（逐元素），GS = Σ gs
        let env2 = Env::new().with("DRFG", vec![10.0, 20.0, 0.0]);
        let gs = Expr::mul(Expr::Const(0.24), Expr::var("DRFG"));
        let gs_total = Expr::reduce(ReduceKind::Sum, gs);
        assert!((gs_total.eval_scalar(&env2).unwrap() - 0.24 * 30.0).abs() < EPS);

        // 经解析（S 表达式）：(vsum (vector 1 2 3)) = 6；(dot (vector 3 4) (vector 3 4)) = 25
        assert_eq!(
            parse_to_expr("(vsum (vector 1 2 3))").unwrap().eval_scalar(&Env::new()).unwrap(),
            6.0
        );
        assert_eq!(
            parse_to_expr("(dot (vector 3 4) (vector 3 4))").unwrap().eval_scalar(&Env::new()).unwrap(),
            25.0
        );
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
