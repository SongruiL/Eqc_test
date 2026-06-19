//! 目标表达式求值：**时间归约词汇** + 把目标/约束 S 表达式归约成一个标量。
//!
//! 见 `docs/spec-optimization.md` §3。核心思路（**复用现有解析器/AST/求值器**）：
//!
//! 1. 目标是「一条方程」，但它的「变量」不是逐日量，而是对**整段轨迹**的**时间归约**
//!    （末值 / 峰值 / 均值 / 累计 …）。
//! 2. 所以求值分两步：先用 sexpr 解析器把目标串解析成 [`SExpr`]；在 **SExpr 层**把每个
//!    «归约子式»（如 `(final Y)`）就地替换成一个数（从 [`SimOutput`] 轨迹算出）；再用现成的
//!    `convert` + `eval` 把剩下的**纯算术**求成标量。
//! 3. 这样时间归约只是 SExpr 层的一道小预处理——**不新增 AST 变体**（不污染 360 变体枚举、
//!    不必动 codegen），也**不与逐元素 `max/min` 冲突**（见下「消歧规则」）。
//!
//! # 时间归约词汇（作用于 [`SimOutput`] 的一条序列，区别于逐日算子与 `vsum`）
//!
//! - `(final X)` —— 末值（最常用：最终产量）
//! - `(at X t)`  —— 第 `t` 天（1 起，= 内置 `DAT`）的值
//! - `(max X)` / `(min X)` / `(mean X)` / `(total X)` —— 峰值 / 谷值 / 均值 / 全季累计
//!
//! # 消歧规则（`max`/`min` 同时是逐元素算子）
//!
//! - `final` / `at` / `total` / `mean` 是**归约专用词**：必须形如 `(final 轨迹变量)` /
//!   `(at 轨迹变量 天)`，否则报错。
//! - `max` / `min` 仅当形如 `(max 单个轨迹变量)` 时作时间归约；其余（如
//!   `(max (final A) (final B))` 这种对已归约标量取较大者）**原样保留**，交给普通求值器按逐元素
//!   `max`/`min` 处理。

use std::collections::HashMap;

use crate::eval::{Env, EvalError};
use crate::sexpr::SExpr;
use crate::sim::SimOutput;

/// 时间归约词（保留字）。
pub const REDUCTIONS: &[&str] = &["final", "at", "max", "min", "mean", "total"];

/// 误差算子（保留字）：把**仿真序列**与**实测序列**逐（观测）日比对、归约成标量。
/// 用于参数标定（目标 = 预测 vs 实测的误差）。见 `docs/spec-calibration.md` §2。
pub const ERROR_OPS: &[&str] = &["rmse", "mae", "nse", "bias"];

/// 实测数据：变量名 → 稀疏 `[(天, 值)]`（天为 1 起的 `DAT`；只在有观测的天给值）。
pub type ObservedData = HashMap<String, Vec<(usize, f64)>>;

/// 目标/约束表达式求值错误。
#[derive(Debug, Clone, PartialEq)]
pub enum ObjError {
    /// 目标串解析失败（sexpr 语法 / 未知算子）。
    Parse(String),
    /// 归约后的纯算术表达式求值出错。
    Eval(EvalError),
    /// 归约引用了一个不存在的轨迹变量（向量变量请用 `名[1]` 形式）。
    UnknownTrajectory(String),
    /// 误差算子引用了一个不存在的实测变量列。
    UnknownObserved(String),
    /// 误差算子的实测序列为空（无观测点）。
    NoObservations(String),
    /// 归约写法不合法（如 `at` 缺天数、`final` 套了子表达式而非轨迹变量）。
    BadReduction(String),
    /// 轨迹为空（0 步）。
    EmptyTrajectory(String),
    /// `at` 的天数越界。
    DayOutOfRange { var: String, day: usize, len: usize },
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjError::Parse(s) => write!(f, "目标表达式解析失败: {s}"),
            ObjError::Eval(e) => write!(f, "目标表达式求值出错: {e}"),
            ObjError::UnknownTrajectory(n) => {
                write!(f, "归约引用了未知轨迹变量 '{n}'（向量变量请用 '{n}[1]' 形式）")
            }
            ObjError::UnknownObserved(n) => write!(f, "误差算子引用了未知实测列 '{n}'"),
            ObjError::NoObservations(n) => write!(f, "实测列 '{n}' 无观测点"),
            ObjError::BadReduction(s) => write!(f, "时间归约写法不合法: {s}"),
            ObjError::EmptyTrajectory(n) => write!(f, "轨迹 '{n}' 为空（0 步），无法归约"),
            ObjError::DayOutOfRange { var, day, len } => {
                write!(f, "(at {var} {day}) 越界：仅有第 1..{len} 天")
            }
        }
    }
}

impl std::error::Error for ObjError {}

/// 求值一条目标 / 约束 S 表达式串，归约成一个标量。
///
/// - `expr_src`：S 表达式串（如 `(sub (mul (final Y) price) (mul CO2 co2_cost))`）。
/// - `out`：前向仿真的整季轨迹；时间归约从这里取序列。
/// - `bindings`：目标方程里用到的**非轨迹标量**——旋钮当前值 + `constants`（单价/成本/目标值…）。
pub fn eval_objective(
    expr_src: &str,
    out: &SimOutput,
    bindings: &HashMap<String, f64>,
) -> Result<f64, ObjError> {
    eval_objective_obs(expr_src, out, bindings, &ObservedData::new())
}

/// 同 [`eval_objective`]，但额外提供**实测数据** `observed`——支持误差算子
/// （`rmse/mae/nse/bias`，把仿真序列与实测序列比对）。参数标定用。
pub fn eval_objective_obs(
    expr_src: &str,
    out: &SimOutput,
    bindings: &HashMap<String, f64>,
    observed: &ObservedData,
) -> Result<f64, ObjError> {
    let sx = crate::sexpr::parse(expr_src).map_err(|e| ObjError::Parse(e.to_string()))?;
    let reduced = reduce_sexpr(&sx, out, observed)?;
    let expr = crate::sexpr::convert(&reduced).map_err(|e| ObjError::Parse(e.to_string()))?;
    let mut env = Env::new();
    for (k, v) in bindings {
        env.set(k.clone(), *v);
    }
    expr.eval_scalar(&env).map_err(ObjError::Eval)
}

/// 在 SExpr 层把每个时间归约 / 误差子式替换成一个数（其余结构原样递归）。
fn reduce_sexpr(sx: &SExpr, out: &SimOutput, observed: &ObservedData) -> Result<SExpr, ObjError> {
    match sx {
        SExpr::Number(_) | SExpr::Symbol(_) => Ok(sx.clone()),
        SExpr::List(items) => {
            if let Some(SExpr::Symbol(head)) = items.first() {
                // 时间归约 (final/at/max/min/mean/total)
                if REDUCTIONS.contains(&head.as_str()) {
                    if let Some(v) = try_reduce(head, &items[1..], out)? {
                        return Ok(SExpr::Number(v));
                    }
                    // None = 不是时间归约（如逐元素 max/min），落到下面原样递归
                }
                // 误差算子 (rmse/mae/nse/bias 仿真变量 实测列)
                if ERROR_OPS.contains(&head.as_str()) {
                    let v = try_error(head, &items[1..], out, observed)?;
                    return Ok(SExpr::Number(v));
                }
            }
            let mut new_items = Vec::with_capacity(items.len());
            for it in items {
                new_items.push(reduce_sexpr(it, out, observed)?);
            }
            Ok(SExpr::List(new_items))
        }
    }
}

/// 计算误差算子 `(head 仿真变量 实测列)`：在有观测的天上把 `sim[天-1]` 与实测对比。
fn try_error(head: &str, args: &[SExpr], out: &SimOutput, observed: &ObservedData) -> Result<f64, ObjError> {
    let (simvar, obsname) = match args {
        [SExpr::Symbol(s), SExpr::Symbol(o)] => (s, o),
        _ => return Err(ObjError::BadReduction(format!("{head} 须写成 ({head} 仿真变量 实测列名)"))),
    };
    let sim = out.series(simvar).ok_or_else(|| ObjError::UnknownTrajectory(simvar.clone()))?;
    let obs = observed.get(obsname).ok_or_else(|| ObjError::UnknownObserved(obsname.clone()))?;
    if obs.is_empty() {
        return Err(ObjError::NoObservations(obsname.clone()));
    }
    // 对齐：每个观测点 (天, 实测) → (sim[天-1], 实测)
    let mut pairs: Vec<(f64, f64)> = Vec::with_capacity(obs.len());
    for &(day, ov) in obs {
        if day < 1 || day > sim.len() {
            return Err(ObjError::DayOutOfRange { var: simvar.clone(), day, len: sim.len() });
        }
        pairs.push((sim[day - 1], ov));
    }
    let n = pairs.len() as f64;
    Ok(match head {
        "rmse" => (pairs.iter().map(|(s, o)| (s - o) * (s - o)).sum::<f64>() / n).sqrt(),
        "mae" => pairs.iter().map(|(s, o)| (s - o).abs()).sum::<f64>() / n,
        "bias" => pairs.iter().map(|(s, o)| s - o).sum::<f64>() / n,
        "nse" => {
            let mean_o = pairs.iter().map(|(_, o)| *o).sum::<f64>() / n;
            let denom: f64 = pairs.iter().map(|(_, o)| (o - mean_o) * (o - mean_o)).sum();
            if denom == 0.0 {
                return Err(ObjError::BadReduction("nse 分母为 0（实测无变化）".into()));
            }
            let num: f64 = pairs.iter().map(|(s, o)| (s - o) * (s - o)).sum();
            1.0 - num / denom
        }
        other => return Err(ObjError::BadReduction(format!("未知误差算子 '{other}'"))),
    })
}

/// 尝试把 `(head args...)` 解释为时间归约。
///
/// - `Ok(Some(v))`：是时间归约，归约值为 `v`。
/// - `Ok(None)`：不是时间归约（仅 `max`/`min` 会走到这里），交回普通求值器。
/// - `Err(_)`：归约专用词写法不合法 / 轨迹不存在 / 越界。
fn try_reduce(head: &str, args: &[SExpr], out: &SimOutput) -> Result<Option<f64>, ObjError> {
    // 取「单个符号实参且命名了一条轨迹」时的序列
    let single_traj = |args: &[SExpr]| -> Option<(String, Vec<f64>)> {
        if args.len() == 1 {
            if let SExpr::Symbol(name) = &args[0] {
                if let Some(s) = out.series(name) {
                    return Some((name.clone(), s.to_vec()));
                }
            }
        }
        None
    };

    match head {
        "at" => {
            // (at 轨迹变量 天)
            if let [SExpr::Symbol(name), SExpr::Number(d)] = args {
                let series = out
                    .series(name)
                    .ok_or_else(|| ObjError::UnknownTrajectory(name.clone()))?;
                if series.is_empty() {
                    return Err(ObjError::EmptyTrajectory(name.clone()));
                }
                let day = *d as usize;
                if day < 1 || day > series.len() {
                    return Err(ObjError::DayOutOfRange { var: name.clone(), day, len: series.len() });
                }
                Ok(Some(series[day - 1]))
            } else {
                Err(ObjError::BadReduction("at 须写成 (at 轨迹变量 天)".into()))
            }
        }
        "final" | "total" | "mean" => match single_traj(args) {
            Some((name, series)) => Ok(Some(reduce_one(&series, head, &name)?)),
            None => Err(ObjError::BadReduction(format!(
                "{head} 须写成 ({head} 轨迹变量)（实参须是一个已存在的轨迹变量名）"
            ))),
        },
        // max/min：单轨迹变量 → 时间归约；否则原样保留为逐元素算子
        "max" | "min" => match single_traj(args) {
            Some((name, series)) => Ok(Some(reduce_one(&series, head, &name)?)),
            None => Ok(None),
        },
        _ => Ok(None),
    }
}

/// 对一条非空轨迹做单点归约（final/max/min/mean/total）。
fn reduce_one(series: &[f64], kind: &str, name: &str) -> Result<f64, ObjError> {
    if series.is_empty() {
        return Err(ObjError::EmptyTrajectory(name.to_string()));
    }
    Ok(match kind {
        "final" => *series.last().unwrap(),
        "max" => series.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "min" => series.iter().copied().fold(f64::INFINITY, f64::min),
        "mean" => series.iter().sum::<f64>() / series.len() as f64,
        "total" => series.iter().sum::<f64>(),
        other => return Err(ObjError::BadReduction(format!("未知归约词 '{other}'"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    /// 构造一个合成 SimOutput（无需真跑模型）。
    fn out_of(pairs: &[(&str, Vec<f64>)]) -> SimOutput {
        let mut t: IndexMap<String, Vec<f64>> = IndexMap::new();
        let steps = pairs.first().map(|(_, v)| v.len()).unwrap_or(0);
        for (k, v) in pairs {
            t.insert(k.to_string(), v.clone());
        }
        SimOutput { steps, trajectories: t }
    }

    fn binds(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn test_six_reductions() {
        let out = out_of(&[("Y", vec![1.0, 2.0, 3.0, 10.0])]);
        let b = HashMap::new();
        assert_eq!(eval_objective("(final Y)", &out, &b).unwrap(), 10.0);
        assert_eq!(eval_objective("(at Y 2)", &out, &b).unwrap(), 2.0); // 第2天 = 索引1
        assert_eq!(eval_objective("(max Y)", &out, &b).unwrap(), 10.0);
        assert_eq!(eval_objective("(min Y)", &out, &b).unwrap(), 1.0);
        assert_eq!(eval_objective("(mean Y)", &out, &b).unwrap(), 4.0); // (1+2+3+10)/4
        assert_eq!(eval_objective("(total Y)", &out, &b).unwrap(), 16.0);
    }

    #[test]
    fn test_objective_with_arithmetic_and_bindings() {
        // 利润：末产量·单价 − CO2·CO2成本
        let out = out_of(&[("Y", vec![1.0, 2.0, 10.0])]);
        let b = binds(&[("price", 30.0), ("CO2", 800.0), ("co2_cost", 0.002)]);
        let v = eval_objective(
            "(sub (mul (final Y) price) (mul CO2 co2_cost))",
            &out,
            &b,
        )
        .unwrap();
        assert_eq!(v, 10.0 * 30.0 - 800.0 * 0.002); // 300 - 1.6 = 298.4
    }

    #[test]
    fn test_market_window_objective() {
        // 命中上市期：让第 3 天产量贴近目标 5 → |at(Y,3) − target|
        let out = out_of(&[("Y", vec![1.0, 2.0, 4.0, 8.0])]);
        let b = binds(&[("target", 5.0)]);
        let v = eval_objective("(abs (sub (at Y 3) target))", &out, &b).unwrap();
        assert_eq!(v, 1.0); // |4 - 5|
    }

    #[test]
    fn test_max_of_scalars_falls_through() {
        // (max (final A) (final B)) —— 两个已归约标量取较大者（逐元素 max，不是时间归约）
        let out = out_of(&[("A", vec![1.0, 3.0]), ("B", vec![7.0, 2.0])]);
        let b = HashMap::new();
        let v = eval_objective("(max (final A) (final B))", &out, &b).unwrap();
        assert_eq!(v, 3.0); // final A=3, final B=2 → max=3
    }

    #[test]
    fn test_unknown_trajectory_errors() {
        let out = out_of(&[("Y", vec![1.0, 2.0])]);
        let b = HashMap::new();
        assert_eq!(
            eval_objective("(final Z)", &out, &b),
            Err(ObjError::BadReduction(
                "final 须写成 (final 轨迹变量)（实参须是一个已存在的轨迹变量名）".into()
            ))
        );
        // at 引用未知轨迹 → UnknownTrajectory
        assert_eq!(
            eval_objective("(at Z 1)", &out, &b),
            Err(ObjError::UnknownTrajectory("Z".into()))
        );
    }

    #[test]
    fn test_at_day_out_of_range() {
        let out = out_of(&[("Y", vec![1.0, 2.0, 3.0])]);
        let b = HashMap::new();
        assert_eq!(
            eval_objective("(at Y 99)", &out, &b),
            Err(ObjError::DayOutOfRange { var: "Y".into(), day: 99, len: 3 })
        );
    }

    fn obs_of(pairs: &[(&str, Vec<(usize, f64)>)]) -> ObservedData {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_error_ops_rmse_mae_bias() {
        // 仿真 Y=[1,2,3,4,5]；实测在第 2、4 天 = 2.5、3.0 → sim 取 [2,4]
        let out = out_of(&[("Y", vec![1.0, 2.0, 3.0, 4.0, 5.0])]);
        let obs = obs_of(&[("obs_Y", vec![(2, 2.5), (4, 3.0)])]);
        let b = HashMap::new();
        // 残差 sim−obs = [2−2.5, 4−3.0] = [−0.5, 1.0]
        let rmse = eval_objective_obs("(rmse Y obs_Y)", &out, &b, &obs).unwrap();
        assert!((rmse - (((0.25 + 1.0) / 2.0) as f64).sqrt()).abs() < 1e-12); // √0.625
        let mae = eval_objective_obs("(mae Y obs_Y)", &out, &b, &obs).unwrap();
        assert!((mae - 0.75).abs() < 1e-12); // (0.5+1.0)/2
        let bias = eval_objective_obs("(bias Y obs_Y)", &out, &b, &obs).unwrap();
        assert!((bias - 0.25).abs() < 1e-12); // (−0.5+1.0)/2
    }

    #[test]
    fn test_error_op_nse_perfect() {
        // sim 在观测点与实测完全一致 → NSE = 1
        let out = out_of(&[("Y", vec![10.0, 20.0, 30.0])]);
        let obs = obs_of(&[("obs_Y", vec![(1, 10.0), (2, 20.0), (3, 30.0)])]);
        let nse = eval_objective_obs("(nse Y obs_Y)", &out, &HashMap::new(), &obs).unwrap();
        assert!((nse - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_error_op_composite_weighted() {
        // 多变量加权拟合：rmse(Y) + 0.5·rmse(LAI)
        let out = out_of(&[("Y", vec![1.0, 3.0]), ("LAI", vec![2.0, 2.0])]);
        let obs = obs_of(&[("oY", vec![(2, 4.0)]), ("oL", vec![(2, 1.0)])]);
        let b = binds(&[("w", 0.5)]);
        // rmse(Y)=|3−4|=1；rmse(LAI)=|2−1|=1 → 1 + 0.5·1 = 1.5
        let v = eval_objective_obs("(add (rmse Y oY) (mul w (rmse LAI oL)))", &out, &b, &obs).unwrap();
        assert!((v - 1.5).abs() < 1e-12);
    }

    #[test]
    fn test_error_op_unknown_observed() {
        let out = out_of(&[("Y", vec![1.0, 2.0])]);
        let obs = ObservedData::new();
        assert_eq!(
            eval_objective_obs("(rmse Y nope)", &out, &HashMap::new(), &obs),
            Err(ObjError::UnknownObserved("nope".into()))
        );
    }

    #[test]
    fn test_decision_objective_ignores_observed() {
        // 决策目标（无误差算子）走 eval_objective（空实测）仍正常
        let out = out_of(&[("Y", vec![1.0, 2.0, 10.0])]);
        assert_eq!(eval_objective("(final Y)", &out, &HashMap::new()).unwrap(), 10.0);
    }
}
