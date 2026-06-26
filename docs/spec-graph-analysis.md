# spec：模型图论分析 + 3D 拓扑（草案 v1，待首席科学家批准）

> 理论背景见 [`theory-model-graph-analysis.md`](theory-model-graph-analysis.md)。
> 设计立场（与全项目一致）：**EQC 持有事实**（图指标 / 3D 坐标 = 契约**只增不改**的新字段），**前端只渲染**；
> 纯 Rust、数据无关、可单测、合成数据优先验证；不破坏现有任何路径。

---

## 1. 动机

- **方法论贡献**：把模型当图来严谨分析——适定性/求解结构（结构分析）+ 枢纽/社区/演化（网络分析）。单一真相源架构让这几乎免费（AST 即图）。
- **喂养已有 arc**：① 结构可辨识性 → 互补 `eqc identify`（标定）；② 版本图 diff → 量化 GP 进化的结构变化；③ DM 分解 → 严谨化拓扑排序/环检测、定位代数环（接隐式求解器缺口）。
- **驱动 3D 可视化**：网络指标（中心性/社区/演化代际）正是 3D 布局的"颜色/大小/分层"输入；GP 生长动画 = 图 diff 的视觉对应物。**2D 仍为默认分析视图，3D 做补充（总览 + 生长动画）。**

## 2. 范围 / 非目标

**做**：二部图 + 匹配 + DM 分解；结构可辨识性（图论必要条件版）；中心性/社区/图 diff；3D 力导向坐标（Rust 算）+ 契约字段。
**不做（本 arc）**：完整微分代数可辨识性（交 SIAN 类）；隐式/刚性数值求解器（另案，本 arc 只**定位**代数环不求解）；前端 3D 渲染实现（单独 Studio 子 arc，本 arc 只产坐标）；精确 GED（用带标签的版本对齐近似）。

## 3. 架构（两层 + 一个可视化出口）

```
src/graph/                         ← 新模块（cli 或默认均可，纯算法无 IO）
  ├─ bipartite.rs   二部图构造（从 EquationFile，复用 collect_refs）+ 关联矩阵
  ├─ matching.rs    最大匹配（Hopcroft–Karp）+ 完美匹配判定
  ├─ dm.rs          Dulmage–Mendelsohn 分解（欠定/方定/超定 + 方定块 SCC 块三角）
  ├─ identifiability.rs  结构可辨识性（可达性 param→measurable、混淆候选）
  ├─ metrics.rs     网络指标（度/介数/PageRank 中心性、社区/模块度、DAG 深度）
  ├─ diff.rs        版本结构 diff（带标签节点对齐 → 增删点/边 + 距离）
  └─ layout3d.rs    3D 力导向坐标（扩 report/layout.rs 的思路到 z 轴）
```
- 复用：`dag::build_dag`（已有节点/边）、`petgraph`（SCC/拓扑/最短路）、`optimize::identifiability`（数值版，做对照/融合）、`schema`（measurable/gp_target/meta.modules）。
- **契约（`export.rs`）只增字段**：`schema_version` 不变；新字段 `#[serde(skip_serializing_if)]` 缺省省略，老前端不受影响。

## 4. 分阶段（GA = Graph Analysis；每阶段 cargo test 绿、可独立交付）

### GA-1 结构分析地基（二部图 + 匹配 + DM）——**最高优先** ✅ 已落地
> 实现：`src/graph/{bipartite,matching,dm}.rs` + `src/validator/structural_checker.rs` + CLI `eqc structure [--json]` + `export.rs::StructureJson`。
> 取舍：用作者 `output:` 给方定块定向做 SCC 块三角（与现有计算语义一致），另用 Hopcroft–Karp 做独立结构奇异性检查。
> 诚实边界：单文件重复 output / 代数环现有 validate 已覆盖；本轮 validate 只补**跨模块系统级过定**这一新缺口。其余结构信息走 `eqc structure`。
> 关键洞察：动态模型状态量本步是携带自由变量，结构分析正确分离「本步代数依赖」与「跨步状态耦合」。

- `bipartite.rs`：`EquationFile → BipartiteGraph{eqs, vars, edges}`（边 = 方程 refs；参数/驱动量/状态量都算变量节点）。
- `matching.rs`：Hopcroft–Karp 求最大匹配；与作者 `output:` 指派对照（一致？是否唯一？结构是否奇异？）。
- `dm.rs`：DM 三块分解 + 方定块 SCC（petgraph）→ `StructureReport{ free_vars, solve_blocks(块三角顺序), algebraic_loops(SCC>1 的块), over_determined, under_determined }`。
- **CLI**：`eqc structure <model>` 打印报告；并把"过/欠定、结构奇异"接入 `eqc validate`（结构 bug 跑前即报）。
- **契约**：可选 `StructureJson`（自由变量、求解块顺序、代数环块）——前端可据此画"求解顺序/代数环"高亮。
- **验证**：合成模型已知 DM 结果（链式=全单点块三角；故意造环=一个 SCC 块；故意写重/漏方程=过/欠定）逐一对拍。在真模型（草莓 S4/S8）上跑，与现拓扑排序一致 + 报出代数环（若有）。

### GA-2 结构可辨识性（图论版，互补 identify）✅ 已落地
> 实现：`src/graph/identifiability.rs`（专建有向影响图，含 rate→state / prev→semistate 积分延迟边）+ CLI `eqc structure --identifiability` + `StructureJson.identifiability` 可选字段 + `bipartite.rs::NodeResolver`（节点命名单一真相源，GA-1/2 共用）。
> 混淆判据：进入完全相同方程集合（局部、便宜、necessary-not-sufficient，喂数值版确认）。可测集回退 type:output。
> 真模型草莓 S4：正确标出 {Cref,Kc} 混淆候选（同进 CO₂ 响应式），= 理论预言现象。

- `identifiability.rs`：给 `measurable` 集，算每个参数到任一可测输出的**可达性**（不可达 = 结构不可辨识）；找**混淆候选**（参数对沿相同路径集影响相同观测）。
- **CLI**：`eqc identify` 增 `--structural`（图论先验筛）段，或新 `eqc structure --identifiability`；与数值版交叉（图说"可能混淆"，数值敏感性确认）。
- **契约**：可选 `identifiability: {unidentifiable:[...], confounded_pairs:[...]}`。
- **验证**：合成模型（造一个 `Y=F·Pd` 型混淆、一个 CO₂≡参考点的不可辨识参数）→ 图法应正确标出，且与 `eqc identify` 数值结论一致。

### GA-3 网络指标（中心性 + 社区）✅ 已落地
> 实现：`src/graph/digraph.rs`（共享有向影响图，GA-2/3/5 复用）+ `src/graph/metrics.rs`（度/介数Brandes/PageRank + 破环最长路深度 + 单层 Louvain 模块度）+ CLI `eqc structure --metrics` + `StructureJson.metrics`。
> 全确定性无 RNG。真模型枢纽命中：草莓 S8 = DDM（介数居首），温室 v1 = rate_T/T_air。
> 诚实边界：单层 Louvain 在树状计算图上会碎（S8→40 社区）；多层聚合本轮未做（不影响枢纽/Q 对照）。

- `metrics.rs`：度/介数（Brandes）/PageRank 中心性；社区发现（Louvain 式贪心模块度，或先用 `meta.modules` 当 ground-truth 验证）；DAG 深度。
- **契约**：每节点加可选 `centrality`、`community`（additive，喂 3D 大小/分色 + 找枢纽）。
- **验证**：合成图已知中心性/社区（星形=中心介数最高；两团一桥=两社区）对拍；真模型上社区与手标 `meta.modules` 的吻合度（模块度/NMI）。

### GA-4 版本结构 diff（喂 GP "进化"）✅ 已落地
> 实现：`src/graph/diff.rs`（共享 `DiGraph` 上三层 diff：节点/边/方程）+ CLI `eqc diff <旧> <新> [--json]` + `GraphDiffJson`。
> 对齐键 = 本地名（去模块前缀，跨版本 meta.id 不同也对齐）；changed 方程用 Debug 串指纹（refs 同形式异也能抓）。distance=图编辑数、edge_similarity=边 Jaccard。
> GP evolve 接线未做（留 GP arc 调 `diff_models`）。真模型 S4→S8：distance=145，捕获钙/氮/蒸腾/EC 子系统 + DDM 形式改变 SB-03→SB-NPROD。

- `diff.rs`：两个 `EquationFile`（或 GP patch 前后）→ 带标签节点对齐 → `{added, removed, kept}` 点/边 + 结构距离标量。
- **CLI**：`eqc diff <modelA> <modelB>`（结构层）；GP `eqc evolve` 采纳后可选输出"这次进化的结构 diff"。
- **契约**：`graph_diff`（给 3D 生长动画 + 溯源体系记录"长出了什么"）。
- **验证**：对同一模型加一条方程/参数 → diff 精确报出新增节点/边 + 距离。

### GA-5 3D 坐标（Rust 算，前端渲染的输入）✅ 已落地
> 实现：`src/graph/layout3d.rs`（3D FR，扩展 2D 力导向）+ CLI `eqc structure --layout3d` + `StructureJson.layout3d`。
> 确定性无 RNG（坐标逐位可复现）。深度软锚定 z（C 决策）+ 社区质心簇位 + 介数定 size；归一化 [-1,1]³。
> 真模型 S8：104 节点坐标合理，枢纽 DDM size=1.0。渲染本身见 GA-6。

- `layout3d.rs`：把 `report/layout.rs` 的力导向扩到 3D（z 轴），**确定性**（golden-angle/无 RNG，可复现，同现有 2D 力导向做法）；可被 GA-1/3 指标驱动（模块→簇位、中心性→建议大小、代际→建议色）。
- **契约**：`/api/report` 或新 `/api/layout3d?...` 输出每节点 `{x,y,z}` + 边 + 指标。**坐标在 Rust 算 = 单一真相源不破**。
- **非目标**：渲染本身（见 GA-6）。
- **验证**：坐标确定性（同输入逐位一致）、无 NaN/越界、连通分量不重叠（沿用 2D 力导向的帧约束经验）。

### GA-6 前端 3D 渲染（**单独 Studio 子 arc，首次碰前端**）✅ v1 已落地
> 实现：后端 `/api/layout3d?model=`（薄端点，`serve.rs` 复用 `export::layout3d_json_string` + `graph::layout3d`，每请求新鲜算）；前端 `frontend/src/components/Topology3d.svelte`（three.js）；`Structure.svelte` 加「2D 报告 ↔ 3D 拓扑」切换（2D 默认，视图态提到 `store.structureView`）；命令 `view_topology_3d`/`view_structure_2d` 入注册表（⌘K + AI 同获）。注释/配色逻辑抽到 `lib/annotate.ts`（与 2D 共用单一真相源）。
> 决策（用户已批准 A–G）：A=raw three.js（非 threlte）；B=Structure 工作区内切换、2D 默认；C=v1 交互 轨道(转/缩/平)+节点球+边+hover 注释+点选联动 `store.selectedVars`；D=GP 生长动画拆 **GA-6b**（本轮不做）；E=节点按 Forrester 类配色（复用 2D 报告配色）、size∝介数；F=`/api/layout3d` 端点；G=★命令登记。
> 验证：svelte-check 0 错；`npm run build` 出 `studio_v2.html`；两 feature 配置 284 lib 绿（端点 additive、零回归）；`/api/layout3d` 真模型实测（草莓 104 节点 / 温室耦合 148 节点，坐标∈[-1,1]）；Playwright 冒烟（`e2e/topology3d.spec.cjs`：切 3D → canvas 挂载 + 无运行时错误）。**3D 渲染美观度由用户肉眼确认。**
>
> **GA-6 增强（配色模式 + 图例 + 可读性）✅ 已落地**：让非专家看懂。
> - **按子系统配色**：契约 `Node3dJson` 加 additive `module`（`schema_version` 仍=1），`graph/layout3d.rs::node_modules` 复用 `dag::build_dag` 子模块字段——**只取作者 `meta.modules` 显式命名的子系统**，自动桶（参数/驱动/未分组）回 `None`。关键判断：用 `meta.modules`（草莓 9/温室 5，有名可写图例）**而非 GA-3 Louvain**（单层在 S8 碎成 ~40、不可读、写不出含义）。纯前端不可行的原因：契约 `modules[]` 是**文件级**（单模型=1 个），子系统藏在文件内 `meta.modules`、原本不在契约里 → 故加后端字段。未声明子系统的模型优雅降级（禁用/回退按类别）。
> - **前端**：`store.topoColorMode`（默认 class）；`Structure.svelte` 加「按类别/按子系统」分段控件；`Topology3d.svelte` 左下角可折叠**常驻图例**（只列出现的项，按类别带一句话含义）；配色/含义文案全在 `annotate.ts` 单一真相源（`CLASS_LEGEND`/`MODULE_PALETTE`/`moduleColorMap`）。
> - **可读性**：辅助 vs 参数两灰拉开明度（仅 3D 副本 `CLASS_COLOR_3D`）；半立体光照（球 `emissive=自身色`+暗光，不洗白）；选中改白描边光环。
> - **★命令** `set_topology_color_by`（`mode: class|module`）入注册表。验证：svelte-check 0 错、build 出包、两配置 **286 lib 绿**（+2 测试）、Playwright 冒烟扩一条（配色切换/图例存在/折叠）。
>
> **2D↔3D 配色统一（Forrester 合并）✅ 已落地**：用户要"2D/3D 同一套配色、减少两套维护"。
> - **单一真相源** `src/palette.rs`：16 基准鲜色（=3D 深底）+ 按 `meta.modules` **声明顺序**分槽 + 2D 浅调由鲜色**向白混合**派生（保留色相+相对饱和，胜过纯色相——棕/橙色相近混白后仍可分）。契约 `Node3dJson.module_color`（鲜调，additive）；3D 改读它。同子系统 2D 浅 / 3D 鲜、同色相。
> - **2D Forrester 加 `ColorMode`**：按子系统时 **形状=类别**（保留虚实线）、**色=子系统**（浅调填充+中性描边）、主标签**中文名**；图例随模式。冗余的「依赖关系图(DAG)」从变量级**退役**并入此模式；方程级粒度退役。`/api/report` 加 `color=`；契约 `ModelJson.has_modules` 驱动切换启用态；2D 变量级与 3D 共用 `store.topoColorMode`。
> - 验证：两配置 **288 lib 绿**（+2 palette 测试）、Playwright +1（2D 配色切换改 iframe `color=`）、截图肉眼（2D 浅/3D 鲜同色相、Forrester 形状+中文名）。**遗留**：8 类 Forrester 色仍 Rust CSS + TS 两处（低频、未纳入；模块色已统一）。
- 立场重申：**2D（现有 SVG 报告/Forrester）仍是默认分析视图**；3D 是总览 + 生长叙事的补充。3D 需 WebGL（Studio-only、不进零-JS 离线报告）。
- **GA-6b（待做）**：GP 采纳/进化时按 GA-4 diff 播"长出新枝"动画——需 GP arc 活跃 + 两版本 diff 端点。

## 5. 设计决策（草案，待确认）

1. **结构分析优先于网络分析**（前者硬、直接喂 identify/GP/隐式求解器；后者软、主要驱动可视化）。先 GA-1/2，再 GA-3/4，最后 3D。
2. **复用 petgraph**（SCC/拓扑/最短路）而非自造；匹配/DM/Brandes 自实现（petgraph 未必全有，且要可控可测）。
3. **契约只增不改**：所有图指标/坐标都是 `export.rs` 的可选新字段，`schema_version` 不动，老前端零回归。
4. **3D 坐标在 Rust 算**（不在前端），守单一真相源；前端只渲染。
5. **2D 默认、3D 补充**（不取代）。
6. **合成数据优先验证**（已知 DM/中心性/可辨识性/diff 的玩具图逐一对拍），类比标定/GP 的"复原已知"节奏。
7. **不碰数值求解**：本 arc 只**定位**代数环（DM），隐式/刚性求解另案（[[eqc-implicit-solver]] 缺口）。

## 6. 与现有的接口

- `validate`：吸收 GA-1 的过/欠定 + 结构奇异检查。
- `identify`：GA-2 图论可辨识性作先验段，与数值版互补/交叉。
- `evolve`（GP）：GA-4 diff 记录每次进化的结构变化，回流溯源体系（"长出/改写了什么形式"）。
- `report` / Studio：GA-3 指标 + GA-5 坐标驱动 2D 增强（求解块/代数环/枢纽高亮）与 GA-6 的 3D。

## 7. 里程碑 / 验收

- GA-1 落地即可发一个小里程碑（结构报告 + validate 增强 + 合成对拍）。
- 全程 `cargo test`（两 feature 配置）绿；真模型（草莓/番茄/蓝莓/温室）跑通且与现拓扑/identify 结论一致；契约 `schema_version` 不变、老前端零回归。

## 8. 风险 / 诚实边界

- **结构 ≠ 数值**：结构分析给必要条件，不替代数值奇异/病态检查。
- **网络指标的"玄学"风险**：必须问题驱动（绑定到 identify/GP/枢纽定位），不堆砌好看的数。
- **3D 可读性权衡**：3D 缓解拥挤 + 炫，但精细读取通常不如 2D（遮挡/深度歧义）；故 2D 默认、3D 补充。
- **图论可辨识性是必要非充分**：重判定仍交微分代数（SIAN 类）。
