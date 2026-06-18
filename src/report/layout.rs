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

use std::collections::{BTreeMap, HashMap};

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
        // Forrester 学术风尚未实现，暂用分层占位（切换管道仍可验证）。
        LayoutKind::Layered | LayoutKind::Forrester => layered(nodes, edges, g),
    }
}

/// 自上而下分层：`layer = max(前驱 layer)+1`，最长路径松弛（对 DAG 精确，遇环有界终止）。
fn layered<'a>(nodes: &[&'a str], edges: &[(&'a str, &'a str)], g: Geom) -> Layout<'a> {
    if nodes.is_empty() {
        return Layout { pos: HashMap::new(), width: 80.0, height: 80.0 };
    }
    let mut layer: HashMap<&str, usize> = nodes.iter().map(|&n| (n, 0usize)).collect();
    for _ in 0..nodes.len() {
        let mut changed = false;
        for &(a, b) in edges {
            let la = *layer.get(a).unwrap_or(&0);
            let lb = *layer.get(b).unwrap_or(&0);
            if lb < la + 1 {
                layer.insert(b, la + 1);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
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
}
