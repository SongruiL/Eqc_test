//! 二部图最大匹配（Hopcroft–Karp）+ 与作者 `output:` 指派的对照。
//!
//! **匹配** = 一组两两不共端点的边；**完美匹配** = 覆盖每个方程（每方程配到一个**各不相同**的变量）。
//! 结构适定性的**必要条件**：存在覆盖全部方程的匹配 ⇒ 结构非奇异；找不到 ⇒ 某处过/欠定（结构奇异）。
//!
//! ⚠️ 结构 ≠ 数值：结构非奇异是必要非充分（系数恰好抵消仍可能数值奇异）。这里只做便宜的结构先验。
//!
//! 算法：Hopcroft–Karp，O(E·√V)，BFS 分层 + DFS 找增广路。左=方程，右=变量。

use super::bipartite::BipartiteGraph;

/// 最大匹配结果 + 与作者匹配的对照报告。
#[derive(Debug, Clone)]
pub struct MatchingReport {
    /// 算法求得的最大匹配大小。
    pub max_matching_size: usize,
    /// 方程总数。
    pub n_equations: usize,
    /// 作者 `output:` 是否本身就是一个覆盖全部方程的完美匹配（各 output 互不相同）。
    pub author_is_perfect: bool,
    /// 结构是否奇异：最大匹配 < 方程数 ⇒ 无法给每方程配 distinct 变量 ⇒ 过/欠定。
    pub structurally_singular: bool,
    /// 最大匹配是否唯一（best-effort：在匹配子图上找 M-交替环；找不到 = 唯一）。
    /// `None` = 未判定。非唯一 ⇒ 作者在多个合法 output 指派中做了一个选择。
    pub unique: Option<bool>,
    /// 算法匹配里、与作者 output 指派**不同**的方程键（仅当两者都把方程匹配上、但配到不同变量）。
    pub differs_from_author: Vec<String>,
}

/// 用 Hopcroft–Karp 求方程→变量的最大匹配。
///
/// 返回 `match_eq[eq_idx] = Some(var_idx)`（未匹配 = None）。
pub fn max_matching(g: &BipartiteGraph) -> Vec<Option<usize>> {
    let adj = g.eq_adjacency();
    let n_eq = g.n_equations();
    let n_var = g.n_variables();

    let mut match_eq: Vec<Option<usize>> = vec![None; n_eq]; // 方程 → 变量
    let mut match_var: Vec<Option<usize>> = vec![None; n_var]; // 变量 → 方程

    const INF: i64 = i64::MAX;
    let mut dist: Vec<i64> = vec![INF; n_eq];

    // BFS：给未匹配方程分层，返回是否存在增广路。
    let bfs = |match_eq: &Vec<Option<usize>>,
               match_var: &Vec<Option<usize>>,
               dist: &mut Vec<i64>|
     -> bool {
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for u in 0..n_eq {
            if match_eq[u].is_none() {
                dist[u] = 0;
                queue.push_back(u);
            } else {
                dist[u] = INF;
            }
        }
        let mut found = false;
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if let Some(w) = match_var[v] {
                    if dist[w] == INF {
                        dist[w] = dist[u] + 1;
                        queue.push_back(w);
                    }
                } else {
                    found = true; // 到达一个未匹配变量 = 增广路存在
                }
            }
        }
        found
    };

    // DFS：沿分层找一条增广路并翻转。
    fn dfs(
        u: usize,
        adj: &[Vec<usize>],
        match_eq: &mut Vec<Option<usize>>,
        match_var: &mut Vec<Option<usize>>,
        dist: &mut Vec<i64>,
    ) -> bool {
        for i in 0..adj[u].len() {
            let v = adj[u][i];
            let ok = match match_var[v] {
                None => true,
                Some(w) => {
                    if dist[w] == dist[u] + 1 {
                        dfs(w, adj, match_eq, match_var, dist)
                    } else {
                        false
                    }
                }
            };
            if ok {
                match_var[v] = Some(u);
                match_eq[u] = Some(v);
                return true;
            }
        }
        dist[u] = i64::MAX;
        false
    }

    while bfs(&match_eq, &match_var, &mut dist) {
        for u in 0..n_eq {
            if match_eq[u].is_none()
                && dfs(u, &adj, &mut match_eq, &mut match_var, &mut dist)
            {
                // 匹配在 dfs 内已更新
            }
        }
    }

    match_eq
}

/// 对照作者 `output:` 指派，生成 [`MatchingReport`]。
pub fn analyze(g: &BipartiteGraph) -> MatchingReport {
    let m = max_matching(g);
    let max_size = m.iter().filter(|x| x.is_some()).count();
    let n_eq = g.n_equations();

    // 作者匹配是否完美：各 output 互不相同 ⇔ output 变量集合大小 == 方程数。
    let author = g.author_matching();
    let mut seen = std::collections::HashSet::new();
    let author_is_perfect = author.iter().all(|&v| seen.insert(v)) && author.len() == n_eq;

    // 算法匹配与作者匹配的差异（仅对两者都匹配上的方程比较）。
    let mut differs = Vec::new();
    for (i, eq) in g.equations.iter().enumerate() {
        if let Some(mv) = m[i] {
            if mv != author[i] {
                differs.push(eq.key.clone());
            }
        }
    }

    MatchingReport {
        max_matching_size: max_size,
        n_equations: n_eq,
        author_is_perfect,
        structurally_singular: max_size < n_eq,
        unique: Some(is_unique_matching(g, &m)),
        differs_from_author: differs,
    }
}

/// best-effort 最大匹配唯一性：在「匹配边 + 非匹配边」构成的有向图里找 M-交替环。
/// 存在交替环 ⇒ 可沿环翻转得到另一个同样大小的最大匹配 ⇒ 非唯一。
///
/// 构图：对每条二部边 (u=eq, v=var)，若是匹配边则 var→eq 方向，否则 eq→var 方向。
/// 该有向图里的任何有向环都对应一条 M-交替闭合路。用 DFS 找环即可。
fn is_unique_matching(g: &BipartiteGraph, m: &[Option<usize>]) -> bool {
    let n_eq = g.n_equations();
    let n_var = g.n_variables();
    // 节点编号：方程 0..n_eq，变量 n_eq..n_eq+n_var。
    let var_node = |v: usize| n_eq + v;
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n_eq + n_var];
    let matched_var: Vec<bool> = {
        let mut mv = vec![false; n_var];
        for &mo in m.iter().flatten() {
            mv[mo] = true;
        }
        mv
    };
    for (u, eq) in g.equations.iter().enumerate() {
        for &v in &eq.var_indices {
            let is_matched_edge = m[u] == Some(v);
            if is_matched_edge {
                out[var_node(v)].push(u); // var → eq
            } else {
                out[u].push(var_node(v)); // eq → var
            }
        }
    }
    // 只在被匹配覆盖的子图里找环（自由顶点上的交替路是增广路，已被最大匹配排除）。
    // 标准 DFS 找有向环。
    let n = n_eq + n_var;
    let mut color = vec![0u8; n]; // 0=白 1=灰 2=黑
    fn has_cycle(u: usize, out: &[Vec<usize>], color: &mut [u8]) -> bool {
        color[u] = 1;
        for &w in &out[u] {
            if color[w] == 1 {
                return true;
            }
            if color[w] == 0 && has_cycle(w, out, color) {
                return true;
            }
        }
        color[u] = 2;
        false
    }
    // 只从「被匹配的变量」出发能进入环（环必含至少一条匹配边）。
    for v in 0..n_var {
        if matched_var[v] && color[var_node(v)] == 0 && has_cycle(var_node(v), &out, &mut color) {
            return false; // 找到交替环 → 非唯一
        }
    }
    // 也兜一遍其余白节点（保险）。
    for u in 0..n {
        if color[u] == 0 && has_cycle(u, &out, &mut color) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::super::bipartite::tests::toy;
    use super::*;

    #[test]
    fn chain_is_perfect_unique() {
        let f = toy(vec![
            ("e1", "y", vec!["a", "x"]),
            ("e2", "z", vec!["y", "b"]),
            ("e3", "w", vec!["z", "y"]),
        ]);
        let g = BipartiteGraph::from_files(&[f]);
        let r = analyze(&g);
        assert_eq!(r.max_matching_size, 3);
        assert!(r.author_is_perfect);
        assert!(!r.structurally_singular);
        assert_eq!(r.unique, Some(true));
    }

    #[test]
    fn duplicate_output_is_singular() {
        // 两条方程都写 y → 无法各配 distinct 变量（y,z 只有 2 个解但 e1,e2 抢 y）
        // e1—{y,a} e2—{y,b}：最大匹配只能给一条配 y、另一条只能配 a/b。
        // 其实 a,b 是参数自由变量；结构上 e1→y,e2→b 仍可匹配 → 不奇异。
        // 用真正过定：两方程、只有一个变量。
        let f = toy(vec![("e1", "y", vec!["y"]), ("e2", "y", vec!["y"])]);
        let g = BipartiteGraph::from_files(&[f]);
        let r = analyze(&g);
        // 变量只有 {y}，两方程 → 最大匹配 = 1 < 2 → 奇异
        assert_eq!(g.n_variables(), 1);
        assert_eq!(r.max_matching_size, 1);
        assert!(r.structurally_singular);
        assert!(!r.author_is_perfect); // 两 output 都是 y，不 distinct
    }

    #[test]
    fn non_unique_matching_detected() {
        // e1—{x,y} e2—{x,y}：可 (e1→x,e2→y) 或 (e1→y,e2→x) → 非唯一。
        let f = toy(vec![("e1", "x", vec!["y"]), ("e2", "y", vec!["x"])]);
        let g = BipartiteGraph::from_files(&[f]);
        let r = analyze(&g);
        assert_eq!(r.max_matching_size, 2);
        assert_eq!(r.unique, Some(false));
    }
}
