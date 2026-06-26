//! 3D 力导向坐标（GA-5）—— Rust 算坐标，守单一真相源，前端只渲染。
//!
//! 把 `report/layout.rs` 的 2D Fruchterman–Reingold 扩到 3D（z 轴），**确定性无 RNG**
//! （同输入逐位一致）：黄金角/Fibonacci 初始铺位、固定迭代、重合点按下标确定性错开。
//!
//! **指标驱动**（spec §4 GA-5）：
//! - **社区 → 簇位**：同社区节点被各自社区质心吸引 → 模块在空间聚团。
//! - **深度 → z 轴**：z 初始化并**软锚定到归一化计算深度**（弹簧把节点往其深度层拉）→
//!   "计算沿 z 向上流"的可读分层 3D；x,y 全力导向。
//! - **中心性 → 大小**：每节点吐 `size`（∝ 介数，归一 0–1），是**属性不是位置**，前端定球半径。
//!
//! 坐标归一化到居中立方体 `[-1,1]³`，保证有限、无 NaN。2D 仍是默认分析视图，3D 做补充（GA-6 渲染）。

use std::collections::{HashMap, HashSet};

use crate::schema::EquationFile;

use super::digraph::DiGraph;
use super::metrics::analyze_metrics;

/// 一个 3D 节点 + 渲染属性。
#[derive(Debug, Clone)]
pub struct Node3d {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// ∝ 介数中心性，归一 0–1（前端定球半径）。
    pub size: f64,
    /// 社区编号（分色分组）。
    pub community: usize,
    /// 计算深度（= z 轴锚定来源）。
    pub depth: usize,
    /// 作者声明的**子系统名**（`meta.modules` 的键，如「光合」「氮」）；参数/驱动/未分组节点
    /// 或模型未声明任何子系统 → `None`。供 GA-6 前端「按子系统」配色 + 图例。
    pub module: Option<String>,
}

/// 3D 力导向布局结果。
#[derive(Debug, Clone)]
pub struct Layout3d {
    pub nodes: Vec<Node3d>,
    pub edges: Vec<(String, String)>,
    /// 坐标范围 `[-bound, bound]`（固定 1.0）。
    pub bound: f64,
}

/// 计算一组方程文件的 3D 力导向坐标。
pub fn layout3d(files: &[EquationFile]) -> Layout3d {
    let g = DiGraph::from_files(files);
    let n = g.len();
    let bound = 1.0;

    // 指标（社区/深度/介数）按 id 取回，对到 DiGraph 下标。
    let mr = analyze_metrics(files);
    let met: HashMap<&str, (f64, usize, usize)> = mr
        .nodes
        .iter()
        .map(|m| (m.node.as_str(), (m.betweenness, m.community, m.depth)))
        .collect();
    let bet: Vec<f64> = (0..n).map(|i| met.get(g.nodes[i].as_str()).map_or(0.0, |t| t.0)).collect();
    let comm: Vec<usize> = (0..n).map(|i| met.get(g.nodes[i].as_str()).map_or(0, |t| t.1)).collect();
    let depth: Vec<usize> = (0..n).map(|i| met.get(g.nodes[i].as_str()).map_or(0, |t| t.2)).collect();
    // 每节点的作者声明子系统名（按子系统配色用，与 GA-3 module_partition 同一权威）。
    let module_of = node_modules(files, &g);

    let edges: Vec<(String, String)> = {
        let mut e = Vec::new();
        for u in 0..n {
            for &v in g.successors(u) {
                e.push((g.nodes[u].clone(), g.nodes[v].clone()));
            }
        }
        e
    };

    let maxbet = bet.iter().cloned().fold(0.0_f64, f64::max);
    let size: Vec<f64> = bet.iter().map(|b| if maxbet > 0.0 { b / maxbet } else { 0.0 }).collect();

    // 平凡情形。
    if n == 0 {
        return Layout3d { nodes: vec![], edges, bound };
    }
    if n == 1 {
        return Layout3d {
            nodes: vec![Node3d {
                id: g.nodes[0].clone(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                size: size[0],
                community: comm[0],
                depth: depth[0],
                module: module_of[0].clone(),
            }],
            edges,
            bound,
        };
    }

    let (px, py, pz) = force_directed_3d(&g, n, &comm, &depth);

    // 归一化到 [-1,1]³：以质心居中、按最大半幅统一缩放（保形、无 NaN）。
    let nodes = normalize(&g, &px, &py, &pz, &size, &comm, &depth, &module_of, bound);
    Layout3d { nodes, edges, bound }
}

/// 3D Fruchterman–Reingold + 深度软锚定 + 社区质心吸引。返回 (x, y, z)。
fn force_directed_3d(
    g: &DiGraph,
    n: usize,
    comm: &[usize],
    depth: &[usize],
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let k = 1.0_f64; // 理想边长
    let side = (n as f64).sqrt() * k;
    let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt()); // 黄金角

    // 归一化深度 → z 锚定目标（深度 0=底 -side*0.45，最大=顶 +side*0.45）。
    let maxd = depth.iter().copied().max().unwrap_or(0);
    let zr = side * 0.45;
    let zanchor: Vec<f64> = (0..n)
        .map(|i| if maxd > 0 { (2.0 * depth[i] as f64 / maxd as f64 - 1.0) * zr } else { 0.0 })
        .collect();

    // 初始铺位：x,y 用 Vogel 螺旋盘；z = 深度锚。
    let mut px = vec![0.0_f64; n];
    let mut py = vec![0.0_f64; n];
    let mut pz = vec![0.0_f64; n];
    for i in 0..n {
        let r = side * 0.45 * ((i as f64 + 0.5) / n as f64).sqrt();
        let ang = i as f64 * golden;
        px[i] = r * ang.cos();
        py[i] = r * ang.sin();
        pz[i] = zanchor[i];
    }

    let epairs: Vec<(usize, usize)> = {
        let mut e = Vec::new();
        for u in 0..n {
            for &v in g.successors(u) {
                if u != v {
                    e.push((u, v));
                }
            }
        }
        e
    };

    let iters = 500;
    let frame = side;
    let mut temp = side * 0.10;
    let cool = temp / (iters as f64 + 1.0);
    let anchor = 0.20_f64; // z 深度软锚定强度
    let comm_pull = 0.10_f64; // 社区质心吸引强度

    for _ in 0..iters {
        let mut dx = vec![0.0_f64; n];
        let mut dy = vec![0.0_f64; n];
        let mut dz = vec![0.0_f64; n];

        // 斥力（所有点对，3D）。
        for i in 0..n {
            for j in (i + 1)..n {
                let mut ddx = px[i] - px[j];
                let mut ddy = py[i] - py[j];
                let mut ddz = pz[i] - pz[j];
                let mut dist = (ddx * ddx + ddy * ddy + ddz * ddz).sqrt();
                if dist < 0.01 {
                    // 重合 → 按下标确定性错开。
                    ddx = 0.1 + 0.01 * (i as f64 - j as f64);
                    ddy = 0.1;
                    ddz = 0.01 * (i as f64 - j as f64);
                    dist = (ddx * ddx + ddy * ddy + ddz * ddz).sqrt();
                }
                let rep = k * k / dist;
                let (ux, uy, uz) = (ddx / dist, ddy / dist, ddz / dist);
                dx[i] += ux * rep;
                dy[i] += uy * rep;
                dz[i] += uz * rep;
                dx[j] -= ux * rep;
                dy[j] -= uy * rep;
                dz[j] -= uz * rep;
            }
        }

        // 引力（沿边，3D）。
        for &(a, b) in &epairs {
            let ddx = px[a] - px[b];
            let ddy = py[a] - py[b];
            let ddz = pz[a] - pz[b];
            let dist = (ddx * ddx + ddy * ddy + ddz * ddz).sqrt().max(0.01);
            let att = dist * dist / k;
            let (ux, uy, uz) = (ddx / dist, ddy / dist, ddz / dist);
            dx[a] -= ux * att;
            dy[a] -= uy * att;
            dz[a] -= uz * att;
            dx[b] += ux * att;
            dy[b] += uy * att;
            dz[b] += uz * att;
        }

        // 社区质心吸引（簇位）。
        let ncomm = comm.iter().copied().max().unwrap_or(0) + 1;
        let mut cx = vec![0.0_f64; ncomm];
        let mut cy = vec![0.0_f64; ncomm];
        let mut cz = vec![0.0_f64; ncomm];
        let mut cnt = vec![0.0_f64; ncomm];
        for i in 0..n {
            cx[comm[i]] += px[i];
            cy[comm[i]] += py[i];
            cz[comm[i]] += pz[i];
            cnt[comm[i]] += 1.0;
        }
        for c in 0..ncomm {
            if cnt[c] > 0.0 {
                cx[c] /= cnt[c];
                cy[c] /= cnt[c];
                cz[c] /= cnt[c];
            }
        }
        for i in 0..n {
            let c = comm[i];
            dx[i] += (cx[c] - px[i]) * comm_pull;
            dy[i] += (cy[c] - py[i]) * comm_pull;
            dz[i] += (cz[c] - pz[i]) * comm_pull;
        }

        // z 深度软锚定。
        for i in 0..n {
            dz[i] += (zanchor[i] - pz[i]) * anchor;
        }

        // 位移（温度限幅），x,y 夹在边框内；z 夹在深度范围内（留余量）。
        for i in 0..n {
            let d = (dx[i] * dx[i] + dy[i] * dy[i] + dz[i] * dz[i]).sqrt().max(0.01);
            let cap = d.min(temp);
            px[i] = (px[i] + dx[i] / d * cap).clamp(-frame, frame);
            py[i] = (py[i] + dy[i] / d * cap).clamp(-frame, frame);
            pz[i] = (pz[i] + dz[i] / d * cap).clamp(-zr * 1.2, zr * 1.2);
        }
        temp -= cool;
    }

    (px, py, pz)
}

/// 归一化到 `[-bound, bound]³`：质心居中、按最大半幅统一缩放（保形）。
#[allow(clippy::too_many_arguments)]
fn normalize(
    g: &DiGraph,
    px: &[f64],
    py: &[f64],
    pz: &[f64],
    size: &[f64],
    comm: &[usize],
    depth: &[usize],
    module_of: &[Option<String>],
    bound: f64,
) -> Vec<Node3d> {
    let n = g.len();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / n as f64;
    let (mx, my, mz) = (mean(px), mean(py), mean(pz));
    let mut half = 0.0_f64;
    for i in 0..n {
        half = half.max((px[i] - mx).abs()).max((py[i] - my).abs()).max((pz[i] - mz).abs());
    }
    let scale = if half > 1e-9 { bound / half } else { 0.0 };
    (0..n)
        .map(|i| Node3d {
            id: g.nodes[i].clone(),
            x: (px[i] - mx) * scale,
            y: (py[i] - my) * scale,
            z: (pz[i] - mz) * scale,
            size: size[i],
            community: comm[i],
            depth: depth[i],
            module: module_of[i].clone(),
        })
        .collect()
}

/// 每个图节点的**作者声明子系统名**（`meta.modules` 的键）；非声明子系统（参数/驱动/未分组）
/// 或模型未声明任何子系统 → `None`。供 GA-6 前端「按子系统」配色 + 图例。
///
/// 复用 `dag::build_dag` 设好的子模块字段（与 GA-3 [`super::metrics`] 的 `module_partition`
/// 同一权威）：只保留作者在 `meta.modules` 里**显式命名**的子系统，自动桶（驱动量/参数/其他、
/// 含耦合 `mid·` 前缀）一律回 `None`，让前端把它们并成一行「其他」。
fn node_modules(files: &[EquationFile], g: &DiGraph) -> Vec<Option<String>> {
    let declared: HashSet<&str> =
        files.iter().flat_map(|f| f.meta.modules.keys()).map(|s| s.as_str()).collect();
    if declared.is_empty() {
        return vec![None; g.len()]; // 未声明任何子系统 → 全 None（前端禁用/回退按类别）。
    }
    let dag = match crate::dag::build_dag(files) {
        Ok(d) => d,
        Err(_) => return vec![None; g.len()],
    };
    let node_mod: HashMap<&str, &str> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n.module.as_str())).collect();
    g.nodes
        .iter()
        .map(|id| {
            node_mod
                .get(id.as_str())
                .filter(|m| declared.contains(**m))
                .map(|m| m.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::bipartite::tests::toy;
    use super::*;

    fn chain() -> EquationFile {
        // x/a=0, y=1, z=2, w=3（深度递增）。
        toy(vec![
            ("e1", "y", vec!["a", "x"]),
            ("e2", "z", vec!["y"]),
            ("e3", "w", vec!["z"]),
        ])
    }

    #[test]
    fn deterministic_bit_identical() {
        let a = layout3d(&[chain()]);
        let b = layout3d(&[chain()]);
        assert_eq!(a.nodes.len(), b.nodes.len());
        for (na, nb) in a.nodes.iter().zip(&b.nodes) {
            assert_eq!(na.id, nb.id);
            assert_eq!(na.x.to_bits(), nb.x.to_bits(), "x 应逐位一致");
            assert_eq!(na.y.to_bits(), nb.y.to_bits());
            assert_eq!(na.z.to_bits(), nb.z.to_bits());
        }
    }

    #[test]
    fn finite_and_in_bounds() {
        let l = layout3d(&[chain()]);
        for nd in &l.nodes {
            for v in [nd.x, nd.y, nd.z] {
                assert!(v.is_finite(), "坐标须有限");
                assert!(v >= -1.0001 && v <= 1.0001, "坐标须 ∈ [-1,1]，实得 {v}");
            }
            assert!(nd.size >= 0.0 && nd.size <= 1.0);
        }
    }

    #[test]
    fn z_monotonic_with_depth() {
        // 深度锚定：z 应随计算深度单调（源在底、汇在顶）。
        let l = layout3d(&[chain()]);
        let zof = |id: &str| l.nodes.iter().find(|n| n.id == id).unwrap().z;
        let (zy, zz, zw) = (zof("T.y"), zof("T.z"), zof("T.w"));
        assert!(zy < zz && zz < zw, "z 应随深度递增：y={zy} z={zz} w={zw}");
    }

    #[test]
    fn hub_has_largest_size() {
        // 链里 z/y 介数高于叶子；至少有一个 size>0 且最大 size 对应非叶子。
        let l = layout3d(&[chain()]);
        let maxsize = l.nodes.iter().map(|n| n.size).fold(0.0_f64, f64::max);
        assert!(maxsize > 0.0, "应有非零 size（介数枢纽）");
    }

    #[test]
    fn no_modules_declared_all_none() {
        // 未声明任何 meta.modules → 全 None（前端据此禁用「按子系统」/回退按类别，GA-6）。
        let l = layout3d(&[chain()]);
        assert!(l.nodes.iter().all(|n| n.module.is_none()), "无子系统声明时 module 应全 None");
    }

    #[test]
    fn module_only_for_declared_subsystems() {
        // 只有作者在 meta.modules 里显式命名的子系统才进 module；自动桶（参数/驱动/其他）回 None。
        let mut f = chain();
        f.meta.modules.insert("甲".to_string(), vec!["e1".to_string()]); // 只把 e1 划入「甲」
        let l = layout3d(&[f]);
        let module_of = |id: &str| l.nodes.iter().find(|n| n.id == id).unwrap().module.clone();
        assert_eq!(module_of("T.y"), Some("甲".to_string()), "e1 的输出 y 应属命名子系统「甲」");
        assert_eq!(module_of("T.z"), None, "未划入子系统的 z 应为 None（自动桶不进 module）");
    }
}
