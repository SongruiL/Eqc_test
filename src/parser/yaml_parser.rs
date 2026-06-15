//! YAML 文件解析器

use std::fs;
use std::path::Path;

use crate::error::{CompileError, CompileResult};
use crate::schema::EquationFile;

/// 解析单个方程文件
///
/// # 参数
/// - `path`: 方程文件路径（.eq.yaml）
///
/// # 错误
/// - 文件不存在
/// - YAML 格式错误
pub fn parse_file(path: &Path) -> CompileResult<EquationFile> {
    let content = fs::read_to_string(path).map_err(|e| CompileError::io(path, e))?;

    let file: EquationFile =
        serde_yaml::from_str(&content).map_err(|e| CompileError::yaml_parse(path, e.to_string()))?;

    Ok(file)
}

/// 解析目录下所有方程文件
///
/// 递归搜索所有 `.eq.yaml` 文件。
///
/// # 参数
/// - `dir`: 目录路径
///
/// # 错误
/// - 目录不存在
/// - 任一文件解析失败
pub fn parse_directory(dir: &Path) -> CompileResult<Vec<EquationFile>> {
    if !dir.exists() {
        return Err(CompileError::io(
            dir,
            std::io::Error::new(std::io::ErrorKind::NotFound, "目录不存在"),
        ));
    }

    let mut files = Vec::new();
    collect_equation_files(dir, &mut files)?;

    Ok(files)
}

/// 递归收集方程文件
fn collect_equation_files(dir: &Path, files: &mut Vec<EquationFile>) -> CompileResult<()> {
    let entries = fs::read_dir(dir).map_err(|e| CompileError::io(dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| CompileError::io(dir, e))?;
        let path = entry.path();

        if path.is_dir() {
            collect_equation_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
            // 检查是否是 .eq.yaml 文件
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name.contains(".eq.") {
                let file = parse_file(&path)?;
                files.push(file);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_simple_file() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_content = r#"
meta:
  id: "TEST"
  model: "TestModel"
  name_cn: "测试模块"

parameters:
  p1:
    name_cn: "参数1"
    default: 1.0

variables:
  x:
    type: input
    description: "输入变量"

equations:
  - id: "TEST-01"
    name: "测试方程"
    output: "y"
    expression:
      op: add
      args:
        - { ref: p1 }
        - { ref: x }
"#;

        create_test_file(temp_dir.path(), "test.eq.yaml", yaml_content);

        let result = parse_file(&temp_dir.path().join("test.eq.yaml"));
        assert!(result.is_ok());

        let file = result.unwrap();
        assert_eq!(file.meta.id, "TEST");
        assert_eq!(file.parameters.len(), 1);
        assert_eq!(file.equations.len(), 1);
    }
}
