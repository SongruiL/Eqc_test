//! 方程定义

use serde::{Deserialize, Serialize};

use crate::ast::Expr;

/// 方程定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equation {
    /// 方程 ID（如 "PHOTO-01"）
    pub id: String,

    /// 方程名称
    pub name: String,

    /// 输出变量名
    pub output: String,

    /// 表达式 AST
    pub expression: Expr,

    /// 可读公式（仅供展示）
    #[serde(default)]
    pub formula_display: Option<String>,

    /// 参考文献
    #[serde(default)]
    pub reference: Option<String>,
}

impl Equation {
    /// 获取表达式中引用的所有变量
    pub fn get_variable_refs(&self) -> Vec<String> {
        self.expression.get_variable_refs()
    }

    /// 获取表达式中引用的所有参数
    pub fn get_parameter_refs(&self) -> Vec<String> {
        self.expression.get_parameter_refs()
    }

    /// 获取完整的依赖列表（变量 + 参数）
    pub fn get_all_dependencies(&self) -> Vec<String> {
        let mut deps = self.get_variable_refs();
        deps.extend(self.get_parameter_refs());
        deps
    }
}
