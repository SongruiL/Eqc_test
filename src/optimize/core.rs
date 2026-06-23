//! 目标评估核：**`(模型, 旋钮赋值) → 跑前向模型 → 归约成一个标量代价（+ 约束惩罚）`**。
//!
//! 见 `docs/spec-optimization.md` §1、§7。这一层是三种用途（决策优化 / 参数标定 /
//! 未来 GP-fitness）的**公共底座**——只写一次。它把搜索算法（DE/网格/…）与前向模型隔开：
//! 搜索器只管「给一组旋钮值、拿回一个越小越好的代价」。
//!
//! # 鲁棒性（spec §5）
//!
//! 某些旋钮组合会让仿真发散 / 出 NaN / 缺驱动 / 目标无法求值——评估核**捕获一切失败、
//! 返回最差代价 [`WORST_COST`]（而非崩溃或 inf）**，让搜索器把这些「垃圾候选」自然淘汰。
//! （呼应「GP 时让 NaN 当惩罚」。）

use std::collections::{HashMap, HashSet};

use crate::schema::EquationFile;
use crate::sim::{build_plan, simulate, SimInput, SimOutput};

use super::objective::{eval_objective_obs, ObservedData};
use super::problem::{KnobKind, Objective, Problem, Sense};

/// 垃圾候选（仿真发散/出错/目标无法求值/约束求值失败）的代价：一个很大的**有限**值
/// （不用 `f64::INFINITY`，保持 DE 的算术良性）。
pub const WORST_COST: f64 = 1e18;

/// 约束违反的默认线性外罚权重（`cost += weight · Σ违反量`）。
/// 决策 spec 可用 `penalty_weight:` 覆盖。
pub const DEFAULT_PENALTY_WEIGHT: f64 = 1e9;

/// 单条约束的状态（约束一等公民：报告哪条满足/违反、违反多少）。
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintStatus {
    /// 约束的 S 表达式。
    pub expr: String,
    /// 约束式当前求值（对轨迹归约/旋钮）。
    pub value: f64,
    /// 上界（约束为 `value ≤ max`）。
    pub max: f64,
    /// 违反量 `max(0, value − max)`（0 = 满足）。
    pub violation: f64,
}

/// 一次目标评估的结果。
#[derive(Debug, Clone)]
pub struct EvalOutcome {
    /// DE **最小化**用的代价：`sense调整(目标) + 约束惩罚`；垃圾候选 = [`WORST_COST`]。恒有限。
    pub cost: f64,
    /// 原始目标值（用户写的那个量，未经 sense/惩罚调整）；垃圾候选为 `None`。
    pub objective: Option<f64>,
    /// 约束惩罚（Σ 违反量，≥0）。
    pub penalty: f64,
    /// 是否满足全部约束。
    pub feasible: bool,
    /// 逐约束状态明细（与 `problem.constraints` 一一对应；垃圾候选为空）。
    pub constraints: Vec<ConstraintStatus>,
    /// 垃圾候选/出错原因（诊断用）。
    pub note: Option<String>,
}

impl EvalOutcome {
    fn garbage(note: String) -> Self {
        Self {
            cost: WORST_COST,
            objective: None,
            penalty: 0.0,
            feasible: false,
            constraints: Vec::new(),
            note: Some(note),
        }
    }
}

/// 评估一个候选：旋钮赋值 → 装配 [`SimInput`] → 跑 [`simulate`] → 目标归约 + sense + 约束惩罚。
///
/// - `knob_values`：与 `problem.knobs` **一一对应**的当前取值。
/// - `drivers`：不可控环境的驱动量时间序列（`driver_const` 旋钮会在其上覆盖整列常数）。
/// - `steps`：仿真步数。
///
/// 失败（缺驱动/发散/非有限/目标或约束求值错）一律 → [`WORST_COST`]，不崩溃。
pub fn evaluate(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
) -> EvalOutcome {
    evaluate_obs(file, problem, knob_values, drivers, steps, &ObservedData::new())
}

/// 同 [`evaluate`]，但额外提供**实测数据**——目标/约束可用误差算子（`rmse` 等）。参数标定用。
pub fn evaluate_obs(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
    observed: &ObservedData,
) -> EvalOutcome {
    let prep = match prepare(file, problem, knob_values, drivers, steps, observed) {
        Ok(p) => p,
        Err(note) => return EvalOutcome::garbage(note),
    };

    // 目标值
    let obj = match eval_objective_obs(&problem.objective.expr, &prep.out, &prep.bindings, observed) {
        Ok(v) if v.is_finite() => v,
        Ok(_) => return EvalOutcome::garbage("目标值非有限（NaN/Inf）".into()),
        Err(e) => return EvalOutcome::garbage(format!("目标求值失败: {e}")),
    };

    let feasible = prep.penalty == 0.0;
    let weight = problem.penalty_weight.unwrap_or(DEFAULT_PENALTY_WEIGHT);
    let cost = sense_cost(problem.objective.sense, obj) + weight * prep.penalty;

    EvalOutcome {
        cost,
        objective: Some(obj),
        penalty: prep.penalty,
        feasible,
        constraints: prep.constraints,
        note: None,
    }
}

/// 多目标一次评估的结果（雏形：2 目标）。
#[derive(Debug, Clone)]
pub struct MoOutcome {
    /// 每个目标的**最小化代价**（含约束惩罚，故不可行解被可行解支配）；垃圾候选全 [`WORST_COST`]。
    pub costs: Vec<f64>,
    /// 每个目标的原始值（用户写的量，未经 sense/惩罚调整）；垃圾候选为 `None`。
    pub objectives: Option<Vec<f64>>,
    pub penalty: f64,
    pub feasible: bool,
    pub constraints: Vec<ConstraintStatus>,
    pub note: Option<String>,
}

impl MoOutcome {
    fn garbage(note: String, nobj: usize) -> Self {
        Self {
            costs: vec![WORST_COST; nobj],
            objectives: None,
            penalty: 0.0,
            feasible: false,
            constraints: Vec::new(),
            note: Some(note),
        }
    }
}

/// 多目标评估：同一仿真 + 约束惩罚，求**两个目标**的代价向量（供 Pareto 支配比较）。
/// 调用方须保证 `problem.objective2` 为 `Some`（多目标模式）。
pub fn evaluate_mo(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
) -> MoOutcome {
    evaluate_mo_obs(file, problem, knob_values, drivers, steps, &ObservedData::new())
}

/// 同 [`evaluate_mo`]，但额外提供实测数据（多目标标定/拟合权衡可用误差算子）。
pub fn evaluate_mo_obs(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
    observed: &ObservedData,
) -> MoOutcome {
    let objs: Vec<&Objective> = match &problem.objective2 {
        Some(o2) => vec![&problem.objective, o2],
        None => vec![&problem.objective], // 退化：单目标也可用（caller 通常已判定多目标）
    };
    let prep = match prepare(file, problem, knob_values, drivers, steps, observed) {
        Ok(p) => p,
        Err(note) => return MoOutcome::garbage(note, objs.len()),
    };

    let mut raw = Vec::with_capacity(objs.len());
    for o in &objs {
        match eval_objective_obs(&o.expr, &prep.out, &prep.bindings, observed) {
            Ok(v) if v.is_finite() => raw.push(v),
            _ => return MoOutcome::garbage(format!("目标求值失败/非有限: {}", o.expr), objs.len()),
        }
    }
    let weight = problem.penalty_weight.unwrap_or(DEFAULT_PENALTY_WEIGHT);
    let feasible = prep.penalty == 0.0;
    // 惩罚加到每个目标的代价上 → 不可行解在所有目标上都变差、被可行解支配。
    let costs: Vec<f64> = objs
        .iter()
        .zip(&raw)
        .map(|(o, &v)| sense_cost(o.sense, v) + weight * prep.penalty)
        .collect();

    MoOutcome { costs, objectives: Some(raw), penalty: prep.penalty, feasible, constraints: prep.constraints, note: None }
}

/// sense → 最小化代价：最大化目标取负、最小化目标原样。
fn sense_cost(sense: Sense, v: f64) -> f64 {
    match sense {
        Sense::Max => -v,
        Sense::Min => v,
    }
}

/// 一个候选的「仿真 + 绑定 + 约束惩罚」公共准备（单/多目标共用）。
/// `Err(note)` 表示垃圾候选（仿真/约束求值失败）。
struct Prep {
    out: SimOutput,
    bindings: HashMap<String, f64>,
    penalty: f64,
    constraints: Vec<ConstraintStatus>,
}

fn prepare(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
    observed: &ObservedData,
) -> Result<Prep, String> {
    let input = build_input(problem, knob_values, drivers, steps);
    let out = simulate(file, &input).map_err(|e| format!("仿真失败: {e}"))?;
    let bindings = build_bindings(file, problem, knob_values);

    // 约束 expr ≤ max：违反量 = max(0, c − max)；惩罚 = Σ 违反量。
    let mut penalty = 0.0;
    let mut constraints = Vec::with_capacity(problem.constraints.len());
    for c in &problem.constraints {
        match eval_objective_obs(&c.expr, &out, &bindings, observed) {
            Ok(cv) if cv.is_finite() => {
                let violation = (cv - c.max).max(0.0);
                penalty += violation;
                constraints.push(ConstraintStatus { expr: c.expr.clone(), value: cv, max: c.max, violation });
            }
            _ => return Err(format!("约束求值失败/非有限: {}", c.expr)),
        }
    }
    Ok(Prep { out, bindings, penalty, constraints })
}

/// 用一组旋钮赋值跑一次仿真、返回完整轨迹（可辨识性分析用：要看各观测变量的整条序列）。
pub fn simulate_candidate(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
) -> Result<SimOutput, String> {
    let input = build_input(problem, knob_values, drivers, steps);
    simulate(file, &input).map_err(|e| format!("仿真失败: {e}"))
}

/// 把旋钮值装配进 [`SimInput`]：param→参数覆盖，init→初值覆盖，driver_const→整列常数。
fn build_input(
    problem: &Problem,
    knob_values: &[f64],
    drivers: &HashMap<String, Vec<f64>>,
    steps: usize,
) -> SimInput {
    let mut input = SimInput::new(steps);
    input.drivers = drivers.clone();
    for (k, &val) in problem.knobs.iter().zip(knob_values) {
        match k.kind {
            KnobKind::Param => {
                input.param_overrides.insert(k.var.clone(), val);
            }
            KnobKind::Init => {
                input.init_overrides.insert(k.var.clone(), val);
            }
            KnobKind::DriverConst => {
                input.drivers.insert(k.var.clone(), vec![val; steps]);
            }
            // 耦合优化专用旋钮——单模型路径不应到达（validate_problem 已拦），保险起见忽略。
            KnobKind::FastParam | KnobKind::SlowParam => {}
        }
    }
    input
}

/// 目标/约束方程里可引用的**非轨迹标量**绑定：
/// 模型标量参数默认值 → 常量 → **旋钮当前值（优先级最高）**。
/// 这样目标里的 `Pd`（同时是旋钮）取当前试验值，未作旋钮的参数取默认，单价/成本取常量。
fn build_bindings(
    file: &EquationFile,
    problem: &Problem,
    knob_values: &[f64],
) -> HashMap<String, f64> {
    let mut b: HashMap<String, f64> = HashMap::new();
    // 模型标量参数默认值（向量参数跳过）
    for (name, p) in &file.parameters {
        if p.values.is_none() {
            b.insert(name.clone(), p.default);
        }
    }
    // 常量
    for (name, v) in &problem.constants {
        b.insert(name.clone(), *v);
    }
    // 旋钮当前值（最高优先级）
    for (k, &val) in problem.knobs.iter().zip(knob_values) {
        b.insert(k.var.clone(), val);
    }
    b
}

/// 在跑优化前校验决策 spec 与模型是否吻合（早失败、给清晰错误）。
pub fn validate_problem(file: &EquationFile, problem: &Problem) -> Result<(), String> {
    if problem.knobs.is_empty() {
        return Err("决策 spec 未声明任何旋钮 (knobs)".into());
    }
    // 用步进计划判定哪些是驱动量
    let plan = build_plan(file).map_err(|e| format!("模型不可仿真: {e}"))?;
    let drivers: HashSet<&str> = plan.drivers.iter().copied().collect();

    for k in &problem.knobs {
        if k.bounds[0] > k.bounds[1] {
            return Err(format!(
                "旋钮 '{}' 边界非法: [{}, {}]（须 lo ≤ hi）",
                k.var, k.bounds[0], k.bounds[1]
            ));
        }
        match k.kind {
            KnobKind::Param => match file.parameters.get(&k.var) {
                None => {
                    return Err(format!("旋钮 '{}' kind=param，但模型 parameters 中无此参数", k.var))
                }
                Some(p) if p.values.is_some() => {
                    return Err(format!(
                        "旋钮 '{}' 是向量参数（cohort 种子），不能作标量旋钮",
                        k.var
                    ))
                }
                _ => {}
            },
            KnobKind::Init => match file.variables.get(&k.var) {
                None => {
                    return Err(format!("旋钮 '{}' kind=init，但模型 variables 中无此变量", k.var))
                }
                Some(v) if !(v.is_integrator() || v.is_delay()) => {
                    return Err(format!(
                        "旋钮 '{}' kind=init，但它不是状态量/延迟寄存器（无 init 可调）",
                        k.var
                    ))
                }
                _ => {}
            },
            KnobKind::DriverConst => {
                if !drivers.contains(k.var.as_str()) {
                    return Err(format!(
                        "旋钮 '{}' kind=driver_const，但它不是模型的驱动量（候选驱动: {:?}）",
                        k.var, plan.drivers
                    ));
                }
            }
            KnobKind::FastParam | KnobKind::SlowParam => {
                return Err(format!(
                    "旋钮 '{}' kind={} 只用于耦合优化（spec 须有 coupling 块）",
                    k.var,
                    k.kind.as_str()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    use crate::optimize::problem::parse_problem;
    use std::io::Write;
    use tempfile::TempDir;

    /// 小动态模型：Y 积分 (drive·gain)。drive=[1,1,1]、gain=2 → r=2 每步 → Y=2,4,6。
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
        let file = parse_file(&path).unwrap();
        (dir, file)
    }

    fn drivers3() -> HashMap<String, Vec<f64>> {
        let mut d = HashMap::new();
        d.insert("drive".to_string(), vec![1.0, 1.0, 1.0]);
        d
    }

    #[test]
    fn test_evaluate_param_knob_maximize() {
        let (_d, file) = model();
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y), sense: max }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let drv = drivers3();

        // gain=2 → Y final=6；sense=max → cost=-6
        let e2 = evaluate(&file, &p, &[2.0], &drv, 3);
        assert_eq!(e2.objective, Some(6.0));
        assert_eq!(e2.cost, -6.0);
        assert!(e2.feasible);

        // gain=3 → Y final=9；cost=-9（更小=更优）
        let e3 = evaluate(&file, &p, &[3.0], &drv, 3);
        assert_eq!(e3.objective, Some(9.0));
        assert!(e3.cost < e2.cost);
    }

    #[test]
    fn test_evaluate_driver_const_knob() {
        let (_d, file) = model();
        // 把 drive 整列设成常数（覆盖 drivers3 的 [1,1,1]）；gain 默认 2
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: drive, kind: driver_const, bounds: [0, 10] }\n",
        )
        .unwrap();
        let drv = drivers3();
        // drive=5 → r=10 每步 → Y=10,20,30 → final=30
        let e = evaluate(&file, &p, &[5.0], &drv, 3);
        assert_eq!(e.objective, Some(30.0));
    }

    #[test]
    fn test_evaluate_minimize_sense_and_constant_binding() {
        let (_d, file) = model();
        // 利润 = final(Y)·price − gain·cost；最小化（演示 sense=min + 常量绑定）
        let p = parse_problem(
            "optimize:\n  objective: { expr: (sub (mul (final Y) price) (mul gain cost)), sense: min }\n  constants: { price: 2.0, cost: 1.0 }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let drv = drivers3();
        // gain=2 → final Y=6 → 6·2 − 2·1 = 10；sense=min → cost=+10
        let e = evaluate(&file, &p, &[2.0], &drv, 3);
        assert_eq!(e.objective, Some(10.0));
        assert_eq!(e.cost, 10.0);
    }

    #[test]
    fn test_constraint_penalty() {
        let (_d, file) = model();
        // 约束 final(Y) ≤ 5；gain=2 → final Y=6 → 违反量 1 → 惩罚
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y), sense: max }\n  constraints:\n    - { expr: (sub (final Y) 5), max: 0 }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let drv = drivers3();
        let e = evaluate(&file, &p, &[2.0], &drv, 3);
        assert_eq!(e.objective, Some(6.0));
        assert!(!e.feasible);
        assert!((e.penalty - 1.0).abs() < 1e-12);
        // cost = -6 + 1e9·1 ≈ 1e9（惩罚主导，远差于可行解）
        assert!(e.cost > 1e8);
        // 逐约束明细：约束式 (sub (final Y) 5) 求值=6−5=1，max=0，violation=max(0,1−0)=1
        assert_eq!(e.constraints.len(), 1);
        assert_eq!(e.constraints[0].value, 1.0);
        assert_eq!(e.constraints[0].max, 0.0);
        assert!((e.constraints[0].violation - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_penalty_weight_override() {
        let (_d, file) = model();
        // 同上约束，但把 penalty_weight 覆盖成 10 → cost = -6 + 10·1 = 4
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y), sense: max }\n  penalty_weight: 10.0\n  constraints:\n    - { expr: (sub (final Y) 5), max: 0 }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let drv = drivers3();
        let e = evaluate(&file, &p, &[2.0], &drv, 3);
        assert!((e.cost - 4.0).abs() < 1e-9);
        assert!(!e.feasible);
    }

    #[test]
    fn test_feasible_constraint_reported() {
        let (_d, file) = model();
        // 约束 final(Y) ≤ 10；gain=2 → final Y=6 ≤ 10 → 满足，无惩罚
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Y), sense: max }\n  constraints:\n    - { expr: (sub (final Y) 10), max: 0 }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let drv = drivers3();
        let e = evaluate(&file, &p, &[2.0], &drv, 3);
        assert!(e.feasible);
        assert_eq!(e.penalty, 0.0);
        assert_eq!(e.cost, -6.0); // 无惩罚
        assert_eq!(e.constraints.len(), 1);
        assert_eq!(e.constraints[0].violation, 0.0);
    }

    #[test]
    fn test_garbage_candidate_gets_worst_cost() {
        let (_d, file) = model();
        // 目标引用不存在的轨迹 → 评估核返回最差代价、不崩溃
        let p = parse_problem(
            "optimize:\n  objective: { expr: (final Nope) }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        let drv = drivers3();
        let e = evaluate(&file, &p, &[2.0], &drv, 3);
        assert_eq!(e.cost, WORST_COST);
        assert!(e.objective.is_none());
        assert!(e.note.is_some());
    }

    #[test]
    fn test_validate_problem() {
        let (_d, file) = model();
        // 合法
        let ok = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain, kind: param, bounds: [1, 5] }\n    - { var: drive, kind: driver_const, bounds: [0, 10] }\n    - { var: Y, kind: init, bounds: [0, 5] }\n",
        )
        .unwrap();
        assert!(validate_problem(&file, &ok).is_ok());

        // param 旋钮指向不存在参数
        let bad_param = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: nope, kind: param, bounds: [1, 5] }\n",
        )
        .unwrap();
        assert!(validate_problem(&file, &bad_param).is_err());

        // driver_const 旋钮指向非驱动量（gain 是参数）
        let bad_drv = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain, kind: driver_const, bounds: [0, 10] }\n",
        )
        .unwrap();
        assert!(validate_problem(&file, &bad_drv).is_err());

        // 边界非法
        let bad_bounds = parse_problem(
            "optimize:\n  objective: { expr: (final Y) }\n  knobs:\n    - { var: gain, kind: param, bounds: [5, 1] }\n",
        )
        .unwrap();
        assert!(validate_problem(&file, &bad_bounds).is_err());
    }
}
