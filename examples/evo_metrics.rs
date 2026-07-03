//! 进化图论最小验证片（2026-07-03·crop-decision-optimization-arc）。
//! 对单个模型算图论指标（网络指标 + DM 结构/反馈环 + 结构可辨识性），输出一行 JSON。
//! 用途：沿草莓进化链 s1..s8.1 逐版本跑，看指标轨迹能否挖出机理规律。
//! ★临时一次性分析工具——只 use 已 pub 的 lib API、不改任何核心逻辑（零运行时行为改动）。
//! 用：cargo build --example evo_metrics ；然后 target/debug/examples/evo_metrics <model.eq.yaml>
use std::path::Path;

use equation_compiler::graph::{analyze_identifiability, analyze_metrics, analyze_structure};
use equation_compiler::parse_file;

fn main() {
    let path = std::env::args().nth(1).expect("用法: evo_metrics <model.eq.yaml>");
    let ef = parse_file(Path::new(&path)).expect("解析模型失败");
    let files = vec![ef];

    let m = analyze_metrics(&files);
    let id = analyze_identifiability(&files);
    let st = analyze_structure(&files);

    let nodes = m.nodes.len();
    let edges: usize = m.nodes.iter().map(|n| n.out_degree).sum();
    let depth = m.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let loops = st.algebraic_loops().len();
    let mod_mod = m
        .modularity_modules
        .map(|q| format!("{:.4}", q))
        .unwrap_or_else(|| "null".to_string());
    // 前 3 枢纽（analyze_metrics 已按介数降序排）
    let hubs: Vec<String> = m.nodes.iter().take(3).map(|n| n.node.clone()).collect();

    // 明细：具体的异参同效对 + 不可辨识参数（机理归因用；debug 格式，人读）
    let confound_pairs: Vec<String> = id
        .confounded_candidates
        .iter()
        .map(|(a, b)| format!("{}~{}", a, b))
        .collect();

    println!(
        "{{\"nodes\":{},\"edges\":{},\"depth\":{},\"algebraic_loops\":{},\"n_communities\":{},\"modularity_detected\":{:.4},\"modularity_modules\":{},\"params\":{},\"measurable\":{},\"unidentifiable\":{},\"confounded_pairs\":{},\"hubs\":{:?}}}",
        nodes,
        edges,
        depth,
        loops,
        m.n_communities,
        m.modularity_detected,
        mod_mod,
        id.params.len(),
        id.measurable.len(),
        id.unidentifiable.len(),
        id.confounded_candidates.len(),
        hubs
    );
    eprintln!("  confound_pairs = {:?}", confound_pairs);
    eprintln!("  unidentifiable = {:?}", id.unidentifiable);
}
