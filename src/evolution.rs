//! 进化图论分析器：沿模型**血缘链**走 git 历史，逐版本算图论指标 + 相邻版本结构 diff，
//! 合成一份**标定坑清单**。是 `docs/spec-model-evolution-arc.md` 的「分析层」核心。
//!
//! ## 单一真相源
//! CLI `eqc evolution` 与（后续）serve `/api/evolution` 共用本模块的 [`analyze_lineage`]。
//! **零核心改动**——只 use 已 pub 的 `graph::{analyze_metrics, analyze_identifiability,
//! analyze_structure, diff_models}` + `parse_str`；本模块不碰求解器、不改任何既有逻辑。
//!
//! ## 统一 all-output 口径（头号前置）
//! 跨版本一律清掉各版本 `measurable` 标注，逼 `analyze_identifiability` 回退到「全 output
//! 变量」当可测集。否则「s8 首次手标 measurable 白名单」会把**标注变化伪装成结构信号**
//! （spec §2.4 踩过一次的坑）。核心发现「混淆团大小 = 经验式系数数」本就与口径无关（纯结构），
//! 统一口径只为让**可辨识性轨迹**可比。最新版另按诚实白名单补报一次真田间可辨识性。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::graph::{analyze_identifiability, analyze_metrics, analyze_structure, diff_models};
use crate::parse_str;
use crate::schema::{Equation, EquationFile};

// ============================================================================
// 输入：血缘清单（evolution.yaml）
// ============================================================================

/// 血缘清单（`evolution.yaml`）：一条进化链的显式里程碑序列（时间正序）。
///
/// 每个里程碑 = 一次 version bump = 进化图上一个节点；分析器逐个 `git show` 取历史源码。
/// 旧版本文件常已从工作区删除（谱系交给 git 历史），故用 `commit` ref 而非活文件。
#[derive(Debug, Clone, Deserialize)]
pub struct EvolutionManifest {
    /// 逻辑模型名（如 `strawberry`）。
    pub model: String,
    /// git 仓根路径（相对本清单文件所在目录解析；可被 CLI `--repo` 覆盖）。
    pub repo: String,
    /// 默认模型文件路径（相对仓根）；各节点可用自己的 `path` 覆盖（历史上文件重命名过）。
    #[serde(default)]
    pub path: Option<String>,
    /// 进化链（时间正序）。
    pub chain: Vec<LineageNode>,
}

/// 血缘链上一个里程碑节点。
#[derive(Debug, Clone, Deserialize)]
pub struct LineageNode {
    /// 版本标签（如 `v1` / `s4` / `8.1`）。**不依赖模型 `meta.version`**——早期版本没写。
    pub version: String,
    /// 该版本对应的 git commit（模型所在仓的 ref）。
    pub commit: String,
    /// 覆盖顶层 `path`（该版本文件相对仓根的路径）。
    #[serde(default)]
    pub path: Option<String>,
    /// 这一步做了什么（人读演化说明）。
    #[serde(default)]
    pub step: String,
}

// ============================================================================
// 输出：进化报告（结构化 JSON 契约）
// ============================================================================

/// 一条进化链的完整分析产物：逐版本指标轨迹 + 相邻版本 diff + 标定坑清单。
#[derive(Debug, Clone, Serialize)]
pub struct EvolutionReport {
    pub model: String,
    /// 可测集口径（恒 `"all-output"`，诚实标注跨版本一致口径）。
    pub measurable_convention: String,
    /// 逐版本图论指标 + 明细（时间正序）。
    pub versions: Vec<VersionPoint>,
    /// 相邻版本结构 diff（长度 = versions.len()-1）。
    pub diffs: Vec<VersionDiff>,
    /// ★最硬产出：结构坑清单（混淆系数簇 + 不可辨识阈值参数），供标定实验设计。
    pub calibration_pitlist: Vec<PitlistEntry>,
    /// 最新版按**诚实白名单**（真田间可测量 measurable:true）另算一次可辨识性；
    /// 若最新版没标 measurable（= 白名单口径与 all-output 相同）则为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_honest_identifiability: Option<HonestIdentifiability>,
}

/// 血缘链上一个版本的图论指标（all-output 口径）+ 可辨识性明细。
#[derive(Debug, Clone, Serialize)]
pub struct VersionPoint {
    pub version: String,
    pub commit: String,
    pub step: String,
    // —— 网络/结构指标 ——
    pub nodes: usize,
    pub edges: usize,
    pub depth: usize,
    pub algebraic_loops: usize,
    pub n_communities: usize,
    pub modularity_detected: f64,
    /// 作者 `meta.modules` 划分的模块度 Q（未声明=None）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modularity_modules: Option<f64>,
    pub params: usize,
    /// all-output 口径下的可测量数（= output 变量数，跨版本同口径）。
    pub measurable: usize,
    pub unidentifiable: usize,
    pub confounded_pairs: usize,
    /// 介数前 3 枢纽（本地名）。
    pub hubs: Vec<String>,
    // —— 明细（机理归因用；本地名） ——
    pub confounded: Vec<[String; 2]>,
    pub unidentifiable_params: Vec<String>,
}

/// 相邻版本的结构演化（哪些点/边/方程/参数长出来了 + 这一步新引入的混淆对）。
#[derive(Debug, Clone, Serialize)]
pub struct VersionDiff {
    pub from: String,
    pub to: String,
    pub distance: usize,
    pub edge_similarity: f64,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    /// 新增参数（added_nodes 里 kind == "parameter"）。
    pub added_params: Vec<String>,
    pub added_equations: Vec<String>,
    /// 同 output、表达式形式变了的方程（GP 进化核心信号）。
    pub changed_equations: Vec<String>,
    pub added_edges: usize,
    pub removed_edges: usize,
    /// 这一步**新引入**的混淆对（= 新经验式带来的系数簇；spec 验证过的规律）。
    pub new_confounded: Vec<[String; 2]>,
}

/// 标定坑清单一条：一个结构上定不准的东西 + 归因 + 建议。
#[derive(Debug, Clone, Serialize)]
pub struct PitlistEntry {
    /// `"confounding-clique"`（异参同效系数簇）| `"unidentifiable-threshold"`（不可辨识阈值参数）。
    pub kind: String,
    /// 涉及的参数（系数簇 或 单个阈值参数）。
    pub params: Vec<String>,
    /// 这坑在哪个版本引入。
    pub introduced_at: String,
    /// 机理归因（共享方程的中文名，自动派生；拿不到=None）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mechanism: Option<String>,
    /// 关联方程的 output（自动派生）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equation: Option<String>,
    /// 标定建议。
    pub advice: String,
}

/// 最新版按诚实白名单（真田间可测量）的可辨识性——回答「真采集下哪些还标不出」。
#[derive(Debug, Clone, Serialize)]
pub struct HonestIdentifiability {
    pub version: String,
    /// 白名单（真田间可测量）节点（本地名）。
    pub measurable_whitelist: Vec<String>,
    pub unidentifiable: Vec<String>,
    pub confounded: Vec<[String; 2]>,
    pub note: String,
}

// ============================================================================
// 分析主流程
// ============================================================================

/// 从清单文件分析：读 `evolution.yaml`、解析 repo 路径（`repo_override` 优先），跑 [`analyze_lineage`]。
pub fn analyze_manifest_file(
    manifest_path: &Path,
    repo_override: Option<&Path>,
) -> Result<EvolutionReport, String> {
    let text = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("读清单失败 {}: {e}", manifest_path.display()))?;
    let manifest: EvolutionManifest =
        serde_yaml::from_str(&text).map_err(|e| format!("解析清单 YAML 失败: {e}"))?;

    // repo 解析：CLI 覆盖 > 清单 repo（相对清单所在目录）。
    let repo_root: PathBuf = match repo_override {
        Some(p) => p.to_path_buf(),
        None => {
            let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
            base.join(&manifest.repo)
        }
    };
    analyze_lineage(&manifest, &repo_root)
}

/// 核心：沿 `manifest.chain` 逐版本走 git 历史算指标 + diff + 坑清单。
pub fn analyze_lineage(
    manifest: &EvolutionManifest,
    repo_root: &Path,
) -> Result<EvolutionReport, String> {
    if manifest.chain.is_empty() {
        return Err("血缘链为空（manifest.chain 无节点）".into());
    }

    let mut versions: Vec<VersionPoint> = Vec::with_capacity(manifest.chain.len());
    // 逐版本的原始解析结果（供相邻 diff + 机理归因；measurable 无关，用原始即可）。
    let mut per_version_files: Vec<Vec<EquationFile>> = Vec::with_capacity(manifest.chain.len());

    for node in &manifest.chain {
        let rel_path = node
            .path
            .as_deref()
            .or(manifest.path.as_deref())
            .ok_or_else(|| format!("版本 {} 无 path（顶层与节点都缺）", node.version))?;

        let content = git_show(repo_root, &node.commit, rel_path)?;
        let ef = parse_str(&content).map_err(|e| {
            format!("解析版本 {} ({}:{}) 失败: {e}", node.version, node.commit, rel_path)
        })?;
        let raw_files = vec![ef];

        // 指标/结构与 measurable 无关，用原始版本算。
        let metrics = analyze_metrics(&raw_files);
        let structure = analyze_structure(&raw_files);

        // 可辨识性：统一 all-output 口径（清 measurable → 回退全 output）。
        let mut norm_files = raw_files.clone();
        normalize_measurable(&mut norm_files);
        let id = analyze_identifiability(&norm_files);

        let edges: usize = metrics.nodes.iter().map(|n| n.out_degree).sum();
        let depth = metrics.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
        let hubs: Vec<String> = metrics.nodes.iter().take(3).map(|n| local(&n.node)).collect();
        let confounded: Vec<[String; 2]> = id
            .confounded_candidates
            .iter()
            .map(|(a, b)| [local(a), local(b)])
            .collect();
        let unidentifiable_params: Vec<String> =
            id.unidentifiable.iter().map(|s| local(s)).collect();

        versions.push(VersionPoint {
            version: node.version.clone(),
            commit: node.commit.clone(),
            step: node.step.clone(),
            nodes: metrics.nodes.len(),
            edges,
            depth,
            algebraic_loops: structure.algebraic_loops().len(),
            n_communities: metrics.n_communities,
            modularity_detected: metrics.modularity_detected,
            modularity_modules: metrics.modularity_modules,
            params: id.params.len(),
            measurable: id.measurable.len(),
            unidentifiable: unidentifiable_params.len(),
            confounded_pairs: confounded.len(),
            hubs,
            confounded,
            unidentifiable_params,
        });
        per_version_files.push(raw_files);
    }

    // —— 相邻版本 diff ——
    let mut diffs: Vec<VersionDiff> = Vec::with_capacity(versions.len().saturating_sub(1));
    for i in 1..versions.len() {
        let d = diff_models(&per_version_files[i - 1], &per_version_files[i]);
        let added_params: Vec<String> = d
            .added_nodes
            .iter()
            .filter(|n| n.kind == "parameter")
            .map(|n| n.id.clone())
            .collect();
        // 这一步新引入的混淆对 = 本版 confounded 集 − 上一版 confounded 集（无序对比较）。
        let prev_set: std::collections::BTreeSet<[String; 2]> =
            versions[i - 1].confounded.iter().map(sorted_pair).collect();
        let new_confounded: Vec<[String; 2]> = versions[i]
            .confounded
            .iter()
            .map(sorted_pair)
            .filter(|p| !prev_set.contains(p))
            .collect();

        diffs.push(VersionDiff {
            from: versions[i - 1].version.clone(),
            to: versions[i].version.clone(),
            distance: d.distance,
            edge_similarity: d.edge_similarity,
            added_nodes: d.added_nodes.iter().map(|n| n.id.clone()).collect(),
            removed_nodes: d.removed_nodes.iter().map(|n| n.id.clone()).collect(),
            added_params,
            added_equations: d.added_equations.clone(),
            changed_equations: d.changed_equations.iter().map(|c| c.output.clone()).collect(),
            added_edges: d.added_edges.len(),
            removed_edges: d.removed_edges.len(),
            new_confounded,
        });
    }

    // —— 最新版诚实白名单可辨识性（先算：坑清单②的阈值不可辨识要从它揭示）——
    let final_honest_identifiability = honest_final(manifest, &per_version_files);

    // —— 标定坑清单 ——
    let calibration_pitlist =
        build_pitlist(&versions, &diffs, &per_version_files, final_honest_identifiability.as_ref());

    Ok(EvolutionReport {
        model: manifest.model.clone(),
        measurable_convention: "all-output".into(),
        versions,
        diffs,
        calibration_pitlist,
        final_honest_identifiability,
    })
}

// ============================================================================
// 坑清单合成
// ============================================================================

fn build_pitlist(
    versions: &[VersionPoint],
    diffs: &[VersionDiff],
    per_version_files: &[Vec<EquationFile>],
    honest: Option<&HonestIdentifiability>,
) -> Vec<PitlistEntry> {
    let mut out = Vec::new();

    // ① 混淆系数簇：每个 diff 里 new_confounded 的连通分量 = 一个新经验式的系数簇。
    //    ★严谨语义 = 「该经验式方程集签名**所独有**的系数集」——共享的物理常数（如消光系数 k
    //    也进 Beer 公式、EC_thresh 也进 f_EC）方程集签名不同、被正确排除在团外，只留真正异参同效者。
    for (i, d) in diffs.iter().enumerate() {
        if d.new_confounded.is_empty() {
            continue;
        }
        let ver_ef = &per_version_files[i + 1][0]; // diff 的 "to" 版本
        for clique in cliques(&d.new_confounded) {
            let host = clique_mechanism(ver_ef, &clique);
            out.push(PitlistEntry {
                kind: "confounding-clique".into(),
                params: clique,
                introduced_at: d.to.clone(),
                mechanism: host.map(|e| e.name.clone()),
                equation: host.map(|e| e.output.clone()),
                advice: "该经验式的系数簇结构上异参同效——需一起标定，或加正交/多工况对照实验拆开"
                    .into(),
            });
        }
    }

    // ② 不可辨识阈值参数：来自最新版**诚实白名单**——all-output 口径下一切可达（unid 恒 0），
    //    只有真田间可测量白名单才暴露「结构够不到数据」的阈值常数。回溯首次作为参数出现的版本 + 归因宿主方程。
    if let (Some(h), Some(final_v)) = (honest, per_version_files.last()) {
        let final_ef = &final_v[0];
        for param in &h.unidentifiable {
            let introduced_at = versions
                .iter()
                .zip(per_version_files.iter())
                .find(|(_v, files)| files.iter().any(|f| f.parameters.contains_key(param.as_str())))
                .map(|(v, _)| v.version.clone())
                .unwrap_or_else(|| h.version.clone());
            let host = final_ef.equations.iter().find(|e| eq_refs(e).iter().any(|r| r == param));
            out.push(PitlistEntry {
                kind: "unidentifiable-threshold".into(),
                params: vec![param.clone()],
                introduced_at,
                mechanism: host.map(|e| e.name.clone()),
                equation: host.map(|e| e.output.clone()),
                advice: "结构上到任何真田间可测量都无路径（纯阈值/临界常数、下游只连未标可测输出）——数据定不了、只能靠先验"
                    .into(),
            });
        }
    }

    out
}

/// 最新版按诚实白名单（真 measurable:true）算可辨识性。若最新版没标 measurable，则口径同
/// all-output、无额外信息 → None。
fn honest_final(
    manifest: &EvolutionManifest,
    per_version_files: &[Vec<EquationFile>],
) -> Option<HonestIdentifiability> {
    let last_ef = per_version_files.last()?;
    let has_whitelist = last_ef.iter().any(|f| f.variables.values().any(|v| v.measurable));
    if !has_whitelist {
        return None;
    }
    let id = analyze_identifiability(last_ef); // 不清 measurable = 用真白名单
    let last_version = manifest.chain.last().map(|n| n.version.clone()).unwrap_or_default();
    Some(HonestIdentifiability {
        version: last_version,
        measurable_whitelist: id.measurable.iter().map(|s| local(s)).collect(),
        unidentifiable: id.unidentifiable.iter().map(|s| local(s)).collect(),
        confounded: id.confounded_candidates.iter().map(|(a, b)| [local(a), local(b)]).collect(),
        note: "按真田间可测量白名单（measurable:true）算；比 all-output 更贴真实观测能力".into(),
    })
}

// ============================================================================
// 助手
// ============================================================================

/// 在模型所在 git 仓里取某版本某文件的源码：`git -C <repo> show <commit>:<path>`。
fn git_show(repo: &Path, commit: &str, rel_path: &str) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("show")
        .arg(format!("{commit}:{rel_path}"))
        .output()
        .map_err(|e| format!("git show 执行失败（repo={}）: {e}", repo.display()))?;
    if !out.status.success() {
        return Err(format!(
            "git show {commit}:{rel_path} 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("历史源码非 UTF-8: {e}"))
}

/// 清掉所有变量的 measurable 标注 → 逼 `analyze_identifiability` 回退「全 output」口径。
fn normalize_measurable(files: &mut [EquationFile]) {
    for f in files.iter_mut() {
        for (_name, v) in f.variables.iter_mut() {
            v.measurable = false;
        }
    }
}

/// 去掉 `MODULE.` 前缀取本地名（与 data-var / `graph::diff` 本地名对齐）。
///
/// ⚠️ 已知边界（同 `graph/diff.rs` 的对齐假设）：**假定单模块进化链**。版本谱系天然是
/// per-model 单模块（草莓/番茄/蓝莓各一条链），故此假设成立；若将来对**多模块耦合**模型
/// 跑进化分析，跨模块同名变量会被 `local()` 合并 → 机理归因/`new_confounded` 差集静默出错，
/// 届时需改用带模块前缀的全名对齐。
fn local(name: &str) -> String {
    match name.find('.') {
        Some(i) => name[i + 1..].to_string(),
        None => name.to_string(),
    }
}

/// 无序对归一（字典序），供跨版本混淆对差集比较。
fn sorted_pair(p: &[String; 2]) -> [String; 2] {
    if p[0] <= p[1] {
        [p[0].clone(), p[1].clone()]
    } else {
        [p[1].clone(), p[0].clone()]
    }
}

/// 方程引用的全部名字（变量 + 参数）。
fn eq_refs(e: &Equation) -> Vec<String> {
    let mut refs = e.expression.get_variable_refs();
    refs.extend(e.expression.get_parameter_refs());
    refs
}

/// 混淆对图的连通分量（每个分量 = 一个全连通异参同效簇）。
fn cliques(pairs: &[[String; 2]]) -> Vec<Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut adj: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for [a, b] in pairs {
        adj.entry(a.as_str()).or_default().insert(b.as_str());
        adj.entry(b.as_str()).or_default().insert(a.as_str());
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::new();
    for start in adj.keys() {
        if seen.contains(start) {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![*start];
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            comp.push(n.to_string());
            if let Some(nb) = adj.get(n) {
                for m in nb {
                    if !seen.contains(m) {
                        stack.push(m);
                    }
                }
            }
        }
        comp.sort();
        out.push(comp);
    }
    out
}

/// 给一个混淆簇找机理宿主方程：引用了簇内 ≥2 个参数的第一条方程。
fn clique_mechanism<'a>(ef: &'a EquationFile, clique: &[String]) -> Option<&'a Equation> {
    ef.equations.iter().find(|e| {
        let refs = eq_refs(e);
        clique.iter().filter(|p| refs.contains(p)).count() >= 2
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn local_strips_module_prefix() {
        assert_eq!(local("STRAWBERRY_S8.Cref"), "Cref");
        assert_eq!(local("bare_name"), "bare_name");
        // 只去第一个点前缀（变量名本身不含点）
        assert_eq!(local("MID.a.b"), "a.b");
    }

    #[test]
    fn sorted_pair_orders_lexicographically() {
        assert_eq!(sorted_pair(&[s("Kc"), s("Cref")]), [s("Cref"), s("Kc")]);
        assert_eq!(sorted_pair(&[s("Cref"), s("Kc")]), [s("Cref"), s("Kc")]);
    }

    #[test]
    fn cliques_are_connected_components() {
        // a-b-c 一团、x-y 另一团 → 两个连通分量（= 两个异参同效簇）
        let pairs = vec![
            [s("a"), s("b")],
            [s("b"), s("c")],
            [s("x"), s("y")],
        ];
        let mut comps = cliques(&pairs);
        comps.sort_by_key(|c| c.len());
        assert_eq!(comps.len(), 2);
        assert_eq!(comps[0], vec![s("x"), s("y")]);
        assert_eq!(comps[1], vec![s("a"), s("b"), s("c")]);
    }

    #[test]
    fn cliques_empty_when_no_pairs() {
        assert!(cliques(&[]).is_empty());
    }
}
