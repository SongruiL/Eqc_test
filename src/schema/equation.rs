//! 方程定义

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::ast::Expr;

/// 遗传编程（GP）进化靶点标记（受约束 GP；详见 docs/spec-genetic-programming.md）。
///
/// 出现在某方程上 = 该方程是「假设留白（🟠）」、属可进化集合；不出现 = 机理基座（🟢/🔵）、冻结。
/// 进化-冻结边界来自理论溯源（crop-models/理论溯源/）的逐方程分类。
/// 本结构是 **additive 契约字段**（`#[serde(default)]`，缺省不影响既有模型）；
/// 各字段在 G1+（语法/算子/主循环）消费，G0 仅落地字段 + 导出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpTarget {
    /// 引用的候选形式语法名（§5），如 "monotone_gate" / "allocation_fraction"。
    pub grammar: String,

    /// 候选式可用变量白名单（默认空 = G1 时取该方程当前 refs ∪ 同模块在范围变量）。
    #[serde(default)]
    pub inputs: Vec<String>,

    /// 先验：输出有界 [lo, hi]（如门控 [0,1]）。
    #[serde(default)]
    pub output_bounds: Option<[f64; 2]>,

    /// 先验：对某变量单调（值为 "increasing" / "decreasing"）。
    #[serde(default)]
    pub monotone: IndexMap<String, String>,

    /// 临时冻结一个已标的靶（默认 false = 可进化）；用于「标了但本轮不进化」。
    #[serde(default)]
    pub frozen: bool,
}

/// 方程定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equation {
    /// 方程 ID（如 "PHOTO-01"）
    pub id: String,

    /// 方程名称
    pub name: String,

    /// 输出变量名
    pub output: String,

    /// 表达式 AST
    pub expression: Expr,

    /// 可读公式（仅供展示）
    #[serde(default)]
    pub formula_display: Option<String>,

    /// 参考文献
    #[serde(default)]
    pub reference: Option<String>,

    /// GP 进化靶点标记（受约束 GP；缺省 = 机理基座、冻结）。
    #[serde(default)]
    pub gp_target: Option<GpTarget>,
}

impl Equation {
    /// 获取表达式中引用的所有变量
    pub fn get_variable_refs(&self) -> Vec<String> {
        self.expression.get_variable_refs()
    }

    /// 获取表达式中引用的所有参数
    pub fn get_parameter_refs(&self) -> Vec<String> {
        self.expression.get_parameter_refs()
    }

    /// 获取完整的依赖列表（变量 + 参数）
    pub fn get_all_dependencies(&self) -> Vec<String> {
        let mut deps = self.get_variable_refs();
        deps.extend(self.get_parameter_refs());
        deps
    }
}
