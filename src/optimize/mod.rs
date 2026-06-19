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
//! 本模块自下而上分块实现：
//! - [`objective`]：**时间归约词汇** + 目标/约束 S 表达式 → 标量（本步）。
//! - 决策 spec + 目标评估核（候选 → 标量）、DE 优化器：后续步骤。

pub mod objective;

pub use objective::{eval_objective, ObjError, REDUCTIONS};
