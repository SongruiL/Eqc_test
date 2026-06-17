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
        assert_eq!(e.eval(&Env::new()).unwrap(), -1.0);
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
        assert!((e.eval(&Env::new()).unwrap() - 2.0).abs() < 1e-12);
        let rust = e.to_rust();
        assert!(rust.contains(".floor()"), "rust = {rust}");
        assert!(!rust.contains("rem_euclid"), "rust 不应再用 rem_euclid: {rust}");
        // Python 的 np.mod 本身即 floored
        assert!(e.to_python("").contains("np.mod"));
    }
}
