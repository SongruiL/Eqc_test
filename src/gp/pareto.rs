//! GP G4：memetic 内层常数标定 + 多目标 Pareto（精度 vs 复杂度）。
//!
//! 设计（docs/spec-genetic-programming.md §3 D4/D5）：
//! - **memetic**：候选结构的适应度 = 用内层 DE 标定其 `consts` 向量后的最佳拟合（在合法区内）。
//!   复用 `optimize::de::differential_evolution`——consts 向量正是 G2 分离出的"常数基因"。
//! - **Pareto**：维护 (error, complexity) 非支配前沿（NSGA-II 式非支配排序 + 拥挤截断），
//!   让首席科学家在"拟合好 vs 简洁"的拐点挑形式，防过拟合/bloat。

use std::collections::HashMap;

use crate::optimize::de::{differential_evolution, DeConfig};
use crate::units::Dimension;

use super::constraints::check_candidate;
use super::grammar::{sample, Candidate, GpContext};
use super::operators::{complexity, crossover, mutate};

const WORST: f64 = 1e18;

/// 用内层 DE 在**合法区内**标定候选的常数向量，最小化误差。返回 (最佳consts, 最佳误差)。
/// 无可调常数 → 直接评估。
pub fn calibrate_consts<F: FnMut(&Candidate) -> f64>(
    cand: &Candidate,
    ctx: &GpContext,
    unit_env: &HashMap<String, Dimension>,
    sweep_hi: f64,
    de: &DeConfig,
    error_fn: &mut F,
) -> (Vec<f64>, f64) {
    if cand.consts.is_empty() {
        let e = error_fn(cand);
        return (vec![], if e.is_finite() { e } else { WORST });
    }
    // 每常数在其当前值的加性邻域 [c−span, c+span]，span = max(|c|,1)·2。
    let bounds: Vec<(f64, f64)> = cand
        .consts
        .iter()
        .map(|&c| {
            let s = c.abs().max(1.0) * 2.0;
            (c - s, c + s)
        })
        .collect();
    let res = differential_evolution(&bounds, de, |x: &[f64]| {
        let trial = Candidate { expr: cand.expr.clone(), consts: x.to_vec() };
        if !check_candidate(&trial, ctx, unit_env, sweep_hi).all_ok() {
            return WORST;
        }
        let e = error_fn(&trial);
        if e.is_finite() {
            e
        } else {
            WORST
        }
    });
    (res.best_x, res.best_cost)
}

/// Pareto 前沿一项。
#[derive(Debug, Clone)]
pub struct ParetoEntry {
    pub cand: Candidate,
    pub error: f64,
    pub complexity: usize,
}

/// 多目标进化配置。
#[derive(Debug, Clone)]
pub struct ParetoConfig {
    pub pop: usize,
    pub gens: usize,
    pub seed: u64,
    pub sweep_hi: f64,
    /// 前沿归档上限（拥挤截断到此数）。
    pub archive_cap: usize,
    /// Some → 每候选用内层 DE 标定常数（memetic）；None → 用当前 consts（co-evolve）。
    pub memetic: Option<DeConfig>,
}
impl Default for ParetoConfig {
    fn default() -> Self {
        Self { pop: 40, gens: 30, seed: 42, sweep_hi: 50.0, archive_cap: 24, memetic: None }
    }
}

/// 双目标（均最小化：error, complexity）支配：a 支配 b。
fn dominates(a: &ParetoEntry, b: &ParetoEntry) -> bool {
    let (ae, ac) = (a.error, a.complexity as f64);
    let (be, bc) = (b.error, b.complexity as f64);
    ae <= be && ac <= bc && (ae < be || ac < bc)
}

/// 快速非支配排序 → 分层（每层为 entries 下标）。
fn nondominated_fronts(es: &[ParetoEntry]) -> Vec<Vec<usize>> {
    let n = es.len();
    let mut dominated: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut dom_count = vec![0usize; n];
    for p in 0..n {
        for q in 0..n {
            if p == q {
                continue;
            }
            if dominates(&es[p], &es[q]) {
                dominated[p].push(q);
            } else if dominates(&es[q], &es[p]) {
                dom_count[p] += 1;
            }
        }
    }
    let mut fronts: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = (0..n).filter(|&i| dom_count[i] == 0).collect();
    while !cur.is_empty() {
        let mut next = Vec::new();
        for &p in &cur {
            for &q in &dominated[p] {
                dom_count[q] -= 1;
                if dom_count[q] == 0 {
                    next.push(q);
                }
            }
        }
        fronts.push(cur);
        cur = next;
    }
    fronts
}

/// 在一层里按拥挤距离选 k 个（保边界 + 均匀分布）。
fn crowding_select(es: &[ParetoEntry], front: &[usize], k: usize) -> Vec<usize> {
    if front.len() <= k {
        return front.to_vec();
    }
    let m = front.len();
    let mut dist = vec![0.0f64; m];
    // 两目标各排序、边界设 ∞、内部累加归一化间距
    let objs: [Box<dyn Fn(usize) -> f64>; 2] = [
        Box::new(|i: usize| es[i].error),
        Box::new(|i: usize| es[i].complexity as f64),
    ];
    for obj in &objs {
        let mut order: Vec<usize> = (0..m).collect();
        order.sort_by(|&a, &b| obj(front[a]).partial_cmp(&obj(front[b])).unwrap_or(std::cmp::Ordering::Equal));
        dist[order[0]] = f64::INFINITY;
        dist[order[m - 1]] = f64::INFINITY;
        let lo = obj(front[order[0]]);
        let hi = obj(front[order[m - 1]]);
        let span = (hi - lo).abs().max(1e-12);
        for j in 1..m - 1 {
            let d = obj(front[order[j + 1]]) - obj(front[order[j - 1]]);
            dist[order[j]] += d / span;
        }
    }
    let mut idx: Vec<usize> = (0..m).collect();
    idx.sort_by(|&a, &b| dist[b].partial_cmp(&dist[a]).unwrap_or(std::cmp::Ordering::Equal));
    idx.into_iter().take(k).map(|j| front[j]).collect()
}

/// 确定性 PRNG（复用 DE 的）需 pub(crate)；这里用 DE 的 Rng。
use crate::optimize::de::Rng;

fn eval_entry<F: FnMut(&Candidate) -> f64>(
    cand: &Candidate,
    ctx: &GpContext,
    unit_env: &HashMap<String, Dimension>,
    cfg: &ParetoConfig,
    error_fn: &mut F,
) -> ParetoEntry {
    let (consts, error) = match &cfg.memetic {
        Some(de) => calibrate_consts(cand, ctx, unit_env, cfg.sweep_hi, de, error_fn),
        None => {
            let e = error_fn(cand);
            (cand.consts.clone(), if e.is_finite() { e } else { WORST })
        }
    };
    let c2 = Candidate { expr: cand.expr.clone(), consts };
    let cplx = complexity(&c2.expr);
    ParetoEntry { cand: c2, error, complexity: cplx }
}

/// 多目标进化，返回 (error, complexity) 非支配前沿（按复杂度升序）。
pub fn evolve_pareto<F: FnMut(&Candidate) -> f64>(
    grammar: &str,
    ctx: &GpContext,
    unit_env: &HashMap<String, Dimension>,
    cfg: &ParetoConfig,
    mut error_fn: F,
) -> Vec<ParetoEntry> {
    let mut rng = Rng::new(cfg.seed);
    // 初始归档
    let mut archive: Vec<ParetoEntry> = (0..cfg.pop.max(1))
        .filter_map(|_| sample(grammar, ctx, &mut rng))
        .map(|c| eval_entry(&c, ctx, unit_env, cfg, &mut error_fn))
        .collect();
    if archive.is_empty() {
        return vec![];
    }

    for _gen in 0..cfg.gens.max(1) {
        // 繁殖：从归档随机选亲 → 交叉 + 变异 → 评估
        let mut offspring: Vec<ParetoEntry> = Vec::with_capacity(cfg.pop);
        for _ in 0..cfg.pop {
            let a = &archive[rng.next_usize(archive.len())].cand;
            let b = &archive[rng.next_usize(archive.len())].cand;
            let child = crossover(a, b, ctx, unit_env, cfg.sweep_hi, &mut rng);
            let child = mutate(&child, grammar, ctx, unit_env, cfg.sweep_hi, &mut rng);
            offspring.push(eval_entry(&child, ctx, unit_env, cfg, &mut error_fn));
        }
        // 合并 → 非支配分层 → 填下一归档至 cap（最后一层拥挤截断）
        let mut combined = archive;
        combined.append(&mut offspring);
        let fronts = nondominated_fronts(&combined);
        let mut next: Vec<ParetoEntry> = Vec::with_capacity(cfg.archive_cap);
        for front in fronts {
            if next.len() + front.len() <= cfg.archive_cap {
                for i in front {
                    next.push(combined[i].clone());
                }
            } else {
                let keep = crowding_select(&combined, &front, cfg.archive_cap - next.len());
                for i in keep {
                    next.push(combined[i].clone());
                }
                break;
            }
        }
        archive = next;
    }

    // 返回最终前沿（非支配集），按复杂度升序、同复杂度按误差升序
    let fronts = nondominated_fronts(&archive);
    let mut front: Vec<ParetoEntry> = fronts
        .first()
        .map(|f| f.iter().map(|&i| archive[i].clone()).collect())
        .unwrap_or_default();
    front.sort_by(|a, b| {
        a.complexity
            .cmp(&b.complexity)
            .then(a.error.partial_cmp(&b.error).unwrap_or(std::cmp::Ordering::Equal))
    });
    front
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

    // 误差闭包：候选 vs 真值 sigmoid 在 chill∈[0,50] 上的 rmse。
    fn make_err() -> impl FnMut(&Candidate) -> f64 {
        let truth = |c: f64| 1.0 / (1.0 + (-0.3 * (c - 20.0)).exp());
        let xs: Vec<f64> = (0..=50).map(|i| i as f64).collect();
        move |cand: &Candidate| {
            let mut se = 0.0;
            for &x in &xs {
                match eval_candidate(cand, &[("ChillAccum", x), ("GDD", 0.0)]) {
                    Some(y) => se += (y - truth(x)).powi(2),
                    None => return WORST,
                }
            }
            (se / xs.len() as f64).sqrt()
        }
    }

    /// memetic 内层标定：对一个采样候选标定常数后，误差 ≤ 标定前。
    #[test]
    fn test_calibrate_improves() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        let mut rng = Rng::new(3);
        let cand = sample("monotone_gate", &ctx, &mut rng).unwrap();
        let mut ef = make_err();
        let before = ef(&cand);
        let de = DeConfig { pop: 16, iters: 30, seed: 1, f: 0.6, cr: 0.9 };
        let (consts, after) = calibrate_consts(&cand, &ctx, &env, 50.0, &de, &mut ef);
        assert!(after <= before + 1e-9, "标定应不变差: {after} vs {before}");
        // 标定后的候选仍合法
        let c2 = Candidate { expr: cand.expr.clone(), consts };
        assert!(check_candidate(&c2, &ctx, &env, 50.0).all_ok(), "标定后应仍合法");
    }

    /// Pareto 前沿：非支配、含低误差解、确定性。
    #[test]
    fn test_pareto_front() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        let cfg = ParetoConfig { pop: 40, gens: 20, seed: 1, archive_cap: 16, ..Default::default() };
        let front = evolve_pareto("monotone_gate", &ctx, &env, &cfg, make_err());
        assert!(!front.is_empty(), "前沿非空");
        // 互不支配
        for i in 0..front.len() {
            for j in 0..front.len() {
                if i != j {
                    assert!(!dominates(&front[i], &front[j]), "前沿内不应互相支配");
                }
            }
        }
        // 含一个拟合不错的解
        let best = front.iter().map(|e| e.error).fold(f64::INFINITY, f64::min);
        assert!(best < 0.05, "前沿应含低误差解: {best}");
        // 复杂度升序
        for w in front.windows(2) {
            assert!(w[0].complexity <= w[1].complexity);
        }
    }

    /// 确定性：同种子 → 同前沿。
    #[test]
    fn test_pareto_deterministic() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        let cfg = ParetoConfig { pop: 24, gens: 10, seed: 5, archive_cap: 10, ..Default::default() };
        let run = || {
            let f = evolve_pareto("monotone_gate", &ctx, &env, &cfg, make_err());
            f.iter().map(|e| format!("{:.6}|{}", e.error, e.complexity)).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// memetic 模式也能跑通并给出前沿（含低误差，常数被标定得更好）。
    #[test]
    fn test_pareto_memetic() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let ctx = gate_ctx();
        let cfg = ParetoConfig {
            pop: 10,
            gens: 4,
            seed: 2,
            archive_cap: 8,
            memetic: Some(DeConfig { pop: 10, iters: 10, seed: 1, f: 0.6, cr: 0.9 }),
            ..Default::default()
        };
        let front = evolve_pareto("monotone_gate", &ctx, &env, &cfg, make_err());
        assert!(!front.is_empty());
        let best = front.iter().map(|e| e.error).fold(f64::INFINITY, f64::min);
        assert!(best < 0.05, "memetic 前沿应含低误差解: {best}");
    }
}
