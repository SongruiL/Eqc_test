//! GP 多槽位**联合进化**：一次进化模型的多个 🟠 靶点（槽位），基因组 = 一组候选
//! （每槽位一棵树），适应度 = 把**全部**槽位 patch 进模型→**一次仿真**→对观测算误差。
//! 捕捉槽位间相互作用（如蓝莓碳平衡里 休眠门控↔分配↔果实 的耦合），单槽位独立进化做不到。
//!
//! 复用：grammar/operators/constraints（逐槽位不变）、sim、单槽位 patch 思路（扩成多槽位 +
//! 常数命名空间化 `__s{k}_c{i}` 防跨槽撞名）。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::optimize::de::Rng;
use crate::schema::{EquationFile, Parameter};
use crate::sim::{self, SimInput};
use crate::units::Dimension;

use super::fitness::Observed;
use super::grammar::{sample, Candidate, GpContext};
use super::operators::{complexity, crossover, mutate};

const WORST: f64 = 1e18;

/// 一个进化槽位的固定配置（靶点方程 + 语法 + 上下文）。
#[derive(Debug, Clone)]
pub struct Slot {
    pub target_id: String,
    pub grammar: String,
    pub ctx: GpContext,
}

/// 从模型读出所有（或指定子集）`gp_target` 槽位。
pub fn slots_from_model(model: &EquationFile, only: Option<&[String]>) -> Vec<Slot> {
    model
        .equations
        .iter()
        .filter_map(|e| {
            let gt = e.gp_target.as_ref()?;
            if let Some(ids) = only {
                if !ids.iter().any(|x| x == &e.id) {
                    return None;
                }
            }
            let inputs = if gt.inputs.is_empty() {
                e.get_variable_refs()
            } else {
                gt.inputs.clone()
            };
            Some(Slot {
                target_id: e.id.clone(),
                grammar: gt.grammar.clone(),
                ctx: GpContext {
                    inputs,
                    output_bounds: gt.output_bounds,
                    monotone: gt.monotone.clone(),
                },
            })
        })
        .collect()
}

/// 把候选的可调常数 `__c{i}` 重命名为槽位命名空间 `__s{k}_c{i}`（防跨槽撞名），返回 (新expr, 参数表)。
fn namespaced(cand: &Candidate, slot: usize) -> (Expr, Vec<(String, f64)>) {
    let mut expr = cand.expr.clone();
    let mut params = Vec::with_capacity(cand.consts.len());
    for (i, v) in cand.consts.iter().enumerate() {
        let new = format!("__s{slot}_c{i}");
        expr = expr.substitute(&Candidate::const_name(i), &Expr::param(new.clone()));
        params.push((new, *v));
    }
    (expr, params)
}

/// clone 模型，把各槽位候选 patch 进对应方程、注入各自命名空间化的常数参数。
pub fn patch_multi(base: &EquationFile, slots: &[Slot], genome: &[Candidate]) -> Option<EquationFile> {
    if slots.len() != genome.len() {
        return None;
    }
    let mut m = base.clone();
    for (k, slot) in slots.iter().enumerate() {
        let (expr, params) = namespaced(&genome[k], k);
        {
            let eq = m.equations.iter_mut().find(|e| e.id == slot.target_id)?;
            eq.expression = expr;
        }
        for (name, val) in params {
            m.parameters.insert(
                name.clone(),
                Parameter {
                    name_cn: name,
                    name_en: None,
                    dtype: Default::default(),
                    default: val,
                    values: None,
                    unit: None,
                    bounds: None,
                    optimizable: true,
                    management: false,
                    description: None,
                    provenance: None,
                },
            );
        }
    }
    Some(m)
}

fn rmse_one(out: &sim::SimOutput, var: &str, obs: &[(usize, f64)]) -> Option<f64> {
    let traj = out.series(var)?;
    if obs.is_empty() {
        return None;
    }
    let mut se = 0.0;
    let mut n = 0;
    for &(day, val) in obs {
        let y = *traj.get(day.checked_sub(1)?)?;
        if !y.is_finite() {
            return None;
        }
        se += (y - val).powi(2);
        n += 1;
    }
    if n == 0 {
        None
    } else {
        Some((se / n as f64).sqrt())
    }
}

/// 多槽位模型级适应度：patch 全部槽位 → 一次仿真 → **所有观测变量的平均 rmse**。失败→WORST。
pub fn evaluate_multi(
    base: &EquationFile,
    slots: &[Slot],
    genome: &[Candidate],
    input: &SimInput,
    observed: &Observed,
) -> f64 {
    let model = match patch_multi(base, slots, genome) {
        Some(m) => m,
        None => return WORST,
    };
    let out = match sim::simulate(&model, input) {
        Ok(o) => o,
        Err(_) => return WORST,
    };
    let mut sum = 0.0;
    let mut n = 0;
    for (var, obs) in observed {
        if let Some(e) = rmse_one(&out, var, obs) {
            sum += e;
            n += 1;
        }
    }
    if n == 0 {
        WORST
    } else {
        sum / n as f64
    }
}

/// 联合进化配置。
#[derive(Debug, Clone)]
pub struct JointConfig {
    pub pop: usize,
    pub gens: usize,
    pub seed: u64,
    pub tournament_k: usize,
    pub elitism: usize,
    pub parsimony: f64,
    pub sweep_hi: f64,
    /// Pareto 归档上限（仅 evolve_joint_pareto 用）。
    pub archive_cap: usize,
}
impl Default for JointConfig {
    fn default() -> Self {
        Self { pop: 40, gens: 30, seed: 42, tournament_k: 3, elitism: 2, parsimony: 0.0, sweep_hi: 100.0, archive_cap: 24 }
    }
}

/// 联合进化结果：每槽位的最佳候选 + 总误差 + 收敛史。
#[derive(Debug, Clone)]
pub struct JointResult {
    pub best: Vec<Candidate>,
    pub best_error: f64,
    pub best_cost: f64,
    pub history: Vec<f64>,
}

/// 总复杂度 = 各槽位骨架节点数之和。
fn total_complexity(genome: &[Candidate]) -> usize {
    genome.iter().map(|c| complexity(&c.expr)).sum()
}

/// 逐槽位变异（每槽 0.5 概率，用各槽 grammar/ctx；后代由算子契约保证合法）。
fn mutate_multi(
    genome: &[Candidate],
    slots: &[Slot],
    unit_env: &HashMap<String, Dimension>,
    sweep_hi: f64,
    rng: &mut Rng,
) -> Vec<Candidate> {
    genome
        .iter()
        .enumerate()
        .map(|(k, c)| {
            if rng.next_f64() < 0.5 {
                mutate(c, &slots[k].grammar, &slots[k].ctx, unit_env, sweep_hi, rng)
            } else {
                c.clone()
            }
        })
        .collect()
}

/// 逐槽位交叉。
fn crossover_multi(
    a: &[Candidate],
    b: &[Candidate],
    slots: &[Slot],
    unit_env: &HashMap<String, Dimension>,
    sweep_hi: f64,
    rng: &mut Rng,
) -> Vec<Candidate> {
    a.iter()
        .zip(b)
        .enumerate()
        .map(|(k, (ca, cb))| crossover(ca, cb, &slots[k].ctx, unit_env, sweep_hi, rng))
        .collect()
}

fn tournament<'a>(scored: &'a [(Vec<Candidate>, f64, f64)], k: usize, rng: &mut Rng) -> &'a Vec<Candidate> {
    let mut best = rng.next_usize(scored.len());
    for _ in 1..k.max(1) {
        let i = rng.next_usize(scored.len());
        if scored[i].1 < scored[best].1 {
            best = i;
        }
    }
    &scored[best].0
}

/// 联合进化（单目标）。`error_fn(genome)`=多槽位拟合误差（越小越好）。
pub fn evolve_joint<F: FnMut(&[Candidate]) -> f64>(
    slots: &[Slot],
    unit_env: &HashMap<String, Dimension>,
    cfg: &JointConfig,
    mut error_fn: F,
) -> JointResult {
    let mut rng = Rng::new(cfg.seed);
    // 初始种群：每个体 = 各槽位采样一个候选
    let sample_genome = |rng: &mut Rng| -> Vec<Candidate> {
        slots
            .iter()
            .map(|s| {
                sample(&s.grammar, &s.ctx, rng).unwrap_or(Candidate {
                    expr: Expr::constant(0.0),
                    consts: vec![],
                })
            })
            .collect()
    };
    let mut pop: Vec<Vec<Candidate>> = (0..cfg.pop.max(1)).map(|_| sample_genome(&mut rng)).collect();

    let mut best: Option<(Vec<Candidate>, f64, f64)> = None;
    let mut history = Vec::with_capacity(cfg.gens);

    for _gen in 0..cfg.gens.max(1) {
        let mut scored: Vec<(Vec<Candidate>, f64, f64)> = pop
            .iter()
            .map(|g| {
                let err = error_fn(g);
                let err = if err.is_finite() { err } else { WORST };
                let cost = err + cfg.parsimony * total_complexity(g) as f64;
                (g.clone(), cost, err)
            })
            .collect();
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if best.as_ref().map_or(true, |b| scored[0].1 < b.1) {
            best = Some(scored[0].clone());
        }
        history.push(best.as_ref().unwrap().1);

        let mut next: Vec<Vec<Candidate>> = scored
            .iter()
            .take(cfg.elitism.min(scored.len()))
            .map(|s| s.0.clone())
            .collect();
        while next.len() < cfg.pop {
            let pa = tournament(&scored, cfg.tournament_k, &mut rng).clone();
            let pb = tournament(&scored, cfg.tournament_k, &mut rng).clone();
            let child = crossover_multi(&pa, &pb, slots, unit_env, cfg.sweep_hi, &mut rng);
            let child = mutate_multi(&child, slots, unit_env, cfg.sweep_hi, &mut rng);
            next.push(child);
        }
        pop = next;
    }

    let (bc, bcost, berr) = best.unwrap();
    JointResult { best: bc, best_error: berr, best_cost: bcost, history }
}

/// 联合 Pareto 前沿一项（整模型配置：每槽一个候选 + 总误差 + 总复杂度）。
#[derive(Debug, Clone)]
pub struct JointParetoEntry {
    pub genome: Vec<Candidate>,
    pub error: f64,
    pub complexity: usize,
}

fn eval_genome<F: FnMut(&[Candidate]) -> f64>(g: &[Candidate], error_fn: &mut F) -> JointParetoEntry {
    let e = error_fn(g);
    let e = if e.is_finite() { e } else { WORST };
    JointParetoEntry { genome: g.to_vec(), error: e, complexity: total_complexity(g) }
}

/// **Pareto-joint**：多槽位联合进化，返回 (总误差, 总复杂度) 非支配前沿——
/// 每个前沿点 = 一**套**形式（每槽一个），代表整模型的"精度 vs 简洁"权衡，科学家挑拐点。
/// 复用 NSGA-II 助手（`pareto::nondominated_fronts_obj`/`crowding_select_obj`）。
pub fn evolve_joint_pareto<F: FnMut(&[Candidate]) -> f64>(
    slots: &[Slot],
    unit_env: &HashMap<String, Dimension>,
    cfg: &JointConfig,
    error_fn: F,
) -> Vec<JointParetoEntry> {
    // 默认无进度回调；异步任务用 `evolve_joint_pareto_cb` 拿每代进度。
    evolve_joint_pareto_cb(slots, unit_env, cfg, error_fn, &mut |_, _| {})
}

/// 同 [`evolve_joint_pareto`]，额外每代回调 `progress(gen_1based, best_total_error)`——供异步任务画收敛曲线。
pub fn evolve_joint_pareto_cb<F: FnMut(&[Candidate]) -> f64>(
    slots: &[Slot],
    unit_env: &HashMap<String, Dimension>,
    cfg: &JointConfig,
    mut error_fn: F,
    progress: &mut dyn FnMut(usize, f64),
) -> Vec<JointParetoEntry> {
    use super::pareto::{crowding_select_obj, nondominated_fronts_obj};
    let mut rng = Rng::new(cfg.seed);
    let best_of = |a: &[JointParetoEntry]| -> f64 {
        a.iter().map(|e| e.error).fold(f64::INFINITY, f64::min)
    };
    let sample_genome = |rng: &mut Rng| -> Vec<Candidate> {
        slots
            .iter()
            .map(|s| {
                sample(&s.grammar, &s.ctx, rng).unwrap_or(Candidate { expr: Expr::constant(0.0), consts: vec![] })
            })
            .collect()
    };
    let mut archive: Vec<JointParetoEntry> = (0..cfg.pop.max(1))
        .map(|_| {
            let g = sample_genome(&mut rng);
            eval_genome(&g, &mut error_fn)
        })
        .collect();

    for gen in 0..cfg.gens.max(1) {
        let mut offspring: Vec<JointParetoEntry> = Vec::with_capacity(cfg.pop);
        for _ in 0..cfg.pop {
            let a = archive[rng.next_usize(archive.len())].genome.clone();
            let b = archive[rng.next_usize(archive.len())].genome.clone();
            let child = crossover_multi(&a, &b, slots, unit_env, cfg.sweep_hi, &mut rng);
            let child = mutate_multi(&child, slots, unit_env, cfg.sweep_hi, &mut rng);
            offspring.push(eval_genome(&child, &mut error_fn));
        }
        let mut combined = archive;
        combined.append(&mut offspring);
        let objs: Vec<(f64, f64)> = combined.iter().map(|e| (e.error, e.complexity as f64)).collect();
        let fronts = nondominated_fronts_obj(&objs);
        let mut next: Vec<JointParetoEntry> = Vec::with_capacity(cfg.archive_cap);
        for front in fronts {
            if next.len() + front.len() <= cfg.archive_cap {
                for i in front {
                    next.push(combined[i].clone());
                }
            } else {
                for i in crowding_select_obj(&objs, &front, cfg.archive_cap - next.len()) {
                    next.push(combined[i].clone());
                }
                break;
            }
        }
        archive = next;
        progress(gen + 1, best_of(&archive)); // 每代末：当前代号 + 归档最小总误差
    }

    let objs: Vec<(f64, f64)> = archive.iter().map(|e| (e.error, e.complexity as f64)).collect();
    let fronts = nondominated_fronts_obj(&objs);
    let mut front: Vec<JointParetoEntry> = fronts
        .first()
        .map(|f| f.iter().map(|&i| archive[i].clone()).collect())
        .unwrap_or_default();
    front.sort_by(|a, b| {
        a.complexity
            .cmp(&b.complexity)
            .then(a.error.partial_cmp(&b.error).unwrap_or(std::cmp::Ordering::Equal))
    });
    // 去重：收敛到同一 (复杂度,误差) 的多份拷贝只留一个（显示成不同权衡点才有意义）
    front.dedup_by(|a, b| a.complexity == b.complexity && (a.error - b.error).abs() < 1e-9);
    front
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gp::constraints::eval_candidate;
    use crate::gp::grammar::sample_form;
    use indexmap::IndexMap;

    fn slot(target: &str, grammar: &str, inputs: &[&str], bounds: Option<[f64; 2]>, mono: &[(&str, &str)]) -> Slot {
        let mut m = IndexMap::new();
        for (v, d) in mono {
            m.insert(v.to_string(), d.to_string());
        }
        Slot {
            target_id: target.to_string(),
            grammar: grammar.to_string(),
            ctx: GpContext {
                inputs: inputs.iter().map(|s| s.to_string()).collect(),
                output_bounds: bounds,
                monotone: m,
            },
        }
    }

    /// ★ 联合合成复原：两个槽位各设已知真值函数，联合进化应**同时复原两者**。
    /// 误差闭包 = 两槽位输出各自 vs 真值的平均 rmse（表达式级，免搭整模型）。
    #[test]
    fn test_joint_recovers_two_slots() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let slots = vec![
            slot("A", "monotone_gate", &["chill", "gdd"], Some([0.0, 1.0]), &[("chill", "increasing")]),
            slot("B", "saturating_sink", &["lai", "laip"], Some([0.0, 1.0]), &[("lai", "decreasing")]),
        ];
        // 真值：A=sigmoid 门控；B=线性饱和
        let truth_a = |x: f64| 1.0 / (1.0 + (-0.3 * (x - 20.0)).exp());
        let truth_b = |x: f64| (1.0 - x / 3.0).max(0.0);
        let xa: Vec<f64> = (0..=40).map(|i| i as f64).collect();
        let xb: Vec<f64> = (0..=30).map(|i| i as f64 * 0.2).collect();
        let mut ef = |g: &[Candidate]| {
            let mut ea = 0.0;
            for &x in &xa {
                match eval_candidate(&g[0], &[("chill", x), ("gdd", 0.0)]) {
                    Some(y) => ea += (y - truth_a(x)).powi(2),
                    None => return WORST,
                }
            }
            let mut eb = 0.0;
            for &x in &xb {
                match eval_candidate(&g[1], &[("lai", x), ("laip", 3.0)]) {
                    Some(y) => eb += (y - truth_b(x)).powi(2),
                    None => return WORST,
                }
            }
            ((ea / xa.len() as f64).sqrt() + (eb / xb.len() as f64).sqrt()) / 2.0
        };
        let cfg = JointConfig { pop: 60, gens: 40, seed: 1, ..Default::default() };
        let res = evolve_joint(&slots, &env, &cfg, &mut ef);
        assert_eq!(res.best.len(), 2, "两槽位");
        assert!(res.best_error < 0.05, "联合应同时复原两者: {}", res.best_error);
        for w in res.history.windows(2) {
            assert!(w[1] <= w[0] + 1e-12, "history 单调不增");
        }
    }

    /// 联合 Pareto 进度回调：每代调一次（gen 1..=gens 递增），cb(no-op) 与 evolve_joint_pareto 结果一致。
    #[test]
    fn test_joint_pareto_progress_cb() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let slots = vec![
            slot("A", "monotone_gate", &["chill", "gdd"], Some([0.0, 1.0]), &[("chill", "increasing")]),
            slot("B", "saturating_sink", &["lai", "laip"], Some([0.0, 1.0]), &[("lai", "decreasing")]),
        ];
        let mk = || {
            let truth_a = |x: f64| 1.0 / (1.0 + (-0.3 * (x - 20.0)).exp());
            move |g: &[Candidate]| {
                let mut ea = 0.0;
                for i in 0..=20 {
                    let x = i as f64;
                    match eval_candidate(&g[0], &[("chill", x), ("gdd", 0.0)]) {
                        Some(y) => ea += (y - truth_a(x)).powi(2),
                        None => return WORST,
                    }
                }
                (ea / 21.0).sqrt()
            }
        };
        let cfg = JointConfig { pop: 20, gens: 6, seed: 1, archive_cap: 10, ..Default::default() };
        let mut gens_seen: Vec<usize> = Vec::new();
        let front_cb = evolve_joint_pareto_cb(&slots, &env, &cfg, mk(), &mut |g, _e| gens_seen.push(g));
        assert_eq!(gens_seen, (1..=6).collect::<Vec<_>>(), "应每代回调一次、代号递增");
        let front_plain = evolve_joint_pareto(&slots, &env, &cfg, mk());
        let key = |f: &[JointParetoEntry]| f.iter().map(|e| format!("{:.9}|{}", e.error, e.complexity)).collect::<Vec<_>>();
        assert_eq!(key(&front_cb), key(&front_plain));
    }

    /// patch_multi 命名空间化常数、替换两方程。
    #[test]
    fn test_patch_multi_namespaces() {
        let g = vec![
            Candidate { expr: Expr::param("__c0"), consts: vec![1.5] },
            Candidate { expr: Expr::add(Expr::param("__c0"), Expr::param("__c1")), consts: vec![2.0, 3.0] },
        ];
        let slots = vec![
            slot("A", "monotone_gate", &["chill"], None, &[]),
            slot("B", "saturating_sink", &["lai"], None, &[]),
        ];
        // 用一个最小模型测命名空间（两个目标方程）
        let m = mini_two_slot();
        let p = patch_multi(&m, &slots, &g).unwrap();
        // 槽 0 → __s0_c0=1.5；槽 1 → __s1_c0=2.0, __s1_c1=3.0（不撞名）
        assert_eq!(p.parameters.get("__s0_c0").unwrap().default, 1.5);
        assert_eq!(p.parameters.get("__s1_c0").unwrap().default, 2.0);
        assert_eq!(p.parameters.get("__s1_c1").unwrap().default, 3.0);
        assert!(p.parameters.get("__c0").is_none(), "原始 __c0 不应残留");
    }

    fn mini_two_slot() -> EquationFile {
        use crate::schema::{Equation, GpTarget, Metadata, Variable, VariableType};
        let mut variables = IndexMap::new();
        for nm in ["chill", "lai", "yA", "yB"] {
            variables.insert(
                nm.to_string(),
                Variable {
                    var_type: if nm.starts_with('y') { VariableType::Output } else { VariableType::Input },
                    dtype: Default::default(),
                    unit: None,
                    description: None,
                    label: None,
                    measurable: nm.starts_with('y'),
                    stress_factor: None,
                    stress_reduce: None,
                    source: None,
                    class: if nm.starts_with('y') { None } else { Some(crate::schema::VarClass::Driving) },
                    init: None,
                    rate: None,
                    prev: None,
                 instance: None },
            );
        }
        let gt = |g: &str| Some(GpTarget {
            grammar: g.to_string(),
            inputs: vec![],
            output_bounds: None,
            monotone: IndexMap::new(),
            frozen: false,
        });
        EquationFile {
            meta: Metadata {
                id: "M2".into(), model: "M2".into(), name_cn: "双槽".into(), name_en: None,
                version: "1".into(), description: None, reference: None, source_files: vec![],
                dt: 1.0, dt_seconds: None, calibration: None, modules: Default::default(), balance: vec![], lineage: None,
            },
            parameters: IndexMap::new(),
            variables,
            equations: vec![
                Equation { id: "A".into(), name: "A".into(), output: "yA".into(), expression: Expr::var("chill"), formula_display: None, reference: None, gp_target: gt("monotone_gate") , provenance: None, instance: None },
                Equation { id: "B".into(), name: "B".into(), output: "yB".into(), expression: Expr::var("lai"), formula_display: None, reference: None, gp_target: gt("saturating_sink") , provenance: None, instance: None },
            ],
         structure: None }
    }

    /// ★ Pareto-joint：联合前沿非支配、含低误差、复杂度升序。
    #[test]
    fn test_joint_pareto_front() {
        let env: HashMap<String, Dimension> = HashMap::new();
        let slots = vec![
            slot("A", "monotone_gate", &["chill", "gdd"], Some([0.0, 1.0]), &[("chill", "increasing")]),
            slot("B", "saturating_sink", &["lai", "laip"], Some([0.0, 1.0]), &[("lai", "decreasing")]),
        ];
        let truth_a = |x: f64| 1.0 / (1.0 + (-0.3 * (x - 20.0)).exp());
        let truth_b = |x: f64| (1.0 - x / 3.0).max(0.0);
        let xa: Vec<f64> = (0..=40).map(|i| i as f64).collect();
        let xb: Vec<f64> = (0..=30).map(|i| i as f64 * 0.2).collect();
        let mut ef = |g: &[Candidate]| {
            let mut ea = 0.0;
            for &x in &xa {
                match eval_candidate(&g[0], &[("chill", x), ("gdd", 0.0)]) {
                    Some(y) => ea += (y - truth_a(x)).powi(2),
                    None => return WORST,
                }
            }
            let mut eb = 0.0;
            for &x in &xb {
                match eval_candidate(&g[1], &[("lai", x), ("laip", 3.0)]) {
                    Some(y) => eb += (y - truth_b(x)).powi(2),
                    None => return WORST,
                }
            }
            ((ea / xa.len() as f64).sqrt() + (eb / xb.len() as f64).sqrt()) / 2.0
        };
        let cfg = JointConfig { pop: 40, gens: 20, seed: 1, archive_cap: 16, ..Default::default() };
        let front = evolve_joint_pareto(&slots, &env, &cfg, &mut ef);
        assert!(!front.is_empty(), "前沿非空");
        // 每点是 2 槽位的整模型配置
        for e in &front {
            assert_eq!(e.genome.len(), 2);
        }
        // 互不支配
        for i in 0..front.len() {
            for j in 0..front.len() {
                if i != j {
                    let a = (front[i].error, front[i].complexity as f64);
                    let b = (front[j].error, front[j].complexity as f64);
                    assert!(!super::super::pareto::dominates_obj(a, b), "前沿内不应互相支配");
                }
            }
        }
        let best = front.iter().map(|e| e.error).fold(f64::INFINITY, f64::min);
        assert!(best < 0.06, "前沿应含低误差整模型配置: {best}");
        for w in front.windows(2) {
            assert!(w[0].complexity <= w[1].complexity, "复杂度升序");
        }
    }

    /// slots_from_model 抽出全部 gp_target 槽位。
    #[test]
    fn test_slots_from_model() {
        let m = mini_two_slot();
        let slots = slots_from_model(&m, None);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].target_id, "A");
        assert_eq!(slots[1].grammar, "saturating_sink");
        // 指定子集
        let one = slots_from_model(&m, Some(&["B".to_string()]));
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].target_id, "B");
    }
}
