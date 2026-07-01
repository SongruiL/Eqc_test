//! 方程文件结构

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Equation, Parameter, Variable};

/// 方程文件（对应一个 .eq.yaml 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationFile {
    /// 元数据
    pub meta: Metadata,

    /// 参数定义（IndexMap：保留 YAML 声明顺序，保证输出可复现）
    #[serde(default)]
    pub parameters: IndexMap<String, Parameter>,

    /// 变量定义（IndexMap：保留 YAML 声明顺序，保证输出可复现）
    #[serde(default)]
    pub variables: IndexMap<String, Variable>,

    /// 方程定义
    #[serde(default)]
    pub equations: Vec<Equation>,

    /// FSPM 结构（器官实例 + 拓扑的单一真相源，地基见 `docs/spec-fspm-foundation.md`）。
    ///
    /// 由 `structure:` 段（或 `cohorts:` lower）加载期实例化时填；**引擎不读、下游读**。
    /// `None` = 纯 Functional 模型。additive：缺省时序列化跳过，现有模型逐字节不变。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<super::StructureInfo>,
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// 模块 ID
    pub id: String,

    /// 所属模型名称
    pub model: String,

    /// 中文名称
    pub name_cn: String,

    /// 英文名称
    #[serde(default)]
    pub name_en: Option<String>,

    /// 版本
    #[serde(default = "default_version")]
    pub version: String,

    /// 描述
    #[serde(default)]
    pub description: Option<String>,

    /// 参考文献
    #[serde(default)]
    pub reference: Option<String>,

    /// 源代码文件（参考用）
    #[serde(default)]
    pub source_files: Vec<String>,

    /// 仿真时间步长（与速率方程的时间单位一致；缺省 1.0 = 现有日步长模型，行为逐位不变）。
    /// 状态量积分按 `X[n] = X[n-1] + rate[n]·dt`；亚日动态模型（如温室气候 ODE）设更小值
    /// （秒/分钟级）。可被 `SimInput.dt` / CLI `--dt` 覆盖。
    #[serde(default = "default_dt")]
    pub dt: f64,

    /// 步长折合**秒数**（耦合仿真用）。`dt` 是各模型自己时间单位下的步长（温室 dt=10 即 10s、
    /// 日级作物 dt=1 即 1 天），单位不互通；多速率耦合需统一到秒，故模型自描述其步长的秒数
    /// （温室=10、日级作物=86400、小时级=3600）。缺省 None = 单模型仿真不需要、不影响。
    #[serde(default)]
    pub dt_seconds: Option<f64>,

    /// 标定状态（看懂输出的「可信度徽章」用）。缺省 None = 视为未标定。
    #[serde(default)]
    pub calibration: Option<Calibration>,

    /// 子模块划分（DAG「模块级」视图用）：模块名 → 该模块的方程 id 列表。
    ///
    /// 作者声明（写在 `meta:` 下），如 `modules: {光合: [SB-01, SB-02], 水: [SB-ES, SB-VPD]}`。
    /// DAG 模块级把每个方程节点折叠进其模块、聚合跨模块边 → 一眼看整体运算逻辑。
    /// 未列入任何模块的方程归「未分组」。缺省空 = 无模块级视图（回退方程级）。
    #[serde(default)]
    pub modules: IndexMap<String, Vec<String>>,

    /// 守恒律声明（CLI `--check-balance` 用）：模型自声明守什么（碳/水/氮…），CLI 据此逐步核
    /// `|Δstock − dt·(Σsources − Σsinks)| ≤ tol`。**单一真相源**：守恒结构进契约，标定全程可验。
    /// additive、`#[serde(default)]` 缺省空 = 不声明=不检查；现有模型逐字节不变。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub balance: Vec<BalanceLaw>,
}

/// 守恒律（`meta.balance` 一条）：存量的逐步差分应等于源减汇（乘 dt）。
///
/// 让「守恒」从测试时手算 → 模型自带、CLI 可验、标定全程的安全带（FSPM F5c）。
/// 例：`{ name: 碳, stock: C_system, sources: [A_gross], sinks: [resp_total], tol: 1e-6 }`。
///
/// 带 `cap` 时核算 `|Δstock − dt·(Σsources − Σsinks)/cap| ≤ tol`，用于「dX/dt = 净通量 / 容量」
/// 型平衡（温室能量 cap=ρcp·h、湿度 cap=h）；缺省 cap≡1（碳/水/氮等直接源-汇守恒）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceLaw {
    /// 守恒律名（如「碳」「水」「氮」）
    pub name: String,
    /// 存量变量名（如 C_system）；逐步差分 `Δstock` 应 = `dt·(Σsources − Σsinks)/cap`
    pub stock: String,
    /// 源项（流入）变量名列表（如 `[A_gross]`）
    #[serde(default)]
    pub sources: Vec<String>,
    /// 汇项（流出）变量名列表（如 `[resp_total]`）
    #[serde(default)]
    pub sinks: Vec<String>,
    /// 可选「有效容量」**变量名**：存量差分 = `dt·净流量 / cap`。用于状态量不是通量直接积分、
    /// 而是「净通量 ÷ 容量」的平衡（温室能量 cap=ρcp·h、湿度 cap=h）。须是**轨迹里的变量**
    /// （同 sources/sinks；若容量由参数构成，请先在 yaml 里抽成辅助变量再引用）。
    /// 缺省 None = cap≡1（现有碳/水/氮守恒行为逐字节不变）。additive。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<String>,
    /// 绝对残差容差上限（超过即判不守恒）
    #[serde(default = "default_balance_tol")]
    pub tol: f64,
}

fn default_balance_tol() -> f64 {
    1e-6
}

/// 模型标定状态：诚实告知非数学用户「此结果可不可信」。
///
/// 未标定 = 参数为占位/文献值（合成情景），结果**仅供方向参考**；
/// 已标定 = 用田间实测反推过参数，量级可信。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibration {
    /// 是否已用实测数据标定过。
    #[serde(default)]
    pub calibrated: bool,

    /// 说明（如"全部参数为占位值，待云南 2026-07 数据标定"或"已用一区数据标定 LUE/SLA"）。
    #[serde(default)]
    pub note: Option<String>,

    /// 标定日期（可选，ISO 如 2026-07-15）。
    #[serde(default)]
    pub date: Option<String>,
}

fn default_version() -> String {
    "1.0".to_string()
}

fn default_dt() -> f64 {
    1.0
}

impl EquationFile {
    /// 获取所有参数名称
    pub fn parameter_names(&self) -> Vec<&str> {
        self.parameters.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有变量名称
    pub fn variable_names(&self) -> Vec<&str> {
        self.variables.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有方程 ID
    pub fn equation_ids(&self) -> Vec<&str> {
        self.equations.iter().map(|e| e.id.as_str()).collect()
    }

    /// 获取输出变量列表
    pub fn output_variables(&self) -> Vec<(&str, &Variable)> {
        self.variables
            .iter()
            .filter(|(_, v)| v.var_type == super::VariableType::Output)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// 获取输入变量列表
    pub fn input_variables(&self) -> Vec<(&str, &Variable)> {
        self.variables
            .iter()
            .filter(|(_, v)| v.var_type == super::VariableType::Input)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// 某个名字（变量/参数/方程输出）的**友好显示名**，优先级：
    /// 变量 `label` → 方程中文名（该名字是某方程的 output）→ 参数 `name_cn` → 代号（兜底）。
    ///
    /// EQC 单一权威：DAG 节点标签（[`crate::dag::build_dag`] 后置）与 JSON 契约
    /// （[`crate::export`] 的 `display_name`）共用此优先级，保证结构图/图表/勾选框显示一致。
    /// 兜底回代号者 = 该变量缺 `label`/中文名 → 属逐作物补标注的建模缺口。
    pub fn display_name(&self, name: &str) -> String {
        if let Some(v) = self.variables.get(name) {
            if let Some(label) = &v.label {
                return label.clone();
            }
        }
        if let Some(eq) = self.equations.iter().find(|e| e.output == name) {
            return eq.name.clone();
        }
        if let Some(p) = self.parameters.get(name) {
            return p.name_cn.clone();
        }
        // 延迟寄存器（`prev: 源`）无 label → 派生「源的友好名（上一步）」，
        // 这样给源变量标一次 label 就顺带覆盖它的 *_prev 寄存器（如 C_Buf → C_Buf_prev=碳缓冲库（上一步））。
        if let Some(v) = self.variables.get(name) {
            if let Some(src) = &v.prev {
                if src != name {
                    return format!("{}（上一步）", self.display_name(src));
                }
            }
        }
        name.to_string()
    }

    /// 将方程表达式中引用了「参数名」的 `Var` 节点重分类为 `Param`。
    ///
    /// EQC 解析单个名字时没有上下文（不知道 parameters 列表），所有 `{ref: x}` 先一律
    /// 解析为 `Var`。在整个文件加载、parameters 已知之后调用本方法做修正——这样参数就能
    /// 用任意有意义的名字（如 `Tbase`、`AMAX`），而不必非叫 `p1`、`p2`。
    pub fn reclassify_parameters(&mut self) {
        use crate::ast::Expr;
        let pnames: Vec<String> = self.parameters.keys().cloned().collect();
        for eq in &mut self.equations {
            for pname in &pnames {
                eq.expression = eq
                    .expression
                    .substitute(pname, &Expr::Param(pname.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::schema::DataType;

    #[test]
    fn test_reclassify_parameters() {
        let mut parameters = IndexMap::new();
        parameters.insert(
            "Tbase".to_string(),
            Parameter {
                name_cn: "基点温度".into(),
                name_en: None,
                dtype: DataType::Float,
                default: 3.0,
                values: None,
                unit: Some("degC".into()),
                bounds: None,
                optimizable: true,
                management: false,
                description: None,
                provenance: None,
            },
        );

        // 表达式引用 Tbase（非 p+数字，故先被解析为 Var）
        let expr = Expr::sub(Expr::var("Tavg"), Expr::var("Tbase"));
        assert!(
            expr.get_parameter_refs().is_empty(),
            "重分类前 Tbase 应是变量"
        );

        let mut file = EquationFile {
            meta: Metadata {
                id: "T".into(),
                model: "T".into(),
                name_cn: "".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(), balance: vec![],
            },
            parameters,
            variables: Default::default(),
            equations: vec![Equation {
                id: "E".into(),
                name: "".into(),
                output: "y".into(),
                expression: expr,
                formula_display: None,
                reference: None, gp_target: None, provenance: None,
             instance: None }],
         structure: None };

        file.reclassify_parameters();
        let pref = file.equations[0].expression.get_parameter_refs();
        let vref = file.equations[0].expression.get_variable_refs();
        assert!(pref.contains(&"Tbase".to_string()), "Tbase 应被重分类为参数: {pref:?}");
        assert!(vref.contains(&"Tavg".to_string()), "Tavg 仍应是变量");
        assert!(!vref.contains(&"Tbase".to_string()), "Tbase 不应再是变量");
    }

    #[test]
    fn test_display_name_priority() {
        use crate::schema::Variable;
        // 最小变量构造（label 可选）
        let var = |label: Option<&str>| Variable {
            var_type: super::super::VariableType::Output,
            dtype: DataType::Float,
            unit: None,
            description: Some("一段较长的描述".into()),
            label: label.map(|s| s.to_string()),
            measurable: false,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class: None,
            init: None,
            rate: None,
            prev: None,
         instance: None };
        let mut variables = IndexMap::new();
        variables.insert("Y".to_string(), var(Some("鲜重产量"))); // ① 有 label
        variables.insert("DM".to_string(), var(None)); // ② 无 label，是方程输出
        let mut parameters = IndexMap::new();
        parameters.insert(
            "Kc".to_string(),
            Parameter {
                name_cn: "作物系数".into(),
                name_en: None,
                dtype: DataType::Float,
                default: 1.0,
                values: None,
                unit: None,
                bounds: None,
                optimizable: true,
                management: false,
                description: None,
                provenance: None,
            }, // ③ 参数 → name_cn
        );
        let file = EquationFile {
            meta: Metadata {
                id: "M".into(),
                model: "M".into(),
                name_cn: "".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(), balance: vec![],
            },
            parameters,
            variables,
            equations: vec![Equation {
                id: "E1".into(),
                name: "干物质".into(),
                output: "DM".into(),
                expression: Expr::var("Y"),
                formula_display: None,
                reference: None, gp_target: None, provenance: None,
             instance: None }],
         structure: None };
        assert_eq!(file.display_name("Y"), "鲜重产量"); // ① 变量 label 最高优先
        assert_eq!(file.display_name("DM"), "干物质"); // ② 回退到方程中文名（不取 description）
        assert_eq!(file.display_name("Kc"), "作物系数"); // ③ 参数 name_cn
        assert_eq!(file.display_name("ghost"), "ghost"); // ④ 三级皆缺 → 兜底代号
    }

    #[test]
    fn test_display_name_prev_derive() {
        use crate::schema::{VarClass, Variable, VariableType};
        let state = Variable {
            var_type: VariableType::Output,
            dtype: DataType::Float,
            unit: None,
            description: None,
            label: Some("碳缓冲库".into()),
            measurable: false,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class: Some(VarClass::State),
            init: Some(0.0),
            rate: Some("rate_CBuf".into()),
            prev: None,
         instance: None };
        let prev = Variable {
            var_type: VariableType::Intermediate,
            dtype: DataType::Float,
            unit: None,
            description: None,
            label: None, // 无 label → 应派生
            measurable: false,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class: Some(VarClass::SemiState),
            init: Some(0.0),
            rate: None,
            prev: Some("C_Buf".into()),
         instance: None };
        let mut variables = IndexMap::new();
        variables.insert("C_Buf".to_string(), state);
        variables.insert("C_Buf_prev".to_string(), prev);
        let file = EquationFile {
            meta: Metadata {
                id: "M".into(),
                model: "M".into(),
                name_cn: "".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(), balance: vec![],
            },
            parameters: Default::default(),
            variables,
            equations: vec![],
         structure: None };
        // prev 寄存器派生「源 label（上一步）」
        assert_eq!(file.display_name("C_Buf_prev"), "碳缓冲库（上一步）");
        // 源本身用自己的 label
        assert_eq!(file.display_name("C_Buf"), "碳缓冲库");
    }
}
