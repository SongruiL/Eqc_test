//! GP G5：进化式回流理论溯源体系。
//!
//! 设计（docs/spec-genetic-programming.md §12）：GP 输出回流进 `crop-models/理论溯源/`——
//! 识别 GP 选了哪种**机理形式**（语法 form），与模型现有形式比较：
//! - **撞上现有形式 = rediscovery（验证）** → 建议升 🟠→🟢/🔵（"GP 从数据独立复原现有形式=机理验证"）；
//! - **新形式 = 待证伪假设** → 🟠 + GP 来源 + 拟合优度。
//! 并自动生成一段**溯源条目草稿**（公式 + 分类建议 + why/risk 骨架 + 拟合徽章），首席科学家复核。
//!
//! 关键：GP 候选只来自语法（form 集合），故"它是哪种机理形式"可由骨架结构匹配判定。

use crate::ast::Expr;
use crate::optimize::de::Rng;

use super::grammar::{effective_form_count, form_name, sample_form, Candidate, GpContext};
use super::operators::same_skeleton;

/// 识别一个候选属于语法的哪种 form（按骨架结构匹配各 form 的标准骨架）。
/// 返回 form idx；不匹配任何 form（如被 input-swap 成自定义结构）→ None。
pub fn identify_form(cand: &Candidate, grammar: &str, ctx: &GpContext) -> Option<usize> {
    let n = effective_form_count(grammar, ctx);
    for idx in 0..n {
        // 标准骨架与常数值无关（可调常数是 __c 占位）→ 用任意种子采样
        let mut rng = Rng::new(0);
        if let Some(canon) = sample_form(grammar, idx, ctx, &mut rng) {
            if same_skeleton(&cand.expr, &canon.expr) {
                return Some(idx);
            }
        }
    }
    None
}

/// 把候选渲染成可读公式（__c{i} 代回常数值，输出 Python 风格中缀）。
pub fn render_formula(cand: &Candidate) -> String {
    let mut shown = cand.expr.clone();
    for (i, v) in cand.consts.iter().enumerate() {
        shown = shown.substitute(&Candidate::const_name(i), &Expr::constant(*v));
    }
    shown.to_python("")
}

/// GP 进化结果的溯源报告。
#[derive(Debug, Clone)]
pub struct ProvenanceReport {
    /// 识别出的机理形式名（None = 自定义/不在语法 form 集合）。
    pub form: Option<String>,
    pub formula: String,
    pub error: f64,
    pub complexity: usize,
    /// 是否撞上 baseline 形式（rediscovery）。baseline 缺省时为 false。
    pub rediscovery: bool,
    /// 分类建议：rediscovery→"🟢/🔵 机理验证"；新形式→"🟠 待证伪假设"。
    pub suggestion: String,
}

/// 生成溯源报告。`baseline_form`：模型现有形式名（如已知），用于判 rediscovery。
pub fn form_report(
    cand: &Candidate,
    error: f64,
    complexity: usize,
    grammar: &str,
    ctx: &GpContext,
    baseline_form: Option<&str>,
) -> ProvenanceReport {
    let form_idx = identify_form(cand, grammar, ctx);
    let form = form_idx.map(|i| form_name(grammar, i).to_string());
    let rediscovery = match (&form, baseline_form) {
        (Some(f), Some(b)) => f == b,
        _ => false,
    };
    let suggestion = if rediscovery {
        "🟢/🔵 rediscovery：GP 从数据独立复原现有形式 = 机理验证；建议升 🟠→🟢(若通用定律)/🔵(若平移)".to_string()
    } else if form.is_some() {
        "🟠 新形式假设：GP 提出语法内的另一种机理形式，待田间证伪；保 🟠 + 标 GP 来源 + 拟合优度".to_string()
    } else {
        "🟠 自定义结构：不在标准 form 集合（可能经 input-swap），需人工审视机理合理性".to_string()
    };
    ProvenanceReport {
        form,
        formula: render_formula(cand),
        error,
        complexity,
        rediscovery,
        suggestion,
    }
}

/// 自动生成一段**溯源条目草稿**（markdown，照 理论溯源 §7 模板），供首席科学家复核后贴入。
pub fn provenance_stub(report: &ProvenanceReport, target_id: &str, output: &str, grammar: &str) -> String {
    let form = report.form.as_deref().unwrap_or("(自定义结构)");
    let cls = if report.rediscovery {
        "🟢/🔵（rediscovery，待确认通用定律 vs 平移）"
    } else {
        "🟠（GP 进化假设）"
    };
    format!(
        "### [{target_id}·GP] {output} —— GP 进化候选（语法 {grammar} / 形式 {form}）\n\
         - 公式（常数已标定）：`{formula}`\n\
         - 机理形式：**{form}**（语法 {grammar}）{redisc}\n\
         - 分类（建议）：{cls}\n\
         - 拟合徽章：rmse={error:.4} · 复杂度(节点)={complexity}\n\
         - 来源核实：**GP 受约束进化产出**（只在 🟠 假设留白处进化、冻结 🟢/🔵 机理基座；语法保先验：单调/有界/量纲）。\n\
         - 分类建议依据：{suggestion}\n\
         - 简化理由（why）：（GP 在该靶点的候选形式族内择优；待首席科学家补「为何此形式合理」）\n\
         - 风险/影响（risk）：（GP 由有限观测拟合得；过拟合风险靠 Pareto 复杂度轴 + 留出验证控制；待补）\n\
         - 田间验证钩子：用独立田间数据检验该形式外推；与现有形式对比 ΔAIC/留出误差。\n\
         - 状态：⚠️ GP 草稿，待首席科学家复核确认分类（rediscovery 升级 or 新假设保 🟠）。\n",
        target_id = target_id,
        output = output,
        grammar = grammar,
        form = form,
        formula = report.formula,
        redisc = if report.rediscovery { "（= 模型现有形式，rediscovery）" } else { "" },
        cls = cls,
        error = report.error,
        complexity = report.complexity,
        suggestion = report.suggestion,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gp::grammar::sample_form;
    use indexmap::IndexMap;

    fn gate_ctx() -> GpContext {
        let mut m = IndexMap::new();
        m.insert("ChillAccum".to_string(), "increasing".to_string());
        GpContext {
            inputs: vec!["ChillAccum".to_string(), "GDD".to_string()],
            output_bounds: Some([0.0, 1.0]),
            monotone: m,
        }
    }

    /// 识别：采样某 form 的候选 → identify_form 应识别回同一 form。
    #[test]
    fn test_identify_roundtrip() {
        let ctx = gate_ctx();
        for idx in 0..effective_form_count("monotone_gate", &ctx) {
            let mut rng = Rng::new(100 + idx as u64);
            let cand = sample_form("monotone_gate", idx, &ctx, &mut rng).unwrap();
            assert_eq!(
                identify_form(&cand, "monotone_gate", &ctx),
                Some(idx),
                "form {idx} ({}) 应被识别回",
                form_name("monotone_gate", idx)
            );
        }
    }

    /// rediscovery：识别出的形式 == baseline → rediscovery=true，建议升级。
    #[test]
    fn test_rediscovery_classification() {
        let ctx = gate_ctx();
        let mut rng = Rng::new(7);
        // 采样 form 0 = linear_ramp（蓝莓 BB5-DORM 现式正是它）
        let cand = sample_form("monotone_gate", 0, &ctx, &mut rng).unwrap();
        let rep = form_report(&cand, 0.01, 8, "monotone_gate", &ctx, Some("linear_ramp"));
        assert_eq!(rep.form.as_deref(), Some("linear_ramp"));
        assert!(rep.rediscovery, "撞上 baseline 应 rediscovery");
        assert!(rep.suggestion.contains("rediscovery"));
        // baseline 不同 → 新形式假设
        let cand2 = sample_form("monotone_gate", 1, &ctx, &mut rng).unwrap();
        let rep2 = form_report(&cand2, 0.02, 9, "monotone_gate", &ctx, Some("linear_ramp"));
        assert_eq!(rep2.form.as_deref(), Some("sigmoid"));
        assert!(!rep2.rediscovery);
        assert!(rep2.suggestion.contains("🟠"));
    }

    /// 溯源草稿包含关键字段。
    #[test]
    fn test_stub_contents() {
        let ctx = gate_ctx();
        let mut rng = Rng::new(1);
        let cand = sample_form("monotone_gate", 1, &ctx, &mut rng).unwrap();
        let rep = form_report(&cand, 0.015, 9, "monotone_gate", &ctx, Some("linear_ramp"));
        let stub = provenance_stub(&rep, "BB5-DORM", "dormancy_released", "monotone_gate");
        assert!(stub.contains("BB5-DORM"));
        assert!(stub.contains("sigmoid"));
        assert!(stub.contains("rmse=0.0150"));
        assert!(stub.contains("GP 受约束进化"));
        assert!(stub.contains("⚠️ GP 草稿"));
    }
}
