//! Dulmage–Mendelsohn 分解 → 自由变量 + 求解顺序 + 代数环。
//!
//! 有了二部图（[`super::bipartite`]）与匹配（[`super::matching`]），DM 分解把系统切三块：
//! 1. **欠定块**（变量多于方程）→ **自由变量** = EQC 的参数 + 驱动量 + 无方程的状态量。
//! 2. **方定块**（方程数=变量数，有唯一匹配）→ 真正被解出的变量。
//! 3. **超定块**（方程多于变量）→ 冗余/冲突（EQC 里 = 多条方程写同一个 output）。
//!
//! **方定块再细分**：把「方程 → 它依赖且也在方定块里的变量」看成有向图，求 **SCC**（petgraph）：
//! 无环 ⇒ 每个 SCC 是单点 ⇒ **块下三角** ⇒ 逐步求解顺序（= EQC 现有拓扑排序）。
//! 有环 ⇒ 几个变量缩成一个 SCC 块 ⇒ **代数环：该块须联立（隐式）求解**，块之间仍按三角顺序。
//!
//! 这是 EQC 现有「拓扑排序 + 环检测」的严谨完整版：一次给出 ①自由输入 ②块三角顺序
//! ③代数环精确落在哪个块 ④模型是否过/欠定（跑前即报的结构 bug）。
//!
//! 实现取舍（与首席科学家议定）：用作者 `output:` 指派给方定块**定向**（每方程解它的 output），
//! 在此之上做 SCC 块三角——与 EQC 现有计算语义 100% 一致；另用 Hopcroft–Karp 最大匹配
//! 做**独立的**结构奇异性检查（[`super::matching`]）。

use std::collections::{HashMap, HashSet};

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::schema::EquationFile;

use super::bipartite::BipartiteGraph;
use super::matching::{analyze as analyze_matching, MatchingReport};

/// 方定块里的一个求解块（单点 = 普通逐步求解；多点/自环 = 代数环须联立）。
#[derive(Debug, Clone)]
pub struct SolveBlock {
    /// 本块方程键（`MODULE::eq_id`）。
    pub equations: Vec<String>,
    /// 本块解出的变量节点 id。
    pub variables: Vec<String>,
    /// 是否代数环：SCC 含 >1 方程，或单方程 RHS 自引用其 output（如 `y=y+a`）。
    pub is_algebraic_loop: bool,
}

/// 整个模型的结构分析报告。
#[derive(Debug, Clone)]
pub struct StructureReport {
    /// 欠定块 = 自由变量（参数/驱动/无方程状态量），按二部图变量声明序。
    pub free_vars: Vec<String>,
    /// 方定块的求解块，**已按块下三角求解顺序排列**（依赖在前）。
    pub solve_blocks: Vec<SolveBlock>,
    /// 超定：多条方程写同一个 output（冲突/冗余）的方程键。
    pub over_determined: Vec<String>,
    /// 结构是否奇异（最大匹配 < 方程数 ⇒ 无法每方程配 distinct 变量）。
    pub structurally_singular: bool,
    /// 匹配对照报告（作者 output vs Hopcroft–Karp 最大匹配）。
    pub matching: MatchingReport,
}

impl StructureReport {
    /// 代数环块（`is_algebraic_loop` 的 [`SolveBlock`] 引用）。
    pub fn algebraic_loops(&self) -> Vec<&SolveBlock> {
        self.solve_blocks.iter().filter(|b| b.is_algebraic_loop).collect()
    }
    /// 是否存在任何代数环。
    pub fn has_algebraic_loop(&self) -> bool {
        self.solve_blocks.iter().any(|b| b.is_algebraic_loop)
    }
}

/// 对一组方程文件做完整结构分析。
pub fn analyze_structure(files: &[EquationFile]) -> StructureReport {
    let g = BipartiteGraph::from_files(files);
    analyze_graph(&g)
}

/// 直接对已建好的二部图做结构分析（便于复用/测试）。
pub fn analyze_graph(g: &BipartiteGraph) -> StructureReport {
    let matching = analyze_matching(g);

    // 1) 被解出的变量 = 出现在某方程 output 的变量。其余 = 自由变量（欠定块）。
    let solved: HashSet<&str> = g.equations.iter().map(|e| e.output.as_str()).collect();
    let free_vars: Vec<String> = g
        .variables
        .iter()
        .filter(|v| !solved.contains(v.as_str()))
        .cloned()
        .collect();

    // 2) 超定：按 output 分组，>1 条方程的那些 output 对应的所有方程。
    let mut by_output: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in g.equations.iter().enumerate() {
        by_output.entry(e.output.as_str()).or_default().push(i);
    }
    let mut over_determined: Vec<String> = Vec::new();
    for eqs in by_output.values() {
        if eqs.len() > 1 {
            for &i in eqs {
                over_determined.push(g.equations[i].key.clone());
            }
        }
    }
    over_determined.sort();

    // 3) 方定块细分：在方程上建依赖图（producer → consumer），求 SCC 块三角。
    //    producer(var) = 输出该 var 的方程（取第一条；重复 output 已在 over_determined 报告）。
    let mut producer: HashMap<usize, usize> = HashMap::new(); // var_idx → eq_idx
    for (i, e) in g.equations.iter().enumerate() {
        let vi = g.var_idx(&e.output).expect("output 变量必在图中");
        producer.entry(vi).or_insert(i);
    }

    let n_eq = g.equations.len();
    let mut graph: DiGraph<usize, ()> = DiGraph::new();
    let nodes: Vec<NodeIndex> = (0..n_eq).map(|i| graph.add_node(i)).collect();
    let mut self_loop: HashSet<usize> = HashSet::new();
    for (a, e) in g.equations.iter().enumerate() {
        for &rv in &e.rhs_vars {
            if let Some(&p) = producer.get(&rv) {
                // 边 producer(p) → consumer(a)：p 必须先于 a 求解。
                graph.add_edge(nodes[p], nodes[a], ());
                if p == a {
                    self_loop.insert(a); // RHS 自引用 output → 单点代数环
                }
            }
        }
    }

    // tarjan_scc 返回**逆拓扑序**的 SCC；反转即得求解顺序（依赖在前）。
    let mut sccs = tarjan_scc(&graph);
    sccs.reverse();
    let solve_blocks: Vec<SolveBlock> = sccs
        .into_iter()
        .map(|comp| {
            let eq_idxs: Vec<usize> = comp.iter().map(|&n| graph[n]).collect();
            let is_loop = eq_idxs.len() > 1 || eq_idxs.iter().any(|i| self_loop.contains(i));
            let mut equations = Vec::new();
            let mut variables = Vec::new();
            for &i in &eq_idxs {
                equations.push(g.equations[i].key.clone());
                variables.push(g.equations[i].output.clone());
            }
            SolveBlock {
                equations,
                variables,
                is_algebraic_loop: is_loop,
            }
        })
        .collect();

    StructureReport {
        free_vars,
        solve_blocks,
        over_determined,
        structurally_singular: matching.structurally_singular,
        matching,
    }
}

#[cfg(test)]
mod tests {
    use super::super::bipartite::tests::toy;
    use super::*;

    fn q(s: &str) -> String {
        format!("T.{s}")
    }

    #[test]
    fn chain_all_singleton_blocks() {
        // y=a·x ; z=y+b ; w=z·y → 全单点块三角，求解序 e1,e2,e3；无环。
        let f = toy(vec![
            ("e1", "y", vec!["a", "x"]),
            ("e2", "z", vec!["y", "b"]),
            ("e3", "w", vec!["z", "y"]),
        ]);
        let r = analyze_structure(&[f]);
        assert_eq!(r.solve_blocks.len(), 3);
        assert!(r.solve_blocks.iter().all(|b| !b.is_algebraic_loop));
        // 求解顺序 = e1 → e2 → e3
        assert_eq!(r.solve_blocks[0].variables, vec![q("y")]);
        assert_eq!(r.solve_blocks[1].variables, vec![q("z")]);
        assert_eq!(r.solve_blocks[2].variables, vec![q("w")]);
        // 自由变量 = a,x,b（参数/驱动）
        let mut fv = r.free_vars.clone();
        fv.sort();
        assert_eq!(fv, vec![q("a"), q("b"), q("x")]);
        assert!(!r.structurally_singular);
        assert!(r.over_determined.is_empty());
    }

    #[test]
    fn forced_cycle_is_one_scc_block() {
        // y=z+a ; z=y+b → y↔z 互依赖 → 一个 SCC 块（2 方程）= 代数环。
        let f = toy(vec![("e1", "y", vec!["z", "a"]), ("e2", "z", vec!["y", "b"])]);
        let r = analyze_structure(&[f]);
        let loops = r.algebraic_loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].equations.len(), 2);
        let mut vars = loops[0].variables.clone();
        vars.sort();
        assert_eq!(vars, vec![q("y"), q("z")]);
    }

    #[test]
    fn self_reference_is_singleton_loop() {
        // y=y+a → RHS 自引用 output → 单点代数环。
        let f = toy(vec![("e1", "y", vec!["y", "a"])]);
        let r = analyze_structure(&[f]);
        assert!(r.has_algebraic_loop());
        assert_eq!(r.algebraic_loops()[0].equations, vec!["T::e1"]);
    }

    #[test]
    fn duplicate_output_is_over_determined() {
        // 两条方程都写 y → 超定（冲突）。
        let f = toy(vec![("e1", "y", vec!["a"]), ("e2", "y", vec!["b"])]);
        let r = analyze_structure(&[f]);
        assert_eq!(r.over_determined, vec!["T::e1".to_string(), "T::e2".to_string()]);
        assert!(!r.matching.author_is_perfect);
    }

    #[test]
    fn missing_equation_lands_in_free_vars() {
        // 想算 y,z 但漏了 z 的方程：z 只被引用、无 output → 落入自由变量（欠定块），非错误。
        let f = toy(vec![("e1", "y", vec!["z", "a"])]);
        let r = analyze_structure(&[f]);
        assert!(r.free_vars.contains(&q("z")));
        assert!(r.free_vars.contains(&q("a")));
        assert_eq!(r.solve_blocks.len(), 1); // 只有 e1 解 y
        assert!(!r.structurally_singular); // 欠定是正常的，不算奇异
    }

    #[test]
    fn dynamic_state_is_within_step_free_var() {
        // 动态模型缩影（= 草莓 FF/产量路 vs 光合路 的结构本质）：
        // - 状态量 S 无方程（逐步积分，本步是上步携带值）→ 本步是自由变量；
        // - eq y: out = S + a   （产量从携带状态算，只依赖 S/a，本步无方程依赖 → 源块）；
        // - eq r: rate = d·k    （速率从驱动算，只依赖 d/k → 另一个独立源块）。
        // 二者本步互不依赖，各自单点块、无环。这正是显式 Euler 把「本步代数依赖」与
        // 「跨步状态耦合」分开的结构真相。
        let f = toy(vec![("y", "out", vec!["S", "a"]), ("r", "rate", vec!["d", "k"])]);
        let r = analyze_structure(&[f]);
        // S、d、k、a 都是自由变量（S = 无方程状态量；其余 = 参数/驱动）。
        for v in ["S", "d", "k", "a"] {
            assert!(r.free_vars.contains(&q(v)), "{v} 应是自由变量");
        }
        assert_eq!(r.solve_blocks.len(), 2);
        assert!(r.solve_blocks.iter().all(|b| !b.is_algebraic_loop));
        assert!(!r.structurally_singular);
    }

    #[test]
    fn structurally_singular_more_eqs_than_vars() {
        // e1,e2 都只在变量 y 上 → 最大匹配 1 < 2 → 结构奇异。
        let f = toy(vec![("e1", "y", vec!["y"]), ("e2", "y", vec!["y"])]);
        let r = analyze_structure(&[f]);
        assert!(r.structurally_singular);
    }
}
