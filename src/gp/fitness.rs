//! GP 模型级适应度：把候选 patch 进模型目标方程 → 仿真 → 对观测算误差。
//!
//! 这是 `eqc evolve` 的真用法（表达式级复原见 `evolve` 测试）。复用 `sim::simulate`；
//! 候选的可调常数 `__c{i}` 作为参数注入 patched 模型。失败（patch/sim/对不上观测）→ WORST。

use std::collections::HashMap;

use crate::schema::{EquationFile, Parameter};
use crate::sim::{self, SimInput, SimOutput};

use super::grammar::{Candidate, GpContext};

/// 稀疏观测：变量名 → [(1-based DAT, 值)]。
pub type Observed = HashMap<String, Vec<(usize, f64)>>;

const WORST: f64 = 1e18;

/// 从模型某 gp_target 方程读出 (语法名, GpContext)。非 gp_target 或不存在 → None。
pub fn context_from_target(model: &EquationFile, target_id: &str) -> Option<(String, GpContext)> {
    let eq = model.equations.iter().find(|e| e.id == target_id)?;
    let gt = eq.gp_target.as_ref()?;
    let inputs = if gt.inputs.is_empty() {
        eq.get_variable_refs()
    } else {
        gt.inputs.clone()
    };
    Some((
        gt.grammar.clone(),
        GpContext {
            inputs,
            output_bounds: gt.output_bounds,
            monotone: gt.monotone.clone(),
        },
    ))
}

/// clone 模型，把候选 expr 替进目标方程、注入 `__c{i}` 参数。目标不存在 → None。
pub fn patch_model(base: &EquationFile, target_id: &str, cand: &Candidate) -> Option<EquationFile> {
    let mut m = base.clone();
    {
        let eq = m.equations.iter_mut().find(|e| e.id == target_id)?;
        eq.expression = cand.expr.clone();
    }
    for (i, v) in cand.consts.iter().enumerate() {
        let name = Candidate::const_name(i);
        m.parameters.insert(
            name.clone(),
            Parameter {
                name_cn: name,
                name_en: None,
                dtype: Default::default(),
                default: *v,
                values: None,
                unit: None,
                bounds: None,
                optimizable: true,
                management: false,
                description: None,
            },
        );
    }
    Some(m)
}

/// rmse(sim[output] vs observed[output])，仅在观测日（1-based DAT）上比较。
fn rmse_obs(out: &SimOutput, output: &str, observed: &Observed) -> Option<f64> {
    let traj = out.series(output)?;
    let obs = observed.get(output)?;
    if obs.is_empty() {
        return None;
    }
    let mut se = 0.0;
    let mut n = 0usize;
    for &(day, val) in obs {
        let idx = day.checked_sub(1)?;
        let y = *traj.get(idx)?;
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

/// 模型级适应度：patch 候选 → 仿真 → rmse vs 观测。任何失败 → WORST。
pub fn evaluate_in_model(
    base: &EquationFile,
    target_id: &str,
    output: &str,
    cand: &Candidate,
    input: &SimInput,
    observed: &Observed,
) -> f64 {
    let model = match patch_model(base, target_id, cand) {
        Some(m) => m,
        None => return WORST,
    };
    let out = match sim::simulate(&model, input) {
        Ok(o) => o,
        Err(_) => return WORST,
    };
    rmse_obs(&out, output, observed).unwrap_or(WORST)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::gp::grammar::sample;
    use crate::optimize::de::Rng;
    use crate::schema::{Equation, GpTarget, Metadata, Variable, VariableType};
    use indexmap::IndexMap;

    // 最小模型：驱动量 d → 门控输出 y = gate(d)。y 是 gp_target。
    fn mini_model() -> EquationFile {
        let mut variables = IndexMap::new();
        variables.insert(
            "d".to_string(),
            Variable {
                var_type: VariableType::Input,
                dtype: Default::default(),
                unit: None,
                description: None,
                label: None,
                measurable: false,
                stress_factor: None,
                stress_reduce: None,
                source: None,
                class: Some(crate::schema::VarClass::Driving),
                init: None,
                rate: None,
                prev: None,
             instance: None },
        );
        variables.insert(
            "y".to_string(),
            Variable {
                var_type: VariableType::Output,
                dtype: Default::default(),
                unit: None,
                description: None,
                label: None,
                measurable: true,
                stress_factor: None,
                stress_reduce: None,
                source: None,
                class: None,
                init: None,
                rate: None,
                prev: None,
             instance: None },
        );
        let mut mono = IndexMap::new();
        mono.insert("d".to_string(), "increasing".to_string());
        EquationFile {
            meta: Metadata {
                id: "MINI".into(),
                model: "Mini".into(),
                name_cn: "迷你".into(),
                name_en: None,
                version: "1".into(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(), balance: vec![],
            },
            parameters: IndexMap::new(),
            variables,
            // 初始 expr 占位（GP 会替换）；带 gp_target
            equations: vec![Equation {
                id: "TGT".into(),
                name: "门控".into(),
                output: "y".into(),
                expression: Expr::var("d"),
                formula_display: None,
                reference: None,
                gp_target: Some(GpTarget {
                    grammar: "monotone_gate".into(),
                    inputs: vec!["d".into()],
                    output_bounds: Some([0.0, 1.0]),
                    monotone: mono,
                    frozen: false,
                }),
             provenance: None, instance: None }],
         structure: None }
    }

    /// context_from_target 正确读出语法 + 上下文。
    #[test]
    fn test_context_from_target() {
        let m = mini_model();
        let (g, ctx) = context_from_target(&m, "TGT").unwrap();
        assert_eq!(g, "monotone_gate");
        assert_eq!(ctx.inputs, vec!["d".to_string()]);
        assert_eq!(ctx.output_bounds, Some([0.0, 1.0]));
        assert_eq!(ctx.monotone.get("d").map(String::as_str), Some("increasing"));
        assert!(context_from_target(&m, "NOPE").is_none());
    }

    /// patch 替换方程 expr + 注入 __c 参数。
    #[test]
    fn test_patch_injects_consts() {
        let m = mini_model();
        let cand = Candidate {
            expr: Expr::add(Expr::param("__c0"), Expr::param("__c1")),
            consts: vec![0.3, 20.0],
        };
        let p = patch_model(&m, "TGT", &cand).unwrap();
        assert_eq!(p.parameters.get("__c0").unwrap().default, 0.3);
        assert_eq!(p.parameters.get("__c1").unwrap().default, 20.0);
        let eq = p.equations.iter().find(|e| e.id == "TGT").unwrap();
        assert!(eq.get_parameter_refs().contains(&"__c0".to_string()));
    }

    /// 模型级适应度：把"真值"候选 patch 回去应≈0 误差；差候选误差大。
    #[test]
    fn test_evaluate_in_model_recovers_truth() {
        let m = mini_model();
        // 真值候选：采样一个 gate（确定性），作为"地面真值"
        let mut rng = Rng::new(123);
        let truth = sample("monotone_gate", &context_from_target(&m, "TGT").unwrap().1, &mut rng).unwrap();
        // 用真值生成观测：y[t] = gate(d=t)，d=0..30
        let steps = 31;
        let dvals: Vec<f64> = (0..steps).map(|i| i as f64).collect();
        let truth_model = patch_model(&m, "TGT", &truth).unwrap();
        let mut input = SimInput::new(steps);
        input.drivers.insert("d".to_string(), dvals.clone());
        let truth_out = sim::simulate(&truth_model, &input).unwrap();
        let yseries = truth_out.series("y").unwrap();
        let observed: Observed = {
            let mut o = HashMap::new();
            o.insert(
                "y".to_string(),
                (0..steps).step_by(3).map(|t| (t + 1, yseries[t])).collect(),
            );
            o
        };
        // 真值候选 → 误差≈0
        let e_truth = evaluate_in_model(&m, "TGT", "y", &truth, &input, &observed);
        assert!(e_truth < 1e-9, "真值应≈0: {e_truth}");
        // 一个不同候选 → 误差更大
        let mut rng2 = Rng::new(999);
        let other = sample("monotone_gate", &context_from_target(&m, "TGT").unwrap().1, &mut rng2).unwrap();
        let e_other = evaluate_in_model(&m, "TGT", "y", &other, &input, &observed);
        assert!(e_other > e_truth, "差候选应误差更大: {e_other} vs {e_truth}");
    }
}
