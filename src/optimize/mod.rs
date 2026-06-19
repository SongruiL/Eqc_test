//! 优化层：在前向模型（解释器 `sim`/`eval`）上做**仿真优化**。
//!
//! 见 `docs/spec-optimization.md`。三层架构：
//!
//! ```text
//!   搜索算法（DE / 网格 / …）         ← 怎么找
//!   目标评估核（旋钮赋值 → 跑 sim → 归约成标量 + 约束惩罚）  ← 评价一个候选
//!   前向模型 + 解释器（src/sim, src/eval）                  ← 算一次（唯一权威）
//! ```
//!
//! 本模块自下而上分块：
//! - [`objective`]：**时间归约词汇** + 目标/约束 S 表达式 → 标量。
//! - [`problem`]：**决策 spec**（与模型分离的独立产物：目标 + 旋钮 + 常量 + 约束 + 优化器）。
//! - [`core`]：**目标评估核**（候选旋钮赋值 → 代价；垃圾候选给最差值不崩）。
//! - DE 优化器：后续步骤。

pub mod core;
pub mod objective;
pub mod problem;

pub use core::{evaluate, validate_problem, EvalOutcome, WORST_COST};
pub use objective::{eval_objective, ObjError, REDUCTIONS};
pub use problem::{
    load_problem, parse_problem, Constraint, Knob, KnobKind, Objective, OptimizerCfg, Problem, Sense,
};
