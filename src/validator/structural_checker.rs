//! 结构检查（GA-1）：把图论结构分析里**新增**的结论接入 `validate`。
//!
//! 诚实边界——大部分结构 bug 现有校验已覆盖：
//! - 单文件内重复 output → `reference_checker` 已报 `DuplicateDefinition`；
//! - 代数环 → `cycle_detector` 已报 `CyclicDependency`（本工具仍是显式 Euler，环须报错）。
//!
//! 因此本检查只补一个**现有校验看不到**的缺口：**跨模块系统级过定**——
//! 耦合折叠（`source:`）后，不同模块的两条方程解算同一个变量节点。单文件的逐文件
//! `seen_outputs` 检查无法跨文件发现它。其余结构信息（自由变量/求解顺序/代数环的
//! 描述性细节）走 `eqc structure`，不塞进 validate。

use std::collections::HashMap;

use crate::error::ValidationError;
use crate::graph::bipartite::{BipartiteGraph, EqNode};
use crate::schema::EquationFile;

/// 检查跨模块系统级过定。返回每个「被 ≥2 个**不同模块**的方程解算」的变量一条错误。
pub fn check_structure(files: &[EquationFile]) -> Vec<ValidationError> {
    // 少于 2 个模块时没有跨模块过定的可能，省掉建图。
    if files.len() < 2 {
        return Vec::new();
    }
    let g = BipartiteGraph::from_files(files);

    let mut by_output: HashMap<&str, Vec<&EqNode>> = HashMap::new();
    for e in &g.equations {
        by_output.entry(e.output.as_str()).or_default().push(e);
    }

    let mut errors = Vec::new();
    let mut outputs: Vec<&&str> = by_output.keys().collect();
    outputs.sort(); // 确定性输出顺序
    for out in outputs {
        let group = &by_output[*out];
        if group.len() < 2 {
            continue;
        }
        // 仅当冲突方程跨 ≥2 个模块才报（同模块内的重复已由逐文件检查覆盖）。
        let mut modules: Vec<&str> = group.iter().map(|e| e.module.as_str()).collect();
        modules.sort();
        modules.dedup();
        if modules.len() < 2 {
            continue;
        }
        let mut keys: Vec<String> = group.iter().map(|e| e.key.clone()).collect();
        keys.sort();
        errors.push(ValidationError::StructurallyOverDetermined {
            variable: (*out).to_string(),
            equations: keys.join(", "),
        });
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::bipartite::tests::toy;

    #[test]
    fn single_file_no_structural_error() {
        // 单文件：即便有重复 output，本检查也不报（交给逐文件 DuplicateDefinition）。
        let f = toy(vec![("e1", "y", vec!["a"]), ("e2", "y", vec!["b"])]);
        assert!(check_structure(&[f]).is_empty());
    }

    #[test]
    fn cross_module_over_determination_flagged() {
        // 模块 A 解算 X；模块 B 声明 inX(source: A.X) 又写了一条解算 inX 的方程
        // → 折叠后 B::e1 也解算节点 "A.X" → 跨模块过定。
        let a = {
            let mut f = toy(vec![("e1", "X", vec!["p"])]);
            f.meta.id = "A".to_string();
            f
        };
        let b = {
            use crate::schema::{DataType, Variable, VariableType};
            let mut f = toy(vec![("e1", "inX", vec!["q"])]);
            f.meta.id = "B".to_string();
            f.variables.insert(
                "inX".to_string(),
                Variable {
                    var_type: VariableType::Input,
                    dtype: DataType::Float,
                    unit: None,
                    description: None,
                    label: None,
                    measurable: false,
                    stress_factor: None,
                    stress_reduce: None,
                    source: Some("A.X".to_string()),
                    class: None,
                    init: None,
                    rate: None,
                    prev: None,
                },
            );
            f
        };
        let errors = check_structure(&[a, b]);
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ValidationError::StructurallyOverDetermined { variable, .. } => {
                assert_eq!(variable, "A.X");
            }
            _ => panic!("期望 StructurallyOverDetermined"),
        }
    }
}
