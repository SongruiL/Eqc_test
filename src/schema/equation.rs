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

/// 方程来源档（出处诚实纪律，见 `docs/spec-fspm-development.md` §3）。
///
/// 配合 `reference`（引用文献）= 完整出处。按"来源阶梯"标注每条方程的可信层级：
/// 文献（直接用于本作物的已发表方程，最高）> 平移（从他作物搬）> 推导（从理论推）> 猜测（占位）。
/// **下游用途**：①受约束 GP 自动选靶点（`推导`/`猜测` → 可进化，`文献` → 冻结基座，与 `gp_target` 协同）；
/// ②契约带出 → 生长动画按出处上色。additive：缺省（None）= 未标注，序列化跳过，现有模型逐字节不变。
/// YAML 可写中文（`provenance: 文献`）或英文别名（`literature`/`transferred`/`derived`/`guess`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    /// 直接用于本作物的已发表方程（置信最高，机理基座、宜冻结）。
    #[serde(rename = "文献", alias = "literature")]
    Literature,
    /// 从其它作物平移的方程（应在 `reference` 标源作物 + 可移植性）。
    #[serde(rename = "平移", alias = "transferred")]
    Transferred,
    /// 从作物生长理论 / 第一性原理推导（无直接文献）。
    #[serde(rename = "推导", alias = "derived")]
    Derived,
    /// 占位 / 猜测（连理论都薄；诚实标注，优先交 GP/标定）。
    #[serde(rename = "猜测", alias = "guess")]
    Guess,
}

impl Provenance {
    /// 是否"不确定"（推导/猜测）——受约束 GP 选靶点的默认判据。
    pub fn is_uncertain(&self) -> bool {
        matches!(self, Provenance::Derived | Provenance::Guess)
    }
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

    /// 方程来源档（文献/平移/推导/猜测；出处诚实纪律）。缺省 = 未标注。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,

    /// FSPM 器官实例身份（地基，见 `docs/spec-fspm-foundation.md`）。
    ///
    /// 由 `structure:`/`cohorts:` 加载期实例化时填（对实体每实例展开一份）；**引擎不读、下游读**。
    /// `None` = 整株共享 / 非结构方程。additive：缺省时序列化跳过，现有模型逐字节不变。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<crate::schema::InstanceTag>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_yaml_roundtrip() {
        // 中文值反序列化
        assert_eq!(serde_yaml::from_str::<Provenance>("文献").unwrap(), Provenance::Literature);
        assert_eq!(serde_yaml::from_str::<Provenance>("猜测").unwrap(), Provenance::Guess);
        // 英文别名也接受
        assert_eq!(serde_yaml::from_str::<Provenance>("transferred").unwrap(), Provenance::Transferred);
        assert_eq!(serde_yaml::from_str::<Provenance>("derived").unwrap(), Provenance::Derived);
        // 序列化回中文（契约带出用）
        assert_eq!(serde_yaml::to_string(&Provenance::Literature).unwrap().trim(), "文献");
        // 不确定性判据（GP 选靶点用）
        assert!(Provenance::Derived.is_uncertain() && Provenance::Guess.is_uncertain());
        assert!(!Provenance::Literature.is_uncertain() && !Provenance::Transferred.is_uncertain());
    }

    #[test]
    fn equation_provenance_is_additive() {
        // 无 provenance → None（缺省，现有模型逐字节不变）
        let e: Equation =
            serde_yaml::from_str("{id: E1, name: 测试, output: y, expression: {ref: x}}").unwrap();
        assert!(e.provenance.is_none());
        // 标 provenance: 文献 → Some
        let e2: Equation = serde_yaml::from_str(
            "{id: E2, name: 测试, output: y, expression: {ref: x}, provenance: 文献}",
        )
        .unwrap();
        assert_eq!(e2.provenance, Some(Provenance::Literature));
    }
}
