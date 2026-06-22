//! DAG 构建器

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::error::{CompileError, CompileResult};
use crate::schema::{EquationFile, VariableType};

/// DAG 节点类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    /// 参数节点
    Parameter,
    /// 输入变量
    Input,
    /// 中间变量
    Intermediate,
    /// 输出变量
    Output,
    /// 方程节点
    Equation,
}

/// DAG 边类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    /// 数据流边
    DataFlow,
    /// 模块耦合边
    ModuleCoupling,
}

/// DAG 节点
#[derive(Debug, Clone)]
pub struct DagNode {
    /// 节点 ID
    pub id: String,
    /// 节点类型
    pub node_type: NodeType,
    /// 所属模块
    pub module: String,
    /// 元数据（IndexMap：保证 JSON 序列化键序确定、可复现）
    pub metadata: IndexMap<String, String>,
}

/// DAG 边
#[derive(Debug, Clone)]
pub struct DagEdge {
    /// 源节点 ID
    pub from: String,
    /// 目标节点 ID
    pub to: String,
    /// 边类型
    pub edge_type: EdgeType,
}

/// 完整 DAG
#[derive(Debug, Clone)]
pub struct Dag {
    /// 节点列表
    pub nodes: Vec<DagNode>,
    /// 边列表
    pub edges: Vec<DagEdge>,
    /// 拓扑排序结果（计算顺序）
    pub topological_order: Vec<String>,
}

impl Dag {
    /// 获取节点
    pub fn get_node(&self, id: &str) -> Option<&DagNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// 获取指向节点的所有边（入边）
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<&DagEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    /// 获取从节点出发的所有边（出边）
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<&DagEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// 获取模块耦合边
    pub fn get_coupling_edges(&self) -> Vec<&DagEdge> {
        self.edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::ModuleCoupling)
            .collect()
    }

    /// 获取模块列表
    pub fn get_modules(&self) -> Vec<String> {
        let mut modules: Vec<String> = self
            .nodes
            .iter()
            .map(|n| n.module.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        modules.sort();
        modules
    }
}

/// 从方程文件构建 DAG
pub fn build_dag(files: &[EquationFile]) -> CompileResult<Dag> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_ids: HashSet<String> = HashSet::new();

    // 1. 添加所有节点
    for file in files {
        let module = &file.meta.id;

        // 添加参数节点
        for (name, param) in &file.parameters {
            let id = format!("{}.{}", module, name);
            if node_ids.insert(id.clone()) {
                let mut metadata = IndexMap::new();
                metadata.insert("name_cn".to_string(), param.name_cn.clone());
                metadata.insert("default".to_string(), param.default.to_string());
                if let Some(ref unit) = param.unit {
                    metadata.insert("unit".to_string(), unit.clone());
                }

                nodes.push(DagNode {
                    id,
                    node_type: NodeType::Parameter,
                    module: module.clone(),
                    metadata,
                });
            }
        }

        // 添加变量节点
        for (name, var) in &file.variables {
            let id = format!("{}.{}", module, name);
            if node_ids.insert(id.clone()) {
                let node_type = match var.var_type {
                    VariableType::Input => NodeType::Input,
                    VariableType::Intermediate => NodeType::Intermediate,
                    VariableType::Output => NodeType::Output,
                };

                let mut metadata = IndexMap::new();
                if let Some(ref desc) = var.description {
                    metadata.insert("description".to_string(), desc.clone());
                }
                if let Some(ref unit) = var.unit {
                    metadata.insert("unit".to_string(), unit.clone());
                }

                nodes.push(DagNode {
                    id,
                    node_type,
                    module: module.clone(),
                    metadata,
                });
            }
        }

        // 添加方程输出节点（如果变量中未定义）
        for eq in &file.equations {
            let id = format!("{}.{}", module, eq.output);
            if node_ids.insert(id.clone()) {
                let mut metadata = IndexMap::new();
                metadata.insert("equation_id".to_string(), eq.id.clone());
                metadata.insert("name".to_string(), eq.name.clone());

                nodes.push(DagNode {
                    id,
                    node_type: NodeType::Intermediate,
                    module: module.clone(),
                    metadata,
                });
            }
        }
    }

    // 2. 添加边
    for file in files {
        let module = &file.meta.id;

        // 方程依赖边
        for eq in &file.equations {
            let output_id = format!("{}.{}", module, eq.output);

            // 变量依赖
            for var_ref in eq.get_variable_refs() {
                let from_id = format!("{}.{}", module, var_ref);
                edges.push(DagEdge {
                    from: from_id,
                    to: output_id.clone(),
                    edge_type: EdgeType::DataFlow,
                });
            }

            // 参数依赖
            for param_ref in eq.get_parameter_refs() {
                let from_id = format!("{}.{}", module, param_ref);
                edges.push(DagEdge {
                    from: from_id,
                    to: output_id.clone(),
                    edge_type: EdgeType::DataFlow,
                });
            }
        }

        // 跨模块依赖边
        for (var_name, var) in &file.variables {
            if var.var_type == VariableType::Input {
                if let Some((source_module, source_var)) = var.parse_source() {
                    let from_id = format!("{}.{}", source_module, source_var);
                    let to_id = format!("{}.{}", module, var_name);

                    edges.push(DagEdge {
                        from: from_id,
                        to: to_id,
                        edge_type: EdgeType::ModuleCoupling,
                    });
                }
            }
        }
    }

    // 3. 计算拓扑排序
    let topological_order = compute_topological_order(&nodes, &edges)?;

    Ok(Dag {
        nodes,
        edges,
        topological_order,
    })
}

/// DAG 粒度层级（结构图「参数/方程/模块」切换用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagLevel {
    /// 变量级（最细，现状）：每个变量/参数一个节点。
    Variable,
    /// 方程级：隐去参数叶子，只留"算什么→喂给谁"的计算骨架。
    Equation,
    /// 模块级：按 `meta.modules` 把方程折叠进各子模块、聚合跨模块边。
    Module,
}

impl DagLevel {
    /// 解析 `?level=`（未知/缺省 → 变量级）。
    pub fn parse(s: &str) -> DagLevel {
        match s.trim() {
            "equation" => DagLevel::Equation,
            "module" => DagLevel::Module,
            _ => DagLevel::Variable,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            DagLevel::Variable => "variable",
            DagLevel::Equation => "equation",
            DagLevel::Module => "module",
        }
    }
}

/// 把变量级 DAG 折叠到指定粒度。变量级原样返回。
pub fn collapse_dag(dag: &Dag, files: &[EquationFile], level: DagLevel) -> Dag {
    match level {
        DagLevel::Variable => dag.clone(),
        DagLevel::Equation => collapse_to_equations(dag),
        DagLevel::Module => collapse_to_modules(dag, files),
    }
}

/// 方程级：丢掉参数节点（及其相连的边），只剩方程/变量/驱动/状态的计算骨架。
fn collapse_to_equations(dag: &Dag) -> Dag {
    let keep: HashSet<&str> = dag
        .nodes
        .iter()
        .filter(|n| n.node_type != NodeType::Parameter)
        .map(|n| n.id.as_str())
        .collect();
    let nodes = dag
        .nodes
        .iter()
        .filter(|n| n.node_type != NodeType::Parameter)
        .cloned()
        .collect();
    let edges = dag
        .edges
        .iter()
        .filter(|e| keep.contains(e.from.as_str()) && keep.contains(e.to.as_str()))
        .cloned()
        .collect();
    Dag { nodes, edges, topological_order: dag.topological_order.clone() }
}

/// 模块级：按 `meta.modules`（模块名→方程id）把每个节点归到子模块，折叠成模块节点 + 聚合跨模块边。
/// 参数节点丢弃；驱动量归「驱动量」；状态/延迟量随其 rate/prev 来源的模块；未列入的归「未分组」。
fn collapse_to_modules(dag: &Dag, files: &[EquationFile]) -> Dag {
    // 1) 节点 id（MODULE.name）→ 子模块名
    let mut sub: HashMap<String, String> = HashMap::new();
    for f in files {
        let mid = &f.meta.id;
        // 方程 id → 模块名
        let mut eqmod: HashMap<&str, &str> = HashMap::new();
        for (mname, ids) in &f.meta.modules {
            for id in ids {
                eqmod.insert(id.as_str(), mname.as_str());
            }
        }
        // 方程输出 → 模块
        for eq in &f.equations {
            let node = format!("{mid}.{}", eq.output);
            let m = eqmod.get(eq.id.as_str()).copied().unwrap_or("未分组").to_string();
            sub.insert(node, m);
        }
        // 驱动量（input）→「驱动量」
        for (vname, var) in &f.variables {
            if var.var_type == VariableType::Input {
                sub.entry(format!("{mid}.{vname}")).or_insert_with(|| "驱动量".to_string());
            }
        }
    }
    // 状态/延迟量（无方程）→ 随其 rate/prev 来源的模块
    for f in files {
        let mid = &f.meta.id;
        for (vname, var) in &f.variables {
            let node = format!("{mid}.{vname}");
            if sub.contains_key(&node) {
                continue;
            }
            if let Some(s) = var.rate.as_ref().or(var.prev.as_ref()) {
                if let Some(m) = sub.get(&format!("{mid}.{s}")).cloned() {
                    sub.insert(node, m);
                    continue;
                }
            }
            sub.insert(node, "未分组".to_string());
        }
    }
    // 2) 模块节点（不在 sub 里的节点=参数，已丢弃）
    let mut modnames: Vec<String> =
        sub.values().cloned().collect::<HashSet<_>>().into_iter().collect();
    modnames.sort();
    let nodes: Vec<DagNode> = modnames
        .iter()
        .map(|m| DagNode {
            id: m.clone(),
            node_type: NodeType::Equation,
            module: m.clone(),
            metadata: IndexMap::new(),
        })
        .collect();
    // 3) 聚合跨模块边
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut edges = Vec::new();
    for e in &dag.edges {
        if let (Some(a), Some(b)) = (sub.get(&e.from), sub.get(&e.to)) {
            if a != b && seen.insert((a.clone(), b.clone())) {
                edges.push(DagEdge {
                    from: a.clone(),
                    to: b.clone(),
                    edge_type: EdgeType::DataFlow,
                });
            }
        }
    }
    Dag { nodes, edges, topological_order: vec![] }
}

/// 计算拓扑排序
fn compute_topological_order(nodes: &[DagNode], edges: &[DagEdge]) -> CompileResult<Vec<String>> {
    // 构建 petgraph 图
    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();

    // 添加节点
    for node in nodes {
        let idx = graph.add_node(node.id.clone());
        node_indices.insert(node.id.clone(), idx);
    }

    // 添加边
    for edge in edges {
        if let (Some(&from_idx), Some(&to_idx)) =
            (node_indices.get(&edge.from), node_indices.get(&edge.to))
        {
            graph.add_edge(from_idx, to_idx, ());
        }
    }

    // 拓扑排序
    match toposort(&graph, None) {
        Ok(sorted) => {
            let order = sorted.iter().map(|idx| graph[*idx].clone()).collect();
            Ok(order)
        }
        Err(cycle) => {
            let cycle_node = graph[cycle.node_id()].clone();
            Err(CompileError::CyclicDependency {
                cycle: vec![cycle_node],
            })
        }
    }
}
