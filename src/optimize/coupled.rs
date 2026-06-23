//! 耦合优化（C3，最小可用）：把**多速率耦合仿真**包成前向模型，DE 搜温室/作物参数旋钮，
//! 目标归约**作物（慢）轨迹**。复用 [`super::de`] 的 DE + [`super::objective::eval_objective`]。
//! 见 `docs/spec-coupled-simulation.md` §8。
//!
//! 这正是 `optimize_force_de.py` 在 Python 外面用离线管道做的事，搬进 EQC 一个进程、一份声明式
//! spec、用测过的 DE。**为什么非双向不可**：旋钮（如 CO₂ 注入）的真实边际收益取决于作物回吃
//! CO₂、蒸腾改通风——双向前向模型才让优化器找到对的环控（C2 已坐实闭环）。
//!
//! 范围（v1）：单目标、无约束。约束/Pareto/Studio 面板复用 = 后续（需把 `core` 抽象成前向模型无关）。

use std::collections::HashMap;

use crate::schema::EquationFile;
use crate::sim::{simulate_coupled, CoupledInput, CoupledLink, FeedbackLink};

use super::core::WORST_COST;
use super::de::{differential_evolution, DeConfig};
use super::objective::eval_objective;
use super::problem::{KnobKind, Problem, Sense};

/// 耦合前向模型的固定部分（不含旋钮）：两模型 + 链接/反馈 + 室外天气 + 慢步数。
pub struct CoupledModel<'a> {
    pub fast: &'a EquationFile,
    pub slow: &'a EquationFile,
    pub links: Vec<CoupledLink>,
    pub feedback: Vec<FeedbackLink>,
    pub weather: HashMap<String, Vec<f64>>,
    pub slow_steps: usize,
    /// 温室固定参数（非旋钮的环控设置，如 Q_heat=0）；旋钮在其上再覆盖。
    pub base_fast_params: HashMap<String, f64>,
    /// 作物固定参数。
    pub base_slow_params: HashMap<String, f64>,
}

/// 一次耦合优化的结果。
pub struct CoupledOptimizeResult {
    /// 最优旋钮取值（与 `problem.knobs` 一一对应）。
    pub best_knobs: Vec<f64>,
    /// 最优目标值（**原始**，未取反；按 `sense` 已是最大/最小）。
    pub best_objective: f64,
    /// DE 最小化代价（= `sense调整(目标)`；垃圾候选 = `WORST_COST`）。
    pub best_cost: f64,
    /// 收敛轨迹（每代至今最优代价）。
    pub history: Vec<f64>,
    pub config: DeConfig,
}

/// 跑一次耦合优化：校验旋钮 → DE 搜 → 最优点再评。失败仅在 spec 与模型不匹配 / 优化器不支持时。
pub fn run_coupled(m: &CoupledModel, problem: &Problem) -> Result<CoupledOptimizeResult, String> {
    if problem.optimizer.method != "de" {
        return Err(format!("当前仅支持 method: de（收到 '{}'）", problem.optimizer.method));
    }
    if problem.knobs.is_empty() {
        return Err("耦合优化需至少一个旋钮".into());
    }
    if problem.objective2.is_some() {
        return Err("耦合优化 v1 仅单目标（暂不支持 objective2 多目标）".into());
    }
    if !problem.constraints.is_empty() {
        return Err("耦合优化 v1 暂不支持约束（constraints）".into());
    }
    // 旋钮 kind 必须 fast_param/slow_param，且确为对应模型参数
    for k in &problem.knobs {
        match k.kind {
            KnobKind::FastParam => {
                if !m.fast.parameters.contains_key(&k.var) {
                    return Err(format!("fast_param 旋钮 '{}' 不是温室模型 {} 的参数", k.var, m.fast.meta.id));
                }
            }
            KnobKind::SlowParam => {
                if !m.slow.parameters.contains_key(&k.var) {
                    return Err(format!("slow_param 旋钮 '{}' 不是作物模型 {} 的参数", k.var, m.slow.meta.id));
                }
            }
            other => {
                return Err(format!(
                    "耦合优化旋钮 kind 只支持 fast_param/slow_param（旋钮 '{}' 是 {}）",
                    k.var,
                    other.as_str()
                ))
            }
        }
    }

    let config = DeConfig {
        pop: problem.optimizer.pop,
        iters: problem.optimizer.iters,
        seed: problem.optimizer.seed,
        ..Default::default()
    };
    let bounds: Vec<(f64, f64)> = problem.knobs.iter().map(|k| (k.bounds[0], k.bounds[1])).collect();

    // 前向模型只构建一次（室外天气只克隆一次）；每次评估仅改 params。
    let mut input = CoupledInput::new(
        m.fast,
        m.slow,
        m.links.clone(),
        m.weather.clone(),
        m.slow_steps,
    );
    input.feedback = m.feedback.clone();
    let consts: HashMap<String, f64> =
        problem.constants.iter().map(|(k, v)| (k.clone(), *v)).collect();

    // 用一组旋钮值评一次 → 目标值（原始）。失败 → None。
    let mut eval = |x: &[f64]| -> Option<f64> {
        // 重置到固定基线，旋钮在其上覆盖
        input.fast_params = m.base_fast_params.clone();
        input.slow_params = m.base_slow_params.clone();
        let mut bindings = consts.clone();
        for (k, &v) in problem.knobs.iter().zip(x) {
            match k.kind {
                KnobKind::FastParam => {
                    input.fast_params.insert(k.var.clone(), v);
                }
                KnobKind::SlowParam => {
                    input.slow_params.insert(k.var.clone(), v);
                }
                _ => return None,
            }
            bindings.insert(k.var.clone(), v);
        }
        let out = simulate_coupled(&input).ok()?;
        eval_objective(&problem.objective.expr, &out.slow, &bindings).ok()
    };

    let res = differential_evolution(&bounds, &config, |x| match eval(x) {
        Some(v) if v.is_finite() => match problem.objective.sense {
            Sense::Max => -v,
            Sense::Min => v,
        },
        _ => WORST_COST,
    });
    let best_objective = eval(&res.best_x).unwrap_or(f64::NAN);

    Ok(CoupledOptimizeResult {
        best_knobs: res.best_x,
        best_objective,
        best_cost: res.best_cost,
        history: res.history,
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::problem::parse_problem;
    use crate::parser::parse_file;
    use std::io::Write;
    use tempfile::TempDir;

    fn model(yaml: &str) -> (TempDir, crate::schema::EquationFile) {
        let d = TempDir::new().unwrap();
        let p = d.path().join("m.eq.yaml");
        std::fs::File::create(&p).unwrap().write_all(yaml.as_bytes()).unwrap();
        let f = parse_file(&p).unwrap();
        (d, f)
    }

    /// 耦合优化：快模型 y=u+g（g=fast_param 旋钮），慢模型 z=mean(y)=1+g；max(final z) over g∈[0,5] → g=5。
    #[test]
    fn test_run_coupled_finds_optimum() {
        let (_df, fast) = model(
            "meta: { id: F, model: F, name_cn: 快, dt: 1, dt_seconds: 1 }\nparameters: { g: { name_cn: 旋钮, default: 0.0 } }\nvariables: { u: { type: input, class: driving }, y: { type: output } }\nequations: [ { id: E, name: y, output: y, expression: { op: add, args: [ { ref: u }, { ref: g } ] } } ]\n",
        );
        let (_ds, slow) = model(
            "meta: { id: S, model: S, name_cn: 慢, dt: 1, dt_seconds: 2 }\nvariables: { ybar: { type: input, class: driving }, z: { type: output } }\nequations: [ { id: E, name: z, output: z, expression: { ref: ybar } } ]\n",
        );
        let problem = parse_problem(
            "optimize:\n  knobs: [ { var: g, kind: fast_param, bounds: [0.0, 5.0] } ]\n  objective: { expr: \"(final z)\", sense: max }\n  optimizer: { pop: 8, iters: 20, seed: 1 }\n",
        )
        .unwrap();
        let mut weather = HashMap::new();
        weather.insert("u".to_string(), vec![1.0; 4]); // 2 慢步 × R=2
        let m = CoupledModel {
            fast: &fast,
            slow: &slow,
            links: vec![CoupledLink {
                to: "ybar".into(),
                from: "y".into(),
                agg: crate::sim::Agg::Mean,
                scale: 1.0,
            }],
            feedback: vec![],
            weather,
            slow_steps: 2,
            base_fast_params: HashMap::new(),
            base_slow_params: HashMap::new(),
        };
        let res = run_coupled(&m, &problem).unwrap();
        assert!((res.best_knobs[0] - 5.0).abs() < 0.05, "g={}", res.best_knobs[0]);
        assert!((res.best_objective - 6.0).abs() < 0.05, "z={}", res.best_objective);
    }

    /// 旋钮 kind 非 fast/slow_param → 报错。
    #[test]
    fn test_run_coupled_rejects_bad_knob() {
        let (_df, fast) = model("meta: { id: F, model: F, name_cn: x, dt: 1, dt_seconds: 1 }\nvariables: { u: { type: input }, y: { type: output } }\nequations: [ { id: E, name: y, output: y, expression: { ref: u } } ]\n");
        let (_ds, slow) = model("meta: { id: S, model: S, name_cn: x, dt: 1, dt_seconds: 2 }\nvariables: { a: { type: input }, z: { type: output } }\nequations: [ { id: E, name: z, output: z, expression: { ref: a } } ]\n");
        let problem = parse_problem("optimize:\n  knobs: [ { var: a, kind: param, bounds: [0, 1] } ]\n  objective: { expr: \"(final z)\" }\n").unwrap();
        let mut weather = HashMap::new();
        weather.insert("u".to_string(), vec![1.0; 2]);
        let m = CoupledModel { fast: &fast, slow: &slow, links: vec![CoupledLink { to: "a".into(), from: "y".into(), agg: crate::sim::Agg::Mean, scale: 1.0 }], feedback: vec![], weather, slow_steps: 1, base_fast_params: HashMap::new(), base_slow_params: HashMap::new() };
        assert!(run_coupled(&m, &problem).is_err());
    }
}
