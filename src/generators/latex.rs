//! LaTeX 公式生成器

use std::fs;
use std::path::Path;

use crate::error::{CompileError, CompileResult};
use crate::schema::EquationFile;

/// LaTeX 公式生成器
pub struct LatexGenerator;

impl LatexGenerator {
    /// 生成 LaTeX 文档
    pub fn generate(files: &[EquationFile], output_dir: &Path) -> CompileResult<()> {
        fs::create_dir_all(output_dir).map_err(|e| CompileError::io(output_dir, e))?;

        // 生成主文档
        let main_doc = Self::generate_main_doc(files);
        fs::write(output_dir.join("equations.tex"), main_doc)
            .map_err(|e| CompileError::io(output_dir.join("equations.tex"), e))?;

        // 生成每个模块的文档
        for file in files {
            let module_doc = Self::generate_module_doc(file);
            let file_path = output_dir.join(format!("{}.tex", file.meta.id.to_lowercase()));
            fs::write(&file_path, module_doc).map_err(|e| CompileError::io(&file_path, e))?;
        }

        Ok(())
    }

    fn generate_main_doc(files: &[EquationFile]) -> String {
        let mut lines = vec![
            r"\documentclass{article}".to_string(),
            r"\usepackage{amsmath}".to_string(),
            r"\usepackage{amssymb}".to_string(),
            r"\usepackage[utf8]{inputenc}".to_string(),
            r"\usepackage{CJKutf8}".to_string(),
            "".to_string(),
            r"\title{方程定义文档}".to_string(),
            r"\author{Equation Compiler}".to_string(),
            r"\date{\today}".to_string(),
            "".to_string(),
            r"\begin{document}".to_string(),
            r"\begin{CJK}{UTF8}{gbsn}".to_string(),
            "".to_string(),
            r"\maketitle".to_string(),
            "".to_string(),
            r"\tableofcontents".to_string(),
            r"\newpage".to_string(),
            "".to_string(),
        ];

        for file in files {
            lines.push(format!(r"\section{{{} ({})}}", file.meta.name_cn, file.meta.id));
            lines.push("".to_string());

            if let Some(ref desc) = file.meta.description {
                lines.push(desc.clone());
                lines.push("".to_string());
            }

            lines.push(r"\subsection{方程}".to_string());
            lines.push("".to_string());

            for eq in &file.equations {
                lines.push(format!(r"\subsubsection{{{}: {}}}", eq.id, eq.name));
                lines.push("".to_string());
                lines.push(r"\begin{equation}".to_string());
                lines.push(format!(
                    "    {} = {}",
                    Self::var_to_latex(&eq.output),
                    eq.expression.to_latex()
                ));
                lines.push(r"\end{equation}".to_string());
                lines.push("".to_string());

                if let Some(ref reference) = eq.reference {
                    lines.push(format!(r"\textit{{参考: {}}}", reference));
                    lines.push("".to_string());
                }
            }
        }

        lines.push(r"\end{CJK}".to_string());
        lines.push(r"\end{document}".to_string());

        lines.join("\n")
    }

    fn generate_module_doc(file: &EquationFile) -> String {
        let mut lines = vec![
            format!("% {} ({})", file.meta.name_cn, file.meta.id),
            "% 自动生成，请勿手动编辑".to_string(),
            "".to_string(),
        ];

        for eq in &file.equations {
            lines.push(format!("% {}: {}", eq.id, eq.name));
            lines.push(format!(
                "% {} = {}",
                Self::var_to_latex(&eq.output),
                eq.expression.to_latex()
            ));
            lines.push("".to_string());
        }

        lines.join("\n")
    }

    fn var_to_latex(name: &str) -> String {
        if name.contains('_') {
            let parts: Vec<&str> = name.splitn(2, '_').collect();
            format!("{}_{{{}}}", parts[0], parts[1])
        } else {
            name.to_string()
        }
    }
}
