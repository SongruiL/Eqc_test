//! 网络指标（GA-3）—— 在有向影响图 [`DiGraph`] 上算**描述性**网络度量。
//!
//! 诚实定位（理论笔记 §3）：网络指标是软的、描述性的，价值在于**绑定到具体问题**：
//! - **度 / 介数中心性** → 找**枢纽变量**（计算瓶颈、万物汇聚的状态量）→ 喂 3D 节点大小。
//! - **社区 / 模块度** → **验证/对照手标 `meta.modules`** → 喂 3D 分层/分色。
//! - **DAG 深度** → 计算层数（破环最长路）。
//!
//! 全部**确定性**（无 RNG），守项目"指标/坐标可复现"约束。
//! - 介数：Brandes (2001)，有向、无权，O(VE)。
//! - PageRank：阻尼 0.85，处理悬挂（sink）节点，迭代到收敛。
//! - 社区：确定性贪心模块度（Louvain 单层局部移动，节点按下标固定序、平局取小社区号）。

use std::collections::{HashMap, VecDeque};

use crate::schema::EquationFile;

use super::digraph::DiGraph;

/// 单节点的网络指标。
#[derive(Debug, Clone)]
pub struct NodeMetrics {
    pub node: String,
    pub in_degree: usize,
    pub out_degree: usize,
    /// 介数中心性（有向，未归一）。
    pub betweenness: f64,
    /// PageRank（全图和为 1）。
    pub pagerank: f64,
    /// 计算深度（破环最长路层号，源=0）。
    pub depth: usize,
    /// 所属检测社区编号（0..n_communities）。
    pub community: usize,
}

/// 网络指标报告。
#[derive(Debug, Clone)]
pub struct MetricsReport {
    /// 各节点指标，**按介数降序**（枢纽在前）。
    pub nodes: Vec<NodeMetrics>,
    /// 检测到的社区数。
    pub n_communities: usize,
    /// 检测划分的模块度 Q（越高=社区越内聚）。
    pub modularity_detected: f64,
    /// 作者手标 `meta.modules` 划分的模块度 Q（声明了才算，做对照）；否则 None。
    pub modularity_modules: Option<f64>,
}

/// 对一组方程文件做网络指标分析。
pub fn analyze_metrics(files: &[EquationFile]) -> MetricsReport {
    let g = DiGraph::from_files(files);
    let n = g.len();
    if n == 0 {
        return MetricsReport {
            nodes: vec![],
            n_communities: 0,
            modularity_detected: 0.0,
            modularity_modules: None,
        };
    }

    let betweenness = betweenness(&g);
    let pagerank = pagerank(&g);
    let depth = depths(&g);
    let (community, n_communities) = louvain(&g);

    let (uadj, deg, m2) = undirected(&g);
    let modularity_detected = modularity(&community, &uadj, &deg, m2);
    let modularity_modules = module_partition(files, &g).map(|p| modularity(&p, &uadj, &deg, m2));

    let mut nodes: Vec<NodeMetrics> = (0..n)
        .map(|i| NodeMetrics {
            node: g.nodes[i].clone(),
            in_degree: g.in_degree(i),
            out_degree: g.out_degree(i),
            betweenness: betweenness[i],
            pagerank: pagerank[i],
            depth: depth[i],
            community: community[i],
        })
        .collect();
    // 按介数降序；平局按节点 id 升序（确定性）。
    nodes.sort_by(|a, b| {
        b.betweenness
            .partial_cmp(&a.betweenness)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.node.cmp(&b.node))
    });

    MetricsReport { nodes, n_communities, modularity_detected, modularity_modules }
}

/// Brandes 介数中心性（有向、无权、未归一）。
fn betweenness(g: &DiGraph) -> Vec<f64> {
    let n = g.len();
    let mut cb = vec![0.0_f64; n];
    for s in 0..n {
        let mut stack: Vec<usize> = Vec::new();
        let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut sigma = vec![0.0_f64; n];
        sigma[s] = 1.0;
        let mut dist = vec![-1_i64; n];
        dist[s] = 0;
        let mut q: VecDeque<usize> = VecDeque::new();
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            stack.push(v);
            for &w in g.successors(v) {
                if dist[w] < 0 {
                    dist[w] = dist[v] + 1;
                    q.push_back(w);
                }
                if dist[w] == dist[v] + 1 {
                    sigma[w] += sigma[v];
                    pred[w].push(v);
                }
            }
        }
        let mut delta = vec![0.0_f64; n];
        while let Some(w) = stack.pop() {
            for &v in &pred[w] {
                if sigma[w] != 0.0 {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
            }
            if w != s {
                cb[w] += delta[w];
            }
        }
    }
    cb
}

/// PageRank（阻尼 0.85，悬挂节点质量均摊，迭代到收敛）。
fn pagerank(g: &DiGraph) -> Vec<f64> {
    let n = g.len();
    let d = 0.85_f64;
    let mut pr = vec![1.0 / n as f64; n];
    for _ in 0..200 {
        let dangling: f64 = (0..n).filter(|&i| g.out_degree(i) == 0).map(|i| pr[i]).sum();
        let mut next = vec![(1.0 - d) / n as f64 + d * dangling / n as f64; n];
        for u in 0..n {
            let outd = g.out_degree(u);
            if outd > 0 {
                let share = d * pr[u] / outd as f64;
                for &w in g.successors(u) {
                    next[w] += share;
                }
            }
        }
        let diff: f64 = (0..n).map(|i| (next[i] - pr[i]).abs()).sum();
        pr = next;
        if diff < 1e-12 {
            break;
        }
    }
    pr
}

/// 计算深度（破环最长路）：先定一个确定性节点序，只让"前向边"参与算层，回边忽略。
fn depths(g: &DiGraph) -> Vec<usize> {
    let n = g.len();
    let order = topo_order(g);
    let mut rank = vec![0usize; n];
    for (i, &u) in order.iter().enumerate() {
        rank[u] = i;
    }
    let mut depth = vec![0usize; n];
    for &u in &order {
        for &w in g.successors(u) {
            if rank[u] < rank[w] && depth[w] < depth[u] + 1 {
                depth[w] = depth[u] + 1;
            }
        }
    }
    depth
}

/// 确定性拓扑序（Kahn，按下标小者优先）；环里的节点拓扑不掉、最后按下标补齐。
fn topo_order(g: &DiGraph) -> Vec<usize> {
    let n = g.len();
    let mut indeg: Vec<usize> = (0..n).map(|i| g.in_degree(i)).collect();
    let mut placed = vec![false; n];
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    while let Some(u) = queue.pop_front() {
        if placed[u] {
            continue;
        }
        placed[u] = true;
        order.push(u);
        for &w in g.successors(u) {
            if indeg[w] > 0 {
                indeg[w] -= 1;
                if indeg[w] == 0 {
                    queue.push_back(w);
                }
            }
        }
    }
    for i in 0..n {
        if !placed[i] {
            order.push(i); // 环节点：按下标补
        }
    }
    order
}

/// 无向投影：返回 (对称邻接表 `[(邻居, 权)]`, 各节点加权度, 2m=度之和)。
/// 权 = 两节点间有向边条数（双向各算 1）。
fn undirected(g: &DiGraph) -> (Vec<Vec<(usize, f64)>>, Vec<f64>, f64) {
    let n = g.len();
    let mut w: HashMap<(usize, usize), f64> = HashMap::new();
    for u in 0..n {
        for &v in g.successors(u) {
            if u != v {
                let key = if u < v { (u, v) } else { (v, u) };
                *w.entry(key).or_insert(0.0) += 1.0;
            }
        }
    }
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (&(a, b), &wt) in &w {
        adj[a].push((b, wt));
        adj[b].push((a, wt));
    }
    let deg: Vec<f64> = adj.iter().map(|nbrs| nbrs.iter().map(|(_, w)| *w).sum()).collect();
    let m2: f64 = deg.iter().sum();
    (adj, deg, m2)
}

/// 给定划分的模块度 Q（无向）。`m2` = 2m = 度之和。
fn modularity(comm: &[usize], uadj: &[Vec<(usize, f64)>], deg: &[f64], m2: f64) -> f64 {
    if m2 <= 0.0 {
        return 0.0;
    }
    let ncomm = comm.iter().copied().max().unwrap_or(0) + 1;
    let mut sigma_in = vec![0.0_f64; ncomm]; // 内部边权（双向各计，= 公式里的 2×内部）
    let mut sigma_tot = vec![0.0_f64; ncomm];
    for i in 0..comm.len() {
        sigma_tot[comm[i]] += deg[i];
        for &(j, wt) in &uadj[i] {
            if comm[j] == comm[i] {
                sigma_in[comm[i]] += wt;
            }
        }
    }
    let mut q = 0.0;
    for c in 0..ncomm {
        q += sigma_in[c] / m2 - (sigma_tot[c] / m2).powi(2);
    }
    q
}

/// 确定性贪心模块度（Louvain 单层局部移动）。返回 (各节点社区, 社区数)。
fn louvain(g: &DiGraph) -> (Vec<usize>, usize) {
    let n = g.len();
    let (uadj, deg, m2) = undirected(g);
    if m2 <= 0.0 {
        return ((0..n).collect(), n); // 无边：各自成社区
    }
    let mut comm: Vec<usize> = (0..n).collect();
    let mut sigma_tot: Vec<f64> = deg.clone();

    loop {
        let mut improved = false;
        for i in 0..n {
            let ci = comm[i];
            // 移出 i 所在社区。
            sigma_tot[ci] -= deg[i];
            // 统计 i 到各邻居社区的权（k_i_in）。
            let mut k_in: HashMap<usize, f64> = HashMap::new();
            for &(j, wt) in &uadj[i] {
                if j != i {
                    *k_in.entry(comm[j]).or_insert(0.0) += wt;
                }
            }
            // 候选社区按编号升序遍历（确定性）；以严格大于更新（平局取小号，且不抖动）。
            let mut best_c = ci;
            let mut best_gain = k_in.get(&ci).copied().unwrap_or(0.0) - sigma_tot[ci] * deg[i] / m2;
            let mut cands: Vec<(usize, f64)> = k_in.into_iter().collect();
            cands.sort_by_key(|(c, _)| *c);
            for (c, kic) in cands {
                let gain = kic - sigma_tot[c] * deg[i] / m2;
                if gain > best_gain {
                    best_gain = gain;
                    best_c = c;
                }
            }
            sigma_tot[best_c] += deg[i];
            comm[i] = best_c;
            if best_c != ci {
                improved = true;
            }
        }
        if !improved {
            break;
        }
    }
    // 重标号成 0..k。
    let mut relabel: HashMap<usize, usize> = HashMap::new();
    for c in &comm {
        let next = relabel.len();
        relabel.entry(*c).or_insert(next);
    }
    let k = relabel.len();
    for c in comm.iter_mut() {
        *c = relabel[c];
    }
    (comm, k)
}

/// 作者 `meta.modules` 划分 → 每个图节点的社区编号（若无任何模块声明则 None）。
/// 复用 `build_dag` 设好的子模块字段（声明模块 / 驱动量 / 参数 / 其他）。
fn module_partition(files: &[EquationFile], g: &DiGraph) -> Option<Vec<usize>> {
    if files.iter().all(|f| f.meta.modules.is_empty()) {
        return None;
    }
    let dag = crate::dag::build_dag(files).ok()?;
    let node_mod: HashMap<&str, &str> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n.module.as_str())).collect();
    let mut mod_id: HashMap<String, usize> = HashMap::new();
    let mut part = vec![0usize; g.len()];
    for (i, id) in g.nodes.iter().enumerate() {
        let m = node_mod.get(id.as_str()).copied().unwrap_or("__unmapped");
        let next = mod_id.len();
        let c = *mod_id.entry(m.to_string()).or_insert(next);
        part[i] = c;
    }
    Some(part)
}

#[cfg(test)]
mod tests {
    use super::super::bipartite::tests::toy;
    use super::*;

    fn q(s: &str) -> String {
        format!("T.{s}")
    }

    #[test]
    fn star_center_has_highest_betweenness() {
        // 星形（有向）：c→{a,b,d,e} 不够——介数要中心在最短路上。用 a→c→b, d→c→e 型：
        // 4 个叶子两两经中心 c 通达 → c 介数最高。构造：a→c, d→c（入）, c→b, c→e（出）。
        // 则 a→c→b、a→c→e、d→c→b、d→c→e 四条最短路都过 c。
        let f = toy(vec![
            ("e1", "c", vec!["a", "d"]), // a→c, d→c
            ("e2", "b", vec!["c"]),      // c→b
            ("e3", "e", vec!["c"]),      // c→e
        ]);
        let r = analyze_metrics(&[f]);
        let top = &r.nodes[0]; // 已按介数降序
        assert_eq!(top.node, q("c"), "中心 c 介数应最高");
        assert!(top.betweenness > 0.0);
    }

    #[test]
    fn chain_depth_increments() {
        // y=a·x ; z=y ; w=z → 深度 x/a=0, y=1, z=2, w=3。
        let f = toy(vec![
            ("e1", "y", vec!["a", "x"]),
            ("e2", "z", vec!["y"]),
            ("e3", "w", vec!["z"]),
        ]);
        let r = analyze_metrics(&[f]);
        let depth = |id: &str| r.nodes.iter().find(|m| m.node == id).unwrap().depth;
        assert_eq!(depth(&q("y")), 1);
        assert_eq!(depth(&q("z")), 2);
        assert_eq!(depth(&q("w")), 3);
        assert_eq!(depth(&q("a")), 0);
    }

    #[test]
    fn two_cliques_one_bridge_splits_into_communities() {
        // 两个三角团 {a,b,c} 与 {d,e,f}，一条桥 c—d。社区应 ≥2、模块度 > 0。
        // 用无向化后的连通：方程边构造双向不易，用一串引用造稠密团。
        let f = toy(vec![
            ("e1", "a", vec!["b", "c"]),
            ("e2", "b", vec!["c"]),
            ("e3", "c", vec!["d"]), // 桥
            ("e4", "d", vec!["e", "f"]),
            ("e5", "e", vec!["f"]),
        ]);
        let r = analyze_metrics(&[f]);
        assert!(r.n_communities >= 2, "应检测到 ≥2 个社区，实得 {}", r.n_communities);
        assert!(r.modularity_detected > 0.0, "模块度应为正，实得 {}", r.modularity_detected);
    }

    #[test]
    fn pagerank_sums_to_one() {
        let f = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "z", vec!["y"])]);
        let r = analyze_metrics(&[f]);
        let s: f64 = r.nodes.iter().map(|m| m.pagerank).sum();
        assert!((s - 1.0).abs() < 1e-9, "PageRank 总和应=1，实得 {s}");
    }
}
