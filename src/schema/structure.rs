//! FSPM 结构表示（地基，见 `docs/spec-fspm-foundation.md`）：器官实例身份 + 拓扑的**单一真相源**。
//!
//! 加载期把 `structure:` 段（或 cohort lower）实例化成**带身份标签的标量**后，这些类型作为
//! 结构化身份并行保留：**引擎层（sim/eval/Stepper）一概不读、照跑标量**；下游（NodeResolver/图/
//! 契约/视图）读身份 → 能按器官折叠/按实例上色，**无需反解字符串后缀**（红线：身份永远一等数据）。
//!
//! 全部 additive：纯 Functional 模型 `EquationFile.structure = None`、变量/方程 `instance = None`，
//! 序列化跳过 → 现有模型行为与导出逐字节不变。

use serde::{Deserialize, Serialize};

/// 一个变量/方程的器官实例身份标签（实例化时填）。`None` = 整株共享 / 非结构量。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceTag {
    /// 实体类型名（如 `metamer` / `fruit`）。
    pub entity: String,
    /// 实例 id（路径形式：`"3"` / `"3.2"`）。
    pub id: String,
}

/// 一个实体类型（实例化后的元信息；前端/图层据此知道"有哪些器官类型"）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDecl {
    /// 实体类型名。
    pub name: String,
    /// 实例总数（绝对，或 per-parent 展开后的合计）。
    pub count: usize,
    /// 拓扑种类（`chain` / `per` / `bears`；本期实现这三种）。
    pub topology: String,
}

/// 一个器官实例（id + 所属实体 + 父实例 id）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    /// 实例 id（`"3"` / `"3.2"`）。
    pub id: String,
    /// 所属实体类型名。
    pub entity: String,
    /// 父实例 id（per / bears 关系；chain 顶层无父）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// 一条拓扑边（实例间，带种类）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopoEdge {
    /// 源实例 id。
    pub from: String,
    /// 目标实例 id。
    pub to: String,
    /// 边种类：`succession`（链）/ `contains`（分解，父→子）/ `bears`（横生）。
    pub kind: String,
}

/// 一条聚合关系（FSPM 风险3·可见性）：某变量沿拓扑邻域聚合而来。
/// 聚合在加载期已 lower 成标量 add 链 → 引擎照跑；此处**保留语义出处**，
/// 供分析/前端显示「Σ over children / mean over all」（AST 节点 lower 掉了，靠这条带出来）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregationInfo {
    /// 被聚合算出的变量（基名，未带实例后缀；如 `node_fruit`）。
    pub output: String,
    /// 聚合种类：`sum` / `mean` / `prod` / `min` / `max`。
    pub kind: String,
    /// 邻域选择器：`children`（直接子）/ `all`（实体全集）。
    pub over: String,
    /// 被聚合的目标实体（如 `fruit`）；`children` 未显式 `of:` 时为 `None`（由拓扑可推）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
}

/// 模型结构（声明 + 实例化结果，**单一真相源**）。`EquationFile.structure = None` = 纯 Functional 模型。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StructureInfo {
    /// 实体类型清单。
    #[serde(default)]
    pub entities: Vec<EntityDecl>,
    /// 全部器官实例。
    #[serde(default)]
    pub instances: Vec<Instance>,
    /// 拓扑边。
    #[serde(default)]
    pub topology: Vec<TopoEdge>,
    /// 聚合关系（FSPM 风险3·可见性；空=无聚合，旧结构模型契约逐字节不变）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregations: Vec<AggregationInfo>,
}
