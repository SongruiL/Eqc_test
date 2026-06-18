//! # Equation Compiler 方程编译器
//!
//! 将 S-expression 方程定义编译为多种输出格式的库。
//!
//! ## 核心功能
//!
//! - S-expression 解析 -> AST
//! - 带注解的 S-expression -> 算子定义 (OperatorDef)
//! - 算子定义 -> AST JSON (用于动态注册)
//! - 算子定义 -> SQL 模板 (用于数据库导入)

// 核心模块（始终可用）
pub mod ast;
pub mod error;
pub mod eval;
pub mod ops;
pub mod sexpr;
pub mod units;

// 完整编译器模块（仅 CLI 工具需要）
#[cfg(feature = "cli")]
pub mod dag;
#[cfg(feature = "cli")]
pub mod generators;
#[cfg(feature = "cli")]
pub mod parser;
#[cfg(feature = "cli")]
pub mod report;
#[cfg(feature = "cli")]
pub mod schema;
#[cfg(feature = "cli")]
pub mod sim;
#[cfg(feature = "cli")]
pub mod validator;

// ============================================
// 公开 API 重导出
// ============================================

// 核心类型
pub use ast::{Expr, ExprVisitor};
pub use error::{CompileError, CompileResult};

// 求值器
pub use eval::{Env, EvalError, EvalMode};

// 算子注册表（算子单一真相源）
pub use ops::OperatorSpec;

// 量纲系统（科学正确性护栏）
pub use units::{check_expr, convert, parse_dimension, parse_unit, DimError, Dimension, Unit};

// S表达式解析器
pub use sexpr::{parse as parse_sexpr, parse_to_expr, parse_to_yaml, SExpr, SExprError};

// Workflow 生成器（动态注册版）
pub use sexpr::{
    generate_ast_json, generate_operator_seed_sql, generate_operators, generate_register_code,
    generate_template_sql, generate_workflow_json, parse_annotated_sexpr, InputDef as WorkflowInputDef,
    ModuleDef, OperatorDef, OperatorType, OutputDef as WorkflowOutputDef,
};

// CLI 工具需要的完整 API
#[cfg(feature = "cli")]
pub use dag::{Dag, DagEdge, DagNode, EdgeType, NodeType};
#[cfg(feature = "cli")]
pub use generators::GeneratorKind;
#[cfg(feature = "cli")]
pub use parser::{parse_directory, parse_file};
#[cfg(feature = "cli")]
pub use schema::{DataType, Equation, EquationFile, Metadata, Parameter, VarClass, Variable, VariableType};
#[cfg(feature = "cli")]
pub use sim::{simulate, SimError, SimInput, SimOutput};
#[cfg(feature = "cli")]
pub use validator::{validate, ExprType, ValidationError as ValidatorError};

#[cfg(feature = "cli")]
use std::path::Path;

// ============================================
// 高级 API：Compiler 构建器模式（仅 CLI 工具）
// ============================================

#[cfg(feature = "cli")]
#[derive(Default)]
pub struct Compiler {
    files: Vec<EquationFile>,
    dag: Option<Dag>,
}

#[cfg(feature = "cli")]
impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_file(mut self, path: impl AsRef<Path>) -> CompileResult<Self> {
        let file = parse_file(path.as_ref())?;
        self.files.push(file);
        Ok(self)
    }

    pub fn load_directory(mut self, path: impl AsRef<Path>) -> CompileResult<Self> {
        self.files = parse_directory(path.as_ref())?;
        Ok(self)
    }

    pub fn validate(self) -> CompileResult<Self> {
        validator::validate(&self.files)?;
        Ok(self)
    }

    pub fn build_dag(mut self) -> CompileResult<Self> {
        self.dag = Some(dag::build_dag(&self.files)?);
        Ok(self)
    }

    pub fn generate(
        &self,
        kind: GeneratorKind,
        output_dir: impl AsRef<Path>,
    ) -> CompileResult<()> {
        generators::generate(&self.files, self.dag.as_ref(), kind, output_dir)
    }

    pub fn files(&self) -> &[EquationFile] {
        &self.files
    }

    pub fn dag(&self) -> Option<&Dag> {
        self.dag.as_ref()
    }

    pub fn equation_ids(&self) -> Vec<String> {
        self.files
            .iter()
            .flat_map(|f| f.equations.iter().map(|e| e.id.clone()))
            .collect()
    }

    pub fn module_ids(&self) -> Vec<String> {
        self.files.iter().map(|f| f.meta.id.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sexpr_parse() {
        let expr = crate::parse_sexpr("(add 1 2)").unwrap();
        assert!(!format!("{:?}", expr).is_empty());
    }
}
