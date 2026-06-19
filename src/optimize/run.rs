//! 优化运行编排：把「校验 → DE 搜索 → 最优点再评一次」收成一个库函数，
//! 供 CLI（`eqc optimize`）与 serve（`/api/optimize`）**共用**——两边走同一条计算路径、
//! 产出同一份 JSON（[`result_json`]），不重复实现。
//!
//! 模型 / 决策 spec / 驱动量由**调用方**加载好（CLI 从文件，serve 从预载/spec 的 environment），
//! 这里只接收已就绪的数据。

use std::collections::HashMap;

use crate::schema::EquationFile;

use super::core::{
    evaluate, evaluate_mo, evaluate_obs, simulate_candidate, validate_problem, EvalOutcome,
};
use super::de::{differential_evolution, differential_evolution_mo, DeConfig};
use super::objective::ObservedData;
use super::problem::{Problem, Sense};

/// 一次优化的完整结果（供 CLI 打印 / serve 转 JSON 共用）。
pub struct OptimizeResult {
    /// 最优旋钮取值（与 `problem.knobs` 一一对应）。
    pub best_knobs: Vec<f64>,
    /// 最优点的完整评估（目标值 / 可行性 / 惩罚 / 逐约束明细）。
    pub outcome: EvalOutcome,
    /// 收敛轨迹（每代至今最优代价）。
    pub history: Vec<f64>,
    /// 最优代价（= `outcome.cost`，DE 报告的最优）。
    pub best_cost: f64,
    /// 实际使用的 DE 配置。
    pub config: DeConfig,
}

/// 跑一次优化：校验决策 spec → DE 搜旋钮空间 → 用最优旋钮再评一次拿完整结果。
///
/// 失败仅在「spec 与模型不匹配」或「优化器不支持」时返回 `Err`；搜索过程中的垃圾候选
/// 由评估核映射成最差代价、不影响这里。
pub fn run(
    file: &EquationFile,
    problem: &Problem,
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
) -> Result<OptimizeResult, String> {
    run_obs(file, problem, drivers, steps, &ObservedData::new())
}

/// 同 [`run`]，但额外提供**实测数据**——目标可用误差算子（`rmse` 等）。**参数标定**用：
/// 旋钮=参数、目标=预测 vs 实测的误差。底层与决策优化是同一条计算路径。
pub fn run_obs(
    file: &EquationFile,
    problem: &Problem,
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
    observed: &ObservedData,
) -> Result<OptimizeResult, String> {
    validate_problem(file, problem)?;
    if problem.optimizer.method != "de" {
        return Err(format!(
            "当前仅支持 method: de（收到 '{}'）",
            problem.optimizer.method
        ));
    }
    let config = DeConfig {
        pop: problem.optimizer.pop,
        iters: problem.optimizer.iters,
        seed: problem.optimizer.seed,
        ..Default::default()
    };
    let bounds: Vec<(f64, f64)> =
        problem.knobs.iter().map(|k| (k.bounds[0], k.bounds[1])).collect();

    let res = differential_evolution(&bounds, &config, |x| {
        evaluate_obs(file, problem, x, drivers, steps, observed).cost
    });
    let outcome = evaluate_obs(file, problem, &res.best_x, drivers, steps, observed);

    Ok(OptimizeResult {
        best_knobs: res.best_x,
        outcome,
        history: res.history,
        best_cost: res.best_cost,
        config,
    })
}

/// 单个参数对各候选观测变量的敏感性（可辨识性分析）。
pub struct ParamSens {
    pub param: String,
    /// (观测变量, 敏感度 = 参数扰动引起的整条轨迹 RMS 变化)，按敏感度降序。
    pub per_observable: Vec<(String, f64)>,
    /// 是否可辨识：在候选观测下最大敏感度 ≥ 阈值（否则无观测能约束它）。
    pub identifiable: bool,
}

/// 可辨识性 / 「该测什么」报告。
pub struct IdentReport {
    pub observables: Vec<String>,
    pub params: Vec<ParamSens>,
    /// 可能**异参同效**的参数对（敏感模式高度相关，难分辨）：(参数a, 参数b, 相关系数)。
    pub confounded: Vec<(String, String, f64)>,
}

/// **可辨识性分析**（服务实验设计）：对每个候选参数 ±`percent`% 扰动，量其对**每个候选可观测变量**
/// 整条轨迹的 RMS 影响 → 敏感矩阵。回答：要定准某参数最该测哪个变量、哪些参数无观测能约束（不可辨识）、
/// 哪些参数对可能异参同效。见 `docs/spec-calibration.md` §5。候选参数 = `problem.knobs`（kind=param）。
pub fn identifiability(
    file: &EquationFile,
    problem: &Problem,
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
    observables: &[String],
    percent: f64,
    rel: f64,
) -> Result<IdentReport, String> {
    let nk = problem.knobs.len();
    let baseline: Vec<f64> =
        problem.knobs.iter().map(|k| 0.5 * (k.bounds[0] + k.bounds[1])).collect();

    // 基线轨迹（用于把敏感度归一成**相对**变化，跨不同量级观测可比）。
    let base_out = simulate_candidate(file, problem, &baseline, drivers, steps)?;
    let rms = |s: &[f64]| -> f64 {
        if s.is_empty() {
            0.0
        } else {
            (s.iter().map(|x| x * x).sum::<f64>() / s.len() as f64).sqrt()
        }
    };
    let base_rms: Vec<f64> = observables.iter().map(|v| base_out.series(v).map(rms).unwrap_or(0.0)).collect();

    // 敏感矩阵 mat[参数][观测] = 扰动引起的该观测轨迹**相对** RMS 变化（÷ 基线 RMS）
    let mut mat = vec![vec![0.0_f64; observables.len()]; nk];
    for i in 0..nk {
        let (lo, hi) = (problem.knobs[i].bounds[0], problem.knobs[i].bounds[1]);
        let h = (percent / 100.0) * (hi - lo);
        if h <= 0.0 {
            continue;
        }
        let mut xm = baseline.clone();
        xm[i] = (baseline[i] - h).max(lo);
        let mut xp = baseline.clone();
        xp[i] = (baseline[i] + h).min(hi);
        let om = simulate_candidate(file, problem, &xm, drivers, steps)?;
        let op = simulate_candidate(file, problem, &xp, drivers, steps)?;
        for (j, v) in observables.iter().enumerate() {
            let abs_change = match (om.series(v), op.series(v)) {
                (Some(a), Some(b)) if !a.is_empty() && a.len() == b.len() => {
                    let n = a.len() as f64;
                    (a.iter().zip(b).map(|(x, y)| (y - x) * (y - x)).sum::<f64>() / n).sqrt()
                }
                _ => 0.0, // 该观测变量不在轨迹里 → 无法约束
            };
            // 相对化：÷ 基线 RMS（基线≈0 时退回绝对值，避免除零放大）
            mat[i][j] = if base_rms[j] > 1e-9 { abs_change / base_rms[j] } else { abs_change };
        }
    }

    let gmax = mat.iter().flatten().cloned().fold(0.0_f64, f64::max);
    let thresh = rel * gmax;
    let mut params = Vec::with_capacity(nk);
    for i in 0..nk {
        let mut per: Vec<(String, f64)> =
            observables.iter().cloned().zip(mat[i].iter().cloned()).collect();
        per.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let maxs = mat[i].iter().cloned().fold(0.0_f64, f64::max);
        params.push(ParamSens {
            param: problem.knobs[i].var.clone(),
            per_observable: per,
            identifiable: gmax > 0.0 && maxs > 0.0 && maxs >= thresh,
        });
    }

    // 异参同效：敏感行向量两两相关，高相关 → 可能难分辨
    let mut confounded = Vec::new();
    for a in 0..nk {
        for b in (a + 1)..nk {
            if let Some(r) = pearson(&mat[a], &mat[b]) {
                if r > 0.99 {
                    confounded.push((
                        problem.knobs[a].var.clone(),
                        problem.knobs[b].var.clone(),
                        r,
                    ));
                }
            }
        }
    }
    Ok(IdentReport { observables: observables.to_vec(), params, confounded })
}

/// 皮尔逊相关系数（长度 <2 或方差为 0 → None）。
fn pearson(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len() as f64;
    if a.len() < 2 {
        return None;
    }
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        cov += (x - ma) * (y - mb);
        va += (x - ma) * (x - ma);
        vb += (y - mb) * (y - mb);
    }
    if va <= 0.0 || vb <= 0.0 {
        return None;
    }
    Some(cov / (va.sqrt() * vb.sqrt()))
}

/// 敏感性预筛结果：在搜索前用 OAT 扰动判定哪些旋钮对目标几乎无影响、可固定。
pub struct PrescreenResult {
    /// 每旋钮基线（边界中点）。
    pub baseline: Vec<f64>,
    /// 每旋钮 `|obj(基线+h) − obj(基线−h)|`（h = `percent%` × 边界宽）。
    pub deltas: Vec<f64>,
    /// 值得搜索的旋钮下标（敏感）。
    pub kept: Vec<usize>,
    /// 被判低敏感、将固定在基线的旋钮下标。
    pub dropped: Vec<usize>,
}

/// **敏感性预筛**（优化前）：每个旋钮在基线（边界中点）附近 ±`percent`% 各扰动一次，
/// 看**目标**的变化 `|Δobj|`；变化 < `rel` × 最大变化的旋钮判为低敏感、建议固定。
/// 与 `eqc sweep --sensitivity` 同思路，但作用于**旋钮**（含 init/driver_const）+ **目标**。
/// 单目标用 `problem.objective`。基线候选须可求值，否则报错（无法预筛）。
pub fn prescreen(
    file: &EquationFile,
    problem: &Problem,
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
    percent: f64,
    rel: f64,
) -> Result<PrescreenResult, String> {
    let nk = problem.knobs.len();
    let baseline: Vec<f64> =
        problem.knobs.iter().map(|k| 0.5 * (k.bounds[0] + k.bounds[1])).collect();

    let base = evaluate(file, problem, &baseline, drivers, steps);
    if base.objective.is_none() {
        return Err(format!("预筛基线候选无法求值：{}", base.note.unwrap_or_default()));
    }

    let mut deltas = vec![0.0_f64; nk];
    for i in 0..nk {
        let (lo, hi) = (problem.knobs[i].bounds[0], problem.knobs[i].bounds[1]);
        let h = (percent / 100.0) * (hi - lo);
        if h <= 0.0 {
            continue; // 退化旋钮（边界已收拢）→ 视为零敏感
        }
        let mut xm = baseline.clone();
        xm[i] = (baseline[i] - h).max(lo);
        let mut xp = baseline.clone();
        xp[i] = (baseline[i] + h).min(hi);
        let om = evaluate(file, problem, &xm, drivers, steps).objective;
        let op = evaluate(file, problem, &xp, drivers, steps).objective;
        deltas[i] = match (om, op) {
            (Some(a), Some(b)) => (b - a).abs(),
            _ => 0.0,
        };
    }

    let maxd = deltas.iter().cloned().fold(0.0_f64, f64::max);
    let thresh = rel * maxd;
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for (i, &d) in deltas.iter().enumerate() {
        if maxd > 0.0 && d > 0.0 && d >= thresh {
            kept.push(i);
        } else {
            dropped.push(i);
        }
    }
    // 保险：全被判低敏感（如 maxd=0）→ 全保留，避免空搜索。
    if kept.is_empty() {
        kept = (0..nk).collect();
        dropped.clear();
    }
    Ok(PrescreenResult { baseline, deltas, kept, dropped })
}

/// 多目标前沿上的一个点。
pub struct MoFrontPoint {
    /// 旋钮取值（与 `problem.knobs` 对应）。
    pub knobs: Vec<f64>,
    /// 两个目标的原始值 `[obj1, obj2]`。
    pub objectives: Vec<f64>,
    pub feasible: bool,
    pub penalty: f64,
}

/// 多目标优化结果：一条近似 Pareto 前沿（按目标 1 升序）+ DE 配置。
pub struct MoResult {
    pub front: Vec<MoFrontPoint>,
    pub config: DeConfig,
}

/// 跑一次**多目标**优化（MO-DE，单次运行近似整条前沿）。要求 `problem.objective2` 为 `Some`。
pub fn run_mo(
    file: &EquationFile,
    problem: &Problem,
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
) -> Result<MoResult, String> {
    validate_problem(file, problem)?;
    if problem.optimizer.method != "de" {
        return Err(format!("当前仅支持 method: de（收到 '{}'）", problem.optimizer.method));
    }
    if problem.objective2.is_none() {
        return Err("多目标模式需要 objective2".into());
    }
    let config = DeConfig {
        pop: problem.optimizer.pop,
        iters: problem.optimizer.iters,
        seed: problem.optimizer.seed,
        ..Default::default()
    };
    let bounds: Vec<(f64, f64)> =
        problem.knobs.iter().map(|k| (k.bounds[0], k.bounds[1])).collect();

    let archive = differential_evolution_mo(&bounds, &config, |x| {
        evaluate_mo(file, problem, x, drivers, steps).costs
    });

    // 用前沿各点重评一次，取原始目标值 + 可行性（点数少，开销小）。
    let mut front = Vec::with_capacity(archive.len());
    for s in &archive {
        let mo = evaluate_mo(file, problem, &s.x, drivers, steps);
        front.push(MoFrontPoint {
            knobs: s.x.clone(),
            objectives: mo.objectives.unwrap_or_default(),
            feasible: mo.feasible,
            penalty: mo.penalty,
        });
    }
    Ok(MoResult { front, config })
}

/// 多目标结果 JSON（CLI 写文件 / serve 端点返回，同一份结构）。
pub fn mo_result_json(
    file: &EquationFile,
    problem: &Problem,
    r: &MoResult,
) -> serde_json::Value {
    let sense = |o: &super::problem::Objective| match o.sense {
        Sense::Max => "max",
        Sense::Min => "min",
    };
    let o2 = problem.objective2.as_ref();
    let objectives = serde_json::json!([
        { "expr": problem.objective.expr, "sense": sense(&problem.objective) },
        o2.map(|o| serde_json::json!({ "expr": o.expr, "sense": sense(o) })).unwrap_or(serde_json::Value::Null),
    ]);
    let front: Vec<serde_json::Value> = r
        .front
        .iter()
        .map(|p| {
            let knobs: Vec<serde_json::Value> = problem
                .knobs
                .iter()
                .zip(&p.knobs)
                .map(|(k, v)| {
                    serde_json::json!({ "var": k.var, "kind": k.kind.as_str(), "value": v, "unit": k.unit })
                })
                .collect();
            serde_json::json!({
                "knobs": knobs,
                "objectives": p.objectives,
                "feasible": p.feasible,
            })
        })
        .collect();
    serde_json::json!({
        "model": file.meta.id,
        "multi_objective": true,
        "objectives": objectives,
        "knob_names": problem.knobs.iter().map(|k| k.var.clone()).collect::<Vec<_>>(),
        "front": front,
        "optimizer": { "method": "de", "pop": r.config.pop, "iters": r.config.iters, "seed": r.config.seed },
    })
}

/// 把优化结果序列化成 JSON（CLI 写文件 / serve 端点返回，**同一份结构**）。
pub fn result_json(
    file: &EquationFile,
    problem: &Problem,
    r: &OptimizeResult,
) -> serde_json::Value {
    let sense_str = match problem.objective.sense {
        Sense::Max => "max",
        Sense::Min => "min",
    };
    let best_knobs: Vec<serde_json::Value> = problem
        .knobs
        .iter()
        .zip(&r.best_knobs)
        .map(|(k, v)| {
            serde_json::json!({
                "var": k.var,
                "kind": k.kind.as_str(),
                "value": v,
                "unit": k.unit,
                "bounds": [k.bounds[0], k.bounds[1]],
            })
        })
        .collect();
    let constraints: Vec<serde_json::Value> = r
        .outcome
        .constraints
        .iter()
        .map(|cs| {
            serde_json::json!({
                "expr": cs.expr,
                "value": cs.value,
                "max": cs.max,
                "violation": cs.violation,
                "satisfied": cs.violation == 0.0,
            })
        })
        .collect();
    serde_json::json!({
        "model": file.meta.id,
        "objective": { "expr": problem.objective.expr, "sense": sense_str },
        "best_knobs": best_knobs,
        "objective_value": r.outcome.objective,
        "feasible": r.outcome.feasible,
        "penalty": r.outcome.penalty,
        "constraints": constraints,
        "best_cost": r.best_cost,
        "optimizer": { "method": "de", "pop": r.config.pop, "iters": r.config.iters, "seed": r.config.seed },
        "history": r.history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::problem::parse_problem;
    use crate::parser::parse_file;
    use std::io::Write;
    use tempfile::TempDir;

    fn model() -> (TempDir, EquationFile) {
        let yaml = r#"
meta: { id: OPT, model: Opt, name_cn: 优化测试 }
parameters:
  gain: { name_cn: 增益, default: 2.0 }
variables:
  drive: { type: input, class: driving }
  Y: { type: output, class: state, init: 0.0, rate: r }
  r: { type: intermediate, class: rate }
equations:
  - { id: E1, name: 速率, output: r, expression: { op: mul, args: [ {ref: drive}, {ref: gain} ] } }
"#;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("m.eq.yaml");
        std::fs::File::create(&path).unwrap().write_all(yaml.as_bytes()).unwrap();
        (dir, parse_file(&path).unwrap())
    }

    fn drivers3() -> HashMap<String, Vec<f64>> {
        let mut d = HashMap::new();
        d.insert("drive".to_string(), vec![1.0, 1.0, 1.0]);
        d
    }

    #[test]
    fn test_run_maximize_gain() {
        let (_d, file) = model();
        // max final(Y)，旋钮 gain∈[1,5]；Y final = 3·gain·1（drive=1,3步）→ gain↑则 Y↑ → 最优 gain=5
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y), sense: max }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n  optimizer: { pop: 20, iters: 60, seed: 1 }\n",
        )
        .unwrap();
        let r = run(&file, &p, &drivers3(), 3).unwrap();
        assert!((r.best_knobs[0] - 5.0).abs() < 1e-3, "gain = {}", r.best_knobs[0]);
        assert!((r.outcome.objective.unwrap() - 15.0).abs() < 1e-2); // 3·5·1
        assert!(r.outcome.feasible);
    }

    #[test]
    fn test_run_deterministic() {
        let (_d, file) = model();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n  optimizer: { pop: 15, iters: 30, seed: 7 }\n",
        )
        .unwrap();
        let r1 = run(&file, &p, &drivers3(), 3).unwrap();
        let r2 = run(&file, &p, &drivers3(), 3).unwrap();
        assert_eq!(r1.best_knobs, r2.best_knobs);
        assert_eq!(r1.history, r2.history);
    }

    #[test]
    fn test_result_json_shape() {
        let (_d, file) = model();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n  optimizer: { pop: 10, iters: 10, seed: 1 }\n",
        )
        .unwrap();
        let r = run(&file, &p, &drivers3(), 3).unwrap();
        let j = result_json(&file, &p, &r);
        assert_eq!(j["model"], "OPT");
        assert_eq!(j["objective"]["sense"], "max");
        assert_eq!(j["best_knobs"][0]["var"], "gain");
        assert!(j["history"].as_array().unwrap().len() >= 2);
        assert_eq!(j["feasible"], true);
    }

    #[test]
    fn test_run_rejects_non_de() {
        let (_d, file) = model();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n  optimizer: { method: cmaes }\n",
        )
        .unwrap();
        assert!(run(&file, &p, &drivers3(), 3).is_err());
    }

    /// 带一个对目标无影响的参数 noise（只进入 Z，不进入 Y）的模型。
    fn model_with_inert() -> (TempDir, EquationFile) {
        let yaml = r#"
meta: { id: OPT2, model: Opt2, name_cn: 预筛测试 }
parameters:
  gain:  { name_cn: 增益, default: 2.0 }
  noise: { name_cn: 无关量, default: 1.0 }
variables:
  drive: { type: input, class: driving }
  Y: { type: output, class: state, init: 0.0, rate: r }
  r: { type: intermediate, class: rate }
  Z: { type: output }
equations:
  - { id: E1, name: 速率, output: r, expression: { op: mul, args: [ {ref: drive}, {ref: gain} ] } }
  - { id: E2, name: 旁路, output: Z, expression: { op: mul, args: [ {ref: drive}, {ref: noise} ] } }
"#;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("m.eq.yaml");
        std::fs::File::create(&path).unwrap().write_all(yaml.as_bytes()).unwrap();
        (dir, parse_file(&path).unwrap())
    }

    #[test]
    fn test_calibrate_recovers_param() {
        // recover-the-params：真值 gain=3 → Y=[3,6,9] 当伪实测；从中标定应找回 gain≈3。
        let (_d, file) = model();
        let observed: ObservedData =
            [("obs_Y".to_string(), vec![(1usize, 3.0), (2, 6.0), (3, 9.0)])].into_iter().collect();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (rmse Y obs_Y), sense: min }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n  optimizer: { pop: 20, iters: 80, seed: 1 }\n",
        )
        .unwrap();
        let r = run_obs(&file, &p, &drivers3(), 3, &observed).unwrap();
        assert!((r.best_knobs[0] - 3.0).abs() < 1e-2, "应找回 gain≈3，得 {}", r.best_knobs[0]);
        assert!(r.outcome.objective.unwrap() < 1e-3, "拟合误差应接近 0");
    }

    #[test]
    fn test_identifiability_matches_observables_to_params() {
        // model_with_inert：Y 只受 gain、Z 只受 noise。
        // 候选观测 [Y, Z] → gain 最该测 Y、noise 最该测 Z；都可辨识。
        let (_d, file) = model_with_inert();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain,  kind: param, bounds: [1, 5] }\n    - { var: noise, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let obs = vec!["Y".to_string(), "Z".to_string()];
        let rep = identifiability(&file, &p, &drivers3(), 3, &obs, 10.0, 0.01).unwrap();
        let gain = rep.params.iter().find(|p| p.param == "gain").unwrap();
        let noise = rep.params.iter().find(|p| p.param == "noise").unwrap();
        assert!(gain.identifiable && noise.identifiable);
        assert_eq!(gain.per_observable[0].0, "Y", "gain 最该测 Y");
        assert_eq!(noise.per_observable[0].0, "Z", "noise 最该测 Z");
        // gain 对 Z 无影响、noise 对 Y 无影响
        assert_eq!(gain.per_observable.iter().find(|(v, _)| v == "Z").unwrap().1, 0.0);
        assert_eq!(noise.per_observable.iter().find(|(v, _)| v == "Y").unwrap().1, 0.0);
    }

    #[test]
    fn test_identifiability_flags_unobservable_param() {
        // 只测 Y → noise（只进 Z）不可辨识
        let (_d, file) = model_with_inert();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain,  kind: param, bounds: [1, 5] }\n    - { var: noise, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let rep = identifiability(&file, &p, &drivers3(), 3, &["Y".to_string()], 10.0, 0.01).unwrap();
        let noise = rep.params.iter().find(|p| p.param == "noise").unwrap();
        assert!(!noise.identifiable, "只测 Y 时 noise 不可辨识");
    }

    #[test]
    fn test_prescreen_drops_inert_knob() {
        let (_d, file) = model_with_inert();
        // 目标 (final Y) 只受 gain 影响；noise 只进 Z → 应被预筛剔除
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y), sense: max }\n  knobs:\n    - { var: gain,  kind: param, bounds: [1, 5] }\n    - { var: noise, kind: param, bounds: [0, 10] }\n",
        )
        .unwrap();
        let pr = prescreen(&file, &p, &drivers3(), 3, 10.0, 0.01).unwrap();
        assert!(pr.deltas[0] > 0.0, "gain 应有敏感性");
        assert_eq!(pr.deltas[1], 0.0, "noise 对 Y 零影响");
        assert_eq!(pr.kept, vec![0], "只保留 gain");
        assert_eq!(pr.dropped, vec![1], "剔除 noise");
        // 基线 = 边界中点
        assert_eq!(pr.baseline, vec![3.0, 5.0]);
    }
}
