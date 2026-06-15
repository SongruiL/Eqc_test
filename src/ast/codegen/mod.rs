//! 代码生成模块
//!
//! 提供 Expr 表达式到不同目标语言的代码生成功能。
//!
//! ## 支持的目标语言
//!
//! - **Python**: 使用 NumPy 和 SciPy 库
//! - **Rust**: 使用 std、puruspe、GSL 等库
//! - **LaTeX**: 数学公式排版

mod python;
mod rust;
mod latex;

pub use python::ToPython;
pub use rust::ToRust;
pub use latex::ToLatex;
