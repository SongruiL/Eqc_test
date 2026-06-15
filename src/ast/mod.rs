//! AST（抽象语法树）模块
//!
//! 定义表达式的 AST 节点类型和遍历接口。
//!
//! ## 模块结构
//!
//! - `expr` - 核心 Expr 枚举定义和方法实现
//! - `visitor` - 表达式遍历器接口
//! - `codegen` - 代码生成模块（Python/Rust/LaTeX）
//! - `constructors` - 表达式构造器（按运算符类型分类）
//! - `parse` - YAML 解析模块

mod expr;
mod visitor;
pub mod codegen;
pub mod constructors;
pub mod parse;

pub use expr::Expr;
pub use visitor::ExprVisitor;
pub use codegen::{ToPython, ToRust, ToLatex};
