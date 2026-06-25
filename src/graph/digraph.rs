//! 有向影响图（网络分析 + 可达性的共用对象）。
//!
//! 节点 = 模型全部符号（参数/驱动/状态/中间/输出）；有向边 = "谁影响谁"：
//! - `ref→output`（数据流：方程每个引用 → 它的输出）
//! - `source→input`（跨模块耦合）
//! - `rate源→state`（积分：状态量受其速率影响）
//! - `prev源→semistate`（延迟寄存器）
//!
//! **含积分/延迟边**是关键：动态模型里 `param→rate→state→可测` 才连得通；也因此本图**可能有环**
//! （跨步状态反馈，如 `TDM→LAI→…→DDM→(积分)→TDM`）——介数/PageRank 不需无环，深度计算另行破环。
//!
//! 节点命名复用 [`super::bipartite::NodeResolver`]（与 GA-1 二部图同一套 `MODULE.name` + source 折叠）。
//! GA-2 可辨识性（可达性）与 GA-3 网络指标共用本图。边已去重、去自环。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::schema::{EquationFile, VariableType};

use super::bipartite::NodeResolver;

/// 有向影响图（邻接表 + 反向邻接表，节点用下标索引）。
#[derive(Debug, Clone)]
pub struct DiGraph {
    /// 节点 id（`MODULE.name`），下标即节点编号。
    pub nodes: Vec<String>,
    /// id → 下标。
    index: HashMap<String, usize>,
    /// 后继（出边）邻接表。
    adj: Vec<Vec<usize>>,
    /// 前驱（入边）邻接表。
    radj: Vec<Vec<usize>>,
}

impl DiGraph {
    /// 从一组方程文件建有向影响图（含积分/延迟/耦合边，已去重去自环）。
    pub fn from_files(files: &[EquationFile]) -> DiGraph {
        let resolver = NodeResolver::build(files);

        // 1) 收集全部节点名 + 原始有向边（字符串对）。
        let mut names: Vec<String> = Vec::new();
        let mut seen_name: HashSet<String> = HashSet::new();
        let mut push_name = |id: String, names: &mut Vec<String>, seen: &mut HashSet<String>| {
            if seen.insert(id.clone()) {
                names.push(id);
            }
        };
        let mut raw: Vec<(String, String)> = Vec::new();

        for f in files {
            let m = &f.meta.id;
            for eq in &f.equations {
                let out = resolver.resolve(m, &eq.output);
                push_name(out.clone(), &mut names, &mut seen_name);
                for name in eq.get_variable_refs().iter().chain(eq.get_parameter_refs().iter()) {
                    let from = resolver.resolve(m, name);
                    push_name(from.clone(), &mut names, &mut seen_name);
                    raw.push((from, out.clone()));
                }
            }
            for (vname, var) in &f.variables {
                let node = resolver.resolve(m, vname);
                push_name(node.clone(), &mut names, &mut seen_name);
                if let Some(src) = &var.rate {
                    raw.push((resolver.resolve(m, src), node.clone())); // rate源 → state
                }
                if let Some(src) = &var.prev {
                    raw.push((resolver.resolve(m, src), node.clone())); // prev源 → semistate
                }
                if var.var_type == VariableType::Input {
                    if let Some((sm, sv)) = var.parse_source() {
                        let up = format!("{sm}.{sv}");
                        if up != node {
                            raw.push((up, node.clone()));
                        }
                    }
                }
            }
            for pname in f.parameters.keys() {
                push_name(format!("{m}.{pname}"), &mut names, &mut seen_name);
            }
        }

        // 2) 建索引。
        let mut index: HashMap<String, usize> = HashMap::new();
        for (i, n) in names.iter().enumerate() {
            index.insert(n.clone(), i);
        }
        // raw 里可能有未登记的名字（理论上 push_name 已覆盖，但 source 折叠的上游名兜底登记）。
        for (a, b) in &raw {
            for id in [a, b] {
                if !index.contains_key(id) {
                    index.insert(id.clone(), names.len());
                    names.push(id.clone());
                }
            }
        }

        let n = names.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut radj: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut edge_set: HashSet<(usize, usize)> = HashSet::new();
        for (a, b) in &raw {
            let (fi, ti) = (index[a], index[b]);
            if fi != ti && edge_set.insert((fi, ti)) {
                adj[fi].push(ti);
                radj[ti].push(fi);
            }
        }

        DiGraph { nodes: names, index, adj, radj }
    }

    /// 节点数。
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    /// 是否空图。
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    /// id → 下标。
    pub fn idx(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
    /// 后继下标。
    pub fn successors(&self, i: usize) -> &[usize] {
        &self.adj[i]
    }
    /// 前驱下标。
    pub fn predecessors(&self, i: usize) -> &[usize] {
        &self.radj[i]
    }
    /// 出度。
    pub fn out_degree(&self, i: usize) -> usize {
        self.adj[i].len()
    }
    /// 入度。
    pub fn in_degree(&self, i: usize) -> usize {
        self.radj[i].len()
    }
    /// 去重边总数。
    pub fn edge_count(&self) -> usize {
        self.adj.iter().map(|s| s.len()).sum()
    }

    /// 从 `start`（id）出发沿出边可达的全部节点 id（不含 start 自身）。GA-2 可辨识性复用。
    pub fn reachable(&self, start: &str) -> HashSet<String> {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut q: VecDeque<usize> = VecDeque::new();
        if let Some(&s) = self.index.get(start) {
            for &v in &self.adj[s] {
                if seen.insert(v) {
                    q.push_back(v);
                }
            }
        }
        while let Some(u) = q.pop_front() {
            for &v in &self.adj[u] {
                if seen.insert(v) {
                    q.push_back(v);
                }
            }
        }
        seen.into_iter().map(|i| self.nodes[i].clone()).collect()
    }
}
