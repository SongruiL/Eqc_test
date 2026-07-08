//! 循环依赖检测

use std::collections::{HashMap, HashSet};

use crate::schema::EquationFile;

/// 检测方程间的循环依赖
///
/// 返回循环路径（如果存在）
pub fn detect_cycles(files: &[EquationFile]) -> Option<Vec<String>> {
    // 构建依赖图
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_nodes: HashSet<String> = HashSet::new();

    for file in files {
        for equation in &file.equations {
            let output = equation.output.clone();
            all_nodes.insert(output.clone());

            let deps = equation.get_variable_refs();
            graph.insert(output, deps.clone());

            for dep in deps {
                all_nodes.insert(dep);
            }
        }
    }

    // DFS 检测循环
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for node in &all_nodes {
        if !visited.contains(node) {
            if let Some(cycle) = dfs_detect_cycle(node, &graph, &mut visited, &mut rec_stack, &mut path) {
                return Some(cycle);
            }
        }
    }

    None
}

fn dfs_detect_cycle(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = dfs_detect_cycle(neighbor, graph, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // 找到循环，提取循环路径
                let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                let mut cycle: Vec<String> = path[cycle_start..].to_vec();
                cycle.push(neighbor.clone());
                return Some(cycle);
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::schema::{Equation, EquationFile, Metadata};

    fn create_test_file(equations: Vec<(&str, &str, Vec<&str>)>) -> EquationFile {
        let eqs = equations
            .into_iter()
            .map(|(id, output, deps)| {
                let expr = if deps.is_empty() {
                    Expr::constant(1.0)
                } else {
                    deps.into_iter()
                        .map(Expr::var)
                        .reduce(Expr::add)
                        .unwrap()
                };

                Equation {
                    id: id.to_string(),
                    name: id.to_string(),
                    output: output.to_string(),
                    expression: expr,
                    formula_display: None,
                    reference: None, gp_target: None, provenance: None,
                 instance: None }
            })
            .collect();

        EquationFile {
            meta: Metadata {
                id: "TEST".to_string(),
                model: "Test".to_string(),
                name_cn: "测试".to_string(),
                name_en: None,
                version: "1.0".to_string(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(), balance: vec![], lineage: None,
            },
            parameters: Default::default(),
            variables: Default::default(),
            equations: eqs,
         structure: None }
    }

    #[test]
    fn test_no_cycle() {
        // A -> B -> C (无循环)
        let file = create_test_file(vec![
            ("E1", "A", vec![]),
            ("E2", "B", vec!["A"]),
            ("E3", "C", vec!["B"]),
        ]);

        let result = detect_cycles(&[file]);
        assert!(result.is_none());
    }

    #[test]
    fn test_simple_cycle() {
        // A -> B -> A (循环)
        let file = create_test_file(vec![
            ("E1", "A", vec!["B"]),
            ("E2", "B", vec!["A"]),
        ]);

        let result = detect_cycles(&[file]);
        assert!(result.is_some());
    }
}
