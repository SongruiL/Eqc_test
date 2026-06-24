//! GP 主循环（单槽位）—— 对一个 🟠 靶点进化候选方程结构。
//!
//! 设计（docs/spec-genetic-programming.md §7）：种群循环复用 G1 语法采样初始化、G2 算子繁殖
//! （后代由算子契约保证合法）、确定性 `optimize::de::Rng`。适应度**泛型于一个误差闭包**
//! （`FnMut(&Candidate)->f64`，同 DE 泛型于 cost）——故可插不同适应度：表达式级复原（合成验收）
//! 或模型级（patch+sim+观测误差，见 `fitness`）。`cost = error + parsimony·complexity`。

use std::collections::HashMap;

use crate::optimize::de::Rng;
use crate::units::Dimension;

use super::grammar::{sample, Candidate, GpContext};
use super::operators::{complexity, crossover, mutate};

/// 进化配置。
#[derive(Debug, Clone)]
pub struct EvolveConfig {
    pub pop: usize,
    pub gens: usize,
    pub seed: u64,
    pub tournament_k: usize,
    pub elitism: usize,
    /// 复杂度惩罚权重（parsimony；G4 改多目标 Pareto，G3 用标量惩罚）。
    pub parsimony: f64,
    /// 约束检查输入扫描上界（典型量级）。
    pub sweep_hi: f64,
}
impl Default for EvolveConfig {
    fn default() -> Self {
        Self {
            pop: 40,
            gens: 30,
            seed: 42,
            tournament_k: 3,
            elitism: 2,
            parsimony: 0.0,
            sweep_hi: 50.0,
        }
    }
}

/// 进化结果。
#[derive(Debug, Clone)]
pub struct EvolveResult {
    pub best: Candidate,
    /// 最佳综合代价（error + parsimony·complexity）。
    pub best_cost: f64,
    /// 最佳原始拟合误差（不含 parsimony）。
    pub best_error: f64,
    /// 每代最佳代价（单调不增）。
    pub history: Vec<f64>,
}

const WORST: f64 = 1e18;

/// 锦标赛选择：取 k 个随机个体里代价最低者。
fn tournament<'a>(scored: &'a [(Candidate, f64, f64)], k: usize, rng: &mut Rng) -> &'a Candidate {
    let mut best = rng.next_usize(scored.len());
    for _ in 1..k.max(1) {
        let i = rng.next_usize(scored.len());
        if scored[i].1 < scored[best].1 {
            best = i;
        }
    }
    &scored[best].0
}

/// 进化一个槽位。`error_fn(cand)`=原始拟合误差（越小越好，非有限→视为 WORST）。
pub fn evolve<F: FnMut(&Candidate) -> f64>(
    grammar: &str,
    ctx: &GpContext,
    unit_env: &HashMap<String, Dimension>,
    cfg: &EvolveConfig,
    mut error_fn: F,
) -> EvolveResult {
    let mut rng = Rng::new(cfg.seed);

    // 初始种群：从语法采样（按构造合法）。采样失败（未知语法）→ 空结果保护。
    let mut pop: Vec<Candidate> = (0..cfg.pop.max(1))
        .filter_map(|_| sample(grammar, ctx, &mut rng))
        .collect();
    if pop.is_empty() {
        // 未知语法：返回一个退化结果，避免 panic。
        let dummy = Candidate { expr: crate::ast::Expr::constant(0.0), consts: vec![] };
        return EvolveResult { best: dummy, best_cost: WORST, best_error: WORST, history: vec![] };
    }

    let score = |cand: &Candidate, ef: &mut F| -> (f64, f64) {
        let err = ef(cand);
        let err = if err.is_finite() { err } else { WORST };
        let cost = err + cfg.parsimony * complexity(&cand.expr) as f64;
        (cost, err)
    };

    let mut best: Option<(Candidate, f64, f64)> = None;
    let mut history = Vec::with_capacity(cfg.gens);

    for _gen in 0..cfg.gens.max(1) {
        // 评估
        let mut scored: Vec<(Candidate, f64, f64)> = pop
            .iter()
            .map(|c| {
                let (cost, err) = score(c, &mut error_fn);
                (c.clone(), cost, err)
            })
            .collect();
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // 跟踪全局最优
        if best.as_ref().map_or(true, |b| scored[0].1 < b.1) {
            best = Some(scored[0].clone());
        }
        history.push(best.as_ref().unwrap().1);

        // 下一代：精英保留 + 锦标赛选亲 + 交叉 + 变异（后代均合法）
        let mut next: Vec<Candidate> = scored
            .iter()
            .take(cfg.elitism.min(scored.len()))
            .map(|s| s.0.clone())
            .collect();
        while next.len() < cfg.pop {
            let pa = tournament(&scored, cfg.tournament_k, &mut rng).clone();
            let pb = tournament(&scored, cfg.tournament_k, &mut rng).clone();
            let child = crossover(&pa, &pb, ctx, unit_env, cfg.sweep_hi, &mut rng);
            let child = mutate(&child, grammar, ctx, unit_env, cfg.sweep_hi, &mut rng);
            next.push(child);
        }
        pop = next;
    }

    let (bc, bcost, berr) = best.unwrap();
    EvolveResult { best: bc, best_cost: bcost, best_error: berr, history }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gp::constraints::eval_candidate;
    use indexmap::IndexMap;

    fn gate_ctx() -> GpContext {
        let mut m = IndexMap::new();
        m.insert("ChillAccum".to_string(), "increasing".to_string());
        GpContext {
            inputs: vec!["ChillAccum".to_string(), "GDD".to_string()],
            output_bounds: Some([0.0, 1.0]),
            monotone: m,
        }
    }

    /// ★合成复原验收：把目标设为已知 sigmoid 门控，GP 从随机起点进化应**功能复原**它。
    /// （类比标定工具的 recover LUE=4.0：数据无关、确定性。）
    #[test]
    fn test_synthetic_recovery_sigmoid_gate() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        // 真值：dormancy = 1/(1+exp(−0.3·(ChillAccum−20)))
        let truth = |chill: f64| 1.0 / (1.0 + (-0.3 * (chill - 20.0)).exp());
        let xs: Vec<f64> = (0..=50).map(|i| i as f64).collect();
        let mut error_fn = |cand: &Candidate| {
            let mut se = 0.0;
            let mut n = 0;
            for &x in &xs {
                match eval_candidate(cand, &[("ChillAccum", x), ("GDD", 0.0)]) {
                    Some(y) => {
                        let d = y - truth(x);
                        se += d * d;
                        n += 1;
                    }
                    None => return WORST,
                }
            }
            (se / n as f64).sqrt()
        };
        let cfg = EvolveConfig { pop: 60, gens: 40, seed: 1, parsimony: 0.0, ..Default::default() };
        let res = evolve("monotone_gate", &ctx, &env, &cfg, &mut error_fn);
        // GP 应把 rmse 压到很小（monotone_gate 含 sigmoid 形 → 可近乎精确复原）
        assert!(res.best_error < 0.03, "复原误差应小: rmse={}", res.best_error);
        // history 单调不增（精英保留）
        for w in res.history.windows(2) {
            assert!(w[1] <= w[0] + 1e-12, "history 应单调不增");
        }
    }

    /// 确定性：同种子 → 同最优。
    #[test]
    fn test_evolve_deterministic() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        let truth = |c: f64| if c > 25.0 { 1.0 } else { 0.0 };
        let xs: Vec<f64> = (0..=50).map(|i| i as f64).collect();
        let mk = || {
            let mut ef = |cand: &Candidate| {
                let mut se = 0.0;
                for &x in &xs {
                    let y = eval_candidate(cand, &[("ChillAccum", x), ("GDD", 0.0)]).unwrap_or(1e6);
                    se += (y - truth(x)).powi(2);
                }
                se.sqrt()
            };
            let cfg = EvolveConfig { pop: 30, gens: 15, seed: 7, ..Default::default() };
            let r = evolve("monotone_gate", &gate_ctx(), &env, &cfg, &mut ef);
            (r.best_error, format!("{:?}|{:?}", r.best.expr, r.best.consts))
        };
        let (e1, s1) = mk();
        let (e2, s2) = mk();
        assert_eq!(e1, e2);
        assert_eq!(s1, s2);
    }

    /// 进化确实改进：末代最优 ≤ 初代最优。
    #[test]
    fn test_evolve_improves() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        let truth = |c: f64| 1.0 / (1.0 + (-0.5 * (c - 15.0)).exp());
        let xs: Vec<f64> = (0..=50).map(|i| i as f64).collect();
        let mut ef = |cand: &Candidate| {
            let mut se = 0.0;
            for &x in &xs {
                let y = eval_candidate(cand, &[("ChillAccum", x), ("GDD", 0.0)]).unwrap_or(1e6);
                se += (y - truth(x)).powi(2);
            }
            (se / xs.len() as f64).sqrt()
        };
        let cfg = EvolveConfig { pop: 40, gens: 25, seed: 3, ..Default::default() };
        let res = evolve("monotone_gate", &ctx, &env, &cfg, &mut ef);
        assert!(res.history.last().unwrap() <= res.history.first().unwrap());
        assert!(res.best_error < 0.05, "应较好拟合: {}", res.best_error);
    }
}
