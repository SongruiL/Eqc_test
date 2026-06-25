//! 模型 JSON 契约：把 [`EquationFile`] 导出为一份**稳定、可检视、只增不改**的 JSON。
//!
//! 这是「EQC ↔ 前端」之间唯一的结构化契约（见路线图「交互式前端」段）。原则：
//! - **EQC（Rust）独家拥有并生成这份契约**，前端只消费，不重新实现 EQC 的逻辑。
//! - **只增不改**：新功能加新的可选字段；老前端不读它也照常工作 → 同步低风险、可增量。
//! - **可检视**：`eqc export <模型> -o model.json` 随时能看「什么东西过了边界」；
//!   配合快照测试，结构一变即在 diff 里可见。
//!
//! `schema_version` 用于标记契约版本；破坏性变更才 +1（只加字段不算）。

use serde::Serialize;

use crate::schema::EquationFile;

/// 契约版本。仅当发生**破坏性**变更（删/改字段语义）时 +1；新增可选字段不动它。
pub const SCHEMA_VERSION: u32 = 1;

/// 顶层契约：一个或多个模块。
#[derive(Debug, Clone, Serialize)]
pub struct ModelJson {
    pub schema_version: u32,
    pub modules: Vec<ModuleJson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleJson {
    pub id: String,
    pub model: String,
    pub name_cn: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// 标定状态（看懂输出的可信度徽章）；未声明则省略，前端视为未标定。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<crate::schema::Calibration>,
    pub parameters: Vec<ParamJson>,
    pub variables: Vec<VarJson>,
    pub equations: Vec<EqJson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamJson {
    pub name: String,
    pub name_cn: String,
    /// 友好显示名（参数即 `name_cn`，缺省兜底代号）。结构图/图表/勾选框统一显示用，
    /// 代号 `name` 进 hover。由 [`crate::schema::EquationFile::display_name`] 计算（单一权威）。
    pub display_name: String,
    pub default: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// 向量参数（cohort 种子）的各分量值；标量参数为 `None`。
    /// 前端据此区分：向量参数不可被标量覆盖（情景面板里跳过）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f64>>,
    /// 是否为管理输入（逐处理区可设；园区「本区管理」编辑器据此列出）。false 省略。
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub management: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VarJson {
    pub name: String,
    /// 友好显示名：变量 label → 方程中文名 → 参数中文名 → 代号（兜底）。结构图/图表/勾选框
    /// 统一显示用，代号 `name` 进 hover。由 [`crate::schema::EquationFile::display_name`]
    /// 计算（与 DAG 节点标签同一权威逻辑）。
    pub display_name: String,
    /// 数据流角色：input / intermediate / output。
    pub var_type: String,
    /// Forrester 动力学分类（state/rate/driving/auxiliary/parameter/control/semi_state/boundary）。
    pub class: String,
    /// 是否跨步变量（积分状态量 / 延迟寄存器）。
    pub dynamic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 大白话短名（园区/简明视图显示用）；缺省时前端回退 description→name。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 是否可田间测量（录入网格列、标定观测对象）；false 时省略以保持契约干净。
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub measurable: bool,
    /// 胁迫/健康信号（"factor" 1=好 / "risk" 0=好）；前端据此画红绿灯。非信号则省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stress_factor: Option<String>,
    /// 红绿灯取整季哪个值（"min"/"max"/"final"）；缺省由 kind 推断（factor→min/risk→max）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stress_reduce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EqJson {
    pub id: String,
    pub name: String,
    pub output: String,
    /// 该方程的 MathML（含外层 `<math>`），前端可直接显示。
    pub mathml: String,
    /// 表达式引用到的名字（变量 + 参数）。
    pub refs: Vec<String>,
    /// 来源/参考文献（公式出处；多来源模型每条公式应标注）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// 可读公式（仅供展示）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula_display: Option<String>,
    /// GP 进化靶点标记（受约束 GP；缺省=机理基座冻结，则不输出此字段）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gp_target: Option<crate::schema::GpTarget>,
}

/// 把一组方程文件导出为契约 JSON 结构。
pub fn to_model_json(files: &[EquationFile]) -> ModelJson {
    let modules = files.iter().map(module_json).collect();
    ModelJson { schema_version: SCHEMA_VERSION, modules }
}

fn module_json(f: &EquationFile) -> ModuleJson {
    let parameters = f
        .parameters
        .iter()
        .map(|(name, p)| ParamJson {
            name: name.clone(),
            name_cn: p.name_cn.clone(),
            display_name: f.display_name(name),
            default: p.default,
            unit: p.unit.clone(),
            values: p.values.clone(),
            management: p.management,
        })
        .collect();

    let variables = f
        .variables
        .iter()
        .map(|(name, v)| {
            let var_type = match v.var_type {
                crate::schema::VariableType::Input => "input",
                crate::schema::VariableType::Intermediate => "intermediate",
                crate::schema::VariableType::Output => "output",
            }
            .to_string();
            VarJson {
                name: name.clone(),
                display_name: f.display_name(name),
                var_type,
                class: v.effective_class().as_str().to_string(),
                dynamic: v.is_dynamic(),
                unit: v.unit.clone(),
                description: v.description.clone(),
                label: v.label.clone(),
                measurable: v.measurable,
                stress_factor: v.stress_factor.clone(),
                stress_reduce: v.stress_reduce.clone(),
                init: v.init,
                rate: v.rate.clone(),
                prev: v.prev.clone(),
            }
        })
        .collect();

    let equations = f
        .equations
        .iter()
        .map(|e| {
            let mut refs = e.expression.get_variable_refs();
            refs.extend(e.expression.get_parameter_refs());
            EqJson {
                id: e.id.clone(),
                name: e.name.clone(),
                output: e.output.clone(),
                mathml: crate::report::expr_mathml(&e.expression),
                refs,
                reference: e.reference.clone(),
                formula_display: e.formula_display.clone(),
                gp_target: e.gp_target.clone(),
            }
        })
        .collect();

    ModuleJson {
        id: f.meta.id.clone(),
        model: f.meta.model.clone(),
        name_cn: f.meta.name_cn.clone(),
        name_en: f.meta.name_en.clone(),
        description: f.meta.description.clone(),
        reference: f.meta.reference.clone(),
        calibration: f.meta.calibration.clone(),
        parameters,
        variables,
        equations,
    }
}

/// 序列化为紧凑 JSON 字符串。
pub fn to_json_string(files: &[EquationFile]) -> String {
    serde_json::to_string(&to_model_json(files)).unwrap_or_else(|_| "{}".to_string())
}

// ============================================
// 结构分析契约（GA-1）：独立的 `eqc structure --json` 输出。
// additive：`schema_version` 不动；前端可据此画「求解顺序 / 代数环 / 过欠定」高亮。
// 暂不嵌入 `ModelJson`（待契约稳定后再加可选字段），保持本轮最小面。
// ============================================

/// 一个求解块（块下三角顺序里的一格）。
#[derive(Debug, Clone, Serialize)]
pub struct SolveBlockJson {
    /// 本块方程键（`MODULE::eq_id`）。
    pub equations: Vec<String>,
    /// 本块解出的变量节点 id。
    pub variables: Vec<String>,
    /// 是否代数环（须联立求解；本 arc 只定位不求解）。false 省略。
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub algebraic_loop: bool,
}

/// 单参数可达性（GA-2）。
#[derive(Debug, Clone, Serialize)]
pub struct ParamReachJson {
    pub param: String,
    /// 可达的可测节点。
    pub reaches: Vec<String>,
    pub identifiable: bool,
}

/// 结构可辨识性（GA-2）的 JSON 契约。
#[derive(Debug, Clone, Serialize)]
pub struct IdentifiabilityJson {
    /// 实际采用的可测变量节点。
    pub measurable: Vec<String>,
    /// 不可辨识参数（到任何可测都无路径）。
    pub unidentifiable: Vec<String>,
    /// 混淆候选参数对（结构无法区分；necessary-not-sufficient，喂数值版确认）。空则省略。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub confounded_candidates: Vec<[String; 2]>,
    /// 每参数可达性明细。
    pub params: Vec<ParamReachJson>,
}

/// 模型结构分析的 JSON 契约。
#[derive(Debug, Clone, Serialize)]
pub struct StructureJson {
    pub schema_version: u32,
    /// 自由变量（欠定块）= 参数 + 驱动量 + 无方程状态量。
    pub free_vars: Vec<String>,
    /// 方定块，已按块下三角求解顺序排列。
    pub solve_blocks: Vec<SolveBlockJson>,
    /// 超定方程键（多条方程写同一 output）。空则省略。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub over_determined: Vec<String>,
    /// 结构是否奇异（最大匹配 < 方程数）。
    pub structurally_singular: bool,
    /// 作者 `output:` 是否本身是完美匹配。
    pub author_matching_perfect: bool,
    /// 最大匹配是否唯一（best-effort；未判定则省略）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_unique: Option<bool>,
    /// 结构可辨识性（GA-2，可选；仅 `--identifiability` 时计算并附上）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiability: Option<IdentifiabilityJson>,
}

/// 把可辨识性报告导出为契约结构。
pub fn to_identifiability_json(r: &crate::graph::IdentifiabilityReport) -> IdentifiabilityJson {
    IdentifiabilityJson {
        measurable: r.measurable.clone(),
        unidentifiable: r.unidentifiable.clone(),
        confounded_candidates: r
            .confounded_candidates
            .iter()
            .map(|(a, b)| [a.clone(), b.clone()])
            .collect(),
        params: r
            .params
            .iter()
            .map(|p| ParamReachJson {
                param: p.param.clone(),
                reaches: p.reaches.clone(),
                identifiable: p.identifiable,
            })
            .collect(),
    }
}

/// 把结构报告（+可选可辨识性）导出为契约 JSON 结构。
pub fn to_structure_json(
    report: &crate::graph::StructureReport,
    ident: Option<&crate::graph::IdentifiabilityReport>,
) -> StructureJson {
    StructureJson {
        schema_version: SCHEMA_VERSION,
        free_vars: report.free_vars.clone(),
        solve_blocks: report
            .solve_blocks
            .iter()
            .map(|b| SolveBlockJson {
                equations: b.equations.clone(),
                variables: b.variables.clone(),
                algebraic_loop: b.is_algebraic_loop,
            })
            .collect(),
        over_determined: report.over_determined.clone(),
        structurally_singular: report.structurally_singular,
        author_matching_perfect: report.matching.author_is_perfect,
        matching_unique: report.matching.unique,
        identifiability: ident.map(to_identifiability_json),
    }
}

/// 结构分析 JSON（带缩进，`eqc structure --json` 用）。
pub fn structure_json_pretty(
    report: &crate::graph::StructureReport,
    ident: Option<&crate::graph::IdentifiabilityReport>,
) -> String {
    serde_json::to_string_pretty(&to_structure_json(report, ident))
        .unwrap_or_else(|_| "{}".to_string())
}

/// 序列化为带缩进的 JSON（`eqc export` 用，便于人读）。
pub fn to_json_pretty(files: &[EquationFile]) -> String {
    serde_json::to_string_pretty(&to_model_json(files)).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{DataType, Equation, Metadata, Variable, VariableType};
    use indexmap::IndexMap;

    fn dyn_var(rate: &str) -> Variable {
        Variable {
            var_type: VariableType::Output,
            dtype: DataType::Float,
            unit: Some("g/m2".into()),
            description: Some("累积干物质".into()),
            label: Some("总干物质".into()),
            measurable: true,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class: Some(crate::schema::VarClass::State),
            init: Some(19.9),
            rate: Some(rate.into()),
            prev: None,
        }
    }

    #[test]
    fn test_model_json_contract() {
        let mut variables = IndexMap::new();
        variables.insert("TDM".to_string(), dyn_var("DDM"));
        let file = EquationFile {
            meta: Metadata {
                id: "M".into(),
                model: "Demo".into(),
                name_cn: "演示".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(),
            },
            parameters: Default::default(),
            variables,
            equations: vec![Equation {
                id: "E1".into(),
                name: "干物质".into(),
                output: "DDM".into(),
                expression: crate::ast::Expr::mul(crate::ast::Expr::var("I"), crate::ast::Expr::var("LUE")),
                formula_display: None,
                reference: None, gp_target: None,
            }],
        };
        let files = vec![file];
        let m = to_model_json(&files);
        assert_eq!(m.schema_version, SCHEMA_VERSION);
        let v = &m.modules[0].variables[0];
        assert_eq!(v.name, "TDM");
        assert_eq!(v.class, "state");
        assert!(v.dynamic);
        assert_eq!(v.rate.as_deref(), Some("DDM"));
        assert_eq!(v.label.as_deref(), Some("总干物质"));
        assert_eq!(v.display_name, "总干物质"); // 友好名 = 变量 label（优先级最高）
        assert!(v.measurable);

        let eq = &m.modules[0].equations[0];
        assert!(eq.mathml.contains("<math"));
        assert!(eq.refs.contains(&"I".to_string()));

        // JSON 可序列化、含关键键
        let js = to_json_string(&files);
        assert!(js.contains("\"schema_version\""));
        assert!(js.contains("\"class\":\"state\""));
        assert!(js.contains("\"label\":\"总干物质\""));
        assert!(js.contains("\"display_name\":\"总干物质\""));
        assert!(js.contains("\"measurable\":true"));
    }

    /// G0：gp_target 进化靶点标记 —— 出现则导出、缺省则省略（additive 契约）。
    #[test]
    fn test_gp_target_contract() {
        use crate::ast::Expr;
        use crate::schema::GpTarget;
        let mut monotone = IndexMap::new();
        monotone.insert("ChillAccum".to_string(), "increasing".to_string());
        let tagged = Equation {
            id: "BB5-DORM".into(),
            name: "休眠解除门控".into(),
            output: "dormancy_released".into(),
            expression: Expr::var("ChillAccum"),
            formula_display: None,
            reference: None,
            gp_target: Some(GpTarget {
                grammar: "monotone_gate".into(),
                inputs: vec!["ChillAccum".into(), "GDD".into()],
                output_bounds: Some([0.0, 1.0]),
                monotone,
                frozen: false,
            }),
        };
        let plain = Equation {
            id: "BB5-LAI".into(),
            name: "叶面积".into(),
            output: "LAI".into(),
            expression: Expr::var("W_leaf"),
            formula_display: None,
            reference: None,
            gp_target: None,
        };
        let mut variables = IndexMap::new();
        variables.insert("dormancy_released".to_string(), dyn_var("r1"));
        let file = EquationFile {
            meta: Metadata {
                id: "M".into(), model: "Demo".into(), name_cn: "演示".into(),
                name_en: None, version: "1.0".into(), description: None, reference: None,
                source_files: vec![], dt: 1.0, dt_seconds: None, calibration: None,
                modules: Default::default(),
            },
            parameters: Default::default(),
            variables,
            equations: vec![tagged, plain],
        };
        let files = vec![file];
        let m = to_model_json(&files);
        // 标记方程：契约里带 gp_target
        let gt = m.modules[0].equations[0].gp_target.as_ref().expect("tagged eq has gp_target");
        assert_eq!(gt.grammar, "monotone_gate");
        assert_eq!(gt.inputs, vec!["ChillAccum", "GDD"]);
        assert_eq!(gt.output_bounds, Some([0.0, 1.0]));
        assert!(!gt.frozen);
        // 未标记方程：契约里无 gp_target（缺省冻结）
        assert!(m.modules[0].equations[1].gp_target.is_none());
        // JSON：标记方程出现键，整体只一处 gp_target（plain 省略）
        let js = to_json_string(&files);
        assert!(js.contains("\"gp_target\""));
        assert!(js.contains("\"grammar\":\"monotone_gate\""));
        assert_eq!(js.matches("\"gp_target\"").count(), 1);
    }

    /// G0：YAML 反序列化 gp_target（模型在 .eq.yaml 里声明的路径）。
    #[test]
    fn test_gp_target_yaml_roundtrip() {
        use crate::schema::GpTarget;
        let y = r#"
grammar: monotone_gate
inputs: [ChillAccum, GDD]
output_bounds: [0.0, 1.0]
monotone: { ChillAccum: increasing }
"#;
        let gt: GpTarget = serde_yaml::from_str(y).expect("parse gp_target");
        assert_eq!(gt.grammar, "monotone_gate");
        assert_eq!(gt.inputs.len(), 2);
        assert_eq!(gt.monotone.get("ChillAccum").map(String::as_str), Some("increasing"));
        assert!(!gt.frozen); // 默认可进化
        // 最小声明：只给 grammar，其余默认
        let gt2: GpTarget = serde_yaml::from_str("grammar: allocation_fraction\n").unwrap();
        assert!(gt2.inputs.is_empty() && gt2.output_bounds.is_none() && !gt2.frozen);
    }
}
