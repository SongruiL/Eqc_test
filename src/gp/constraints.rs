//! GP 候选的约束检查：量纲（软过滤）+ 数值先验（单调/有界）。
//!
//! 用途：①保证采样/变异出的候选合法（语法已按构造满足，此为安全网 + G2 变异校验）；
//! ②catch 手工/进化出的违规候选。检查靠**对候选骨架数值求值扫描**（复用 `eval`），
//! 求值时把可调常数 `__c{i}` 绑定到 `consts[i]`。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::eval::Env;
use crate::units::{check_expr, DimError, Dimension};

use super::grammar::{Candidate, GpContext};

/// 一个候选的约束检查结果。
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateCheck {
    pub units_ok: bool,
    pub monotone_ok: bool,
    pub bounds_ok: bool,
}
impl CandidateCheck {
    pub fn all_ok(&self) -> bool {
        self.units_ok && self.monotone_ok && self.bounds_ok
    }
}

/// 把可调常数绑进 env（`__c{i}` → consts[i]）。
fn bind_consts(env: &mut Env, consts: &[f64]) {
    for (i, v) in consts.iter().enumerate() {
        env.put(&Candidate::const_name(i), *v);
    }
}

/// 在给定输入取值下求值一个候选（绑可调常数 + 输入）。非有限/失败 → None。
/// 供 GP 适应度（表达式级复原）/诊断用。
pub fn eval_candidate(cand: &Candidate, inputs: &[(&str, f64)]) -> Option<f64> {
    let mut env = Env::new();
    bind_consts(&mut env, &cand.consts);
    for (n, v) in inputs {
        env.put(n, *v);
    }
    match cand.expr.eval_scalar(&env) {
        Ok(y) if y.is_finite() => Some(y),
        _ => None,
    }
}

/// 量纲软过滤：**只拒绝两个"物理有量纲"（均非无量纲）之间的 Mismatch**。
///
/// 可调常数是 `Param("__c*")` → 不在 `env` → `infer` 返回 None（未知）→ 天然跳过、不误报；
/// 结构常数（0/1/2）是 DIMENSIONLESS。`(T[degC] + VP[Pa])` 这种两已知物理量纲不兼容会被拒。
/// 自定义单位（portions/gCH2O…）解析为 None → 跳过。无 `env`（空）时恒通过。
pub fn units_ok(expr: &Expr, env: &HashMap<String, Dimension>) -> bool {
    let (_, errs) = check_expr(expr, env);
    !errs.iter().any(|e| match e {
        DimError::Mismatch { left, right, .. } => {
            !left.is_dimensionless() && !right.is_dimensionless()
        }
        _ => false,
    })
}

/// 固定其余输入、扫描 `var` 升序取值，检查弱单调（含平台，tol=1e-6）。求值失败的点跳过。
pub fn monotone_ok(
    expr: &Expr,
    consts: &[f64],
    var: &str,
    dir: &str,
    fixed: &[(String, f64)],
    sweep: (f64, f64),
    steps: usize,
) -> bool {
    let (lo, hi) = sweep;
    let tol = 1e-6;
    let steps = steps.max(2);
    let mut prev: Option<f64> = None;
    for i in 0..steps {
        let x = lo + (hi - lo) * (i as f64) / ((steps - 1) as f64);
        let mut env = Env::new();
        bind_consts(&mut env, consts);
        for (n, val) in fixed {
            env.put(n, *val);
        }
        env.put(var, x);
        let y = match expr.eval_scalar(&env) {
            Ok(y) if y.is_finite() => y,
            _ => continue,
        };
        if let Some(py) = prev {
            match dir {
                "increasing" if y < py - tol => return false,
                "decreasing" if y > py + tol => return false,
                _ => {}
            }
        }
        prev = Some(y);
    }
    true
}

/// 网格采样输入点，检查输出落在 [lo,hi]（tol=1e-6）。求值失败/非有限的点跳过。
pub fn bounds_ok(expr: &Expr, consts: &[f64], inputs: &[String], bounds: [f64; 2], sample_hi: f64) -> bool {
    let [lo, hi] = bounds;
    let tol = 1e-6;
    let grid = [0.0, 0.25, 0.5, 0.75, 1.0];
    let n = inputs.len().max(1);
    let total = grid.len().pow(n.min(4) as u32);
    for combo in 0..total {
        let mut env = Env::new();
        bind_consts(&mut env, consts);
        let mut idx = combo;
        for name in inputs.iter().take(4) {
            let g = grid[idx % grid.len()];
            idx /= grid.len();
            env.put(name, g * sample_hi);
        }
        let y = match expr.eval_scalar(&env) {
            Ok(y) if y.is_finite() => y,
            _ => continue,
        };
        if y < lo - tol || y > hi + tol {
            return false;
        }
    }
    true
}

/// 综合检查一个候选：量纲 + 单调（按 ctx.monotone 每条）+ 有界（若声明 output_bounds）。
pub fn check_candidate(
    cand: &Candidate,
    ctx: &GpContext,
    unit_env: &HashMap<String, Dimension>,
    sweep_hi: f64,
) -> CandidateCheck {
    let units_ok = units_ok(&cand.expr, unit_env);

    let mut monotone_ok = true;
    for (var, dir) in &ctx.monotone {
        let fixed: Vec<(String, f64)> = ctx
            .inputs
            .iter()
            .filter(|n| n.as_str() != var)
            .map(|n| (n.clone(), 0.5 * sweep_hi))
            .collect();
        if !self::monotone_ok(&cand.expr, &cand.consts, var, dir, &fixed, (0.0, sweep_hi), 25) {
            monotone_ok = false;
            break;
        }
    }

    let bounds_ok = match ctx.output_bounds {
        Some(b) => self::bounds_ok(&cand.expr, &cand.consts, &ctx.inputs, b, sweep_hi),
        None => true,
    };

    CandidateCheck { units_ok, monotone_ok, bounds_ok }
}
