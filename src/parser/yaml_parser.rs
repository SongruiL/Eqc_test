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

    // 先把 YAML 读成 Value，做 cohort（同期群）展开——把按下标的模板宏展开成纯标量，
    // 之后再反序列化成 EquationFile。无 `cohorts:` 段的模型原样通过，行为不变。
    let raw: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| CompileError::yaml_parse(path, e.to_string()))?;
    let expanded = super::expand_cohorts(raw)
        .map_err(|e| CompileError::yaml_parse(path, format!("cohort 展开失败: {e}")))?;

    let mut file: EquationFile =
        serde_yaml::from_value(expanded).map_err(|e| CompileError::yaml_parse(path, e.to_string()))?;

    // 加载后把引用到参数名的 Var 重分类为 Param（让参数可用任意有意义的名字）。
    file.reclassify_parameters();

    Ok(file)
}

/// 从字符串解析方程文件（与 [`parse_file`] 同一管线，但输入是文本——浏览器内编辑器的
/// `/api/validate` 用：前端递交编辑后的 YAML，EQC parse + 校验，不落盘）。
pub fn parse_str(content: &str) -> CompileResult<EquationFile> {
    let p = Path::new("<编辑>");
    let raw: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| CompileError::yaml_parse(p, e.to_string()))?;
    let expanded = super::expand_cohorts(raw)
        .map_err(|e| CompileError::yaml_parse(p, format!("cohort 展开失败: {e}")))?;
    let mut file: EquationFile =
        serde_yaml::from_value(expanded).map_err(|e| CompileError::yaml_parse(p, e.to_string()))?;
    file.reclassify_parameters();
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

    /// parse_str：从文本解析（编辑器 /api/validate 用），与 parse_file 同结果；坏文本→Err。
    #[test]
    fn test_parse_str() {
        let yaml = r#"
meta: { id: "S", model: "M", name_cn: "字符串解析" }
parameters:
  p1: { name_cn: "参数1", default: 1.0 }
variables:
  x: { type: input }
equations:
  - id: "E-01"
    name: "方程"
    output: "y"
    expression: { op: add, args: [ { ref: p1 }, { ref: x } ] }
"#;
        let f = parse_str(yaml).expect("应解析成功");
        assert_eq!(f.meta.id, "S");
        assert_eq!(f.parameters.len(), 1);
        assert_eq!(f.equations.len(), 1);
        // 坏 YAML → Err（不 panic）
        assert!(parse_str("meta: [this is: not valid").is_err());
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

    #[test]
    fn test_parse_dynamic_variables() {
        use crate::schema::VarClass;

        let temp_dir = TempDir::new().unwrap();
        // 一个最小动态模型片段：积分状态量 TDM、延迟寄存器 RFG_prev、驱动 T。
        let yaml_content = r#"
meta:
  id: "DYN"
  model: "DynModel"
  name_cn: "动态变量测试"

variables:
  T:
    type: input
    class: driving
    description: "日均温"
  DDM:
    type: intermediate
    class: rate
    description: "日干物质生产"
  TDM:
    type: output
    class: state
    init: 19.9
    rate: DDM
    description: "累积干物质"
  RFG:
    type: intermediate
    description: "果实相对生长"
  RFG_prev:
    type: intermediate
    init: 0.000217
    prev: RFG
    description: "上一步 RFG"

equations:
  - id: "DYN-01"
    name: "日干物质生产"
    output: "DDM"
    expression: { const: 5.0 }
"#;

        create_test_file(temp_dir.path(), "dyn.eq.yaml", yaml_content);
        let file = parse_file(&temp_dir.path().join("dyn.eq.yaml")).unwrap();

        let tdm = &file.variables["TDM"];
        assert_eq!(tdm.class, Some(VarClass::State));
        assert_eq!(tdm.init, Some(19.9));
        assert_eq!(tdm.rate.as_deref(), Some("DDM"));
        assert!(tdm.is_integrator() && tdm.is_dynamic());

        let rfg_prev = &file.variables["RFG_prev"];
        assert_eq!(rfg_prev.prev.as_deref(), Some("RFG"));
        assert!(rfg_prev.is_delay());
        // 未显式声明 class，应推断为 SemiState
        assert_eq!(rfg_prev.effective_class(), VarClass::SemiState);

        // 驱动变量与显式 class
        assert_eq!(file.variables["T"].class, Some(VarClass::Driving));
        assert_eq!(file.variables["DDM"].class, Some(VarClass::Rate));
        // 普通变量字段缺省为 None，向后兼容
        assert_eq!(file.variables["RFG"].class, None);
        assert!(!file.variables["RFG"].is_dynamic());
    }
}
