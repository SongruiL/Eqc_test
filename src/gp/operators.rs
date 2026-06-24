//! GP 树遗传算子（grammar-guided）：常数扰动 / 变异 / 交叉 / 复杂度。
//!
//! 设计（docs/spec-genetic-programming.md §6）：算子在**语法空间内**移动，**生成后用
//! [`check_candidate`] 重过滤**——无效则回退父代，故算子永不产出非法候选。
//! 关键：可调常数与结构常数分离（[`Candidate`]）——**扰动只动 `consts` 向量、骨架不变**，
//! 故扰动天然保 [0,1]/单调（结构常数 0/1 原样）。复用确定性 `optimize::de::Rng`。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::optimize::de::Rng;
use crate::units::Dimension;

use super::constraints::check_candidate;
use super::grammar::{sample, Candidate, GpContext};

/// 骨架总节点数（G4 parsimony 复杂度度量）。
pub fn complexity(e: &Expr) -> usize {
    use Expr::*;
    match e {
        Const(_) | Var(_) | Param(_) | Pi | E => 1,
        Neg(a) | Exp(a) | Ln(a) => 1 + complexity(a),
        Add(a, b) | Sub(a, b) | Mul(a, b) | Div(a, b) | Pow(a, b) => {
            1 + complexity(a) + complexity(b)
        }
        Clamp(a, b, c) => 1 + complexity(a) + complexity(b) + complexity(c),
        Max(xs) | Min(xs) => 1 + xs.iter().map(complexity).sum::<usize>(),
        _ => 1,
    }
}

fn valid(cand: &Candidate, ctx: &GpContext, env: &HashMap<String, Dimension>, sweep_hi: f64) -> bool {
    check_candidate(cand, ctx, env, sweep_hi).all_ok()
}

/// 常数扰动：每个可调常数乘一个**保号因子** [1/(1+s), 1+s)（保单调/有界结构）。骨架不变。
pub fn perturb_constants(cand: &Candidate, strength: f64, rng: &mut Rng) -> Candidate {
    let consts = cand
        .consts
        .iter()
        .map(|v| v * rng.next_range(1.0 / (1.0 + strength), 1.0 + strength))
        .collect();
    Candidate { expr: cand.expr.clone(), consts }
}

/// 变异：扰动常数 / 重采样形式 / 换输入变量；生成后重过滤，无效则重试、最终回退父代。
pub fn mutate(
    cand: &Candidate,
    grammar: &str,
    ctx: &GpContext,
    env: &HashMap<String, Dimension>,
    sweep_hi: f64,
    rng: &mut Rng,
) -> Candidate {
    for _ in 0..8 {
        let trial = match rng.next_usize(3) {
            0 => perturb_constants(cand, 0.5, rng),
            1 => sample(grammar, ctx, rng).unwrap_or_else(|| cand.clone()),
            _ => {
                // 换输入变量（结构性变异）：可能破坏单调 → 靠重过滤兜
                let vars = cand.expr.get_variable_refs();
                if !vars.is_empty() && ctx.inputs.len() >= 2 {
                    let from = &vars[rng.next_usize(vars.len())];
                    let to = &ctx.inputs[rng.next_usize(ctx.inputs.len())];
                    Candidate {
                        expr: cand.expr.substitute(from, &Expr::var(to)),
                        consts: cand.consts.clone(),
                    }
                } else {
                    perturb_constants(cand, 0.5, rng)
                }
            }
        };
        if valid(&trial, ctx, env, sweep_hi) {
            return trial;
        }
    }
    cand.clone()
}

/// 交叉：两**同骨架**父代按可调常数位置交叉（每常数随机取一方）；骨架不同或非法 → 回退 a。
pub fn crossover(
    a: &Candidate,
    b: &Candidate,
    ctx: &GpContext,
    env: &HashMap<String, Dimension>,
    sweep_hi: f64,
    rng: &mut Rng,
) -> Candidate {
    if same_skeleton(&a.expr, &b.expr) && a.consts.len() == b.consts.len() {
        let consts = a
            .consts
            .iter()
            .zip(&b.consts)
            .map(|(&x, &y)| if rng.next_f64() < 0.5 { x } else { y })
            .collect();
        let child = Candidate { expr: a.expr.clone(), consts };
        if valid(&child, ctx, env, sweep_hi) {
            return child;
        }
    }
    a.clone()
}

/// 两骨架结构是否相同（忽略结构常数值、可调常数占位名一致即可；只看语法算子集）。
fn same_skeleton(a: &Expr, b: &Expr) -> bool {
    use Expr::*;
    match (a, b) {
        (Const(x), Const(y)) => x == y, // 结构常数须一致
        (Var(n), Var(m)) | (Param(n), Param(m)) => n == m,
        (Pi, Pi) | (E, E) => true,
        (Neg(x), Neg(y)) | (Exp(x), Exp(y)) | (Ln(x), Ln(y)) => same_skeleton(x, y),
        (Add(x1, x2), Add(y1, y2))
        | (Sub(x1, x2), Sub(y1, y2))
        | (Mul(x1, x2), Mul(y1, y2))
        | (Div(x1, x2), Div(y1, y2))
        | (Pow(x1, x2), Pow(y1, y2)) => same_skeleton(x1, y1) && same_skeleton(x2, y2),
        (Clamp(x1, x2, x3), Clamp(y1, y2, y3)) => {
            same_skeleton(x1, y1) && same_skeleton(x2, y2) && same_skeleton(x3, y3)
        }
        (Max(xs), Max(ys)) | (Min(xs), Min(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same_skeleton(x, y))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gp::grammar::sample;
    use indexmap::IndexMap;

    fn ctx(inputs: &[&str], bounds: Option<[f64; 2]>, mono: &[(&str, &str)]) -> GpContext {
        let mut m = IndexMap::new();
        for (v, d) in mono {
            m.insert(v.to_string(), d.to_string());
        }
        GpContext {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            output_bounds: bounds,
            monotone: m,
        }
    }

    fn cases() -> Vec<(&'static str, GpContext, f64)> {
        vec![
            ("monotone_gate", ctx(&["ChillAccum", "GDD"], Some([0.0, 1.0]), &[("ChillAccum", "increasing")]), 50.0),
            ("saturating_sink", ctx(&["LAI", "LAI_pot"], Some([0.0, 1.0]), &[("LAI", "decreasing")]), 5.0),
            ("allocation_fraction", ctx(&["NNI", "VS"], Some([0.0, 1.0]), &[]), 2.0),
            ("temperature_response", ctx(&["T"], Some([0.0, 1.0]), &[]), 40.0),
            ("growth_curve", ctx(&["tau_fruit"], None, &[("tau_fruit", "increasing")]), 1000.0),
        ]
    }

    /// 强保证：mutate / crossover 反复施加，产物**始终合法**（量纲+单调+有界）。
    #[test]
    fn test_operators_never_produce_invalid() {
        let env: HashMap<String, Dimension> = HashMap::new();
        for (g, c, sweep_hi) in cases() {
            let mut rng = Rng::new(2024);
            let mut a = sample(g, &c, &mut rng).unwrap();
            let b = sample(g, &c, &mut rng).unwrap();
            for _ in 0..100 {
                a = mutate(&a, g, &c, &env, sweep_hi, &mut rng);
                assert!(valid(&a, &c, &env, sweep_hi), "{g}: mutate 产出非法");
                let x = crossover(&a, &b, &c, &env, sweep_hi, &mut rng);
                assert!(valid(&x, &c, &env, sweep_hi), "{g}: crossover 产出非法");
            }
        }
    }

    /// 常数扰动保骨架、改值（且仍合法——结构常数未被动）。
    #[test]
    fn test_perturb_keeps_structure_and_validity() {
        let env: HashMap<String, Dimension> = HashMap::new();
        for (g, c, sweep_hi) in cases() {
            let mut rng = Rng::new(5);
            for _ in 0..20 {
                let e = sample(g, &c, &mut rng).unwrap();
                let p = perturb_constants(&e, 0.5, &mut rng);
                assert_eq!(complexity(&e.expr), complexity(&p.expr), "{g}: 扰动应保骨架");
                assert!(same_skeleton(&e.expr, &p.expr), "{g}: 扰动骨架应同构");
                // 扰动后仍合法（关键：结构常数 0/1 未被动 → 不破坏 [0,1]/单调）
                assert!(valid(&p, &c, &env, sweep_hi), "{g}: 扰动后应仍合法");
            }
        }
    }

    /// 交叉两同骨架父代 → 同骨架、常数取自父代、仍合法。
    #[test]
    fn test_crossover_same_skeleton() {
        let c = ctx(&["LAI", "LAI_pot"], Some([0.0, 1.0]), &[("LAI", "decreasing")]);
        let env: HashMap<String, Dimension> = HashMap::new();
        let mut r = Rng::new(11);
        let base = sample("saturating_sink", &c, &mut r).unwrap();
        let pa = perturb_constants(&base, 0.5, &mut r);
        let pb = perturb_constants(&base, 0.5, &mut r);
        let child = crossover(&pa, &pb, &c, &env, 5.0, &mut r);
        assert!(same_skeleton(&child.expr, &base.expr), "交叉保骨架");
        assert!(valid(&child, &c, &env, 5.0));
    }

    /// 确定性：同种子 → 同变异序列。
    #[test]
    fn test_mutate_deterministic() {
        let c = ctx(&["ChillAccum", "GDD"], Some([0.0, 1.0]), &[("ChillAccum", "increasing")]);
        let env: HashMap<String, Dimension> = HashMap::new();
        let run = || {
            let mut rng = Rng::new(99);
            let mut e = sample("monotone_gate", &c, &mut rng).unwrap();
            for _ in 0..15 {
                e = mutate(&e, "monotone_gate", &c, &env, 50.0, &mut rng);
            }
            format!("{:?}|{:?}", e.expr, e.consts)
        };
        assert_eq!(run(), run());
    }
}
