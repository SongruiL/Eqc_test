//! 模型图论分析（结构分析地基）。
//!
//! 把模型当**图**来严谨分析。本模块（GA-1）实现「结构分析」的地基：
//! - [`bipartite`] —— 变量-方程二部图（无向；不预设谁解谁）。
//! - [`matching`] —— Hopcroft–Karp 最大匹配 + 与作者 `output:` 指派对照（适定性的结构必要条件）。
//! - [`dm`] —— Dulmage–Mendelsohn 分解：自由变量 + 块三角求解顺序 + 代数环定位。
//!
//! 纯 Rust、数据无关、可单测；不碰数值求解（只**定位**代数环，隐式求解另案）。
//! 理论见 `docs/theory-model-graph-analysis.md`，实现 spec 见 `docs/spec-graph-analysis.md` §4。

pub mod bipartite;
pub mod dm;
pub mod matching;

pub use bipartite::{BipartiteGraph, EqNode};
pub use dm::{analyze_graph, analyze_structure, SolveBlock, StructureReport};
pub use matching::{analyze as analyze_matching, max_matching, MatchingReport};
