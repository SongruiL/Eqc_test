//! 结构图布局：把「算节点坐标」从 SVG 渲染里抽出来，做成**可切换的布局策略**。
//!
//! Forrester 库存-流量图与角色 DAG **共用本模块**；Studio 通过 `/api/report?layout=` 切换，
//! `eqc report --layout` 亦然。设计：布局只产出坐标（纯几何，无 SVG），渲染交给 `report::*_svg`。
//!
//! 已实现：
//! - [`LayoutKind::Layered`]：自上而下、最长路径分层（最初的布局）。
//! - [`LayoutKind::Force`]：力导向有机网络（Fruchterman-Reingold，确定性、可复现）。
//!
//! 占位（暂回退到分层，留待后续）：
//! - [`LayoutKind::Forrester`]：存量横向主干 + 卫星辅助量（学术论文风）。

use crate::schema::VarClass;
use std::collections::{BTreeMap, HashMap, HashSet};

/// 布局风格。Studio 切换条 / `eqc report --layout` 选用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutKind {
    /// 自上而下、最长路径分层（最初的布局）。
    #[default]
    Layered,
    /// Forrester 学术风：存量横向主干 + 卫星辅助量（**暂回退到分层**，后续实现）。
    Forrester,
    /// 力导向有机网络（Fruchterman-Reingold）。
    Force,
}

impl LayoutKind {
    /// 从查询串 / CLI 取值解析（未知值回退到默认 `Layered`）。
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "forrester" | "forr" => LayoutKind::Forrester,
            "force" | "fd" => LayoutKind::Force,
            _ => LayoutKind::Layered,
        }
    }

    /// 规范化字符串（用于 URL / CLI）。
    pub fn as_str(self) -> &'static str {
        match self {
            LayoutKind::Layered => "layered",
            LayoutKind::Forrester => "forrester",
            LayoutKind::Force => "force",
        }
    }

    /// 边是否按「自由方向」绘制（力导向/Forrester 用中心到中心的曲线；分层用上下贝塞尔）。
    pub fn free_edges(self) -> bool {
        !matches!(self, LayoutKind::Layered)
    }
}

/// 节点框尺寸 + 分层间距。
#[derive(Clone, Copy)]
pub struct Geom {
    pub bw: f64,
    pub bh: f64,
    pub hgap: f64,
    pub vgap: f64,
}

/// 布局结果：节点左上角坐标 + 画布尺寸。
pub struct Layout<'a> {
    pub pos: HashMap<&'a str, (f64, f64)>,
    pub width: f64,
    pub height: f64,
}

/// 画布外边距。
const MARGIN: f64 = 24.0;

/// 计算布局。
///
/// - `nodes`：节点 id（其顺序决定分层内的稳定次序，保证输出可复现）。
/// - `edges`：有向边 `(from, to)`。
pub fn compute<'a>(
    nodes: &[&'a str],
    edges: &[(&'a str, &'a str)],
    kind: LayoutKind,
    g: Geom,
) -> Layout<'a> {
    match kind {
        LayoutKind::Force => force_directed(nodes, edges, g),
        // Forrester 学术风需要节点分类（见 [`compute_forrester`]）。此入口无分类信息
        //（如角色 DAG），回退力导向。
        LayoutKind::Forrester => force_directed(nodes, edges, g),
        LayoutKind::Layered => layered(nodes, edges, g),
    }
}

/// 拓扑排序（Kahn，入度为 0 者按输入顺序出队，确定性）。遇环：环上节点排到最后（按输入顺序）。
fn topo_order<'a>(nodes: &[&'a str], edges: &[(&'a str, &'a str)]) -> Vec<&'a str> {
    let mut indeg: HashMap<&str, usize> = nodes.iter().map(|&n| (n, 0usize)).collect();
    let mut succ: HashMap<&str, Vec<&str>> = nodes.iter().map(|&n| (n, Vec::new())).collect();
    for &(a, b) in edges {
        if indeg.contains_key(b) && succ.contains_key(a) {
            *indeg.get_mut(b).unwrap() += 1;
            succ.get_mut(a).unwrap().push(b);
        }
    }
    let mut queue: Vec<&str> = nodes.iter().copied().filter(|n| indeg[n] == 0).collect();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut order: Vec<&str> = Vec::new();
    let mut qi = 0;
    while qi < queue.len() {
        let u = queue[qi];
        qi += 1;
        if !visited.insert(u) {
            continue;
        }
        order.push(u);
        for &v in &succ[u] {
            let d = indeg.get_mut(v).unwrap();
            *d = d.saturating_sub(1);
            if *d == 0 {
                queue.push(v);
            }
        }
    }
    // 环上未访问节点：按输入顺序补在末尾
    for &n in nodes {
        if visited.insert(n) {
            order.push(n);
        }
    }
    order
}

/// 最长路径分层（破环版）：先拓扑排序定一个节点序，**只让"前向边"（from 在 to 之前）参与算层**，
/// 回边（制造环的积分边等）忽略不算层（但照常绘制）。层号由真实依赖深度限住，不会被环顶飞。
fn longest_path_layers<'a>(
    nodes: &[&'a str],
    edges: &[(&'a str, &'a str)],
) -> HashMap<&'a str, usize> {
    let order = topo_order(nodes, edges);
    let rank: HashMap<&str, usize> = order.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    // 只保留前向边的后继表
    let mut fsucc: HashMap<&str, Vec<&str>> = HashMap::new();
    for &(a, b) in edges {
        if let (Some(&ra), Some(&rb)) = (rank.get(a), rank.get(b)) {
            if ra < rb {
                fsucc.entry(a).or_default().push(b);
            }
        }
    }
    let mut layer: HashMap<&str, usize> = nodes.iter().map(|&n| (n, 0usize)).collect();
    // 按拓扑序处理：处理到 u 时 layer[u] 已定，向前向后继松弛
    for &u in &order {
        let lu = layer[u];
        if let Some(vs) = fsucc.get(u) {
            for &v in vs {
                if layer[v] < lu + 1 {
                    layer.insert(v, lu + 1);
                }
            }
        }
    }
    layer
}

/// 自上而下分层（最长路径，最初的布局）。
fn layered<'a>(nodes: &[&'a str], edges: &[(&'a str, &'a str)], g: Geom) -> Layout<'a> {
    if nodes.is_empty() {
        return Layout { pos: HashMap::new(), width: 80.0, height: 80.0 };
    }
    let layer = longest_path_layers(nodes, edges);
    // 按层分组，层内保持 `nodes` 给定次序
    let mut by_layer: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for &n in nodes {
        by_layer.entry(layer[n]).or_default().push(n);
    }
    let mut pos = HashMap::new();
    let mut width = 0.0_f64;
    for (l, ids) in &by_layer {
        let y = *l as f64 * (g.bh + g.vgap) + MARGIN;
        for (i, id) in ids.iter().enumerate() {
            let x = i as f64 * (g.bw + g.hgap) + MARGIN;
            pos.insert(*id, (x, y));
            width = width.max(x + g.bw + MARGIN);
        }
    }
    let height = by_layer
        .keys()
        .max()
        .map(|m| (*m as f64 + 1.0) * (g.bh + g.vgap) + MARGIN)
        .unwrap_or(80.0);
    Layout { pos, width, height }
}

/// 力导向布局（Fruchterman-Reingold）。
///
/// 斥力遍历所有点对（`k²/d`），引力沿边（`d²/k`），逐步降温。节点才几十个，算力无压力。
/// **确定性**：初始位置按下标在螺旋上铺开（不用随机数），故同输入永远生成相同坐标（报告可复现）。
fn force_directed<'a>(nodes: &[&'a str], edges: &[(&'a str, &'a str)], g: Geom) -> Layout<'a> {
    let n = nodes.len();
    if n == 0 {
        return Layout { pos: HashMap::new(), width: 80.0, height: 80.0 };
    }
    if n == 1 {
        let mut pos = HashMap::new();
        pos.insert(nodes[0], (MARGIN, MARGIN));
        return Layout { pos, width: g.bw + 2.0 * MARGIN, height: g.bh + 2.0 * MARGIN };
    }

    let idx: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    // 理想边长 k ≈ 一个节点宽（连线≈框宽，相邻框相接而不重叠，紧凑、短线、不稀疏）；
    // 画框 side = √n·k 收住整体（经典 FR 约束）。
    let k = g.bw;
    let side = (n as f64).sqrt() * k;
    let center = side / 2.0;

    // 确定性初始铺位：阿基米德螺旋（避免完美对称导致的对称困局）
    let mut px = vec![0.0_f64; n];
    let mut py = vec![0.0_f64; n];
    let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt()); // 黄金角
    for i in 0..n {
        let r = side * 0.45 * ((i as f64 + 0.5) / n as f64).sqrt();
        let ang = i as f64 * golden;
        px[i] = center + r * ang.cos();
        py[i] = center + r * ang.sin();
    }

    let epairs: Vec<(usize, usize)> = edges
        .iter()
        .filter_map(|&(a, b)| Some((*idx.get(a)?, *idx.get(b)?)))
        .filter(|(a, b)| a != b)
        .collect();

    // 经典 FR 边框：把布局约束在 side×side 内（防止多节点时斥力累积「炸开」、治稀疏与长线）。
    let frame = side;
    let iters = 500;
    let mut temp = side * 0.10;
    let cool = temp / (iters as f64 + 1.0);

    for _ in 0..iters {
        let mut dx = vec![0.0_f64; n];
        let mut dy = vec![0.0_f64; n];

        // 斥力（所有点对）
        for i in 0..n {
            for j in (i + 1)..n {
                let mut ddx = px[i] - px[j];
                let mut ddy = py[i] - py[j];
                let mut dist = (ddx * ddx + ddy * ddy).sqrt();
                if dist < 0.01 {
                    // 重合时给个确定性的小扰动，按下标错开
                    ddx = 0.1 + 0.01 * (i as f64 - j as f64);
                    ddy = 0.1;
                    dist = (ddx * ddx + ddy * ddy).sqrt();
                }
                let rep = k * k / dist;
                let (ux, uy) = (ddx / dist, ddy / dist);
                dx[i] += ux * rep;
                dy[i] += uy * rep;
                dx[j] -= ux * rep;
                dy[j] -= uy * rep;
            }
        }

        // 引力（沿边）
        for &(a, b) in &epairs {
            let ddx = px[a] - px[b];
            let ddy = py[a] - py[b];
            let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.01);
            let att = dist * dist / k;
            let (ux, uy) = (ddx / dist, ddy / dist);
            dx[a] -= ux * att;
            dy[a] -= uy * att;
            dx[b] += ux * att;
            dy[b] += uy * att;
        }

        // 位移（受温度限幅），并夹在边框内
        for i in 0..n {
            let d = (dx[i] * dx[i] + dy[i] * dy[i]).sqrt().max(0.01);
            let cap = d.min(temp);
            px[i] = (px[i] + dx[i] / d * cap).clamp(0.0, frame);
            py[i] = (py[i] + dy[i] / d * cap).clamp(0.0, frame);
        }
        temp -= cool;
    }

    // 归一化到 MARGIN 起点 + 计算画布尺寸
    let minx = px.iter().cloned().fold(f64::INFINITY, f64::min);
    let miny = py.iter().cloned().fold(f64::INFINITY, f64::min);
    let mut pos = HashMap::new();
    let mut width = 0.0_f64;
    let mut height = 0.0_f64;
    for (i, &id) in nodes.iter().enumerate() {
        let x = px[i] - minx + MARGIN;
        let y = py[i] - miny + MARGIN;
        pos.insert(id, (x, y));
        width = width.max(x + g.bw + MARGIN);
        height = height.max(y + g.bh + MARGIN);
    }
    Layout { pos, width, height }
}

/// Forrester 学术风布局：存量/半状态/速率/边界排成**横向主干**（按依赖层序，材料流左→右）；
/// 其余（辅助/参数/驱动）作为**卫星**，主干钉死、只对卫星做力松弛，让它们就近其相连节点
/// 摆在主干**上下两侧**（保持各自一侧、不压主干线）——贴近作物模型论文的结构图。
///
/// 需要节点分类 `classes`（由 `report::class_of` 提供）。主干不足 2 个（如纯静态模型）→ 回退力导向。
pub fn compute_forrester<'a>(
    nodes: &[&'a str],
    edges: &[(&'a str, &'a str)],
    classes: &HashMap<&'a str, VarClass>,
    g: Geom,
) -> Layout<'a> {
    let n = nodes.len();
    if n == 0 {
        return Layout { pos: HashMap::new(), width: 80.0, height: 80.0 };
    }
    let idx: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    // 主干 = 真正的"物质管道"：存量 + 速率阀门 + 边界云。
    // 半状态（延迟寄存器 X_prev）是记账副本，不上主干，作卫星就近其来源存量。
    let is_backbone = |id: &str| {
        matches!(
            classes.get(id).copied().unwrap_or(VarClass::Auxiliary),
            VarClass::State | VarClass::Rate | VarClass::Boundary
        )
    };

    // 主干顺序：按 (依赖层, 原序) 排，材料流左→右
    let layer = longest_path_layers(nodes, edges);
    let mut bb: Vec<&str> = nodes.iter().copied().filter(|id| is_backbone(id)).collect();
    bb.sort_by_key(|id| (layer[id], idx[id]));

    // 没有像样的"骨架"→ 退回力导向（不强行画主干）
    if bb.len() < 2 {
        return force_directed(nodes, edges, g);
    }

    // 无向邻接（卫星朝相连节点聚拢用）
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        if let (Some(&ia), Some(&ib)) = (idx.get(a), idx.get(b)) {
            adj[ia].push(ib);
            adj[ib].push(ia);
        }
    }

    let spacing = g.bw * 1.1;
    let mut px = vec![0.0_f64; n];
    let mut py = vec![0.0_f64; n];
    let mut pinned = vec![false; n];
    let mut side_of = vec![1.0_f64; n]; // 卫星固定在 +/- 一侧

    // 放主干（y=0 中线）
    for (rank, &id) in bb.iter().enumerate() {
        let i = idx[id];
        px[i] = rank as f64 * spacing;
        py[i] = 0.0;
        pinned[i] = true;
    }
    let bb_w = (bb.len() as f64 - 1.0) * spacing;
    let center_x = bb_w / 2.0;

    // 初始化卫星：x≈相连主干节点的均值；上下交替分到两侧
    let base_off = g.bh * 2.6;
    let mut side = 1.0_f64;
    for &id in nodes {
        let i = idx[id];
        if pinned[i] {
            continue;
        }
        let (mut sx, mut cnt) = (0.0_f64, 0.0_f64);
        for &j in &adj[i] {
            if pinned[j] {
                sx += px[j];
                cnt += 1.0;
            }
        }
        px[i] = if cnt > 0.0 { sx / cnt } else { center_x };
        side_of[i] = side;
        py[i] = side * base_off;
        side = -side;
    }

    // 力松弛（只动卫星，主干钉死）
    let k = g.bw;
    let min_off = g.bh * 1.4; // 卫星不压到主干线上
    let max_off = (bb_w * 0.5).max(g.bh * 9.0);
    let iters = 320;
    let mut temp = spacing;
    let cool = temp / (iters as f64 + 1.0);

    for _ in 0..iters {
        let mut dx = vec![0.0_f64; n];
        let mut dy = vec![0.0_f64; n];
        // 斥力：所有点对（把卫星推离主干与彼此）
        for i in 0..n {
            for j in (i + 1)..n {
                let mut ddx = px[i] - px[j];
                let mut ddy = py[i] - py[j];
                let mut dist = (ddx * ddx + ddy * ddy).sqrt();
                if dist < 0.01 {
                    ddx = 0.1 + 0.01 * (i as f64 - j as f64);
                    ddy = 0.1;
                    dist = (ddx * ddx + ddy * ddy).sqrt();
                }
                let rep = k * k / dist;
                let (ux, uy) = (ddx / dist, ddy / dist);
                dx[i] += ux * rep;
                dy[i] += uy * rep;
                dx[j] -= ux * rep;
                dy[j] -= uy * rep;
            }
        }
        // 引力：沿边
        for &(a, b) in edges {
            if let (Some(&ia), Some(&ib)) = (idx.get(a), idx.get(b)) {
                let ddx = px[ia] - px[ib];
                let ddy = py[ia] - py[ib];
                let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.01);
                let att = dist * dist / k;
                let (ux, uy) = (ddx / dist, ddy / dist);
                dx[ia] -= ux * att;
                dy[ia] -= uy * att;
                dx[ib] += ux * att;
                dy[ib] += uy * att;
            }
        }
        // 仅移动卫星，并固定在各自一侧、不压主干、不过远
        for i in 0..n {
            if pinned[i] {
                continue;
            }
            let d = (dx[i] * dx[i] + dy[i] * dy[i]).sqrt().max(0.01);
            let cap = d.min(temp);
            px[i] += dx[i] / d * cap;
            py[i] += dy[i] / d * cap;
            py[i] = side_of[i] * py[i].abs().clamp(min_off, max_off);
            px[i] = px[i].clamp(-g.bw, bb_w + g.bw);
        }
        temp -= cool;
    }

    // 归一化到 MARGIN 起点 + 画布尺寸
    let minx = px.iter().cloned().fold(f64::INFINITY, f64::min);
    let miny = py.iter().cloned().fold(f64::INFINITY, f64::min);
    let mut pos = HashMap::new();
    let mut width = 0.0_f64;
    let mut height = 0.0_f64;
    for (i, &id) in nodes.iter().enumerate() {
        let x = px[i] - minx + MARGIN;
        let y = py[i] - miny + MARGIN;
        pos.insert(id, (x, y));
        width = width.max(x + g.bw + MARGIN);
        height = height.max(y + g.bh + MARGIN);
    }
    Layout { pos, width, height }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom() -> Geom {
        Geom { bw: 150.0, bh: 40.0, hgap: 26.0, vgap: 70.0 }
    }

    #[test]
    fn test_parse_roundtrip() {
        assert_eq!(LayoutKind::parse("force"), LayoutKind::Force);
        assert_eq!(LayoutKind::parse("forrester"), LayoutKind::Forrester);
        assert_eq!(LayoutKind::parse("layered"), LayoutKind::Layered);
        assert_eq!(LayoutKind::parse("???"), LayoutKind::Layered); // 未知回退
        assert_eq!(LayoutKind::parse(LayoutKind::Force.as_str()), LayoutKind::Force);
    }

    #[test]
    fn test_layered_chain() {
        // a -> b -> c：应分 3 层，y 递增
        let nodes = ["a", "b", "c"];
        let edges = [("a", "b"), ("b", "c")];
        let l = compute(&nodes, &edges, LayoutKind::Layered, geom());
        assert!(l.pos["a"].1 < l.pos["b"].1);
        assert!(l.pos["b"].1 < l.pos["c"].1);
    }

    #[test]
    fn test_force_deterministic_and_bounded() {
        let nodes = ["a", "b", "c", "d", "e"];
        let edges = [("a", "b"), ("b", "c"), ("c", "d"), ("d", "e"), ("a", "e")];
        let l1 = compute(&nodes, &edges, LayoutKind::Force, geom());
        let l2 = compute(&nodes, &edges, LayoutKind::Force, geom());
        // 确定性：两次完全一致
        for id in &nodes {
            assert_eq!(l1.pos[id], l2.pos[id]);
            // 落在画布内、非负
            assert!(l1.pos[id].0 >= 0.0 && l1.pos[id].1 >= 0.0);
        }
        assert!(l1.width > 0.0 && l1.height > 0.0);
    }

    #[test]
    fn test_empty() {
        let l = compute(&[], &[], LayoutKind::Force, geom());
        assert!(l.pos.is_empty());
    }

    #[test]
    fn test_forrester_backbone_horizontal() {
        // 主干 r1→s1, r2→s2（速率/存量）；卫星 a1→r1, p1→r2（辅助/参数）
        let nodes = ["r1", "s1", "r2", "s2", "a1", "p1"];
        let edges = [("r1", "s1"), ("r2", "s2"), ("a1", "r1"), ("p1", "r2"), ("s1", "r2")];
        let mut classes = HashMap::new();
        classes.insert("r1", VarClass::Rate);
        classes.insert("s1", VarClass::State);
        classes.insert("r2", VarClass::Rate);
        classes.insert("s2", VarClass::State);
        classes.insert("a1", VarClass::Auxiliary);
        classes.insert("p1", VarClass::Parameter);
        let l = compute_forrester(&nodes, &edges, &classes, geom());
        // 主干 4 个节点共线（同一中线 y）
        let yb = l.pos["r1"].1;
        for id in ["s1", "r2", "s2"] {
            assert!((l.pos[id].1 - yb).abs() < 1e-6, "{id} 应与主干共线");
        }
        // 卫星离开主干线
        assert!((l.pos["a1"].1 - yb).abs() > 1.0);
        assert!((l.pos["p1"].1 - yb).abs() > 1.0);
    }

    #[test]
    fn test_layered_cycle_bounded() {
        // 含环 a→b→c→a（+ d→a）：层号应被节点数限住，不会"环顶飞"
        let nodes = ["a", "b", "c", "d"];
        let edges = [("a", "b"), ("b", "c"), ("c", "a"), ("d", "a")];
        let l = compute(&nodes, &edges, LayoutKind::Layered, geom());
        assert_eq!(l.pos.len(), 4);
        assert!(l.height < (nodes.len() as f64 + 2.0) * 200.0, "高度应有界，实际 {}", l.height);
    }

    #[test]
    fn test_forrester_fallback_no_backbone() {
        // 全是辅助量、无主干 → 回退力导向，不 panic、坐标齐全
        let nodes = ["a", "b", "c"];
        let edges = [("a", "b"), ("b", "c")];
        let mut classes = HashMap::new();
        for id in &nodes {
            classes.insert(*id, VarClass::Auxiliary);
        }
        let l = compute_forrester(&nodes, &edges, &classes, geom());
        assert_eq!(l.pos.len(), 3);
    }
}
