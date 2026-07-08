# GP 候选分支化 + 图论硬过滤 + Claude Code 采纳 —— 进化图论 arc · Tier3

> 状态：**spec 定稿（2026-07-08），待下轮开工**。承 `docs/spec-model-evolution-arc.md` §3.4（GP 层）。
> 本文捕获本轮讨论拍板的 4 个决策 + 现有基建复用 + 分阶段施工路线，供下轮实现。

---

## 0. 一句话

GP 跑出的候选方程现在临时不落盘、采纳靠手动复制粘贴。Tier3 把候选变一等公民：
**结构硬过滤（红线）→ 候选报告（含图论证据）→ Claude Code 开发会话多准则判断 → 自主采纳到
`gp/<模型>/<靶点>` 分支（write_source + meta.lineage/version/provenance）→ 你拍板转正合 main。**
图论指标的角色 = 把「这个候选科学上好不好」从纯 RMSE 升级成**可检视的结构证据**。

---

## 1. 定位（承进化图论 arc）

这是进化图论 arc 与 **GP 主线**的接口（arc spec §3.4）。前置：Tier1（分析器/视图/动画）+ Tier2（回退）
已完成并部署。Tier3 复用其成果：`diff_models`、进化分析器（`src/evolution.rs`）、`write_source`、
「看它长出什么」3D 动画（GA-6b Phase 3）。

**两个 AI 的分工（务必分清）**：
- **指月**（前端 GLM·运行时/田间顾问）——**不碰** GP 方程筛选。
- **Claude Code**（开发态 agent·本 session 这种）——GP 候选筛选/采纳 = **模型开发决策**，交给它。
  不在 serve 里新造运行时 LLM（"我不另加 AI"）。

---

## 2. 本轮锁定的 4 个决策（首席拍板 2026-07-08）

1. **形态先轻后重**：先做**形态 A（报告→开发会话）**跑通闭环；候选多/要多视角交叉验证时再上
   **形态 B（多 agent 评审 workflow）**。
2. **硬过滤红线（机械·非 AI 判断）**：淘汰「加代数环 / 破守恒律 / 引入新混淆对（异参同效）」的候选。
   这三条是结构红线，机械筛掉、不进 Claude Code 判断。
3. **★自主边界**：
   - **采纳到 `gp/` 分支 = Claude Code 自主**（安全：过硬过滤 + 分支隔离不碰 main + git 可 Tier2 回退 +
     必报告采纳了什么/为什么）。
   - **转正（gp/ 分支候选合进 main = official 模型）= 首席拍板**（对官方模型保留一道闸）。
   一句话：**自主在分支上进化探索，首席决定哪个探索转正。**
4. **分支粒度 = `gp/<模型>/<靶点>`**（如 `gp/strawberry/DMC_fruit_dyn`）。采纳候选作该分支上的 commit
   （带 meta.lineage/version/provenance）。理由：自然单元=一条方程的进化；per-候选太碎、全 GP 一分支太糊。
   分支在 crop-models 仓内（per-repo，符合 5 仓分散现实）。

---

## 3. 现有基建盘点（Tier3 复用清单·别重造）

| 能力 | 位置 | Tier3 用途 |
|---|---|---|
| `gp::Candidate` | `src/gp/grammar.rs:21` | 候选（表达式树 + 常数）|
| `gp::CandidateCheck` | `src/gp/constraints.rs:17` | GP **生成期** grammar 约束（界/单调）——Tier3 结构硬过滤是**另一档**（post-GP·图级）|
| `candidate_expr(cand)` | `src/serve.rs:2247` | 候选 → `Expr`（常数折回字面值）|
| `gp_structure_diff(file,target,cand)` | `src/serve.rs:2260` | patch 靶方程 + `diff_models` → 结构 diff（已给「看它长出什么」用）|
| `to_yaml::to_yaml_value(&candidate_expr(cand))` | `src/serve.rs:2315` | 候选 → yaml（**采纳落盘写文件用**）|
| `analyze_structure` | `src/graph/dm.rs:67` + `.algebraic_loops()` | 硬过滤①：加代数环？|
| `analyze_identifiability` | `src/graph/identifiability.rs:49` | 硬过滤③：新混淆对？|
| `--check-balance`（meta.balance） | `src/main.rs:678` / `balance_residual` | 硬过滤②：破守恒？（跑短仿真核残差）|
| `diff_models` / `src/evolution.rs` | 已建 | 分支 vs main diff + 分支轨迹进分析器 |
| `write_source`（POST /api/source·校验+备份+原子写） | `src/serve.rs` | 采纳落盘复用 |
| GP evolve 入口 | `src/gp/{evolve,pareto,joint}.rs`、serve `/api/evolve[/start\|/status]`、`run_evolve_job` | 候选来源 |

**关键**：「把候选 patch 进模型算结构 / 转 yaml」这套 serve 里已有（`candidate_expr`+patch），Tier3 硬过滤
和落盘都站在它上面。

---

## 4. 数据流

```
首席: eqc evolve <模型> <靶点>  (或 /api/evolve)  →  Pareto 候选集
        │
        ▼  [Phase1·硬过滤·机械·post-GP 图级]
  对每候选：candidate_expr → patch 靶方程 → 得 patched 模型
    ├ analyze_structure → 有新代数环?          ── 是 → 淘汰
    ├ analyze_identifiability → 有新混淆对?     ── 是 → 淘汰
    └ 短仿真 + check_balance → 破守恒律?        ── 是 → 淘汰
        │  幸存者
        ▼  [Phase1 产出]
  候选报告 candidates.json：表达式 + 拟合 + 图论证据 + provenance（见 §5.2）
        │
        ▼  [Phase3·形态A·Claude Code 开发会话]
  读报告 + 模型上下文 + 数据 → 多准则判断（拟合 vs 简约 vs 机理 vs provenance）→ 排序推荐 + 理由
        │
        ▼  [Phase2·自主落盘]
  采纳最优 → to_yaml patch 进模型文件 → write_source(校验+备份) →
  commit 到 gp/<模型>/<靶点> 分支 + 写 meta.lineage(parent=main当前版) / version / provenance:GP进化
        │  (进化分析器/动画自动纳入这条分支：git show gp/... + diff_models)
        ▼  [转正·首席拍板]
  满意的 commit → cherry-pick/合进 main = official 模型（main 上一条真 lineage 边）
```

---

## 5. 关键接口

### 5.1 硬过滤红线（Phase 1·机械）
三条结构红线，对每个候选 patch 进模型后机械判定，**任一命中即淘汰、不进 Claude Code**：
1. **加代数环**：`analyze_structure(patched).algebraic_loops()` 比 baseline 多 → 淘汰（破求解结构）。
2. **破守恒律**：patched 模型跑短仿真 + `--check-balance`（meta.balance），残差 > tol → 淘汰。
   （模型无 meta.balance 声明则此条跳过——诚实：无守恒律可核。）
3. **引入新混淆对**：`analyze_identifiability(patched).confounded_candidates` 比 baseline 多 → 淘汰
   （新参数变不可辨识、标定永远标不出）。
- 边界：受约束 GP 语法「候选只引 gp_target.inputs + __c 常数、不长新节点」→ 主要新增是 added 边；
  硬过滤在**边级/参数级**捕获结构退化。可选加：量纲一致（units::check）、复杂度上限（节点/边增量帽）。

### 5.2 候选报告 `candidates.json`（Phase 1 产出·喂 Claude Code）
每候选一条：
```json
{
  "target": "DMC_fruit_dyn",
  "expr_yaml": "...", "formula": "y = DMC + k_EC·max(0,EC−thr) + k_c·d2",
  "fitness": { "rmse": 0.021, "per_treatment": {...}, "extrapolation": "..." },
  "structure_diff": { "added_edges": [["d2","DMC_fruit_dyn"]], "changed_equations": ["DMC_fruit_dyn"], "distance": 1 },
  "graph_evidence": {
    "passes_hard_filters": true,
    "adds_algebraic_loop": false, "breaks_conservation": false, "new_confounded_pairs": [],
    "complexity_delta": {"nodes": 0, "edges": 1}, "depth_delta": 0
  },
  "provenance": { "target_provenance": "推导", "evolvable": true }
}
```
- `passes_hard_filters:false` 的候选也可留在报告里（标注淘汰原因），供 Claude Code/首席看"为什么被砍"。

### 5.3 采纳落盘（Phase 2·复用 to_yaml + write_source + git）
1. `to_yaml::to_yaml_value(&candidate_expr(cand))` → 靶方程新 expression 的 yaml。
2. patch 进模型文件源码（替换该 equation 的 expression 块）→ 得新模型源码。
3. `write_source`（校验+备份+原子写）到模型文件（**在 gp/ 分支的工作树**）。
4. 写 `meta.lineage:{parent: <main当前版 MODEL_ID@commit>, step: "GP 采纳候选 <desc>"}` + bump version +
   靶方程 provenance 改标（如 `推导→GP` 或加 gp 采纳标记）。
5. `git checkout -b gp/<模型>/<靶点>`（不存在则建）+ commit。
- **落盘机制约束**：git 操作在 crop-models 仓（per-repo）；采纳前确保工作树干净或在正确分支（人在环/自主都要守）。

### 5.4 分支与分析器/动画集成（白送）
- 进化分析器（`src/evolution.rs`）能 `git show gp/<分支>:<模型文件>` 取候选版 + `diff_models` vs main
  → **分支轨迹和 main 谱系并列可视**（分支 commit 带 meta.lineage → 自动进 §① 自动派生）。
- 「看它长出什么」3D 动画（GA-6b Phase 3·serve gp_structure_diff）已能播候选 diff → 分支候选直接可视。

---

## 6. Claude Code 的参与（形态 A 详 / 形态 B 概要）

### 形态 A（先做·报告→开发会话）
- 触发：首席（或我）跑 `eqc evolve` 出 `candidates.json`。
- Claude Code 开发会话：读 candidates.json + 模型 + 数据 + provenance → **多准则判断**（不是选 RMSE 最低）：
  拟合、简约性、机理合理性、provenance（"猜测/推导"靶点该进化、"文献"基座不该动）、图论证据。
- 输出：**排序推荐 + 每条为什么**（例："候选3 拟合略逊但零新混淆、守恒、落在'推导'靶点 → 荐；
  候选1 RMSE 最低但 added d2→y 让 c 不可辨识 → 否"）。
- 行动：**自主采纳最优到 gp/ 分支**（§5.3）+ 报告采纳了什么/为什么；**转正合 main 等首席拍板**。

### 形态 B（后做·可选·多 agent 评审 workflow）
- 一个 Claude Code workflow：给 N 候选，**每候选/每镜头派评委 agent**（守恒/可辨识/简约/机理各一），
  独立打分 + 对抗质证，再综合排序。适合候选多或要"多视角交叉验证"。用现成多 agent 编排。

---

## 7. 分阶段施工路线

| Phase | 内容 | 估算 | 验证 |
|---|---|---|---|
| **1（后端·轻）** | `eqc evolve` 加结构硬过滤（§5.1）+ 出 `candidates.json`（§5.2）；复用 candidate_expr patch + analyze_structure/identifiability + check-balance | ~2-3 天 | curl/CLI：合成 demo（gpdemo3 互作）跑 evolve→报告含图论证据+硬过滤标注 |
| **2（落盘·复用）** | 采纳 = to_yaml patch + write_source + git 分支 commit + meta.lineage/version/provenance（§5.3）| ~2 天 | 采纳一个候选到 gp/ 分支→分析器能 diff 分支 vs main、动画能播 |
| **3（Claude Code 闭环·形态A）** | 开发会话读报告→判断→自主采纳→报告；转正首席拍板 | ~1 天（主要是流程/prompt/纪律）| 端到端：gpdemo3 evolve→我推荐+采纳到分支→首席看+转正 |
| **4（可选·形态B）** | 多 agent 评审 workflow | ~2-3 天 | 候选多时的交叉验证 |

每 Phase：`cargo test` 两配置绿 + 首席点头再提交（守 EQC 铁律）。

---

## 8. 诚实边界 / 待定

- **守恒硬过滤需仿真**：破守恒律只能跑仿真 + check-balance 检出（非纯结构）；模型无 meta.balance 则跳过。
  成本 = 每候选一次短仿真（GP 本就为 fitness 仿真候选，可搭车）。
- **受约束 GP 不长新节点**（语法限制）→ 硬过滤主要在边/参数级；未来若放开语法长新节点，硬过滤要扩。
- **候选表达式 patch 进 yaml 源码**（§5.3 步2）：需可靠地替换某 equation 的 expression 块——
  最稳是"整份模型 re-serialize"还是"局部替换 expression"待定（局部替换保留注释/格式，但更易碎）。
  **待 Phase 2 定**（倾向：parse 模型→改内存 AST 的该 eq→整份 re-emit；或 to_yaml 局部块替换）。
- **自主采纳的"报告"形式**：我采纳后如何让首席看到（commit message + 一句话总结？一个 adopted.md？）待定。
- **转正机制**：cherry-pick vs merge vs 手动 write_source main——待 Phase 3 定（倾向：首席在进化史视图
  选分支候选→"转正"按钮→write_source 到 main + commit·人在环·像 Tier2 回退的镜像）。
- 样本/数据：GP 需真拟合数据；合成 demo（gpdemo1-3）可跑通闭环，真作物候选需真田间数据（7 月底）。

---

## 9. 与 GP 主线 / 标定 / 前端的接口
- **GP 主线**：Tier3 不改 GP 进化算法，只在其**输出侧**加过滤/报告/落盘。
- **标定**：图论硬过滤"不引入新混淆对"= 直接护住可标定性（承 arc 的标定坑清单思想）。
- **前端**：分支候选走现有「看它长出什么」动画 + 进化史视图 diff；**转正 UI**（Phase 3）可作进化史视图
  的镜像操作（选分支→转正，像 Tier2 回退）。

---

*相关：`docs/spec-model-evolution-arc.md`（母 arc·§3.4 GP 层）· 记忆 [[eqc-model-evolution-arc]]（Tier1/2 + 自动派生 + 2D 同步已建）· [[analytical-agent-design]]（GP 筛选交 Claude Code 的原始决策）· [[eqc-graph-analysis-arc]]（GA-6b「看它长出什么」+ 图论引擎）· [[model-evolution-traceability]]（留痕纪律：采纳=写 meta.lineage/version/provenance）。*
