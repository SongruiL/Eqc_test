//! 流程 JSON 生成器
//!
//! 生成与 lowcode 平台兼容的流程定义 JSON。

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::dag::Dag;
use crate::error::{CompileError, CompileResult};
use crate::schema::EquationFile;

/// 流程 JSON 生成器
pub struct WorkflowJsonGenerator;

impl WorkflowJsonGenerator {
    /// 生成流程 JSON
    pub fn generate(
        files: &[EquationFile],
        dag: Option<&Dag>,
        output_dir: &Path,
    ) -> CompileResult<()> {
        fs::create_dir_all(output_dir).map_err(|e| CompileError::io(output_dir, e))?;

        // 为每个模块生成一个流程 JSON
        for file in files {
            let content = Self::generate_module_workflow(file, dag);
            let file_path = output_dir.join(format!("{}.json", file.meta.id.to_lowercase()));
            fs::write(&file_path, content).map_err(|e| CompileError::io(&file_path, e))?;
        }

        // 生成完整的 DAG JSON
        if let Some(dag) = dag {
            let dag_content = Self::generate_dag_json(dag);
            let dag_path = output_dir.join("dag.json");
            fs::write(&dag_path, dag_content).map_err(|e| CompileError::io(&dag_path, e))?;
        }

        Ok(())
    }

    fn generate_module_workflow(file: &EquationFile, _dag: Option<&Dag>) -> String {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        // 为每个方程创建节点
        for (idx, eq) in file.equations.iter().enumerate() {
            let node_id = format!("node_{}", idx);
            let operator_id = format!(
                "{}.{}",
                file.meta.id.to_lowercase(),
                eq.id.to_lowercase().replace('-', "_")
            );

            nodes.push(json!({
                "id": node_id,
                "operator_id": operator_id,
                "config": {}
            }));

            // 收集输入绑定
            for var in eq.get_variable_refs() {
                inputs.push(json!({
                    "name": var,
                    "bind_to": {
                        "node": node_id,
                        "port": var
                    }
                }));
            }

            for param in eq.get_parameter_refs() {
                inputs.push(json!({
                    "name": param,
                    "bind_to": {
                        "node": node_id,
                        "port": param
                    }
                }));
            }

            // 输出绑定
            outputs.push(json!({
                "name": eq.output,
                "bind_from": {
                    "node": node_id,
                    "port": eq.output
                }
            }));
        }

        // 构建边（基于方程间的依赖）
        for (i, eq_i) in file.equations.iter().enumerate() {
            for (j, eq_j) in file.equations.iter().enumerate() {
                if i != j {
                    // 如果 eq_j 依赖 eq_i 的输出
                    if eq_j.get_variable_refs().contains(&eq_i.output) {
                        edges.push(json!({
                            "from": {
                                "node": format!("node_{}", i),
                                "port": eq_i.output
                            },
                            "to": {
                                "node": format!("node_{}", j),
                                "port": eq_i.output
                            }
                        }));
                    }
                }
            }
        }

        let workflow = json!({
            "name": format!("{} - {}", file.meta.id, file.meta.name_cn),
            "description": file.meta.description,
            "definition": {
                "nodes": nodes,
                "edges": edges,
                "inputs": inputs,
                "outputs": outputs
            }
        });

        serde_json::to_string_pretty(&workflow).unwrap()
    }

    fn generate_dag_json(dag: &Dag) -> String {
        let nodes: Vec<_> = dag
            .nodes
            .iter()
            .map(|n| {
                json!({
                    "id": n.id,
                    "type": format!("{:?}", n.node_type),
                    "module": n.module,
                    "metadata": n.metadata
                })
            })
            .collect();

        let edges: Vec<_> = dag
            .edges
            .iter()
            .map(|e| {
                json!({
                    "from": e.from,
                    "to": e.to,
                    "type": format!("{:?}", e.edge_type)
                })
            })
            .collect();

        let dag_json = json!({
            "nodes": nodes,
            "edges": edges,
            "topological_order": dag.topological_order,
            "modules": dag.get_modules()
        });

        serde_json::to_string_pretty(&dag_json).unwrap()
    }
}
