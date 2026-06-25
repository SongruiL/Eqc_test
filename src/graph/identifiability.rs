//! 结构可辨识性（图论必要条件版）—— 互补数值 `eqc identify`。
//!
//! **问题**：给定哪些变量可测（`measurable`），某参数**在结构上**能否从这些观测被唯一确定
//! （与具体数据无关）？图论给两个**便宜的必要条件筛子**（理论笔记 §2.4）：
//!
//! 1. **不可辨识（可达性）**：参数 p 在有向影响图上**到任何可测变量都无路径** ⇒ 结构不可辨识
//!    （数据再多也定不出）。necessary：不可达 ⇒ 一定不可辨识。
//! 2. **混淆候选**：两参数进入**完全相同的方程集合** ⇒ 下游影响路径集必然相同 ⇒ 无观测能区分
//!    （只能定其组合）。necessary-not-sufficient：是喂数值版确认的高置信候选。
//!
//! ⚠️ 严谨边界：图法只给**必要条件**（可达性、混淆候选），不给充分判定；**完整**判定用微分代数
//! （Lie 导数、特征集，见 SIAN / StructuralIdentifiability.jl）。定位为快速筛 + 可视化。
//!
//! **有向影响图**（[`super::digraph::DiGraph`]，含 `rate→state`/`prev→semistate` 积分延迟边——
//! 动态模型里 `param→rate→state→可测` 才连得通，否则误报不可辨识）上做参数→可测的可达性。

use std::collections::{HashMap, HashSet};

use crate::schema::{EquationFile, VariableType};

use super::bipartite::NodeResolver;
use super::digraph::DiGraph;

/// 单个参数的可达性结论。
#[derive(Debug, Clone)]
pub struct ParamReach {
    /// 参数节点 id（`MODULE.name`）。
    pub param: String,
    /// 该参数在影响图上可达的**可测**节点 id（已排序）。
    pub reaches: Vec<String>,
    /// 是否（结构上）可辨识：能到达至少一个可测变量。
    pub identifiable: bool,
}

/// 结构可辨识性报告。
#[derive(Debug, Clone)]
pub struct IdentifiabilityReport {
    /// 实际采用的可测变量节点（来自 `measurable:true`；若无则回退所有 output 型变量）。
    pub measurable: Vec<String>,
    /// 每个参数的可达性。
    pub params: Vec<ParamReach>,
    /// 不可辨识参数（到任何可测都无路径），按节点 id 排序。
    pub unidentifiable: Vec<String>,
    /// 混淆候选参数对（进入完全相同方程集合、且各自可达可测），按字典序。
    pub confounded_candidates: Vec<(String, String)>,
}

/// 对一组方程文件做结构可辨识性分析。
pub fn analyze_identifiability(files: &[EquationFile]) -> IdentifiabilityReport {
    let resolver = NodeResolver::build(files);
    let graph = DiGraph::from_files(files);

    // 1) 可测集 = measurable:true 的变量；若一个都没标，回退所有 output 型变量（同数值版默认）。
    let mut measurable: Vec<String> = Vec::new();
    for f in files {
        let m = &f.meta.id;
        for (vname, var) in &f.variables {
            if var.measurable {
                measurable.push(resolver.resolve(m, vname));
            }
        }
    }
    if measurable.is_empty() {
        for f in files {
            let m = &f.meta.id;
            for (vname, var) in &f.variables {
                if var.var_type == VariableType::Output {
                    measurable.push(resolver.resolve(m, vname));
                }
            }
        }
    }
    measurable.sort();
    measurable.dedup();
    let measurable_set: HashSet<&str> = measurable.iter().map(|s| s.as_str()).collect();

    // 2) 参数节点 = 各文件 parameters。逐个算可达可测集。
    let mut params: Vec<ParamReach> = Vec::new();
    let mut unidentifiable: Vec<String> = Vec::new();
    for f in files {
        let m = &f.meta.id;
        for pname in f.parameters.keys() {
            let node = format!("{m}.{pname}");
            let reach = graph.reachable(&node);
            let mut reaches: Vec<String> = reach
                .iter()
                .filter(|n| measurable_set.contains(n.as_str()))
                .cloned()
                .collect();
            reaches.sort();
            let identifiable = !reaches.is_empty();
            if !identifiable {
                unidentifiable.push(node.clone());
            }
            params.push(ParamReach { param: node, reaches, identifiable });
        }
    }
    unidentifiable.sort();

    // 3) 混淆候选：进入**完全相同方程集合**的参数对（且各自可达可测，否则属不可辨识不重复报）。
    //    方程集合 = 该参数直接出现在哪些方程（按方程键）。用「参数名 ∩ 方程全部依赖」判定出现，
    //    对 AST 是否做过 Var→Param 重分类都稳健（合成模型常未重分类）。
    let mut param_eqs: HashMap<String, Vec<String>> = HashMap::new();
    for f in files {
        let m = &f.meta.id;
        let pnames: HashSet<&str> = f.parameters.keys().map(|s| s.as_str()).collect();
        for eq in &f.equations {
            let key = format!("{m}::{}", eq.id);
            for dep in eq.get_all_dependencies() {
                if pnames.contains(dep.as_str()) {
                    let node = resolver.resolve(m, &dep);
                    param_eqs.entry(node).or_default().push(key.clone());
                }
            }
        }
    }
    // 规范化各集合（排序+去重）便于相等比较。
    let identifiable_set: HashSet<&str> =
        params.iter().filter(|p| p.identifiable).map(|p| p.param.as_str()).collect();
    let mut sig: Vec<(String, Vec<String>)> = Vec::new();
    for (p, mut eqs) in param_eqs {
        if !identifiable_set.contains(p.as_str()) {
            continue; // 不可达的参数不进混淆候选（已在 unidentifiable 报告）
        }
        eqs.sort();
        eqs.dedup();
        sig.push((p, eqs));
    }
    sig.sort();
    let mut confounded_candidates: Vec<(String, String)> = Vec::new();
    for i in 0..sig.len() {
        for j in (i + 1)..sig.len() {
            if sig[i].1 == sig[j].1 {
                confounded_candidates.push((sig[i].0.clone(), sig[j].0.clone()));
            }
        }
    }
    confounded_candidates.sort();

    IdentifiabilityReport { measurable, params, unidentifiable, confounded_candidates }
}

#[cfg(test)]
mod tests {
    use super::super::bipartite::tests::toy;
    use super::*;
    use crate::schema::{DataType, Variable, VariableType};

    fn q(s: &str) -> String {
        format!("T.{s}")
    }

    /// 最小标量参数。
    fn param() -> crate::schema::Parameter {
        crate::schema::Parameter {
            name_cn: String::new(),
            name_en: None,
            dtype: DataType::Float,
            default: 1.0,
            values: None,
            unit: None,
            bounds: None,
            optimizable: true,
            management: false,
            description: None,
        }
    }

    /// 给玩具模型某变量标 measurable（或改类型）。
    fn mark(f: &mut EquationFile, name: &str, measurable: bool, vt: VariableType) {
        f.variables.insert(
            name.to_string(),
            Variable {
                var_type: vt,
                dtype: DataType::Float,
                unit: None,
                description: None,
                label: None,
                measurable,
                stress_factor: None,
                stress_reduce: None,
                source: None,
                class: None,
                init: None,
                rate: None,
                prev: None,
            },
        );
    }

    #[test]
    fn param_reaching_measurable_is_identifiable() {
        // y = a·x ; z = b·x。只测 y → a 可辨识、b 不可辨识（b 只到 z，z 不可测）。
        let mut f = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "z", vec!["b", "x"])]);
        // a,b 当参数；x 驱动；y 可测、z 不可测。
        f.parameters.insert("a".into(), param());
        f.parameters.insert("b".into(), param());
        mark(&mut f, "y", true, VariableType::Output);
        mark(&mut f, "z", false, VariableType::Output);
        let r = analyze_identifiability(&[f]);
        assert_eq!(r.measurable, vec![q("y")]);
        let a = r.params.iter().find(|p| p.param == q("a")).unwrap();
        let b = r.params.iter().find(|p| p.param == q("b")).unwrap();
        assert!(a.identifiable, "a 到 y 可达");
        assert!(!b.identifiable, "b 只到 z（不可测）");
        assert_eq!(r.unidentifiable, vec![q("b")]);
    }

    #[test]
    fn dynamic_chain_reaches_through_integration() {
        // 动态链：rate = k·x ; 状态 S 积分 rate ; y = S。只测 y。
        // 关键：k → rate →(积分边)→ S → y，必须含积分边才连得通。
        let mut f = toy(vec![("e1", "rate", vec!["k", "x"]), ("e2", "y", vec!["S"])]);
        f.parameters.insert("k".into(), param());
        // S = 积分状态量（rate: rate），无方程。
        f.variables.insert(
            "S".to_string(),
            Variable {
                var_type: VariableType::Intermediate,
                dtype: DataType::Float,
                unit: None,
                description: None,
                label: None,
                measurable: false,
                stress_factor: None,
                stress_reduce: None,
                source: None,
                class: Some(crate::schema::VarClass::State),
                init: Some(0.0),
                rate: Some("rate".into()),
                prev: None,
            },
        );
        mark(&mut f, "y", true, VariableType::Output);
        let r = analyze_identifiability(&[f]);
        let k = r.params.iter().find(|p| p.param == q("k")).unwrap();
        assert!(k.identifiable, "k 经 rate→(积分)→S→y 可达 y");
        assert!(r.unidentifiable.is_empty());
    }

    #[test]
    fn same_equation_params_are_confounding_candidates() {
        // y = a·b·x（a,b 同进 e1）；只测 y → a,b 结构混淆候选。
        let mut f = toy(vec![("e1", "y", vec!["a", "b", "x"])]);
        f.parameters.insert("a".into(), param());
        f.parameters.insert("b".into(), param());
        mark(&mut f, "y", true, VariableType::Output);
        let r = analyze_identifiability(&[f]);
        assert_eq!(r.confounded_candidates, vec![(q("a"), q("b"))]);
    }

    #[test]
    fn different_equation_params_not_confounded() {
        // a 进 e1、b 进 e2，不同方程集 → 不配对。
        let mut f = toy(vec![("e1", "y", vec!["a", "x"]), ("e2", "w", vec!["b", "y"])]);
        f.parameters.insert("a".into(), param());
        f.parameters.insert("b".into(), param());
        mark(&mut f, "w", true, VariableType::Output);
        let r = analyze_identifiability(&[f]);
        assert!(r.confounded_candidates.is_empty());
    }
}
