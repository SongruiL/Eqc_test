//! GP 候选**结构硬过滤**（进化图论 arc · Tier3）。
//!
//! 候选 patch 进模型后，用图论 / 守恒**机械**判定三条红线——「机械」= 纯结构/守恒判断、非 AI。
//! 过滤后的候选报告（含图论证据）再喂 Claude Code（开发态 agent）做多准则采纳判断。
//!
//! **三条红线**（任一命中 = 不通过硬过滤 `passes_hard_filters=false`）：
//! 1. **加代数环**：patched 比 baseline 多出代数环块（破求解结构）。
//! 2. **破守恒律**：模型声明了 `meta.balance`、且 patched 跑短仿真后某守恒律残差超容差、而 baseline 不超。
//! 3. **新令既有参数不可辨识/混淆**：patched 新引入的混淆对里，**至少一端是候选自身 `__c` 之外的既有参数**
//!    （候选把既有可辨识参数拖下水）。
//!
//! **★红线③收窄（首席拍板 2026-07-08）**：候选**自身系数簇**（新混淆对里两端全是 `__c` 常数）**不淘汰**——
//! 那是经验响应式的正常性质（本 arc 头号发现：「混淆团 = 经验式系数簇」）。改为 `coefficient_cluster` 报告，
//! 喂标定规划（这簇系数要一起标 / 加正交工况）+ Claude Code 判断。字面「任何新混淆对即淘汰」会误杀几乎
//! 所有 ≥2 常数候选、淘汰掉本系统本要进化的那类经验式 → 自杀。

use std::collections::BTreeSet;

use serde::Serialize;

use crate::graph::{analyze_identifiability, analyze_structure, diff_models, StructureReport};
use crate::schema::EquationFile;
use crate::sim::{check_balance_laws, simulate, SimInput};

use super::fitness::patch_model;
use super::grammar::Candidate;

/// 相对 baseline 的复杂度增量（正 = 候选更复杂）。用**折回字面值**的干净视图算，不含 `__c` 记账节点。
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityDelta {
    /// 节点数增量（added_nodes − removed_nodes）。
    pub nodes: i64,
    /// 边数增量（added_edges − removed_edges）。
    pub edges: i64,
}

/// 一条守恒律在 baseline vs patched 下的残差对比（仅模型声明了 `meta.balance` 时有）。
#[derive(Debug, Clone, Serialize)]
pub struct ConservationCheck {
    /// 守恒律名。
    pub name: String,
    /// baseline 是否守恒。
    pub baseline_ok: bool,
    /// patched 是否守恒。
    pub patched_ok: bool,
    /// baseline 最大残差（无法核算=None）。
    pub baseline_resid: Option<f64>,
    /// patched 最大残差（无法核算=None）。
    pub patched_resid: Option<f64>,
    /// 容差上限。
    pub tol: f64,
}

/// GP 候选的图论证据 + 硬过滤裁决（喂 `candidates.json` / serve `/api/evolve` / Claude Code）。
#[derive(Debug, Clone, Serialize)]
pub struct GraphEvidence {
    /// 三条红线全过？
    pub passes_hard_filters: bool,
    /// 红线①：新增代数环。
    pub adds_algebraic_loop: bool,
    /// 红线②：破守恒律（baseline 守、patched 不守）。无 `meta.balance` 恒 false。
    pub breaks_conservation: bool,
    /// patched 相对 baseline **新引入**的全部混淆对（本地名，透明起见全列，含系数簇）。
    pub new_confounded_pairs: Vec<[String; 2]>,
    /// 红线③命中的子集：新混淆对里至少一端是**既有参数**（候选拖既有可辨识参数下水）。
    pub disqualifying_confounded: Vec<[String; 2]>,
    /// 候选**自身系数簇**（新混淆对里两端全为 `__c` 常数的那些参数）——非淘汰，作标定信号报告。
    pub coefficient_cluster: Vec<String>,
    /// **长出的新边**（本地名对，如 `[d2, y]` = 候选新用了输入 d2）——本 arc 核心信号「看它长出什么」。
    pub added_edges: Vec<[String; 2]>,
    /// 删除的边（候选不再用某输入）。
    pub removed_edges: Vec<[String; 2]>,
    /// 形式改变的方程 output（受约束 GP 靶方程恒在此）。
    pub changed_equations: Vec<String>,
    /// 图编辑距离（增删点+边；机理视图，常数折回字面值）。
    pub distance: usize,
    /// 复杂度增量（node/edge，相对 baseline）。
    pub complexity_delta: ComplexityDelta,
    /// 逐守恒律残差对比（无 `meta.balance` 时空）。
    pub conservation: Vec<ConservationCheck>,
    /// 人读的淘汰原因（`passes_hard_filters=true` 时空）。
    pub reject_reasons: Vec<String>,
}

/// 节点 id（`MODULE.name`）→ 本地名（去模块前缀）。单模块版本链里本地名唯一。
fn local_name(node: &str) -> &str {
    node.rsplit('.').next().unwrap_or(node)
}

/// 是否候选自身注入的常数参数（`__c{i}`，见 [`Candidate::const_name`]）。双下划线 c + 数字是安全判别。
fn is_gp_const(node: &str) -> bool {
    let l = local_name(node);
    l.strip_prefix("__c")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
}

/// 混淆对 → 有序本地名对（集合差与自然排序用）。
fn pair_key(p: &(String, String)) -> [String; 2] {
    let a = local_name(&p.0).to_string();
    let b = local_name(&p.1).to_string();
    if a <= b {
        [a, b]
    } else {
        [b, a]
    }
}

/// 代数环签名集：各环的方程键排序后成一个签名，供 baseline vs patched 集合差。
fn loop_sigs(r: &StructureReport) -> BTreeSet<Vec<String>> {
    r.algebraic_loops()
        .iter()
        .map(|b| {
            let mut e = b.equations.clone();
            e.sort();
            e
        })
        .collect()
}

/// 清空 measurable → 统一 all-output 口径（与 `evolution.rs` 一致：混淆判定是纯结构、跨 baseline/patched 一致）。
fn clear_measurable(files: &mut [EquationFile]) {
    for f in files.iter_mut() {
        for (_n, v) in f.variables.iter_mut() {
            v.measurable = false;
        }
    }
}

/// 候选常数**代回字面值**得到干净表达式（不引入 `__c` 参数节点；供复杂度增量的机理视图）。
fn candidate_folded_expr(cand: &Candidate) -> crate::ast::Expr {
    let mut e = cand.expr.clone();
    for (i, v) in cand.consts.iter().enumerate() {
        e = e.substitute(&Candidate::const_name(i), &crate::ast::Expr::constant(*v));
    }
    e
}

/// **SSOT 硬过滤**：对一个 GP 候选算图论证据 + 三条红线裁决。CLI `eqc evolve` 与 serve `run_evolve_job` 共调。
///
/// - `baseline`：原模型（未 patch）。
/// - `target_id`：靶方程 id（候选替换它的 expression）。
/// - `cand`：GP 候选。
/// - `sim_input`：与 GP fitness 同一份仿真输入（守恒红线搭它的车跑短仿真）。
pub fn graph_evidence(
    baseline: &EquationFile,
    target_id: &str,
    cand: &Candidate,
    sim_input: &SimInput,
) -> GraphEvidence {
    // patch 失败（靶方程不存在）→ 直接不通过、给出原因。
    let patched = match patch_model(baseline, target_id, cand) {
        Some(m) => m,
        None => {
            return GraphEvidence {
                passes_hard_filters: false,
                adds_algebraic_loop: false,
                breaks_conservation: false,
                new_confounded_pairs: vec![],
                disqualifying_confounded: vec![],
                coefficient_cluster: vec![],
                added_edges: vec![],
                removed_edges: vec![],
                changed_equations: vec![],
                distance: 0,
                complexity_delta: ComplexityDelta { nodes: 0, edges: 0 },
                conservation: vec![],
                reject_reasons: vec![format!("靶方程 {target_id} 不存在，无法 patch")],
            };
        }
    };

    // —— 红线①：加代数环 —— patched 比 baseline 多出的代数环块。
    let base_loops = loop_sigs(&analyze_structure(std::slice::from_ref(baseline)));
    let patched_loops = loop_sigs(&analyze_structure(std::slice::from_ref(&patched)));
    let adds_algebraic_loop = patched_loops.difference(&base_loops).next().is_some();

    // —— 红线③：新混淆对 —— all-output 口径下 patched 相对 baseline 新引入的混淆对，按「是否拖既有参数」分流。
    let mut base_norm = vec![baseline.clone()];
    clear_measurable(&mut base_norm);
    let mut patched_norm = vec![patched.clone()];
    clear_measurable(&mut patched_norm);
    let base_conf: BTreeSet<[String; 2]> =
        analyze_identifiability(&base_norm).confounded_candidates.iter().map(pair_key).collect();
    let patched_id = analyze_identifiability(&patched_norm);
    let new_confounded_pairs: Vec<[String; 2]> = patched_id
        .confounded_candidates
        .iter()
        .map(pair_key)
        .filter(|p| !base_conf.contains(p))
        .collect();

    let mut disqualifying_confounded: Vec<[String; 2]> = Vec::new();
    let mut cluster: BTreeSet<String> = BTreeSet::new();
    for p in &new_confounded_pairs {
        if is_gp_const(&p[0]) && is_gp_const(&p[1]) {
            // 两端都是候选自身常数 → 系数簇（报告、非淘汰）。
            cluster.insert(p[0].clone());
            cluster.insert(p[1].clone());
        } else {
            // 至少一端是既有参数 → 候选拖既有可辨识参数下水（红线③命中）。
            disqualifying_confounded.push(p.clone());
        }
    }
    let coefficient_cluster: Vec<String> = cluster.into_iter().collect();

    // —— 红线②：破守恒律 —— 仅模型声明了 meta.balance 时；否则诚实跳过（无守恒律可核）。
    let mut conservation: Vec<ConservationCheck> = Vec::new();
    let mut breaks_conservation = false;
    if !baseline.meta.balance.is_empty() {
        let dt = sim_input.dt.unwrap_or(baseline.meta.dt);
        let base_out = simulate(baseline, sim_input).ok();
        let patched_out = simulate(&patched, sim_input).ok();
        if let (Some(bo), Some(po)) = (&base_out, &patched_out) {
            // §8：rate 变量经 variable.rate 权威解析（baseline/patched 状态-速率结构相同，用 baseline）。
            let bchecks = check_balance_laws(&baseline.meta.balance, bo, dt, baseline);
            let pchecks = check_balance_laws(&baseline.meta.balance, po, dt, baseline);
            for (bc, pc) in bchecks.iter().zip(pchecks.iter()) {
                // 破守恒 = baseline 守、patched 真核算出超容差（跳过/仿真失败不归咎候选）。
                let broke = bc.ok && !pc.ok && pc.residual.is_some();
                if broke {
                    breaks_conservation = true;
                }
                conservation.push(ConservationCheck {
                    name: bc.name.clone(),
                    baseline_ok: bc.ok,
                    patched_ok: pc.ok,
                    baseline_resid: bc.residual.map(|r| r.max_resid),
                    patched_resid: pc.residual.map(|r| r.max_resid),
                    tol: bc.tol,
                });
            }
        }
        // patched 仿真失败 → 无法核守恒；不归咎守恒红线（候选会在 fitness 侧记 WORST）。
    }

    // —— 结构变化 + 复杂度增量（机理视图：常数折回字面值，无 __c 记账）——
    let mut folded = baseline.clone();
    if let Some(eq) = folded.equations.iter_mut().find(|e| e.id == target_id) {
        eq.expression = candidate_folded_expr(cand);
    }
    let diff = diff_models(std::slice::from_ref(baseline), std::slice::from_ref(&folded));
    let to_pair = |e: &(String, String)| [e.0.clone(), e.1.clone()];
    let added_edges: Vec<[String; 2]> = diff.added_edges.iter().map(to_pair).collect();
    let removed_edges: Vec<[String; 2]> = diff.removed_edges.iter().map(to_pair).collect();
    let changed_equations: Vec<String> =
        diff.changed_equations.iter().map(|c| c.output.clone()).collect();
    let complexity_delta = ComplexityDelta {
        nodes: diff.added_nodes.len() as i64 - diff.removed_nodes.len() as i64,
        edges: diff.added_edges.len() as i64 - diff.removed_edges.len() as i64,
    };

    // —— 汇总裁决 ——
    let mut reject_reasons: Vec<String> = Vec::new();
    if adds_algebraic_loop {
        reject_reasons.push("引入新代数环（破求解结构）".to_string());
    }
    if breaks_conservation {
        let broken: Vec<&str> = conservation
            .iter()
            .filter(|c| c.baseline_ok && !c.patched_ok && c.patched_resid.is_some())
            .map(|c| c.name.as_str())
            .collect();
        reject_reasons.push(format!("破守恒律：{}", broken.join("、")));
    }
    if !disqualifying_confounded.is_empty() {
        reject_reasons.push(format!(
            "令既有参数不可辨识/混淆：{} 对",
            disqualifying_confounded.len()
        ));
    }
    let passes_hard_filters = reject_reasons.is_empty();

    GraphEvidence {
        passes_hard_filters,
        adds_algebraic_loop,
        breaks_conservation,
        new_confounded_pairs,
        disqualifying_confounded,
        coefficient_cluster,
        added_edges,
        removed_edges,
        changed_equations,
        distance: diff.distance,
        complexity_delta,
        conservation,
        reject_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gp_const_discriminator() {
        assert!(is_gp_const("__c0"));
        assert!(is_gp_const("M.__c12"));
        assert!(!is_gp_const("__cfoo")); // 非纯数字后缀
        assert!(!is_gp_const("k_DMC_EC")); // 真参数名
        assert!(!is_gp_const("M.EC_thresh"));
        assert!(!is_gp_const("__c")); // 空后缀
    }

    #[test]
    fn pair_key_is_ordered_and_localized() {
        assert_eq!(pair_key(&("M.b".into(), "M.a".into())), ["a".to_string(), "b".to_string()]);
        assert_eq!(pair_key(&("M.a".into(), "M.b".into())), ["a".to_string(), "b".to_string()]);
    }

    /// 最小双输入门控模型（gpdemo3 骨架）：baseline 只用 d1，靶方程 id=GATE。
    const DEMO: &str = r#"
meta: { id: DEMO, model: Demo, name_cn: 硬过滤测试, name_en: hf test }
variables:
  d1: { type: input,  class: driving,   unit: "-", label: d1, description: 输入1 }
  d2: { type: input,  class: driving,   unit: "-", label: d2, description: 输入2 }
  y:  { type: output, class: auxiliary, unit: "-", label: y,  description: 输出, measurable: true }
parameters:
  gate_a: { name_cn: 起点, default: 5.0, unit: "-", provenance: 猜测 }
  gate_b: { name_cn: 宽,   default: 3.0, unit: "-", provenance: 猜测 }
equations:
  - { id: GATE, name: 门, output: y, provenance: 猜测,
      expression: { op: div, args: [ { op: sub, args: [ { ref: d1 }, { ref: gate_a } ] }, { ref: gate_b } ] } }
"#;

    /// 候选新用输入 d2 + 两个常数 → 长出 [d2,y] 新边、系数簇报告、不淘汰、无守恒律（诚实跳过）。
    #[test]
    fn candidate_grows_edge_and_reports_cluster_not_disqualified() {
        let baseline = crate::parse_str(DEMO).expect("parse demo");
        // y = __c0·d2 + __c1·d1（用 d2=新边；__c0,__c1 同进靶方程=系数簇）。
        let expr = crate::ast::Expr::add(
            crate::ast::Expr::mul(crate::ast::Expr::param("__c0"), crate::ast::Expr::var("d2")),
            crate::ast::Expr::mul(crate::ast::Expr::param("__c1"), crate::ast::Expr::var("d1")),
        );
        let cand = Candidate { expr, consts: vec![0.5, 0.3] };
        let g = graph_evidence(&baseline, "GATE", &cand, &SimInput::new(5));

        assert!(g.passes_hard_filters, "合法候选不该被淘汰: {:?}", g.reject_reasons);
        assert!(!g.adds_algebraic_loop);
        assert!(!g.breaks_conservation);
        assert!(g.conservation.is_empty(), "无 meta.balance → 守恒诚实跳过");
        assert_eq!(g.added_edges, vec![["d2".to_string(), "y".to_string()]], "应长出 d2→y 新边");
        // __c0,__c1 同进靶方程 → 系数簇（报告），非淘汰。
        assert_eq!(g.coefficient_cluster, vec!["__c0".to_string(), "__c1".to_string()]);
        assert!(g.disqualifying_confounded.is_empty(), "候选自身系数簇不该判红线③");
    }

    /// 引入代数环的候选 → 红线①触发、被淘汰。模型：Y 输出 y，Z 输出 z=y+1（z 依赖 y）；
    /// 候选让 Y 引用 z → y↔z 互依 = 代数环。
    #[test]
    fn candidate_adding_loop_is_rejected() {
        const LOOP_MODEL: &str = r#"
meta: { id: LOOP, model: Loop, name_cn: 环测试, name_en: loop test }
variables:
  d: { type: input,  class: driving,   unit: "-", label: d, description: 驱动 }
  y: { type: output, class: auxiliary, unit: "-", label: y, description: 输出y, measurable: true }
  z: { type: output, class: auxiliary, unit: "-", label: z, description: 输出z, measurable: true }
parameters:
  k: { name_cn: k, default: 1.0, unit: "-", provenance: 猜测 }
equations:
  - { id: Y, name: Y方程, output: y, provenance: 猜测,
      expression: { op: mul, args: [ { ref: k }, { ref: d } ] } }
  - { id: Z, name: Z方程, output: z, provenance: 猜测,
      expression: { op: add, args: [ { ref: y }, { const: 1 } ] } }
"#;
        let baseline = crate::parse_str(LOOP_MODEL).expect("parse loop model");
        // 候选让 Y = __c0·z → y 依赖 z、z 依赖 y = 代数环。
        let cand = Candidate {
            expr: crate::ast::Expr::mul(
                crate::ast::Expr::param("__c0"),
                crate::ast::Expr::var("z"),
            ),
            consts: vec![1.0],
        };
        let g = graph_evidence(&baseline, "Y", &cand, &SimInput::new(5));
        assert!(g.adds_algebraic_loop, "候选引入 y↔z 环，应检出代数环");
        assert!(!g.passes_hard_filters, "加代数环的候选应被硬过滤淘汰");
        assert!(g.reject_reasons.iter().any(|r| r.contains("代数环")));
    }

    /// 靶方程不存在 → 优雅不通过、给原因（不 panic）。
    #[test]
    fn missing_target_fails_gracefully() {
        let baseline = crate::parse_str(DEMO).expect("parse demo");
        let cand = Candidate { expr: crate::ast::Expr::var("d1"), consts: vec![] };
        let g = graph_evidence(&baseline, "NO_SUCH_EQ", &cand, &SimInput::new(5));
        assert!(!g.passes_hard_filters);
        assert!(!g.reject_reasons.is_empty());
    }
}
