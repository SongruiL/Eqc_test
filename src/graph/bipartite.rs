//! 变量-方程二部图（结构分析的核心对象）
//!
//! 把一组 [`EquationFile`] 重组成**无向二部图** G = (E ∪ V, A)：
//! - 方程节点 E、变量节点 V（参数/驱动/状态/中间/输出**都算变量节点**——
//!   让后续 DM 分解**自动**把「自由变量 = 参数+驱动」分到欠定块，无需预分类）；
//! - 边 (e, v) ⇔ 变量 v 在方程 e 中出现。关键：EQC 里 `eq.output` 是 LHS、**不在** refs 里，
//!   所以一个方程的边集 = `refs ∪ {output}`（对上理论笔记的例子 `eq1—{a,x,y}`）。
//!
//! 与 [`crate::dag::build_dag`] 的区别：DAG 是**有向**的（每方程靠 `output:` 指向它解的变量）；
//! 二部图是**无向**的，不预设谁解谁——`output:` 只是作者手工指定的**一个匹配**（见 [`super::matching`]）。
//!
//! 节点命名复用 DAG 约定 `MODULE.name`；跨模块 `source:` 输入折叠进上游 output 节点
//! （即耦合边两端归一成同一个变量节点），使多模块系统作为**一个**结构系统分析。

use std::collections::HashMap;

use crate::schema::{EquationFile, VariableType};

/// 节点命名的**单一真相源**：把「(模块, 本地名)」规范化成图节点 id `MODULE.name`，
/// 并把跨模块 `source:` 输入折叠进上游 output 节点（耦合边两端归一）。
///
/// GA-1 二部图与 GA-2 有向影响图共用，保证两张图节点命名一致。
#[derive(Debug, Clone)]
pub struct NodeResolver {
    /// (模块 id, 本地变量名) → 规范化节点 id。
    canon: HashMap<(String, String), String>,
}

impl NodeResolver {
    /// 扫描所有文件的变量声明，建解析表。
    pub fn build(files: &[EquationFile]) -> NodeResolver {
        let mut canon: HashMap<(String, String), String> = HashMap::new();
        for f in files {
            let m = &f.meta.id;
            for (vname, var) in &f.variables {
                let id = if var.var_type == VariableType::Input {
                    match var.parse_source() {
                        Some((sm, sv)) => format!("{sm}.{sv}"),
                        None => format!("{m}.{vname}"),
                    }
                } else {
                    format!("{m}.{vname}")
                };
                canon.insert((m.clone(), vname.clone()), id);
            }
        }
        NodeResolver { canon }
    }

    /// 在某模块上下文里规范化一个被引用/输出的名字（未声明的名字 → 直接 `模块.名字`）。
    pub fn resolve(&self, module: &str, name: &str) -> String {
        self.canon
            .get(&(module.to_string(), name.to_string()))
            .cloned()
            .unwrap_or_else(|| format!("{module}.{name}"))
    }
}

/// 二部图里的一个方程节点。
#[derive(Debug, Clone)]
pub struct EqNode {
    /// 全局唯一键：`MODULE::eq_id`（用 `::` 与变量节点的 `MODULE.name` 区分书写习惯）。
    pub key: String,
    /// 所属模块 id。
    pub module: String,
    /// 方程原始 id（如 "PHOTO-01"）。
    pub eq_id: String,
    /// 作者指定的输出变量节点 id（已规范化、已折叠 source）。= 作者匹配里这条方程配到的变量。
    pub output: String,
    /// 本方程**触及**的全部变量节点下标（= refs ∪ {output}，已去重）。索引指向 [`BipartiteGraph::variables`]。
    pub var_indices: Vec<usize>,
    /// 本方程 **RHS 变量引用**的节点下标（仅 `get_variable_refs`，不含参数、不含 LHS——
    /// 但若 RHS 自引用 output（如 `y=y+a`）则**含** output。供 DM 方定块建依赖图/查自环用）。
    pub rhs_vars: Vec<usize>,
}

/// 变量-方程二部图。
#[derive(Debug, Clone)]
pub struct BipartiteGraph {
    /// 方程节点（左侧），下标即 eq_idx。
    pub equations: Vec<EqNode>,
    /// 变量节点 id（右侧，规范化后的 `MODULE.name`），下标即 var_idx。
    pub variables: Vec<String>,
    /// 无向边 `(eq_idx, var_idx)`，含 output 边、已去重。
    pub edges: Vec<(usize, usize)>,
    /// 变量 id → var_idx（便于查找）。
    var_index: HashMap<String, usize>,
}

impl BipartiteGraph {
    /// 从一组方程文件构造二部图（单/多文件均可；多文件按 `source:` 折叠成一个系统）。
    pub fn from_files(files: &[EquationFile]) -> BipartiteGraph {
        // 1) 节点命名走共享解析器（跨模块 source: 折叠进上游 output）。
        let resolver = NodeResolver::build(files);
        let resolve = |module: &str, name: &str| -> String { resolver.resolve(module, name) };

        let mut variables: Vec<String> = Vec::new();
        let mut var_index: HashMap<String, usize> = HashMap::new();
        let mut intern = |id: String, vars: &mut Vec<String>, idx: &mut HashMap<String, usize>| -> usize {
            if let Some(&i) = idx.get(&id) {
                i
            } else {
                let i = vars.len();
                idx.insert(id.clone(), i);
                vars.push(id);
                i
            }
        };

        let mut equations: Vec<EqNode> = Vec::new();
        let mut edges: Vec<(usize, usize)> = Vec::new();

        // 2) 逐方程建节点 + 边（refs ∪ {output}）。
        for f in files {
            let m = &f.meta.id;
            for eq in &f.equations {
                let output_id = resolve(m, &eq.output);
                let out_idx = intern(output_id.clone(), &mut variables, &mut var_index);

                // RHS 变量引用（仅变量、保留自引用）。
                let rhs_vars: Vec<usize> = eq
                    .get_variable_refs()
                    .iter()
                    .map(|name| intern(resolve(m, name), &mut variables, &mut var_index))
                    .collect();

                // 触及的变量节点：先放 output，再放 refs（变量 + 参数），去重。
                let mut touched: Vec<usize> = vec![out_idx];
                for name in eq.get_variable_refs().iter().chain(eq.get_parameter_refs().iter()) {
                    let vid = resolve(m, name);
                    let vi = intern(vid, &mut variables, &mut var_index);
                    if !touched.contains(&vi) {
                        touched.push(vi);
                    }
                }

                let eq_idx = equations.len();
                for &vi in &touched {
                    edges.push((eq_idx, vi));
                }
                equations.push(EqNode {
                    key: format!("{m}::{}", eq.id),
                    module: m.clone(),
                    eq_id: eq.id.clone(),
                    output: output_id,
                    var_indices: touched,
                    rhs_vars,
                });
            }
        }

        // 3) 把仅在变量/参数表里声明、却从未被任何方程触及的孤立符号也补成节点
        //    （结构上它们是无边的自由变量；DM 会归入欠定块）。
        for f in files {
            let m = &f.meta.id;
            for vname in f.variables.keys() {
                let id = resolve(m, vname);
                intern(id, &mut variables, &mut var_index);
            }
            for pname in f.parameters.keys() {
                let id = format!("{m}.{pname}");
                intern(id, &mut variables, &mut var_index);
            }
        }

        BipartiteGraph {
            equations,
            variables,
            edges,
            var_index,
        }
    }

    /// 变量 id → var_idx。
    pub fn var_idx(&self, id: &str) -> Option<usize> {
        self.var_index.get(id).copied()
    }

    /// 方程数。
    pub fn n_equations(&self) -> usize {
        self.equations.len()
    }

    /// 变量数。
    pub fn n_variables(&self) -> usize {
        self.variables.len()
    }

    /// 每个方程触及的变量下标（邻接表，供匹配算法用）。
    pub fn eq_adjacency(&self) -> Vec<Vec<usize>> {
        self.equations.iter().map(|e| e.var_indices.clone()).collect()
    }

    /// 作者匹配：eq_idx → 它的 output 变量 var_idx。
    pub fn author_matching(&self) -> Vec<usize> {
        self.equations
            .iter()
            .map(|e| self.var_index[&e.output])
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::schema::{Equation, Metadata};

    /// 造一个最小单文件模型：`equations` = (id, output, rhs_var_refs)。供 graph 模块各测试共用。
    pub(crate) fn toy(equations: Vec<(&str, &str, Vec<&str>)>) -> EquationFile {
        let eqs = equations
            .into_iter()
            .map(|(id, output, deps)| {
                let expr = if deps.is_empty() {
                    Expr::constant(1.0)
                } else {
                    deps.into_iter().map(Expr::var).reduce(Expr::add).unwrap()
                };
                Equation {
                    id: id.to_string(),
                    name: id.to_string(),
                    output: output.to_string(),
                    expression: expr,
                    formula_display: None,
                    reference: None,
                    gp_target: None,
                 instance: None }
            })
            .collect();
        EquationFile {
            meta: Metadata {
                id: "T".to_string(),
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
                modules: Default::default(),
            },
            parameters: Default::default(),
            variables: Default::default(),
            equations: eqs,
         structure: None }
    }

    #[test]
    fn chain_graph_shape() {
        // y=a·x ; z=y+b ; w=z·y  →  eq1—{a,x,y} eq2—{b,y,z} eq3—{y,z,w}
        let f = toy(vec![
            ("e1", "y", vec!["a", "x"]),
            ("e2", "z", vec!["y", "b"]),
            ("e3", "w", vec!["z", "y"]),
        ]);
        let g = BipartiteGraph::from_files(&[f]);
        assert_eq!(g.n_equations(), 3);
        // 变量节点：y,a,x,z,b,w = 6
        assert_eq!(g.n_variables(), 6);
        // 作者匹配把 e1→y e2→z e3→w
        let am = g.author_matching();
        assert_eq!(g.variables[am[0]], "T.y");
        assert_eq!(g.variables[am[2]], "T.w");
        // e1 触及 {y,a,x} = 3 个
        assert_eq!(g.equations[0].var_indices.len(), 3);
        assert!(g.var_idx("T.a").is_some());
    }
}
