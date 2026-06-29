//! 集成测试
#![cfg(feature = "cli")] // 用 cli-only API（Compiler/generators）；默认配置（无 cli）跳过整个测试

use equation_compiler::{Compiler, GeneratorKind};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn create_test_equation_file(dir: &std::path::Path) {
    let content = r#"
meta:
  id: "PHOTO"
  model: "QualiTree"
  name_cn: "光合作用"
  name_en: "Photosynthesis"
  version: "1.0"
  description: "计算叶片光合速率"

parameters:
  p1:
    name_cn: "基础光饱和光合"
    type: float
    default: 20.14
    unit: "μmol CO₂/m²/s"
    optimizable: true
    
  p2:
    name_cn: "储备反馈系数"
    type: float
    default: -66.95
    unit: "μmol CO₂/m²/s"
    optimizable: true

variables:
  reserve_ratio:
    type: input
    dtype: float
    unit: "dimensionless"
    description: "储备/生物量比"
    
  Pmax_l:
    type: output
    dtype: float
    unit: "μmol CO₂/m²/s"
    description: "动态光饱和光合速率"

equations:
  - id: "PHOTO-01"
    name: "动态Pmax"
    output: Pmax_l
    expression:
      op: add
      args:
        - { ref: p1 }
        - op: mul
          args:
            - { ref: p2 }
            - { ref: reserve_ratio }
    formula_display: "Pmax_l = p1 + p2 × reserve_ratio"
    reference: "Lescourret 1998"
"#;

    let file_path = dir.join("photo.eq.yaml");
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

#[test]
fn test_parse_and_validate() {
    let temp_dir = TempDir::new().unwrap();
    create_test_equation_file(temp_dir.path());

    let result = Compiler::new()
        .load_directory(temp_dir.path())
        .and_then(|c| c.validate());

    assert!(result.is_ok());

    let compiler = result.unwrap();
    assert_eq!(compiler.files().len(), 1);
    assert_eq!(compiler.files()[0].meta.id, "PHOTO");
    assert_eq!(compiler.files()[0].equations.len(), 1);
}

#[test]
fn test_build_dag() {
    let temp_dir = TempDir::new().unwrap();
    create_test_equation_file(temp_dir.path());

    let result = Compiler::new()
        .load_directory(temp_dir.path())
        .and_then(|c| c.validate())
        .and_then(|c| c.build_dag());

    assert!(result.is_ok());

    let compiler = result.unwrap();
    let dag = compiler.dag().unwrap();

    assert!(!dag.nodes.is_empty());
    assert!(!dag.edges.is_empty());
}

#[test]
fn test_generate_python() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    create_test_equation_file(temp_dir.path());

    let result = Compiler::new()
        .load_directory(temp_dir.path())
        .and_then(|c| c.validate())
        .and_then(|c| c.build_dag())
        .and_then(|c| {
            c.generate(GeneratorKind::Python, output_dir.path())?;
            Ok(c)
        });

    assert!(result.is_ok());

    // 检查生成的文件
    let python_dir = output_dir.path().join("python");
    assert!(python_dir.join("__init__.py").exists());
    assert!(python_dir.join("photo.py").exists());
    assert!(python_dir.join("params.py").exists());
}

#[test]
fn test_generate_all() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    create_test_equation_file(temp_dir.path());

    let result = Compiler::new()
        .load_directory(temp_dir.path())
        .and_then(|c| c.validate())
        .and_then(|c| c.build_dag())
        .and_then(|c| {
            c.generate(GeneratorKind::All, output_dir.path())?;
            Ok(c)
        });

    assert!(result.is_ok());

    // 检查所有输出目录
    assert!(output_dir.path().join("python").exists());
    assert!(output_dir.path().join("rust").exists());
    assert!(output_dir.path().join("workflows").exists());
    assert!(output_dir.path().join("docs").exists());
    assert!(output_dir.path().join("latex").exists());
}
