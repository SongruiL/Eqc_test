//! 代码生成器模块

mod latex;
mod markdown;
mod python;
mod python_sim;
mod rust_operator;
mod workflow_json;

pub use latex::LatexGenerator;
pub use markdown::MarkdownGenerator;
pub use python::PythonGenerator;
pub use rust_operator::RustOperatorGenerator;
pub use workflow_json::WorkflowJsonGenerator;

use std::path::Path;

use crate::dag::Dag;
use crate::error::{CompileError, CompileResult};
use crate::schema::EquationFile;

/// 输出格式类型
#[derive(Debug, Clone, Copy)]
pub enum GeneratorKind {
    /// Python 可执行代码
    Python,
    /// Rust 算子代码（用于 lowcode 平台）
    RustOperator,
    /// 流程 JSON（用于 lowcode 平台导入）
    WorkflowJson,
    /// Markdown 文档
    Markdown,
    /// LaTeX 公式
    Latex,
    /// 全部格式
    All,
}

/// 统一生成接口
pub fn generate(
    files: &[EquationFile],
    dag: Option<&Dag>,
    kind: GeneratorKind,
    output_dir: impl AsRef<Path>,
) -> CompileResult<()> {
    let output_dir = output_dir.as_ref();

    // 确保输出目录存在
    std::fs::create_dir_all(output_dir)
        .map_err(|e| CompileError::io(output_dir, e))?;

    match kind {
        GeneratorKind::Python => {
            let python_dir = output_dir.join("python");
            PythonGenerator::generate(files, &python_dir)
        }
        GeneratorKind::RustOperator => {
            let rust_dir = output_dir.join("rust");
            RustOperatorGenerator::generate(files, &rust_dir)
        }
        GeneratorKind::WorkflowJson => {
            let json_dir = output_dir.join("workflows");
            WorkflowJsonGenerator::generate(files, dag, &json_dir)
        }
        GeneratorKind::Markdown => {
            let docs_dir = output_dir.join("docs");
            MarkdownGenerator::generate(files, dag, &docs_dir)
        }
        GeneratorKind::Latex => {
            let latex_dir = output_dir.join("latex");
            LatexGenerator::generate(files, &latex_dir)
        }
        GeneratorKind::All => {
            PythonGenerator::generate(files, &output_dir.join("python"))?;
            RustOperatorGenerator::generate(files, &output_dir.join("rust"))?;
            WorkflowJsonGenerator::generate(files, dag, &output_dir.join("workflows"))?;
            MarkdownGenerator::generate(files, dag, &output_dir.join("docs"))?;
            LatexGenerator::generate(files, &output_dir.join("latex"))?;
            Ok(())
        }
    }
}
