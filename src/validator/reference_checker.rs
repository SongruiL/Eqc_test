//! 引用完整性检查
//!
//! 检查表达式中引用的变量和参数是否已定义。

use std::collections::{HashMap, HashSet};

use crate::error::ValidationError;
use crate::schema::{EquationFile, VariableType};

/// 仿真器注入的保留内置变量（无需在模型中声明即可引用）。
/// 目前仅 `DAT`（days after transplanting/start，从 1 起 = 当前天数，供物候/开花门控）。
const RESERVED_VARS: &[&str] = &["DAT"];

fn is_reserved_var(name: &str) -> bool {
    RESERVED_VARS.contains(&name)
}

/// 检查单个文件的引用完整性
pub fn check_references(file: &EquationFile) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 收集所有已定义的名称
    let defined_params: HashSet<&str> = file.parameters.keys().map(|s| s.as_str()).collect();
    let defined_vars: HashSet<&str> = file.variables.keys().map(|s| s.as_str()).collect();

    // 收集所有方程输出（可作为中间变量引用）
    let equation_outputs: HashSet<&str> = file.equations.iter().map(|e| e.output.as_str()).collect();

    // 检查每个方程
    for equation in &file.equations {
        // 检查变量引用（使用强类型 AST 的 get_variable_refs）
        let var_refs = equation.expression.get_variable_refs();
        for var_ref in var_refs {
            // 变量引用应在 variables 中定义，或是其他方程的输出，或是保留内置变量
            if !defined_vars.contains(var_ref.as_str())
                && !equation_outputs.contains(var_ref.as_str())
                && !is_reserved_var(var_ref.as_str())
            {
                errors.push(ValidationError::UndefinedReference {
                    kind: "变量".to_string(),
                    name: var_ref,
                    location: format!("方程 {}", equation.id),
                });
            }
        }

        // 检查参数引用（使用强类型 AST 的 get_parameter_refs）
        let param_refs = equation.expression.get_parameter_refs();
        for param_ref in param_refs {
            if !defined_params.contains(param_ref.as_str()) {
                errors.push(ValidationError::UndefinedReference {
                    kind: "参数".to_string(),
                    name: param_ref,
                    location: format!("方程 {}", equation.id),
                });
            }
        }
    }

    // 检查 output 类型变量是否有对应方程。
    // 跨步变量（积分状态量 / 延迟寄存器）的值由仿真器按 init+rate/prev 维护，
    // 本就不在 equations: 里写表达式，故豁免。
    for (var_name, var) in &file.variables {
        if var.var_type == VariableType::Output && !var.is_dynamic() {
            let has_equation = file.equations.iter().any(|e| &e.output == var_name);
            if !has_equation {
                errors.push(ValidationError::MissingOutputEquation {
                    variable: var_name.clone(),
                });
            }
        }
    }

    // 检查重复的方程 ID
    let mut seen_ids = HashSet::new();
    for equation in &file.equations {
        if !seen_ids.insert(&equation.id) {
            errors.push(ValidationError::DuplicateDefinition {
                kind: "方程".to_string(),
                name: equation.id.clone(),
            });
        }
    }

    // 检查重复的方程输出
    let mut seen_outputs = HashSet::new();
    for equation in &file.equations {
        if !seen_outputs.insert(&equation.output) {
            errors.push(ValidationError::DuplicateDefinition {
                kind: "方程输出".to_string(),
                name: equation.output.clone(),
            });
        }
    }

    errors
}

/// 检查跨模块引用
pub fn check_cross_module_references(files: &[EquationFile]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 构建模块输出映射
    let mut module_outputs: HashMap<String, HashSet<String>> = HashMap::new();
    for file in files {
        let mut outputs = HashSet::new();
        for (var_name, var) in &file.variables {
            if var.var_type == VariableType::Output {
                outputs.insert(var_name.clone());
            }
        }
        module_outputs.insert(file.meta.id.clone(), outputs);
    }

    // 检查 input 变量的 source 引用
    for file in files {
        for (var_name, var) in &file.variables {
            if var.var_type == VariableType::Input {
                if let Some(ref source) = var.source {
                    match var.parse_source() {
                        Some((module_id, output_name)) => {
                            // 检查模块是否存在
                            if let Some(outputs) = module_outputs.get(module_id) {
                                // 检查输出变量是否存在
                                if !outputs.contains(output_name) {
                                    errors.push(ValidationError::UndefinedReference {
                                        kind: "输出变量".to_string(),
                                        name: format!("{}.{}", module_id, output_name),
                                        location: format!(
                                            "模块 {} 变量 {}",
                                            file.meta.id, var_name
                                        ),
                                    });
                                }
                            } else {
                                errors.push(ValidationError::UndefinedReference {
                                    kind: "模块".to_string(),
                                    name: module_id.to_string(),
                                    location: format!(
                                        "模块 {} 变量 {}",
                                        file.meta.id, var_name
                                    ),
                                });
                            }
                        }
                        None => {
                            errors.push(ValidationError::InvalidSourceReference(source.clone()));
                        }
                    }
                }
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use crate::ast::Expr;
    use crate::schema::{Equation, Metadata, Parameter, Variable};

    fn create_test_metadata() -> Metadata {
        Metadata {
            id: "TEST".to_string(),
            model: "TestModel".to_string(),
            name_cn: "测试模块".to_string(),
            name_en: None,
            version: "1.0".to_string(),
            description: None,
            reference: None,
            source_files: vec![],
            dt: 1.0,
            dt_seconds: None,
            calibration: None,
            modules: Default::default(),
        }
    }

    #[test]
    fn test_valid_references() {
        let mut parameters = IndexMap::new();
        parameters.insert(
            "p1".to_string(),
            Parameter {
                name_cn: "参数1".to_string(),
                name_en: None,
                dtype: crate::schema::DataType::Float,
                default: 1.0,
                values: None,
                unit: None,
                bounds: None,
                optimizable: false,
                management: false,
                description: None,
            },
        );

        let mut variables = IndexMap::new();
        variables.insert(
            "x".to_string(),
            Variable {
                var_type: VariableType::Input,
                dtype: crate::schema::DataType::Float,
                unit: None,
                description: None,
                label: None,
                measurable: false,
                stress_factor: None,
                stress_reduce: None,
                source: None,
                class: None,
                init: None,
                rate: None,
                prev: None,
            },
        );

        let file = EquationFile {
            meta: create_test_metadata(),
            parameters,
            variables,
            equations: vec![Equation {
                id: "E1".to_string(),
                name: "测试方程".to_string(),
                output: "y".to_string(),
                expression: Expr::add(Expr::param("p1"), Expr::var("x")),
                formula_display: None,
                reference: None, gp_target: None,
            }],
        };

        let errors = check_references(&file);
        assert!(errors.is_empty(), "不应有错误: {:?}", errors);
    }

    #[test]
    fn test_undefined_parameter() {
        let file = EquationFile {
            meta: create_test_metadata(),
            parameters: Default::default(), // 没有定义参数
            variables: Default::default(),
            equations: vec![Equation {
                id: "E1".to_string(),
                name: "测试方程".to_string(),
                output: "y".to_string(),
                expression: Expr::param("p1"), // 引用未定义的参数
                formula_display: None,
                reference: None, gp_target: None,
            }],
        };

        let errors = check_references(&file);
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ValidationError::UndefinedReference { kind, name, .. } => {
                assert_eq!(kind, "参数");
                assert_eq!(name, "p1");
            }
            _ => panic!("错误类型不匹配"),
        }
    }

    #[test]
    fn test_undefined_variable() {
        let file = EquationFile {
            meta: create_test_metadata(),
            parameters: Default::default(),
            variables: Default::default(), // 没有定义变量
            equations: vec![Equation {
                id: "E1".to_string(),
                name: "测试方程".to_string(),
                output: "y".to_string(),
                expression: Expr::var("x"), // 引用未定义的变量
                formula_display: None,
                reference: None, gp_target: None,
            }],
        };

        let errors = check_references(&file);
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ValidationError::UndefinedReference { kind, name, .. } => {
                assert_eq!(kind, "变量");
                assert_eq!(name, "x");
            }
            _ => panic!("错误类型不匹配"),
        }
    }

    #[test]
    fn test_intermediate_variable_reference() {
        // 方程 E2 引用方程 E1 的输出作为中间变量
        let file = EquationFile {
            meta: create_test_metadata(),
            parameters: Default::default(),
            variables: Default::default(),
            equations: vec![
                Equation {
                    id: "E1".to_string(),
                    name: "方程1".to_string(),
                    output: "a".to_string(),
                    expression: Expr::constant(1.0),
                    formula_display: None,
                    reference: None, gp_target: None,
                },
                Equation {
                    id: "E2".to_string(),
                    name: "方程2".to_string(),
                    output: "b".to_string(),
                    expression: Expr::var("a"), // 引用 E1 的输出
                    formula_display: None,
                    reference: None, gp_target: None,
                },
            ],
        };

        let errors = check_references(&file);
        assert!(errors.is_empty(), "引用其他方程输出应该有效: {:?}", errors);
    }

    #[test]
    fn test_dynamic_model_state_and_dat() {
        // 状态量输出 TDM（init+rate，无方程）+ 引用保留内置变量 DAT，都应通过校验
        let mut variables = IndexMap::new();
        variables.insert(
            "TDM".to_string(),
            Variable {
                var_type: VariableType::Output,
                dtype: crate::schema::DataType::Float,
                unit: None,
                description: None,
                label: None,
                measurable: false,
                stress_factor: None,
                stress_reduce: None,
                source: None,
                class: Some(crate::schema::VarClass::State),
                init: Some(0.0),
                rate: Some("DDM".to_string()),
                prev: None,
            },
        );
        let file = EquationFile {
            meta: create_test_metadata(),
            parameters: Default::default(),
            variables,
            equations: vec![Equation {
                id: "E1".to_string(),
                name: "速率".to_string(),
                output: "DDM".to_string(),
                // 引用保留内置变量 DAT（无需声明）
                expression: Expr::mul(Expr::var("DAT"), Expr::constant(2.0)),
                formula_display: None,
                reference: None, gp_target: None,
            }],
        };
        let errors = check_references(&file);
        assert!(errors.is_empty(), "状态量无方程 + 引用 DAT 应通过: {errors:?}");
    }

    #[test]
    fn test_duplicate_equation_id() {
        let file = EquationFile {
            meta: create_test_metadata(),
            parameters: Default::default(),
            variables: Default::default(),
            equations: vec![
                Equation {
                    id: "E1".to_string(),
                    name: "方程1".to_string(),
                    output: "a".to_string(),
                    expression: Expr::constant(1.0),
                    formula_display: None,
                    reference: None, gp_target: None,
                },
                Equation {
                    id: "E1".to_string(), // 重复 ID
                    name: "方程2".to_string(),
                    output: "b".to_string(),
                    expression: Expr::constant(2.0),
                    formula_display: None,
                    reference: None, gp_target: None,
                },
            ],
        };

        let errors = check_references(&file);
        assert!(errors.iter().any(|e| matches!(
            e,
            ValidationError::DuplicateDefinition { kind, .. } if kind == "方程"
        )));
    }
}
