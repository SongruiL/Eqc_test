//! 决策 spec：与模型文件**分离**的独立产物（见 `docs/spec-optimization.md` §2、§4）。
//!
//! 核心哲学：**「可控」不是变量的固有属性，而是『问题/场景』的属性**——同一个变量在不同
//! 问题里是决策还是固定环境会变。所以「哪些是旋钮、能设到多少、什么代价」不写进模型，
//! 单独放在这一层（与「模型结构 vs 情景数据」分离同理）。
//!
//! ```yaml
//! optimize:
//!   objective:
//!     expr: (sub (mul (final Y) price) (mul CO2 co2_cost))   # 目标 S 表达式（含时间归约）
//!     sense: max                                             # max / min
//!   knobs:
//!     - { var: CO2, kind: driver_const, bounds: [400, 1200], unit: ppm }  # 恒定驱动
//!     - { var: Pd,  kind: param,        bounds: [5, 10] }                 # 标量参数
//!     - { var: TDM, kind: init,         bounds: [10, 30] }                # 状态初值
//!   constants:                  # 目标/约束里用到的非模型量（单价、成本系数、目标值…）
//!     price: 30.0
//!     co2_cost: 0.002
//!   constraints:
//!     - { expr: (sub (total energy) budget), max: 0 }        # expr ≤ max（惩罚法）
//!   environment: weather.csv    # 未被选为旋钮的驱动 = 不可控环境（相对 spec 文件目录解析）
//!   optimizer:
//!     method: de
//!     pop: 30
//!     iters: 100
//!     seed: 42                  # 定种子 → 结果可复现
//! ```

use std::path::Path;

use indexmap::IndexMap;
use serde::Deserialize;

/// 决策 spec 文件的顶层（只有一个 `optimize:` 键）。
#[derive(Debug, Clone, Deserialize)]
struct ProblemFile {
    optimize: Problem,
}

/// 一个优化「问题」：目标 + 旋钮 + 常量 + 约束 + 环境 + 优化器配置。
#[derive(Debug, Clone, Deserialize)]
pub struct Problem {
    pub objective: Objective,
    /// 第二目标（可选）。提供则进入**多目标模式**：输出两目标的 Pareto 权衡前沿
    /// （`objective` 为目标 1、`objective2` 为目标 2）。雏形仅支持 2 目标。
    #[serde(default)]
    pub objective2: Option<Objective>,
    pub knobs: Vec<Knob>,
    /// 目标/约束方程引用的非模型标量（单价、成本系数、目标值…）。
    #[serde(default)]
    pub constants: IndexMap<String, f64>,
    /// 一般约束：`expr ≤ max`，用惩罚法。
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// **最优点报告量**（决策优化专用）：在求得的最优旋钮处，对每条命名 S 表达式求值，
    /// 随结果 JSON 一并返回（如「预期鲜产 = (final Y_fresh)」「预期糖度 = (mean Brix)」）。
    /// 只在最优点算一次、**不进 DE 内循环**；缺省空 = 不报告。供 GIS「本区最优管理」面板显预期产量/品质。
    #[serde(default)]
    pub report: Vec<Report>,
    /// 约束惩罚权重（线性外罚 `cost += weight·Σ违反量`）。缺省见 `core::DEFAULT_PENALTY_WEIGHT`（1e9）。
    #[serde(default)]
    pub penalty_weight: Option<f64>,
    /// 不可控环境（驱动量时间序列 CSV）；相对 spec 文件目录解析。可缺省（用 CLI `--drivers`）。
    #[serde(default)]
    pub environment: Option<String>,
    /// 实测数据 CSV（参数标定用）；相对 spec 目录解析。可缺省（用 CLI `--observed`）。
    #[serde(default)]
    pub observed: Option<String>,
    /// 候选可观测变量（可辨识性分析用 `eqc identify`）：园区**能测**哪些变量。
    /// 缺省 = 模型所有 `type: output` 标量变量。可被 CLI `--observables` 覆盖。
    #[serde(default)]
    pub observables: Option<Vec<String>>,
    /// **处理矩阵**（可辨识性分析用 `eqc identify`）：每个元素 = 一组【模型标量参数覆盖】，
    /// 代表一个管理/实验工作点（如 `{EC_feed: 5.0, Irrig: 8.0}`）。identify 会**逐处理**跑 OAT
    /// 灵敏度、把各处理的敏感子矩阵**横向拼接**后再判可辨识 / 异参同效——使单一工作点下共线的
    /// 参数（阈值⟂斜率、多个叠加项）被【对比梯度】分开，并给出「在哪个处理测哪个观测」的实验设计。
    /// 缺省 = 空 = 单一默认工作点（现行为，向后兼容）。处理参数应与 knobs 不相交。
    #[serde(default)]
    pub treatments: Vec<IndexMap<String, f64>>,
    /// **耦合优化**（C3）：提供则前向模型 = 多速率耦合仿真（温室↔作物），旋钮为温室/作物
    /// 参数（kind=`fast_param`/`slow_param`），目标归约作物轨迹。见 `docs/spec-coupled-simulation.md` §8。
    #[serde(default)]
    pub coupling: Option<CouplingSpec>,
    #[serde(default)]
    pub optimizer: OptimizerCfg,
}

/// 耦合优化的前向模型配置（spec 的 `coupling:` 块；路径相对 spec 目录解析）。
#[derive(Debug, Clone, Deserialize)]
pub struct CouplingSpec {
    /// 快模型（温室）`.eq.yaml`。
    pub fast: String,
    /// 慢模型（作物）`.eq.yaml`。
    pub slow: String,
    /// 快模型室外驱动 CSV（相对 spec 目录）。
    #[serde(default)]
    pub weather: Option<String>,
    /// 快→慢链接。
    #[serde(default)]
    pub links: Vec<LinkSpec>,
    /// 慢→快反馈（双向）。
    #[serde(default)]
    pub feedback: Vec<FeedbackSpec>,
    /// 慢步数（缺省 = 室外驱动行数 / R）。
    #[serde(default)]
    pub steps: Option<usize>,
    /// 快模型（温室）**固定**参数覆盖（非旋钮的环控设置，如 Q_heat=0）；旋钮在其上再覆盖。
    #[serde(default)]
    pub fast_params: IndexMap<String, f64>,
    /// 慢模型（作物）固定参数覆盖。
    #[serde(default)]
    pub slow_params: IndexMap<String, f64>,
}

/// 一条快→慢链接（spec）。
#[derive(Debug, Clone, Deserialize)]
pub struct LinkSpec {
    pub to: String,
    pub from: String,
    #[serde(default = "default_agg")]
    pub agg: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
}

/// 一条慢→快反馈（spec）。
#[derive(Debug, Clone, Deserialize)]
pub struct FeedbackSpec {
    pub to: String,
    pub from: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub init: f64,
}

fn default_agg() -> String {
    "mean".to_string()
}
fn default_scale() -> f64 {
    1.0
}

/// 目标：一条 S 表达式 + 取向（最大化/最小化）。
#[derive(Debug, Clone, Deserialize)]
pub struct Objective {
    pub expr: String,
    #[serde(default)]
    pub sense: Sense,
}

/// 目标取向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sense {
    Max,
    Min,
}

impl Default for Sense {
    fn default() -> Self {
        Sense::Max
    }
}

impl Sense {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sense::Max => "max",
            Sense::Min => "min",
        }
    }
}

/// 一个旋钮（决策变量）：把一个数赋给某个**外部输入**。
#[derive(Debug, Clone, Deserialize)]
pub struct Knob {
    /// 被赋值的外部输入名（参数 / 状态初值 / 驱动量）。
    pub var: String,
    pub kind: KnobKind,
    /// 箱形边界 `[lo, hi]`。
    pub bounds: [f64; 2],
    #[serde(default)]
    pub unit: Option<String>,
}

/// 旋钮种类（阶段 1 仅标量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnobKind {
    /// 标量参数（覆盖 `parameters:` 默认值）。
    Param,
    /// 状态量 / 延迟寄存器的初值（覆盖 `init:`）。
    Init,
    /// 恒定驱动：把某驱动量整列设成一个常数。
    DriverConst,
    /// 耦合优化：快模型（温室）参数覆盖。
    FastParam,
    /// 耦合优化：慢模型（作物）参数覆盖。
    SlowParam,
}

impl KnobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            KnobKind::Param => "param",
            KnobKind::Init => "init",
            KnobKind::DriverConst => "driver_const",
            KnobKind::FastParam => "fast_param",
            KnobKind::SlowParam => "slow_param",
        }
    }
}

/// 一般约束：`expr ≤ max`（默认 `max = 0`），违反时按惩罚法加大额代价。
#[derive(Debug, Clone, Deserialize)]
pub struct Constraint {
    pub expr: String,
    #[serde(default)]
    pub max: f64,
}

/// 一条「最优点报告量」：命名 + S 表达式（可含 final/mean/max 等时间归约）+ 单位（仅展示用）。
/// 在最优旋钮处求值（复用目标的轨迹归约求值），随结果 JSON 返回给前端展示。
#[derive(Debug, Clone, Deserialize)]
pub struct Report {
    pub name: String,
    pub expr: String,
    #[serde(default)]
    pub unit: Option<String>,
}

/// 优化器配置（阶段 1：差分进化 DE）。
#[derive(Debug, Clone, Deserialize)]
pub struct OptimizerCfg {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_pop")]
    pub pop: usize,
    #[serde(default = "default_iters")]
    pub iters: usize,
    #[serde(default = "default_seed")]
    pub seed: u64,
}

fn default_method() -> String {
    "de".to_string()
}
fn default_pop() -> usize {
    30
}
fn default_iters() -> usize {
    100
}
fn default_seed() -> u64 {
    42
}

impl Default for OptimizerCfg {
    fn default() -> Self {
        Self {
            method: default_method(),
            pop: default_pop(),
            iters: default_iters(),
            seed: default_seed(),
        }
    }
}

impl Problem {
    /// 是否多目标模式（提供了 `objective2`）。
    pub fn is_multi(&self) -> bool {
        self.objective2.is_some()
    }
}

/// 从 YAML 串解析决策 spec。
pub fn parse_problem(yaml: &str) -> Result<Problem, String> {
    let pf: ProblemFile =
        serde_yaml::from_str(yaml).map_err(|e| format!("决策 spec 解析失败: {e}"))?;
    Ok(pf.optimize)
}

/// 从文件读取决策 spec。
pub fn load_problem(path: &Path) -> Result<Problem, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("读取决策 spec {} 失败: {e}", path.display()))?;
    parse_problem(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_spec() {
        let yaml = r#"
optimize:
  objective:
    expr: (sub (mul (final Y) price) (mul CO2 co2_cost))
    sense: max
  knobs:
    - { var: CO2, kind: driver_const, bounds: [400, 1200], unit: ppm }
    - { var: Pd,  kind: param,        bounds: [5, 10] }
    - { var: TDM, kind: init,         bounds: [10, 30] }
  constants:
    price: 30.0
    co2_cost: 0.002
  constraints:
    - { expr: (sub (total energy) budget), max: 0 }
  environment: weather.csv
  optimizer:
    method: de
    pop: 40
    iters: 80
    seed: 7
"#;
        let p = parse_problem(yaml).unwrap();
        assert_eq!(p.objective.sense, Sense::Max);
        assert_eq!(p.knobs.len(), 3);
        assert_eq!(p.knobs[0].var, "CO2");
        assert_eq!(p.knobs[0].kind, KnobKind::DriverConst);
        assert_eq!(p.knobs[0].bounds, [400.0, 1200.0]);
        assert_eq!(p.knobs[1].kind, KnobKind::Param);
        assert_eq!(p.knobs[2].kind, KnobKind::Init);
        assert_eq!(p.constants.get("price"), Some(&30.0));
        assert_eq!(p.constraints.len(), 1);
        assert_eq!(p.environment.as_deref(), Some("weather.csv"));
        assert_eq!(p.optimizer.pop, 40);
        assert_eq!(p.optimizer.seed, 7);
    }

    #[test]
    fn test_defaults() {
        // 最小 spec：只有目标和一个旋钮，其余取默认。
        let yaml = r#"
optimize:
  objective:
    expr: (final Y)
  knobs:
    - { var: Pd, kind: param, bounds: [5, 10] }
"#;
        let p = parse_problem(yaml).unwrap();
        assert_eq!(p.objective.sense, Sense::Max); // 默认 max
        assert_eq!(p.optimizer.method, "de");
        assert_eq!(p.optimizer.pop, 30);
        assert_eq!(p.optimizer.iters, 100);
        assert_eq!(p.optimizer.seed, 42);
        assert!(p.constants.is_empty());
        assert!(p.constraints.is_empty());
        assert!(p.environment.is_none());
    }
}
