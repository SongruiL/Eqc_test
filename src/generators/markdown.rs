//! Markdown 文档生成器

use std::fs;
use std::path::Path;

use crate::dag::Dag;
use crate::error::{CompileError, CompileResult};
use crate::schema::EquationFile;

/// Markdown 文档生成器
pub struct MarkdownGenerator;

impl MarkdownGenerator {
    /// 生成 Markdown 文档
    pub fn generate(
        files: &[EquationFile],
        dag: Option<&Dag>,
        output_dir: &Path,
    ) -> CompileResult<()> {
        fs::create_dir_all(output_dir).map_err(|e| CompileError::io(output_dir, e))?;

        // 生成主文档
        let main_doc = Self::generate_main_doc(files, dag);
        fs::write(output_dir.join("equations.md"), main_doc)
            .map_err(|e| CompileError::io(output_dir.join("equations.md"), e))?;

        // 生成每个模块的文档
        for file in files {
            let module_doc = Self::generate_module_doc(file);
            let file_path = output_dir.join(format!("{}.md", file.meta.id.to_lowercase()));
            fs::write(&file_path, module_doc).map_err(|e| CompileError::io(&file_path, e))?;
        }

        // 生成 DAG 文档
        if let Some(dag) = dag {
            let dag_doc = Self::generate_dag_doc(dag);
            fs::write(output_dir.join("dag.md"), dag_doc)
                .map_err(|e| CompileError::io(output_dir.join("dag.md"), e))?;
        }

        Ok(())
    }

    fn generate_main_doc(files: &[EquationFile], dag: Option<&Dag>) -> String {
        let mut lines = vec![
            "# 方程文档".to_string(),
            "".to_string(),
            "> 此文档由 equation-compiler 自动生成".to_string(),
            "".to_string(),
            "## 模块概览".to_string(),
            "".to_string(),
            "| 模块 ID | 名称 | 模型 | 方程数 |".to_string(),
            "|---------|------|------|--------|".to_string(),
        ];

        for file in files {
            lines.push(format!(
                "| {} | {} | {} | {} |",
                file.meta.id,
                file.meta.name_cn,
                file.meta.model,
                file.equations.len()
            ));
        }

        lines.push("".to_string());

        // 添加 DAG 图
        if let Some(dag) = dag {
            lines.push("## 模块依赖图".to_string());
            lines.push("".to_string());
            lines.push("```mermaid".to_string());
            lines.push("graph LR".to_string());

            for edge in dag.get_coupling_edges() {
                let from_module = edge.from.split('.').next().unwrap_or(&edge.from);
                let to_module = edge.to.split('.').next().unwrap_or(&edge.to);
                if from_module != to_module {
                    lines.push(format!("    {} --> {}", from_module, to_module));
                }
            }

            lines.push("```".to_string());
            lines.push("".to_string());
        }

        lines.join("\n")
    }

    fn generate_module_doc(file: &EquationFile) -> String {
        let mut lines = vec![
            format!("# {} ({})", file.meta.name_cn, file.meta.id),
            "".to_string(),
            format!("**模型**: {}", file.meta.model),
        ];

        if let Some(ref desc) = file.meta.description {
            lines.push(format!("**描述**: {}", desc));
        }

        if let Some(ref reference) = file.meta.reference {
            lines.push(format!("**参考文献**: {}", reference));
        }

        lines.push("".to_string());

        // 参数表
        if !file.parameters.is_empty() {
            lines.push("## 参数".to_string());
            lines.push("".to_string());
            lines.push("| 名称 | 中文名 | 默认值 | 单位 | 可优化 |".to_string());
            lines.push("|------|--------|--------|------|--------|".to_string());

            for (name, param) in &file.parameters {
                lines.push(format!(
                    "| {} | {} | {} | {} | {} |",
                    name,
                    param.name_cn,
                    param.default,
                    param.unit.as_deref().unwrap_or("-"),
                    if param.optimizable { "是" } else { "否" }
                ));
            }

            lines.push("".to_string());
        }

        // 变量表
        if !file.variables.is_empty() {
            lines.push("## 变量".to_string());
            lines.push("".to_string());
            lines.push("| 名称 | 类型 | 单位 | 描述 |".to_string());
            lines.push("|------|------|------|------|".to_string());

            for (name, var) in &file.variables {
                lines.push(format!(
                    "| {} | {:?} | {} | {} |",
                    name,
                    var.var_type,
                    var.unit.as_deref().unwrap_or("-"),
                    var.description.as_deref().unwrap_or("-")
                ));
            }

            lines.push("".to_string());
        }

        // 方程列表
        lines.push("## 方程".to_string());
        lines.push("".to_string());

        for eq in &file.equations {
            lines.push(format!("### {} - {}", eq.id, eq.name));
            lines.push("".to_string());
            lines.push(format!("**输出**: `{}`", eq.output));
            lines.push("".to_string());

            if let Some(ref formula) = eq.formula_display {
                lines.push("**公式**:".to_string());
                lines.push("".to_string());
                lines.push(format!("$${}$$", eq.expression.to_latex()));
                lines.push("".to_string());
                lines.push(format!("可读形式: `{}`", formula));
                lines.push("".to_string());
            } else {
                lines.push(format!("**公式**: ${}$", eq.expression.to_latex()));
                lines.push("".to_string());
            }

            if let Some(ref reference) = eq.reference {
                lines.push(format!("**参考**: {}", reference));
                lines.push("".to_string());
            }

            // 依赖
            let deps = eq.get_all_dependencies();
            if !deps.is_empty() {
                lines.push(format!("**依赖**: {}", deps.join(", ")));
                lines.push("".to_string());
            }
        }

        lines.join("\n")
    }

    fn generate_dag_doc(dag: &Dag) -> String {
        let mut lines = vec![
            "# DAG 依赖图".to_string(),
            "".to_string(),
            "## 完整依赖图".to_string(),
            "".to_string(),
            "```mermaid".to_string(),
            "graph TD".to_string(),
        ];

        for edge in &dag.edges {
            let style = match edge.edge_type {
                crate::dag::EdgeType::DataFlow => "",
                crate::dag::EdgeType::ModuleCoupling => " -.->|coupling|",
            };

            if style.is_empty() {
                lines.push(format!("    {} --> {}", edge.from, edge.to));
            } else {
                lines.push(format!("    {}{} {}", edge.from, style, edge.to));
            }
        }

        lines.push("```".to_string());
        lines.push("".to_string());

        // 拓扑排序
        lines.push("## 计算顺序".to_string());
        lines.push("".to_string());

        for (i, node) in dag.topological_order.iter().enumerate() {
            lines.push(format!("{}. `{}`", i + 1, node));
        }

        lines.join("\n")
    }
}
