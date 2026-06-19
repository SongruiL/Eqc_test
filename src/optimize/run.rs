//! 优化运行编排：把「校验 → DE 搜索 → 最优点再评一次」收成一个库函数，
//! 供 CLI（`eqc optimize`）与 serve（`/api/optimize`）**共用**——两边走同一条计算路径、
//! 产出同一份 JSON（[`result_json`]），不重复实现。
//!
//! 模型 / 决策 spec / 驱动量由**调用方**加载好（CLI 从文件，serve 从预载/spec 的 environment），
//! 这里只接收已就绪的数据。

use std::collections::HashMap;

use crate::schema::EquationFile;

use super::core::{evaluate, evaluate_mo, validate_problem, EvalOutcome};
use super::de::{differential_evolution, differential_evolution_mo, DeConfig};
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
        evaluate(file, problem, x, drivers, steps).cost
    });
    let outcome = evaluate(file, problem, &res.best_x, drivers, steps);

    Ok(OptimizeResult {
        best_knobs: res.best_x,
        outcome,
        history: res.history,
        best_cost: res.best_cost,
        config,
    })
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
}
