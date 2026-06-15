//! S表达式解析器模块
//!
//! 本模块提供将S表达式（S-Expression）格式的数学公式解析为AST的功能。
//!
//! # 概述
//!
//! S表达式是一种类Lisp的前缀表示法，具有以下优势：
//! - **无歧义**: 完全括号化，无需运算符优先级规则
//! - **易解析**: 语法简单，解析器实现直接
//! - **AI友好**: LLM可以可靠地生成正确的S表达式
//! - **人类可读**: 比YAML更紧凑，比LaTeX更规范
//!
//! # 语法示例
//!
//! ```text
//! ;; 基础运算
//! (add x y)
//! (mul (pow x 2) (sin (div pi 2)))
//!
//! ;; 条件表达式
//! (if (gt x 0) (sqrt x) 0)
//!
//! ;; 求和/连乘
//! (sum i 1 n (pow i 2))
//! (product k 1 10 (add k 1))
//!
//! ;; 分段函数
//! (piecewise
//!   ((lt x 0) (neg x))
//!   ((eq x 0) 0)
//!   :otherwise x)
//! ```
//!
//! # 模块结构
//!
//! - `error` - 错误类型定义
//! - `lexer` - 词法分析器（字符串 -> Token流）
//! - `parser` - 语法分析器（Token流 -> SExpr AST）
//! - `ast` - S表达式AST定义
//! - `converter` - AST转换器（SExpr -> Expr）
//! - `to_yaml` - YAML序列化（Expr -> YAML）
//!
//! # 使用方法
//!
//! ```ignore
//! use equation_compiler::sexpr::{parse, convert, to_yaml};
//!
//! // 解析S表达式
//! let sexpr = parse("(add x (mul y 2))")?;
//!
//! // 转换为Expr AST
//! let expr = convert(&sexpr)?;
//!
//! // 序列化为YAML
//! let yaml = to_yaml(&expr);
//! ```

pub mod error;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod converter;
pub mod to_yaml;
pub mod workflow;
pub mod operator_gen;
pub mod validator;

// 重新导出公共API
pub use error::{SExprError, SExprResult};
pub use lexer::{Lexer, Token, TokenKind};
pub use error::Span;
pub use ast::SExpr;
pub use parser::Parser;
pub use converter::convert;
pub use to_yaml::to_yaml_value;

// Workflow 相关导出
pub use workflow::{
    ModuleDef, OperatorDef, OperatorType, InputDef, OutputDef,
    EdgeDef, BroadcastDef, DistributionDef, MonteCarloConfigDef,
    parse_annotated_sexpr, generate_workflow_json,
};
pub use operator_gen::{
    generate_ast_json, generate_operator_seed_sql, generate_operators, generate_register_code,
    generate_template_sql,
};

// 验证器导出
pub use validator::{
    SExprValidator, ValidationResult, ValidationError, ValidationWarning,
    ErrorType, WarningType, ValidationStats,
    format_validation_result, generate_spec_doc,
};

/// 便捷函数：解析S表达式字符串为SExpr AST
///
/// # 参数
/// - `input`: S表达式字符串
///
/// # 返回
/// - `Ok(SExpr)`: 解析成功
/// - `Err(SExprError)`: 解析失败
///
/// # 示例
/// ```ignore
/// let sexpr = parse("(add 1 2)")?;
/// ```
pub fn parse(input: &str) -> SExprResult<SExpr> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    parser.parse()
}

/// 便捷函数：解析S表达式并转换为Expr AST
///
/// # 参数
/// - `input`: S表达式字符串
///
/// # 返回
/// - `Ok(Expr)`: 解析和转换成功
/// - `Err(SExprError)`: 解析或转换失败
pub fn parse_to_expr(input: &str) -> SExprResult<crate::ast::Expr> {
    let sexpr = parse(input)?;
    convert(&sexpr)
}

/// 便捷函数：解析S表达式并转换为YAML Value
///
/// # 参数
/// - `input`: S表达式字符串
///
/// # 返回
/// - `Ok(serde_yaml::Value)`: 转换成功
/// - `Err(SExprError)`: 解析或转换失败
pub fn parse_to_yaml(input: &str) -> SExprResult<serde_yaml::Value> {
    let expr = parse_to_expr(input)?;
    Ok(to_yaml_value(&expr))
}
