//! 版本结构 diff（GA-4）—— 量化两个模型版本的结构演化（喂 GP 进化溯源 + 3D 生长动画）。
//!
//! 理论 §3.4：精确图编辑距离（GED）是 NP-难，但**版本对比有稳定标签** → 退化成集合差
//! （增删了哪些点/边），便宜且够用。本模块在共享有向影响图 [`DiGraph`] 上做三层 diff：
//! 1. **节点**（added/removed/kept）—— 新长出/删除的变量/参数。
//! 2. **边**（added/removed/kept）—— "重新接线"（谁开始/停止依赖谁）。
//! 3. **方程**（added/removed/**changed**）—— 按 output 对齐；changed = 同 output、表达式形式变了
//!    （`a·x → a·x²` 这类 refs 不变但形式变的演化，纯点/边 diff 抓不到，是 GP 进化的核心信号）。
//!
//! **对齐键 = 本地名**（去掉模块前缀 `MODULE.`）：版本间 `meta.id` 常不同（如 `STRAWBERRY_S4`
//! vs `STRAWBERRY_S8`），用全 id 根本对不齐。本地名对齐让单模块版本对比（GP before/after、
//! S4↔S8）正确。⚠️ 多模块若有跨模块同名变量会碰撞——此为已知边界（版本 diff 的典型对象是单模型）。
//!
//! 距离：`distance` = 图编辑数（增删点 + 增删边，主距离，可解释）；`edge_similarity` = 边 Jaccard
//! （0–1，演化程度）。`changed_equations` 是"同拓扑、换形式"，**不计入 distance**（拓扑没变）。

use std::collections::{HashMap, HashSet};

use crate::schema::EquationFile;

use super::digraph::DiGraph;

/// 一个 diff 节点（本地名 + 角色，角色供 3D 动画上色）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffNode {
    /// 本地名（已去模块前缀）。
    pub id: String,
    /// 角色：`parameter` / Forrester 类（state/rate/...）/ `external`（仅被引用、未声明）。
    pub kind: String,
}

/// 一条方程形式改变（同 output、表达式不同）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqChange {
    pub output: String,
    pub from_id: String,
    pub to_id: String,
}

/// 两版本的结构 diff。
#[derive(Debug, Clone)]
pub struct GraphDiff {
    pub added_nodes: Vec<DiffNode>,
    pub removed_nodes: Vec<DiffNode>,
    pub kept_nodes: usize,
    pub added_edges: Vec<(String, String)>,
    pub removed_edges: Vec<(String, String)>,
    pub kept_edges: usize,
    /// 新增的 output（new 有、old 无对应方程）。
    pub added_equations: Vec<String>,
    /// 删除的 output（old 有、new 无）。
    pub removed_equations: Vec<String>,
    /// 形式改变的方程（同 output、表达式不同）。
    pub changed_equations: Vec<EqChange>,
    /// 图编辑数 = 增删点 + 增删边（不含 changed_equations）。
    pub distance: usize,
    /// 边 Jaccard 相似度（0–1；两版本无边时记 1.0）。
    pub edge_similarity: f64,
}

/// 去模块前缀 → 本地名（`MODULE.name` → `name`；无 `.` 原样返回）。
fn local(id: &str) -> &str {
    id.splitn(2, '.').nth(1).unwrap_or(id)
}

/// 一个模型的（本地名节点集, 本地名有向边集）。
fn node_edge_sets(g: &DiGraph) -> (HashSet<String>, HashSet<(String, String)>) {
    let nodes: HashSet<String> = g.nodes.iter().map(|n| local(n).to_string()).collect();
    let mut edges: HashSet<(String, String)> = HashSet::new();
    for u in 0..g.len() {
        let lu = local(&g.nodes[u]).to_string();
        for &v in g.successors(u) {
            edges.insert((lu.clone(), local(&g.nodes[v]).to_string()));
        }
    }
    (nodes, edges)
}

/// 本地名 → 角色。
fn kind_map(files: &[EquationFile]) -> HashMap<String, String> {
    let mut k = HashMap::new();
    for f in files {
        for pname in f.parameters.keys() {
            k.insert(pname.clone(), "parameter".to_string());
        }
        for (vname, var) in &f.variables {
            k.insert(vname.clone(), var.effective_class().as_str().to_string());
        }
    }
    k
}

/// 本地 output → (方程 id, 表达式规范指纹)。指纹用 Debug 串（`Expr` 不 derive `PartialEq`），
/// 能区分 `a·x` vs `a/x`（refs 同、形式不同）。
fn eq_map(files: &[EquationFile]) -> HashMap<String, (String, String)> {
    let mut m = HashMap::new();
    for f in files {
        for eq in &f.equations {
            m.insert(eq.output.clone(), (eq.id.clone(), format!("{:?}", eq.expression)));
        }
    }
    m
}

/// 对两个模型版本做结构 diff（old → new）。
pub fn diff_models(old: &[EquationFile], new: &[EquationFile]) -> GraphDiff {
    let (on, oe) = node_edge_sets(&DiGraph::from_files(old));
    let (nn, ne) = node_edge_sets(&DiGraph::from_files(new));
    let old_kind = kind_map(old);
    let new_kind = kind_map(new);

    let mk = |id: &str, km: &HashMap<String, String>| DiffNode {
        id: id.to_string(),
        kind: km.get(id).cloned().unwrap_or_else(|| "external".to_string()),
    };

    let mut added_nodes: Vec<DiffNode> = nn.difference(&on).map(|id| mk(id, &new_kind)).collect();
    let mut removed_nodes: Vec<DiffNode> = on.difference(&nn).map(|id| mk(id, &old_kind)).collect();
    added_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    removed_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    let kept_nodes = on.intersection(&nn).count();

    let mut added_edges: Vec<(String, String)> = ne.difference(&oe).cloned().collect();
    let mut removed_edges: Vec<(String, String)> = oe.difference(&ne).cloned().collect();
    added_edges.sort();
    removed_edges.sort();
    let kept_edges = oe.intersection(&ne).count();
    let union = oe.union(&ne).count();
    let edge_similarity = if union == 0 { 1.0 } else { kept_edges as f64 / union as f64 };

    let oeq = eq_map(old);
    let neq = eq_map(new);
    let mut added_equations: Vec<String> =
        neq.keys().filter(|k| !oeq.contains_key(*k)).cloned().collect();
    let mut removed_equations: Vec<String> =
        oeq.keys().filter(|k| !neq.contains_key(*k)).cloned().collect();
    let mut changed_equations: Vec<EqChange> = Vec::new();
    for (out, (oid, ofp)) in &oeq {
        if let Some((nid, nfp)) = neq.get(out) {
            if ofp != nfp {
                changed_equations.push(EqChange {
                    output: out.clone(),
                    from_id: oid.clone(),
                    to_id: nid.clone(),
                });
            }
        }
    }
    added_equations.sort();
    removed_equations.sort();
    changed_equations.sort_by(|a, b| a.output.cmp(&b.output));

    let distance = added_nodes.len() + removed_nodes.len() + added_edges.len() + removed_edges.len();

    GraphDiff {
        added_nodes,
        removed_nodes,
        kept_nodes,
        added_edges,
        removed_edges,
        kept_edges,
        added_equations,
        removed_equations,
        changed_equations,
        distance,
        edge_similarity,
    }
}

#[cfg(test)]
mod tests {
    use super::super::bipartite::tests::toy;
    use super::*;

    #[test]
    fn identical_models_zero_diff() {
        let a = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "z", vec!["y"])]);
        let b = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "z", vec!["y"])]);
        let d = diff_models(&[a], &[b]);
        assert!(d.added_nodes.is_empty() && d.removed_nodes.is_empty());
        assert!(d.added_edges.is_empty() && d.removed_edges.is_empty());
        assert!(d.changed_equations.is_empty());
        assert_eq!(d.distance, 0);
        assert_eq!(d.edge_similarity, 1.0);
    }

    #[test]
    fn added_equation_reports_new_node_and_edges() {
        // new 多一条 w = z·b → 新增节点 w、b（b 是新 ref），新增边 z→w、b→w，新增方程 w。
        let a = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "z", vec!["y"])]);
        let b = toy(vec![
            ("e1", "y", vec!["a", "x"]),
            ("e2", "z", vec!["y"]),
            ("e3", "w", vec!["z", "b"]),
        ]);
        let d = diff_models(&[a], &[b]);
        let added: Vec<&str> = d.added_nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(added.contains(&"w") && added.contains(&"b"), "新增节点 {added:?}");
        assert!(d.added_edges.contains(&("z".to_string(), "w".to_string())));
        assert!(d.added_edges.contains(&("b".to_string(), "w".to_string())));
        assert_eq!(d.added_equations, vec!["w".to_string()]);
        assert!(d.removed_equations.is_empty());
        assert_eq!(d.distance, 2 + 2); // 2 节点 + 2 边
    }

    #[test]
    fn changed_form_same_refs_detected() {
        // 同 output y、同 refs {a,x}，但形式不同：y=a+x → y=a·x。点/边不变，仅 changed。
        let a = toy(vec![("e1", "y", vec!["a", "x"])]); // toy 默认 add
        let mut b = toy(vec![("e1", "y", vec!["a", "x"])]);
        // 把 b 的 e1 表达式换成 mul（toy 默认是 add 链）。
        b.equations[0].expression =
            crate::ast::Expr::mul(crate::ast::Expr::var("a"), crate::ast::Expr::var("x"));
        let d = diff_models(&[a], &[b]);
        assert!(d.added_nodes.is_empty() && d.removed_nodes.is_empty());
        assert!(d.added_edges.is_empty() && d.removed_edges.is_empty(), "refs 没变，边应不变");
        assert_eq!(d.changed_equations.len(), 1);
        assert_eq!(d.changed_equations[0].output, "y");
        assert_eq!(d.distance, 0, "拓扑没变，图编辑距离=0");
        assert!(d.edge_similarity == 1.0);
    }

    #[test]
    fn removed_equation_reported() {
        let a = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "z", vec!["y"])]);
        let b = toy(vec![("e1", "y", vec!["a", "x"])]);
        let d = diff_models(&[a], &[b]);
        assert_eq!(d.removed_equations, vec!["z".to_string()]);
        assert!(d.removed_nodes.iter().any(|n| n.id == "z"));
        assert!(d.removed_edges.contains(&("y".to_string(), "z".to_string())));
    }
}
