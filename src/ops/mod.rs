//! 算子注册表：每个算子在此**定义一次**——语义（求值函数）+ 三种代码生成模板。
//!
//! 求值器（`crate::eval`）和代码生成器（`Expr::to_rust/to_python/to_latex`）都从这里
//! 派生，实现「算子单一真相源」，消除同一算子散落在多处、彼此漂移的问题。
//! 见 `docs/spec-operator-registry-and-evaluator.md`（Phase 2）。
//!
//! 迁移分期进行：本批先纳入核心算子（算术 + sign/mod + greenhouse 用到的超越/三角）。
//! 未纳入的算子，求值仍走 `eval` 内的旧机制、代码生成仍走 `expr.rs` 的大 `match`。

use crate::ast::Expr;
use std::sync::OnceLock;

/// 代码生成模板：给定已生成好的各参数代码串，产出本算子的代码串。
pub type CodeFn = fn(&[String]) -> String;

/// 一个算子的完整定义（语义 + 三种目标代码生成）。
pub struct OperatorSpec {
    /// 规范算子名。
    pub name: &'static str,
    /// 参数个数。
    pub arity: usize,
    /// 数值语义（单一真相源）。
    pub eval: fn(&[f64]) -> f64,
    /// 生成 Rust 代码。
    pub rust: CodeFn,
    /// 生成 Python 代码。
    pub python: CodeFn,
    /// 生成 LaTeX。
    pub latex: CodeFn,
}

/// 按算子名取规格；未纳入注册表的算子返回 `None`。
pub fn spec(name: &str) -> Option<&'static OperatorSpec> {
    registry().iter().find(|s| s.name == name)
}

/// 把 `Expr` 变体映射为 `(算子名, 子参数引用)`。仅覆盖已纳入注册表的纯函数算子；
/// 其它（叶子、特殊形式、尚未迁移的算子）返回 `None`。
///
/// 这一映射只写一次，被求值器与三个代码生成器共用。
pub fn as_operator(expr: &Expr) -> Option<(&'static str, Vec<&Expr>)> {
    Some(match expr {
        Expr::Add(a, b) => ("add", vec![a, b]),
        Expr::Sub(a, b) => ("sub", vec![a, b]),
        Expr::Mul(a, b) => ("mul", vec![a, b]),
        Expr::Div(a, b) => ("div", vec![a, b]),
        Expr::Neg(a) => ("neg", vec![a]),
        Expr::Abs(a) => ("abs", vec![a]),
        Expr::Pow(a, b) => ("pow", vec![a, b]),
        Expr::Mod(a, b) => ("mod", vec![a, b]),
        Expr::Sign(a) => ("sign", vec![a]),
        Expr::Exp(a) => ("exp", vec![a]),
        Expr::Ln(a) => ("ln", vec![a]),
        Expr::Sqrt(a) => ("sqrt", vec![a]),
        Expr::Sin(a) => ("sin", vec![a]),
        Expr::Cos(a) => ("cos", vec![a]),
        Expr::Ceil(a) => ("ceil", vec![a]),
        Expr::Floor(a) => ("floor", vec![a]),
        Expr::Round(a) => ("round", vec![a]),
        Expr::Trunc(a) => ("trunc", vec![a]),
        Expr::Log10(a) => ("log10", vec![a]),
        Expr::Log2(a) => ("log2", vec![a]),
        Expr::Cbrt(a) => ("cbrt", vec![a]),
        Expr::Tan(a) => ("tan", vec![a]),
        Expr::Sinh(a) => ("sinh", vec![a]),
        Expr::Cosh(a) => ("cosh", vec![a]),
        Expr::Tanh(a) => ("tanh", vec![a]),
        Expr::ASin(a) => ("asin", vec![a]),
        Expr::ACos(a) => ("acos", vec![a]),
        Expr::ATan(a) => ("atan", vec![a]),
        Expr::ATan2(a, b) => ("atan2", vec![a, b]),
        Expr::Sec(a) => ("sec", vec![a]),
        Expr::Csc(a) => ("csc", vec![a]),
        Expr::Cot(a) => ("cot", vec![a]),
        Expr::ASinh(a) => ("asinh", vec![a]),
        Expr::ACosh(a) => ("acosh", vec![a]),
        Expr::ATanh(a) => ("atanh", vec![a]),
        Expr::Sech(a) => ("sech", vec![a]),
        Expr::Csch(a) => ("csch", vec![a]),
        Expr::Coth(a) => ("coth", vec![a]),
        Expr::Hypot(a, b) => ("hypot", vec![a, b]),
        Expr::Copysign(a, b) => ("copysign", vec![a, b]),
        Expr::Clamp(a, b, c) => ("clamp", vec![a, b, c]),
        Expr::Fma(a, b, c) => ("fma", vec![a, b, c]),
        Expr::Hypot3(a, b, c) => ("hypot3", vec![a, b, c]),
        Expr::Eq(a, b) => ("eq", vec![a, b]),
        Expr::Lt(a, b) => ("lt", vec![a, b]),
        Expr::Gt(a, b) => ("gt", vec![a, b]),
        Expr::Leq(a, b) => ("leq", vec![a, b]),
        Expr::Geq(a, b) => ("geq", vec![a, b]),
        Expr::Neq(a, b) => ("neq", vec![a, b]),
        Expr::And(a, b) => ("and", vec![a, b]),
        Expr::Or(a, b) => ("or", vec![a, b]),
        Expr::Not(a) => ("not", vec![a]),
        _ => return None,
    })
}

/// 注册表本体（首次访问时构建一次）。
fn registry() -> &'static [OperatorSpec] {
    static R: OnceLock<Vec<OperatorSpec>> = OnceLock::new();
    R.get_or_init(|| {
        vec![
            // === 算术 ===
            OperatorSpec {
                name: "add",
                arity: 2,
                eval: |a| a[0] + a[1],
                rust: |a| format!("({} + {})", a[0], a[1]),
                python: |a| format!("({} + {})", a[0], a[1]),
                latex: |a| format!("{} + {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "sub",
                arity: 2,
                eval: |a| a[0] - a[1],
                rust: |a| format!("({} - {})", a[0], a[1]),
                python: |a| format!("({} - {})", a[0], a[1]),
                latex: |a| format!("{} - {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "mul",
                arity: 2,
                eval: |a| a[0] * a[1],
                rust: |a| format!("({} * {})", a[0], a[1]),
                python: |a| format!("({} * {})", a[0], a[1]),
                latex: |a| format!("{} \\times {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "div",
                arity: 2,
                eval: |a| a[0] / a[1],
                rust: |a| format!("({} / {})", a[0], a[1]),
                python: |a| format!("({} / {})", a[0], a[1]),
                latex: |a| format!("\\frac{{{}}}{{{}}}", a[0], a[1]),
            },
            OperatorSpec {
                name: "neg",
                arity: 1,
                eval: |a| -a[0],
                rust: |a| format!("(-{})", a[0]),
                python: |a| format!("(-{})", a[0]),
                latex: |a| format!("-{}", a[0]),
            },
            OperatorSpec {
                name: "abs",
                arity: 1,
                eval: |a| a[0].abs(),
                rust: |a| format!("{}.abs()", a[0]),
                python: |a| format!("np.abs({})", a[0]),
                latex: |a| format!("|{}|", a[0]),
            },
            OperatorSpec {
                name: "pow",
                arity: 2,
                eval: |a| a[0].powf(a[1]),
                rust: |a| format!("{}.powf({})", a[0], a[1]),
                python: |a| format!("({} ** {})", a[0], a[1]),
                latex: |a| format!("{}^{{{}}}", a[0], a[1]),
            },
            // 数学取模（floored，结果符号随除数）。
            // Rust 无内置 floored 取模，生成块表达式避免重复求值参数；
            // Python 的 np.mod 本身即 floored；LaTeX 仅展示。
            OperatorSpec {
                name: "mod",
                arity: 2,
                eval: |a| a[0] - a[1] * (a[0] / a[1]).floor(),
                rust: |a| format!(
                    "{{ let (a_, b_) = ({}, {}); a_ - b_ * (a_ / b_).floor() }}",
                    a[0], a[1]
                ),
                python: |a| format!("np.mod({}, {})", a[0], a[1]),
                latex: |a| format!("{} \\mod {}", a[0], a[1]),
            },
            // 数学符号函数 sgn(0)=0。Rust 无内置（signum 在 0 处返回 ±1），生成块表达式；
            // Python 的 np.sign(0)=0 已正确。
            OperatorSpec {
                name: "sign",
                arity: 1,
                eval: |a| {
                    let x = a[0];
                    if x > 0.0 {
                        1.0
                    } else if x < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                },
                rust: |a| format!(
                    "{{ let x_ = {}; if x_ > 0.0 {{ 1.0 }} else if x_ < 0.0 {{ -1.0 }} else {{ 0.0 }} }}",
                    a[0]
                ),
                python: |a| format!("np.sign({})", a[0]),
                latex: |a| format!("\\text{{sgn}}({})", a[0]),
            },
            // === 超越 / 三角（greenhouse 用到）===
            OperatorSpec {
                name: "exp",
                arity: 1,
                eval: |a| a[0].exp(),
                rust: |a| format!("{}.exp()", a[0]),
                python: |a| format!("np.exp({})", a[0]),
                latex: |a| format!("e^{{{}}}", a[0]),
            },
            OperatorSpec {
                name: "ln",
                arity: 1,
                eval: |a| a[0].ln(),
                rust: |a| format!("{}.ln()", a[0]),
                python: |a| format!("np.log({})", a[0]),
                latex: |a| format!("\\ln({})", a[0]),
            },
            OperatorSpec {
                name: "sqrt",
                arity: 1,
                eval: |a| a[0].sqrt(),
                rust: |a| format!("{}.sqrt()", a[0]),
                python: |a| format!("np.sqrt({})", a[0]),
                latex: |a| format!("\\sqrt{{{}}}", a[0]),
            },
            OperatorSpec {
                name: "sin",
                arity: 1,
                eval: |a| a[0].sin(),
                rust: |a| format!("{}.sin()", a[0]),
                python: |a| format!("np.sin({})", a[0]),
                latex: |a| format!("\\sin({})", a[0]),
            },
            OperatorSpec {
                name: "cos",
                arity: 1,
                eval: |a| a[0].cos(),
                rust: |a| format!("{}.cos()", a[0]),
                python: |a| format!("np.cos({})", a[0]),
                latex: |a| format!("\\cos({})", a[0]),
            },
            // === 取整 ===
            OperatorSpec {
                name: "ceil",
                arity: 1,
                eval: |a| a[0].ceil(),
                rust: |a| format!("{}.ceil()", a[0]),
                python: |a| format!("np.ceil({})", a[0]),
                latex: |a| format!("\\lceil {} \\rceil", a[0]),
            },
            OperatorSpec {
                name: "floor",
                arity: 1,
                eval: |a| a[0].floor(),
                rust: |a| format!("{}.floor()", a[0]),
                python: |a| format!("np.floor({})", a[0]),
                latex: |a| format!("\\lfloor {} \\rfloor", a[0]),
            },
            OperatorSpec {
                name: "round",
                arity: 1,
                eval: |a| a[0].round(),
                rust: |a| format!("{}.round()", a[0]),
                python: |a| format!("np.round({})", a[0]),
                latex: |a| format!("\\text{{round}}({})", a[0]),
            },
            OperatorSpec {
                name: "trunc",
                arity: 1,
                eval: |a| a[0].trunc(),
                rust: |a| format!("{}.trunc()", a[0]),
                python: |a| format!("np.trunc({})", a[0]),
                latex: |a| format!("\\text{{trunc}}({})", a[0]),
            },
            // === 对数 / 立方根 ===
            OperatorSpec {
                name: "log10",
                arity: 1,
                eval: |a| a[0].log10(),
                rust: |a| format!("{}.log10()", a[0]),
                python: |a| format!("np.log10({})", a[0]),
                latex: |a| format!("\\log_{{10}}({})", a[0]),
            },
            OperatorSpec {
                name: "log2",
                arity: 1,
                eval: |a| a[0].log2(),
                rust: |a| format!("{}.log2()", a[0]),
                python: |a| format!("np.log2({})", a[0]),
                latex: |a| format!("\\log_{{2}}({})", a[0]),
            },
            OperatorSpec {
                name: "cbrt",
                arity: 1,
                eval: |a| a[0].cbrt(),
                rust: |a| format!("{}.cbrt()", a[0]),
                python: |a| format!("np.cbrt({})", a[0]),
                latex: |a| format!("\\sqrt[3]{{{}}}", a[0]),
            },
            // === 三角 / 双曲（补充）===
            OperatorSpec {
                name: "tan",
                arity: 1,
                eval: |a| a[0].tan(),
                rust: |a| format!("{}.tan()", a[0]),
                python: |a| format!("np.tan({})", a[0]),
                latex: |a| format!("\\tan({})", a[0]),
            },
            OperatorSpec {
                name: "sinh",
                arity: 1,
                eval: |a| a[0].sinh(),
                rust: |a| format!("{}.sinh()", a[0]),
                python: |a| format!("np.sinh({})", a[0]),
                latex: |a| format!("\\sinh({})", a[0]),
            },
            OperatorSpec {
                name: "cosh",
                arity: 1,
                eval: |a| a[0].cosh(),
                rust: |a| format!("{}.cosh()", a[0]),
                python: |a| format!("np.cosh({})", a[0]),
                latex: |a| format!("\\cosh({})", a[0]),
            },
            OperatorSpec {
                name: "tanh",
                arity: 1,
                eval: |a| a[0].tanh(),
                rust: |a| format!("{}.tanh()", a[0]),
                python: |a| format!("np.tanh({})", a[0]),
                latex: |a| format!("\\tanh({})", a[0]),
            },
            // === 反三角 ===
            OperatorSpec {
                name: "asin",
                arity: 1,
                eval: |a| a[0].asin(),
                rust: |a| format!("{}.asin()", a[0]),
                python: |a| format!("np.arcsin({})", a[0]),
                latex: |a| format!("\\arcsin({})", a[0]),
            },
            OperatorSpec {
                name: "acos",
                arity: 1,
                eval: |a| a[0].acos(),
                rust: |a| format!("{}.acos()", a[0]),
                python: |a| format!("np.arccos({})", a[0]),
                latex: |a| format!("\\arccos({})", a[0]),
            },
            OperatorSpec {
                name: "atan",
                arity: 1,
                eval: |a| a[0].atan(),
                rust: |a| format!("{}.atan()", a[0]),
                python: |a| format!("np.arctan({})", a[0]),
                latex: |a| format!("\\arctan({})", a[0]),
            },
            OperatorSpec {
                name: "atan2",
                arity: 2,
                eval: |a| a[0].atan2(a[1]),
                rust: |a| format!("{}.atan2({})", a[0], a[1]),
                python: |a| format!("np.arctan2({}, {})", a[0], a[1]),
                latex: |a| format!("\\text{{atan2}}({}, {})", a[0], a[1]),
            },
            // === 倒数三角 ===
            OperatorSpec {
                name: "sec",
                arity: 1,
                eval: |a| 1.0 / a[0].cos(),
                rust: |a| format!("(1.0 / ({}).cos())", a[0]),
                python: |a| format!("(1 / np.cos({}))", a[0]),
                latex: |a| format!("\\sec({})", a[0]),
            },
            OperatorSpec {
                name: "csc",
                arity: 1,
                eval: |a| 1.0 / a[0].sin(),
                rust: |a| format!("(1.0 / ({}).sin())", a[0]),
                python: |a| format!("(1 / np.sin({}))", a[0]),
                latex: |a| format!("\\csc({})", a[0]),
            },
            OperatorSpec {
                name: "cot",
                arity: 1,
                eval: |a| 1.0 / a[0].tan(),
                rust: |a| format!("(1.0 / ({}).tan())", a[0]),
                python: |a| format!("(1 / np.tan({}))", a[0]),
                latex: |a| format!("\\cot({})", a[0]),
            },
            // === 反双曲 ===
            OperatorSpec {
                name: "asinh",
                arity: 1,
                eval: |a| a[0].asinh(),
                rust: |a| format!("{}.asinh()", a[0]),
                python: |a| format!("np.arcsinh({})", a[0]),
                latex: |a| format!("\\text{{asinh}}({})", a[0]),
            },
            OperatorSpec {
                name: "acosh",
                arity: 1,
                eval: |a| a[0].acosh(),
                rust: |a| format!("{}.acosh()", a[0]),
                python: |a| format!("np.arccosh({})", a[0]),
                latex: |a| format!("\\text{{acosh}}({})", a[0]),
            },
            OperatorSpec {
                name: "atanh",
                arity: 1,
                eval: |a| a[0].atanh(),
                rust: |a| format!("{}.atanh()", a[0]),
                python: |a| format!("np.arctanh({})", a[0]),
                latex: |a| format!("\\text{{atanh}}({})", a[0]),
            },
            // === 倒数双曲 ===
            OperatorSpec {
                name: "sech",
                arity: 1,
                eval: |a| 1.0 / a[0].cosh(),
                rust: |a| format!("(1.0 / ({}).cosh())", a[0]),
                python: |a| format!("(1 / np.cosh({}))", a[0]),
                latex: |a| format!("\\text{{sech}}({})", a[0]),
            },
            OperatorSpec {
                name: "csch",
                arity: 1,
                eval: |a| 1.0 / a[0].sinh(),
                rust: |a| format!("(1.0 / ({}).sinh())", a[0]),
                python: |a| format!("(1 / np.sinh({}))", a[0]),
                latex: |a| format!("\\text{{csch}}({})", a[0]),
            },
            OperatorSpec {
                name: "coth",
                arity: 1,
                eval: |a| 1.0 / a[0].tanh(),
                rust: |a| format!("(1.0 / ({}).tanh())", a[0]),
                python: |a| format!("(1 / np.tanh({}))", a[0]),
                latex: |a| format!("\\coth({})", a[0]),
            },
            // === 多参数数值 ===
            OperatorSpec {
                name: "hypot",
                arity: 2,
                eval: |a| a[0].hypot(a[1]),
                rust: |a| format!("({}).hypot({})", a[0], a[1]),
                python: |a| format!("np.hypot({}, {})", a[0], a[1]),
                latex: |a| format!("\\sqrt{{{}^2 + {}^2}}", a[0], a[1]),
            },
            OperatorSpec {
                name: "copysign",
                arity: 2,
                eval: |a| a[0].copysign(a[1]),
                rust: |a| format!("({}).copysign({})", a[0], a[1]),
                python: |a| format!("np.copysign({}, {})", a[0], a[1]),
                latex: |a| format!("\\text{{copysign}}({}, {})", a[0], a[1]),
            },
            OperatorSpec {
                name: "clamp",
                arity: 3,
                eval: |a| a[0].max(a[1]).min(a[2]),
                rust: |a| format!("({}).clamp({}, {})", a[0], a[1], a[2]),
                python: |a| format!("np.clip({}, {}, {})", a[0], a[1], a[2]),
                latex: |a| format!("\\text{{clamp}}({}, {}, {})", a[0], a[1], a[2]),
            },
            OperatorSpec {
                name: "fma",
                arity: 3,
                eval: |a| a[0].mul_add(a[1], a[2]),
                rust: |a| format!("({}).mul_add({}, {})", a[0], a[1], a[2]),
                python: |a| format!("({} * {} + {})", a[0], a[1], a[2]),
                latex: |a| format!("{} \\cdot {} + {}", a[0], a[1], a[2]),
            },
            OperatorSpec {
                name: "hypot3",
                arity: 3,
                eval: |a| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt(),
                rust: |a| format!(
                    "(({}).powi(2) + ({}).powi(2) + ({}).powi(2)).sqrt()",
                    a[0], a[1], a[2]
                ),
                python: |a| format!("np.sqrt({}**2 + {}**2 + {}**2)", a[0], a[1], a[2]),
                latex: |a| format!("\\sqrt{{{}^2 + {}^2 + {}^2}}", a[0], a[1], a[2]),
            },
            // === 关系运算（求值返回 1.0 / 0.0；代码生成产出布尔表达式）===
            OperatorSpec {
                name: "eq",
                arity: 2,
                eval: |a| (a[0] == a[1]) as u8 as f64,
                rust: |a| format!("({} == {})", a[0], a[1]),
                python: |a| format!("({} == {})", a[0], a[1]),
                latex: |a| format!("{} = {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "lt",
                arity: 2,
                eval: |a| (a[0] < a[1]) as u8 as f64,
                rust: |a| format!("({} < {})", a[0], a[1]),
                python: |a| format!("({} < {})", a[0], a[1]),
                latex: |a| format!("{} < {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "gt",
                arity: 2,
                eval: |a| (a[0] > a[1]) as u8 as f64,
                rust: |a| format!("({} > {})", a[0], a[1]),
                python: |a| format!("({} > {})", a[0], a[1]),
                latex: |a| format!("{} > {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "leq",
                arity: 2,
                eval: |a| (a[0] <= a[1]) as u8 as f64,
                rust: |a| format!("({} <= {})", a[0], a[1]),
                python: |a| format!("({} <= {})", a[0], a[1]),
                latex: |a| format!("{} \\leq {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "geq",
                arity: 2,
                eval: |a| (a[0] >= a[1]) as u8 as f64,
                rust: |a| format!("({} >= {})", a[0], a[1]),
                python: |a| format!("({} >= {})", a[0], a[1]),
                latex: |a| format!("{} \\geq {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "neq",
                arity: 2,
                eval: |a| (a[0] != a[1]) as u8 as f64,
                rust: |a| format!("({} != {})", a[0], a[1]),
                python: |a| format!("({} != {})", a[0], a[1]),
                latex: |a| format!("{} \\neq {}", a[0], a[1]),
            },
            // === 逻辑运算（非零视为真）===
            OperatorSpec {
                name: "and",
                arity: 2,
                eval: |a| ((a[0] != 0.0) && (a[1] != 0.0)) as u8 as f64,
                rust: |a| format!("({} && {})", a[0], a[1]),
                python: |a| format!("({} and {})", a[0], a[1]),
                latex: |a| format!("{} \\land {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "or",
                arity: 2,
                eval: |a| ((a[0] != 0.0) || (a[1] != 0.0)) as u8 as f64,
                rust: |a| format!("({} || {})", a[0], a[1]),
                python: |a| format!("({} or {})", a[0], a[1]),
                latex: |a| format!("{} \\lor {}", a[0], a[1]),
            },
            OperatorSpec {
                name: "not",
                arity: 1,
                eval: |a| (a[0] == 0.0) as u8 as f64,
                rust: |a| format!("(!{})", a[0]),
                python: |a| format!("(not {})", a[0]),
                latex: |a| format!("\\neg {}", a[0]),
            },
        ]
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::Expr;
    use crate::Env;

    // 已迁移算子：代码生成输出必须与迁移前一致（注册表模板照搬原模板）。
    #[test]
    fn test_migrated_op_codegen_unchanged() {
        let e = Expr::add(Expr::Const(2.0), Expr::Const(3.0));
        assert_eq!(e.to_rust(), "(2_f64 + 3_f64)");
        assert_eq!(e.to_python(""), "(2 + 3)");
        assert_eq!(e.to_latex(), "2 + 3");
    }

    // sign：求值与代码生成现在一致（都用 sgn(0)=0）。Rust 不再是 signum。
    #[test]
    fn test_sign_eval_and_codegen_consistent() {
        let e = Expr::sign(Expr::Const(-2.0));
        assert_eq!(e.eval_scalar(&Env::new()).unwrap(), -1.0);
        let rust = e.to_rust();
        assert!(rust.contains("if x_ > 0.0"), "rust = {rust}");
        assert!(!rust.contains("signum"), "rust 不应再用 signum: {rust}");
        // Python 一直是 np.sign（0 处=0，本就正确）
        assert!(e.to_python("").contains("np.sign"));
    }

    // mod：求值与代码生成现在一致（数学 floored 取模）。Rust 不再是 rem_euclid。
    #[test]
    fn test_mod_eval_and_codegen_consistent() {
        let e = Expr::modulo(Expr::Const(-7.0), Expr::Const(3.0));
        assert!((e.eval_scalar(&Env::new()).unwrap() - 2.0).abs() < 1e-12);
        let rust = e.to_rust();
        assert!(rust.contains(".floor()"), "rust = {rust}");
        assert!(!rust.contains("rem_euclid"), "rust 不应再用 rem_euclid: {rust}");
        // Python 的 np.mod 本身即 floored
        assert!(e.to_python("").contains("np.mod"));
    }
}
