//! 模型图论分析（结构分析地基）。
//!
//! 把模型当**图**来严谨分析。
//!
//! GA-1（结构分析地基）：
//! - [`bipartite`] —— 变量-方程二部图（无向；不预设谁解谁）+ 节点命名单一真相源 [`NodeResolver`]。
//! - [`matching`] —— Hopcroft–Karp 最大匹配 + 与作者 `output:` 指派对照（适定性的结构必要条件）。
//! - [`dm`] —— Dulmage–Mendelsohn 分解：自由变量 + 块三角求解顺序 + 代数环定位。
//!
//! GA-2（结构可辨识性，必要条件筛子）：
//! - [`digraph`] —— 有向影响图（含积分/延迟边）；GA-2 可达性与 GA-3 网络指标共用。
//! - [`identifiability`] —— 参数→可测变量可达性（不可达=不可辨识）+ 混淆候选。
//!
//! GA-3（网络指标，描述性，绑定到枢纽定位/模块验证）：
//! - [`metrics`] —— 度/介数(Brandes)/PageRank 中心性、社区(贪心模块度)、DAG 深度。
//!
//! GA-4（版本结构 diff，喂 GP 进化溯源 + 3D 生长动画）：
//! - [`diff`] —— 两版本带标签节点对齐 → 增删点/边 + 形式改变的方程 + 结构距离。
//!
//! GA-5（3D 力导向坐标，Rust 算、前端只渲染）：
//! - [`layout3d`] —— 3D Fruchterman–Reingold（深度软锚定 z + 社区簇位 + 介数定大小），确定性。
//!
//! 纯 Rust、数据无关、可单测、确定性（无 RNG）；不碰数值求解（只**定位**代数环，隐式求解另案）。
//! 理论见 `docs/theory-model-graph-analysis.md`，实现 spec 见 `docs/spec-graph-analysis.md` §4。

pub mod bipartite;
pub mod diff;
pub mod digraph;
pub mod dm;
pub mod identifiability;
pub mod layout3d;
pub mod matching;
pub mod metrics;

pub use bipartite::{BipartiteGraph, EqNode, NodeResolver};
pub use diff::{diff_models, DiffNode, EqChange, GraphDiff};
pub use digraph::{integration_edges, DiGraph};
pub use dm::{analyze_graph, analyze_structure, SolveBlock, StructureReport};
pub use identifiability::{analyze_identifiability, IdentifiabilityReport, ParamReach};
pub use layout3d::{layout3d, Layout3d, Node3d};
pub use matching::{analyze as analyze_matching, max_matching, MatchingReport};
pub use metrics::{analyze_metrics, MetricsReport, NodeMetrics};
