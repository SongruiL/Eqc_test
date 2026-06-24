//! 受约束遗传编程（Constrained GP）—— 在机理骨架的「假设留白（🟠）」处进化方程结构。
//!
//! 设计见 `docs/spec-genetic-programming.md`。进化-冻结边界来自理论溯源逐方程分类
//! （🟢有依据/🔵平移=冻结，🟠假设=进化），在模型里以 [`crate::schema::GpTarget`] 标记。
//!
//! - **G0**（已建）：`gp_target` 元数据 + 契约导出（见 `schema::Equation`/`export`）。
//! - **G1**（本模块）：语法（候选形式族）+ 类型/量纲/先验约束。
//!   - [`grammar`]：5 套通用语法 + [`sample`] 采样合法候选 `Expr`。
//!   - [`constraints`]：量纲软过滤 + 单调/有界数值先验检查。
//! - G2 树遗传算子 / G3 主循环+适应度 / G4 Pareto+memetic / G5 多槽位+溯源回流（后续）。

pub mod constraints;
pub mod evolve;
pub mod fitness;
pub mod grammar;
pub mod joint;
pub mod operators;
pub mod pareto;
pub mod provenance;

pub use constraints::{bounds_ok, check_candidate, eval_candidate, monotone_ok, units_ok, CandidateCheck};
pub use evolve::{evolve, EvolveConfig, EvolveResult};
pub use fitness::{context_from_target, evaluate_in_model, patch_model, Observed};
pub use grammar::{
    effective_form_count, form_count, form_name, sample, sample_form, Candidate, GpContext,
    KNOWN_GRAMMARS,
};
pub use joint::{
    evaluate_multi, evolve_joint, evolve_joint_pareto, patch_multi, slots_from_model, JointConfig,
    JointParetoEntry, JointResult, Slot,
};
pub use operators::{complexity, crossover, mutate, perturb_constants};
pub use pareto::{calibrate_consts, evolve_pareto, ParetoConfig, ParetoEntry};
pub use provenance::{
    form_report, identify_form, identify_form_of_expr, provenance_stub, render_formula,
    ProvenanceReport,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::de::Rng;
    use indexmap::IndexMap;
    use std::collections::HashMap;

    fn ctx(inputs: &[&str], bounds: Option<[f64; 2]>, mono: &[(&str, &str)]) -> GpContext {
        let mut m = IndexMap::new();
        for (v, d) in mono {
            m.insert(v.to_string(), d.to_string());
        }
        GpContext {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            output_bounds: bounds,
            monotone: m,
        }
    }

    /// 每套语法采样多次，候选全部满足量纲+单调+有界（按构造保证的回归网）。
    #[test]
    fn test_all_grammars_sample_valid() {
        let env: HashMap<String, crate::units::Dimension> = HashMap::new();
        let cases = [
            ("monotone_gate", ctx(&["ChillAccum", "GDD"], Some([0.0, 1.0]), &[("ChillAccum", "increasing")]), 50.0),
            ("saturating_sink", ctx(&["LAI", "LAI_pot"], Some([0.0, 1.0]), &[("LAI", "decreasing")]), 5.0),
            ("allocation_fraction", ctx(&["NNI", "VS"], Some([0.0, 1.0]), &[]), 2.0),
            ("temperature_response", ctx(&["T"], Some([0.0, 1.0]), &[]), 40.0),
            ("growth_curve", ctx(&["tau_fruit"], None, &[("tau_fruit", "increasing")]), 1000.0),
        ];
        for (g, c, sweep_hi) in &cases {
            let mut rng = Rng::new(42);
            for k in 0..40 {
                let e = sample(g, c, &mut rng).unwrap_or_else(|| panic!("{g} sample"));
                let chk = check_candidate(&e, c, &env, *sweep_hi);
                assert!(
                    chk.all_ok(),
                    "grammar {g} 第{k}个候选不合法: {:?}\n{:?}",
                    chk, e
                );
            }
        }
    }

    /// 确定性：同种子 → 同候选序列（GP 可复现）。
    #[test]
    fn test_sampling_deterministic() {
        let c = ctx(&["ChillAccum", "GDD"], Some([0.0, 1.0]), &[("ChillAccum", "increasing")]);
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..20 {
            let ea = sample("monotone_gate", &c, &mut a).unwrap();
            let eb = sample("monotone_gate", &c, &mut b).unwrap();
            assert_eq!(format!("{ea:?}"), format!("{eb:?}"));
        }
    }

    /// 未知语法 → None；form_count 与采样分支一致。
    #[test]
    fn test_unknown_grammar_and_counts() {
        let c = ctx(&["x"], None, &[]);
        let mut rng = Rng::new(1);
        assert!(sample("not_a_grammar", &c, &mut rng).is_none());
        for g in KNOWN_GRAMMARS {
            assert!(form_count(g) >= 2, "{g} 应有候选形式");
        }
    }

    /// 约束能 catch 违规候选（非仅对合法的放行）。
    #[test]
    fn test_constraints_catch_violations() {
        use crate::ast::Expr;
        // 单调：2·x 对 x 是升的 → 若期望"降"应判失败（consts 空）
        let rising = Expr::mul(Expr::constant(2.0), Expr::var("x"));
        assert!(monotone_ok(&rising, &[], "x", "increasing", &[], (0.0, 10.0), 11));
        assert!(!monotone_ok(&rising, &[], "x", "decreasing", &[], (0.0, 10.0), 11));
        // 有界：2·x 在 [0,1] 外 → 越界
        assert!(!bounds_ok(&rising, &[], &["x".to_string()], [0.0, 1.0], 10.0));
        // 量纲软过滤：两个已知物理量纲相加不兼容 → 拒；与无量纲常数 → 放行
        use crate::units::parse_dimension;
        let mut uenv = HashMap::new();
        uenv.insert("Tair".to_string(), parse_dimension("degC").unwrap());
        uenv.insert("P".to_string(), parse_dimension("Pa").unwrap());
        // Tair[degC] − const → 软过滤放行（const 无量纲通配）
        let t_minus_c = Expr::sub(Expr::var("Tair"), Expr::constant(5.0));
        assert!(units_ok(&t_minus_c, &uenv), "T−常数应放行");
        // Tair[degC] + P[Pa] → 两已知物理量纲不兼容 → 拒
        let t_plus_p = Expr::add(Expr::var("Tair"), Expr::var("P"));
        assert!(!units_ok(&t_plus_p, &uenv), "degC+Pa 应被拒");
    }
}
