# 审计：单一真相源一致性 + FSPM 前置条件

> 目的：在把 Functional 模型演进到 **FSPM（功能-结构植物模型）** 之前，核实"改模型时下游是否自动派生、不用各自维护"这一单一真相源承诺到底守在哪条线上，并列出 FSPM 不变成"屎山代码"必须先固化的前置条件。
> 方法：2026-06-27 逐文件审计（4 路并行：后端派生链 / 配色+算子规范源 / 前端注册表 / FSPM 标量假设），全部 file:line 证据。**本文档只记录现状与建议，未改动代码。**

## 0. 总结论

**计算内核的单一真相源贯彻得很彻底**：解析后的 `EquationFile` 是唯一模型对象；分类、命名、结构、配色、步进、算子、前端契约各有**唯一权威**且被广泛复用，下游无一重建模型结构。证据：本轮加 `gpdemo3` 一个新模型，2D 报告 / 3D 拓扑 / 生长动画 / 结构分析 / GP 面板 / 前端 AI / 结构 diff **全部白送**，没碰这些模块一行代码。

**但有 ~8 处真实债**（多为展示/叙事层的旁路与漂移，均有兜底、非当前 bug），以及 **FSPM 会正面压测的 5 个结构维度缺口**（schema→图层全程没有"器官实例 + 拓扑"概念）。下面分别登记。

---

## 1. 传播图：改模型 → 自动派生（单一真相源成立的部分）

唯一真相源是一条清晰分层链，无第二套模型表示：

| 维度 | 唯一权威 | 文件:行 |
|---|---|---|
| 数据真相 | `EquationFile`（meta/parameters/variables/equations，全 IndexMap 保序） | `schema/equation_file.rs:9` |
| 分类真相 | `Variable::effective_class/is_dynamic/is_integrator/is_delay` | `schema/variable.rs:222` |
| 友好名真相 | `EquationFile::display_name()` | `schema/equation_file.rs:151` |
| 命名/折叠真相 | `NodeResolver`（`MODULE.name` + 跨模块 source 折叠） | `graph/bipartite.rs:24` |
| 结构真相（有向） | `DiGraph::from_files` | `graph/digraph.rs:36` |
| 结构真相（无向） | `BipartiteGraph::from_files` | `graph/bipartite.rs:92` |
| 子模块真相 | `compute_submodules`（build_dag 后置，下游回读 `dag.node.module`） | `dag/builder.rs:371` |
| 配色真相 | `palette::class_color` + `MODULE_VIVID` + `module_slots` | `palette.rs:17,87` |
| 算子真相 | `ops::OperatorSpec`（eval + 3 套 codegen 共用） | `ops/mod.rs:17` |
| 步进真相 | `SimPlan` / `Stepper`（`simulate` 与 `eqc build` 共用） | `sim/mod.rs:162` |
| 前端契约出口 | `export::to_model_json` / `ModelJson`（schema_version 守版本） | `export.rs:129` |

**派生消费者**（全部从 `load_model_files` 拿同一份 `EquationFile` 出发）：`/api/model`、`/api/report`、`/api/layout3d`、`/api/growth`、`/api/simulate`(+耦合)、structure/diff、`/api/optimize|calibrate|evolve`、validate——无一重建依赖图或重新解析分类；想要"作者子系统"时统一回读 `build_dag` 的 `node.module`。前端 3D 优先读契约（class_colors/module_color），本地常量仅作回退。

**原则**：加"模型内容"（变量/方程/机理/整个新模型）= 白送；加"新能力词汇"（算子/语法/求解器/新派生信息）= 一处、共享的扩展点。

---

## 2. 债务登记（按性价比排序）

### 后端
- **B1〔中·最该先修〕双份积分/延迟边。** `rate→state`、`prev→semistate` 边被推导两遍：`DiGraph::from_files`（`graph/digraph.rs:63`）和 `forrester_svg`（`report/mod.rs:462`）。语义现一致，但边规则一改就得改两处，否则 2D Forrester 图与 3D/分析层结构悄悄分叉。建议抽 `integration_edges(files)` helper 两边共用。
- **B2〔低-中〕growth 旁白名硬编码于 Rust。** `growth_narration`（`export.rs:398`）把 ~18 个子系统中文名+文案 `match` 死在 Rust 里——这是全库唯一把"模型特定知识"烤进后端的地方。作者改 `meta.modules` 键名就得回 Rust 改表（有通用兜底句）。正解：旁白挪进 `.eq.yaml` 的 modules 声明。
- **B3〔低〕模块级配色旁路。** `report/mod.rs:228 module_palette()` 用 HSL 自成一套子系统色（只服务模块级 dag_svg），与 palette.rs 的 MODULE_VIVID 是两份真相 → 同一子系统在"模块级"和"变量级按子系统"颜色不一致。建议收编进 `palette`。

### 算子 codegen
- **O1〔中·哑弹〕156 个漂移的死分支。** `expr.rs` 三套 codegen（to_python:4158 / to_rust:4801 / to_latex:5445）在快路径 return 后，仍保留这 52 算子的旧 `match` 分支（共 52×3，不可达）。**其中至少两处语义已与注册表漂移**：`to_rust` 的 `Mod => "rem_euclid"`（`expr.rs:4808`，注册表是 floored 块、测试明确否决 rem_euclid）、`Sign => "signum"`（`expr.rs:4813`，注册表是 sgn(0)=0 块）。当前被快路径屏蔽=正确，但若有人删快路径或 `as_operator` 漏算子，会静默生成错误代码。**只删与 `as_operator` 白名单重合的 52 个 arm**（保留未迁移的特殊函数/向量/微积分分支）。

### 前端
- **F1〔中〕命令注册表能力漂移。** 注册表覆盖了读模型+仿真情景+优化/标定+落盘管理这条主线，但有系统性缺登：**GP 运行全套**（靶点多选/配置/开始进化/点 Pareto，`Gp.svelte:38`——AI 完全无法发起进化，与优化/标定有 run_* 命令不一致，**最大盲区**）、**耦合工作区**（`Couple.svelte`——连 `go.couple` 导航命令都没有）、**录入保存观测**（`Entry.svelte:41`，而同类 save_zone_management 已登记）、**2D 图 level/layout/zoom**（`Structure.svelte` 本地 $state，同一工具栏配色能被 AI 控、粒度/布局/缩放不能）。
- **F2〔低〕契约漂移。** `contract.ts` 的 `VarJson` 漏镜像后端已发出的 `rate?`/`prev?` 两字段（`export.rs:103`）——违背"对着 export 写类型、编译器抓漂移"的承诺。
- **F3〔低〕僵尸 store 状态。** `store.topoHasModules`（`store.svelte.ts:17`）被 Topology3d 写入但**全库无人读取**（真正消费方 Structure 直接读 `contract.has_modules`），注释还误导。建议删。
- **F4〔低〕knob kind 中文标签硬编码三处**（KnobTable/Understand/store）。轻度重复，可收口。
- 非债澄清：`annotate.ts` 的 `CLASS_COLOR_3D`/`MODULE_PALETTE` 是逐值对齐 palette.rs 的**文档化回退**（运行时优先契约），可接受。

---

## 3. FSPM 前置条件（最重要：防屎山的根）

**计算内核对 FSPM 友好、可直接生长**：`Value::{Scalar,Vector,Matrix}` + 广播内核（`eval/mod.rs:66,502`）、算子注册表、SimPlan 单一真相源、additive 契约纪律、NodeResolver 命名咽喉、cohort 的"声明族→实例化"骨架——这些是现成资产，不必返工。

**但"结构维度"从 schema 到图层全程缺失**。当前两套多重性机制都在进入下游前把多重性**擦成字符串后缀**：① cohort 宏展开成 `name__i` 标量（`parser/cohort_expand.rs:397`，刻意降维、展开后下游不知道它们曾是一族）② `Value::Vector` 是匿名定长数组、只有长度无身份（`eval/mod.rs:67`）③ 输出期 `flatten_into` 展平成 `name[i]` 字符串键（`sim/mod.rs:710`）。三者**没有共同的实例身份概念**，靠字符串约定缝合。器官/节间/分枝/3D 几何概念在 `src/**` **零存在**。

### 5 个前置风险（动手 FSPM 前该先固化，按优先级）
1. **〔最高〕实例身份没有单一真相源。** 散在三种字符串编码里。**前置动作**：schema 引入"器官实例/实体维度"的一等表示（新顶层段如 `entities:`/`topology:`，让"果序#2"成为有 id 的对象，三套机制都引用同一身份）。否则字符串拼接会炸成屎山。
2. **〔关键杠杆〕`NodeResolver` 是图层命名唯一咽喉，实例维度必须在这里植入。** 一处实现"实例感知命名"，所有图/可视化/契约自动一致；绕过它在某消费者自己拼实例名 = 屎山起点。**顺带先修一致性裂缝**：`dag/builder.rs` 目前自己 `format!("{module}.{name}")`、没走 NodeResolver，先收编。
3. **AST 缺结构量词/拓扑聚合算子。** `sum_over` 是 YAML 宏、反序列化前就展开掉、AST 看不到（`cohort_expand.rs:347`）。FSPM 的"Σ over children(organ)"必须是**一等 AST 算子**（这样 eval/codegen/MathML/图依赖统一处理），配 eval 实现（架在风险1的身份表+现成广播内核上）。这是统一 cohort 与向量两套机制的关键一步。
4. **单 rate 标量积分假设。** `PlanStep::Integrator{rate:&str}`（`sim/mod.rs:158`）+ `Variable.rate: Option<String>`（`variable.rs:195`）锁死"状态变化=单一速率"。FSPM 器官状态是多源（生长+Σ流入−Σ流出）。前置：把 rate 泛化成可聚合，或确立"拓扑流先经一条聚合方程汇成单 rate"约定（依赖风险3）。
5. **真实空间几何彻底缺席。** `layout3d` 的 x/y/z 是**力导向布局坐标、非植株坐标**（`layout3d.rs:26`）。FSPM 的 3D 几何/光竞争需要器官真实空间位置——全新维度、无可复用资产，但与现有 layout3d 正交、可独立新建（**别复用 layout3d 坐标，会语义污染**）。

> 一句话：FSPM 的前置工作不是"加功能"，而是**先确立"器官实例身份 + 拓扑"这个新的单一真相源**（落在 schema 新段 + NodeResolver + 新 AST 聚合算子三处），再让现有内核沿它生长。最不能拖到中途再补的是**风险1（实例身份）和风险2（NodeResolver 收编命名）**。

---

## 4. 建议的施工次序（待讨论）

- **第一档·低风险清债（可立即做，FSPM 无关也该修）**：F3 删僵尸 `topoHasModules`、F2 补 `contract.ts` rate/prev、O1 删 156 个漂移死分支（含修 mod/sign 哑弹）、B1 抽 `integration_edges` helper 收双份边。
- **第二档·补能力漂移**：F1 给 GP 加运行命令 + 给 Couple 加 `go.couple`（+ 视情补 Entry 保存、2D level/layout/zoom）；B2 旁白下沉到模型；B3 模块级配色收编。
- **第三档·FSPM 地基（设计先行、不急着写）**：风险1+2（实例身份 schema 段 + NodeResolver 实例维度）作为 FSPM 的第一张 spec；风险3（AST 拓扑聚合算子）；风险4（多源 rate）；风险5（真实几何，独立新模块）。

**未决**：数据就绪演练（真数据格式 → 录入 → 标定 → GP → 看 diff → 采纳的端到端 dry-run）——等气象/作物数据到（~7月）再做，与本架构审计正交。
