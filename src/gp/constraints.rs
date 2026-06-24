//! GP 候选的约束检查：量纲（软过滤）+ 数值先验（单调/有界）。
//!
//! 用途：①保证采样/变异出的候选合法（语法已按构造满足，此为安全网 + G2 变异校验）；
//! ②catch 手工/进化出的违规候选。检查靠**对候选 `Expr` 数值求值扫描**（复用 `eval`）。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::eval::Env;
use crate::units::{check_expr, DimError, Dimension};

use super::grammar::GpContext;

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

/// 量纲软过滤：**只拒绝两个"物理有量纲"（均非无量纲）之间的 Mismatch**。
///
/// GP 临时常数 = 无量纲（`Dimension::DIMENSIONLESS`），作维度通配——`(T[degC] − c0)` 不算违规；
/// 而 `T_air[degC] + VP[Pa]`（两个已知物理量纲不兼容）会被拒。自定义单位（portions/gCH2O…）
/// 解析为 None → `infer` 跳过、无误报。无 `env`（空）时恒通过。
pub fn units_ok(expr: &Expr, env: &HashMap<String, Dimension>) -> bool {
    let (_, errs) = check_expr(expr, env);
    !errs.iter().any(|e| match e {
        DimError::Mismatch { left, right, .. } => {
            !left.is_dimensionless() && !right.is_dimensionless()
        }
        _ => false,
    })
}

/// 在固定其余输入、扫描 `var` 升序取值下，检查 `expr` 对 `var` 弱单调（dir="increasing"/"decreasing"）。
///
/// 弱单调（含平台，tol=1e-6）——clamp/饱和形式在区间端会平，仍合法。求值失败的点跳过。
pub fn monotone_ok(
    expr: &Expr,
    var: &str,
    dir: &str,
    fixed: &[(String, f64)],
    sweep: (f64, f64),
    steps: usize,
) -> bool {
    let (lo, hi) = sweep;
    let tol = 1e-6;
    let mut prev: Option<f64> = None;
    for i in 0..steps.max(2) {
        let x = lo + (hi - lo) * (i as f64) / ((steps.max(2) - 1) as f64);
        let mut env = Env::new();
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

/// 随机采样输入点，检查 `expr` 输出落在 [lo,hi]（tol=1e-6）。求值失败的点跳过。
pub fn bounds_ok(expr: &Expr, inputs: &[String], bounds: [f64; 2], sample_hi: f64) -> bool {
    let [lo, hi] = bounds;
    let tol = 1e-6;
    // 确定性网格采样（每输入取若干档），不引随机源（可复现）。
    let grid = [0.0, 0.25, 0.5, 0.75, 1.0];
    let n = inputs.len().max(1);
    let total = grid.len().pow(n.min(4) as u32); // 限制组合爆炸
    for combo in 0..total {
        let mut env = Env::new();
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

/// 综合检查一个候选：量纲 + 单调（按 ctx.monotone 每个约束）+ 有界（若声明 output_bounds）。
///
/// `unit_env`：变量/参数名 → 量纲（调用方从模型 unit 字段建；空 map = 跳过量纲）。
/// `sweep_hi`：输入扫描上界（典型量级，如温度 40、热龄 1000、需冷 50）。
pub fn check_candidate(
    expr: &Expr,
    ctx: &GpContext,
    unit_env: &HashMap<String, Dimension>,
    sweep_hi: f64,
) -> CandidateCheck {
    let units_ok = units_ok(expr, unit_env);

    // 单调：对每个声明了方向的变量扫描；其余输入固定在中点。
    let mut monotone_ok = true;
    for (var, dir) in &ctx.monotone {
        let fixed: Vec<(String, f64)> = ctx
            .inputs
            .iter()
            .filter(|n| n.as_str() != var)
            .map(|n| (n.clone(), 0.5 * sweep_hi))
            .collect();
        if !self::monotone_ok(expr, var, dir, &fixed, (0.0, sweep_hi), 25) {
            monotone_ok = false;
            break;
        }
    }

    let bounds_ok = match ctx.output_bounds {
        Some(b) => self::bounds_ok(expr, &ctx.inputs, b, sweep_hi),
        None => true,
    };

    CandidateCheck { units_ok, monotone_ok, bounds_ok }
}
