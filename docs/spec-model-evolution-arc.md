# 模型进化图论 arc —— spec / 规划报告

> 状态：**最小验证片已通过（2026-07-03）**，架构待建。这是一个「超级大 arc」，预计 ≥1 周专注协作。
> 本文是蓝图 + 已验证成果的存档，供后续迭代。

---

## 0. 一句话

把「模型进化史」从**隐式**（文件名后缀 s1..s8 当版本 + git commit + 进展日志散记）升级为**一等公民的、结构化的、可图论分析的进化图（DAG）**：每次改模型 = 图上一条边；沿这条链跑图论分析，能挖出机理模型的深层规律、并把「进化史」和「标定实验设计 / GP 进化管理」结构性绑定。

---

## 1. 愿景与动机（北极星）

### 1.1 为什么
- **现状乱**：草莓有 v1/v1_vector/s1..s8 十几个文件当版本、番茄 t1..t3、蓝莓 bb1..bb5；git 把它们当无关文件（`git log s8` 看不到 s7 是祖先）；进化史只活在人脑 + 进展日志 doc。用户原话「感觉越来越乱」。
- **单一真相源**：动画（生长 `/api/growth` + GP「看它长出什么」）已是「派生非快照」；进化图应是同一原则的延伸——每次改模型自动更新进化动画/分析。
- **GP 进化管理**：GP 天然产生候选树，分支/图论分析正是它的自然家。

### 1.2 三件要分清的事（避免混淆）
1. **草稿谱系（s1→s8）** = 用「复制文件 + 改后缀」做版本控制 → **该交给 git 历史**（这是「乱」的根源）。
2. **并行变体**（t3 决策 vs tomato_fspm_organ 结构；bb5 vs bb5_gh）= 真·不同模型、不是彼此的版本 → 独立文件，git 分支帮不上。
3. **GP 进化候选** = 真正像分支的东西 → 分支模型对它最有价值。

本 arc 主攻 #1（把谱系交给 git + 结构化 lineage）和 #3（GP 候选作分支），#2 保持独立文件。

---

## 2. 已验证的最小片成果（2026-07-03·本 session）

### 2.1 方法
- 从 git 历史恢复草莓进化链 10 个版本到临时目录：`s1,s2,s3,s4,s5,s6,s6b,s7,s8(改糖度前),s8.1(现)`。
- 写临时分析器 `equation-compiler-main/examples/evo_metrics.rs`（**只 use 已 pub 的 lib API、零核心改动**）：对每个版本调 EQC 现成的 `graph::{analyze_metrics, analyze_identifiability, analyze_structure}`，输出图论指标。
- **关键：EQC 的图论分析引擎已存在**（`src/graph/`：metrics 网络指标 / identifiability 结构可辨识性 / dm Dulmage-Mendelsohn 分解 / bipartite / matching / diff_models）。arc 不是造引擎，是**把已有引擎接上「进化维度」+ 呈现**。

### 2.2 草莓 s1..s8.1 图论轨迹（实测）
| 版本 | 加的机理 | nodes | edges | depth | alg_loops | 参数 | 异参同效对 | 不可辨识 |
|---|---|---|---|---|---|---|---|---|
| s1 | 源库果实骨架 | 35 | 42 | 15 | 0 | 6 | 0 | 0 |
| s4 | +动态LUE/CO₂ | 48 | 61 | 14 | 0 | 10 | 1 (Cref~Kc) | 0 |
| s5 | +氮 | 62 | 79 | 14 | 0 | 14 | 2 (+Na~Nb) | 0 |
| s6 | +水/蒸腾 | 75 | 101 | 14 | 0 | 19 | 3 (+A_tr~B_tr) | 0 |
| s6b | +EC 盐 | 84 | 114 | 14 | 0 | 22 | 4 (+EC_max~EC_thresh) | 0 |
| s7 | +钙 | 94 | 127 | 14 | 0 | 26 | 5 (+Ca_night~Ca_xy) | 0 |
| s8 | 钙器官分配+tipburn | 106 | 148 | 14 | 0 | 29 | 4 (Ca_night~Ca_xy 消失) | 1 (Ca_crit_fruit) |
| s8.1 | +糖度 keystone | 114 | 159 | 14 | 0 | 33 | 6 (+DMC/k_DMC_EC/k_DMC_W 三共线) | 2 (+Brix_grade) |

### 2.3 挖到的真规律（经 3 路对抗验证）
1. **★混淆团大小 = 经验响应式的系数数**：每加一个经验响应子模型，就引入该式系数的**全连通异参同效团**（k 系数 → C(k,2) 对）。逐对方程证据全命中（Cref~Kc CO₂双曲线 / Na~Nb 氮稀释幂律 / A_tr~B_tr 蒸腾两项式 / EC_max~EC_thresh 渗透线 / 糖度三参三共线）。→ **给标定排期直接抓手：每个经验式的系数簇要一起标或加正交实验。**
2. **图论独立重现了 identify 的标定坑预测**：s8.1 糖度三共线，和番茄 T3 用 `eqc identify`（灵敏度分析）对合成孪生给的结构判定（DMC/k_DMC 单工况共线、要多处理拆开）**互证**——两种独立方法（图论结构可达性 vs identify 灵敏度）指向同一坑。
3. **「外挂式生长」而非「重构式生长」**：depth 恒 14（主链固定，胁迫作浅层旁挂乘子接到 DDM）、社区/节点比只微升 0.34→0.40 → 新机理是「新叶子外挂」、几乎不重组已有结构。`DDM`（=DDM_pot·f_N·f_W·f_EC）是全胁迫乘性汇合枢纽、介数第一 = 标定/敏感性的中央阀门。
4. **不可辨识的都是「阈值/临界常数」**（Ca_crit_fruit / Brix_grade）：纯常数除数/门槛、下游只连未标可测的 risk 输出 → 结构够不到数据 → 靠数据定不了、只能靠先验。
5. **alg_loops=0 是诚实假象**：`_prev` 半状态计数随版本升 2→9，机理反馈被 `_prev` 破成 DAG。**结构 DAG ≠ 无机理反馈**。

### 2.4 ★对抗验证救回的 overclaim + 关键教训
- **证伪**：初步分析把「s8 的 measurable 27→13 骤降」当「可观测性突变」是**错的**。根因：s1–s7 没标 `measurable`（引擎回退数全部 output≈27）、s8 首次手标 13 个「真田间可测」白名单。若 s8 按老口径数全 output 是 36、是**上升**。→ 这是「作者引入更诚实观测模型」、不是崩塌。
- **★铁教训**：**跨版本的 measurable / 可辨识性指标必须用统一口径**，否则标注变化会伪装成结构信号。这是进化图论架构的**头号前置**。
- **收敛**：番茄糖度坑是「identify 结构分析预测」（合成孪生），不是「真田间实测」（真数据 7 月底才采）。
- **方法论价值**：这两个纠正证明「进化图论 + 对抗验证」能挖金也能自纠。

---

## 3. 架构分层（待建）

### 3.1 数据层 —— 显式 lineage
- **已首用**（2026-07-03）：strawberry S8.1 写了 `meta.version:"8.1"` + `meta.lineage:{parent:"STRAWBERRY_S8@53f8e4f", step:"..."}`。EQC serde 现忽略未知字段=不崩。
- **待定**：lineage 存哪？
  - (A) 每个模型内 `meta.lineage`（parent 指 git ref）——已起步；但旧版本文件已删、要回填靠独立清单。
  - (B) 独立**进化清单** `evolution.yaml`（一个 DAG：节点=版本 ref，边=parent，边带 step 标签）——不依赖旧文件在工作区、天然支持分支（GP）。**推荐 A+B 混合**：存活正式版带 meta.lineage，全谱系 + 分支在 evolution.yaml。
- **schema 落地**：给 `Metadata` 加 `pub lineage: Option<Lineage>`（现被 serde 忽略）+ `export.rs` 导出 version/lineage 到契约（现 `module_json` 丢了 version）。这是「meta.version 导出」推后项的正式落地。

### 3.2 分析层 —— evo_metrics 升级
- 把临时 `examples/evo_metrics.rs` 升级为正式能力：
  - **★统一 measurable 口径**（头号前置）：跨版本用同一口径（要么全 output、要么统一白名单），否则可辨识性轨迹被标注污染。
  - 输出完整指标 + **具体明细**（哪对系数共线、哪个阈值参数不可辨识、对应哪层机理）。
  - 复用 `diff_models` 做**版本间结构 diff**（加了哪些边/方程/参数）。
- 产出：一条进化链的「指标轨迹 + 标定坑清单 + 版本 diff」。

### 3.3 呈现层 —— serve 端点 + Studio 进化视图 + 进化动画
- **serve 新端点**（复用现有 source 读写 + git 底座）：
  - `GET /api/history?model=` —— 该模型文件的 git 历史（`git -C <repo> log`）。
  - `GET /api/source?model=&rev=` —— 某历史版本源码（`git show rev:file`）。
  - `GET /api/evolution?model=` —— 沿血缘的图论指标轨迹 + 版本 diff（analysis 层产出）。
  - （回退：`git checkout` 或走已有 `write_source` 带校验，人在环确认）。
- **Studio 进化视图**：版本 picker + 「选两个版本 → 看结构 diff + 标定坑增量 + 沿轨迹指标」。承分支路线 Tier1（只读历史/diff·~1-2 天）。
- **进化动画**：沿血缘路径逐版本回放 diff（s1 长成 s2..s8.1，每步高亮新增子图）——区别于现在的生长动画（单模型子系统 reveal）。

### 3.4 GP 层 —— 候选作分支 + 图论筛选
- GP 候选现在临时不落盘、采纳靠手动复制粘贴。分支模型把候选变一等公民：「采纳候选」写盘 + commit 到 `gp/<target>/<candidate>` 分支，`structure_diff` 变真·分支 vs main diff。
- **图论指标作 GP 候选筛选维度**：哪个候选图更简约/更符合守恒/更可辨识 → 喂给「GP 方程筛选交给 Claude Code」（承 [[analytical-agent-design]]）。这是本 arc 和 GP 主线的接口。

---

## 4. 分阶段路线

| 档 | 内容 | 估算 | 前置 |
|---|---|---|---|
| **地基** | 统一 measurable 口径 + evo_metrics 升级正式 + `meta.lineage` schema + version 导出契约 | ~2-3 天 | 头号：统一口径 |
| **Tier 1** | serve `/api/history` `/api/source?rev` `/api/evolution` + `diff_models` 版本 diff + Studio 只读进化视图 | ~2-3 天 | 地基 |
| **Tier 2** | 回退/checkout（走 write_source 带校验 + 人在环确认） | ~1-2 天 | Tier1 |
| **Tier 3** | GP 候选写盘 commit 到分支 + 分支 picker + 图论筛选喂 Claude Code | ~1-2 周 | Tier1 + GP 主线 |
| **进化动画** | 沿血缘回放 diff（派生非快照） | ~2-3 天 | 地基 + Tier1 |

---

## 5. 关键决策 / 待定问题（一周协作要拍板的）
1. **lineage 存哪**：meta.lineage（模型内）vs evolution.yaml（独立清单）vs 混合。→ 倾向混合。
2. **版本节点粒度**：一个 git commit？一个 meta.version bump？一个语义里程碑？→ 倾向「显式里程碑」（bump version = 一个进化节点，不是每 commit）。
3. **★统一 measurable 口径怎么定**：全 output（宽）vs 统一白名单（要给旧版本回填标注）。→ 影响所有可辨识性轨迹。
4. **5-repo 分散**：模型散在 Eqc_test/crop-models/greenhouse-model/strawberry-model/gis 五个 repo，「整 workspace 一个分支」无意义 → git 调用 per-repo；进化图是 per-model 的。workspace 本身（根目录）非 repo、也没版本化。
5. **result.json 过期**：回退模型不重跑 `-o` → 决策面板显旧数字。承 [[model-evolution-traceability]]「改模型必重跑 -o」纪律。

---

## 6. 与标定 / GP / GIS 的接口
- **标定**（7 月底数据）：本 arc 的最硬即时产出 = **给标定一份「结构坑清单」**（哪几对系数共线、哪几个阈值参数不可辨识、要设计什么正交/多工况对照）。承 [[crop-decision-optimization-arc]] 的 identify/多处理标定工作——图论从结构侧、identify 从灵敏度侧，两路互证坑。
- **GP**：图论指标作候选筛选维度（Tier3）。
- **GIS/Studio**：进化视图接进统一外壳（承 [[gis-studio-unification-arc]]）。

---

## 7. 临时产物（本 session 留下）
- `equation-compiler-main/examples/evo_metrics.rs`（untracked·分析器雏形·零核心改动）——**是否提交进 eqc repo 当 Tier0 起点待定**。
- `scratchpad/evo/`（恢复的 10 版本·临时·可删）。

---

## 8. 诚实边界 / 风险
- 样本仅**一条链**（草莓）、合成天气、参数多占位——模块度等绝对值只可比、不可作科学定论。要推广到番茄/蓝莓链验证规律普适性。
- 图论指标是**描述性、软的**（EQC metrics.rs 自己的诚实定位）：价值在「绑定到具体问题」（枢纽=计算瓶颈、社区=对照 meta.modules、可辨识=标定设计），不是硬定论。
- **统一 measurable 口径**没做好 → 整个可辨识性轨迹不可信（本 session 已踩过一次）。

---

*相关记忆：[[crop-decision-optimization-arc]]（末尾「进化图论最小验证片」段）· [[model-evolution-traceability]]（lineage 纪律首用）· [[eqc-graph-analysis-arc]]（图论引擎）· [[analytical-agent-design]]（GP 筛选交 Claude Code）· [[gis-studio-unification-arc]]。*
