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

### GA-3 网络指标（中心性 + 社区）
- `metrics.rs`：度/介数（Brandes）/PageRank 中心性；社区发现（Louvain 式贪心模块度，或先用 `meta.modules` 当 ground-truth 验证）；DAG 深度。
- **契约**：每节点加可选 `centrality`、`community`（additive，喂 3D 大小/分色 + 找枢纽）。
- **验证**：合成图已知中心性/社区（星形=中心介数最高；两团一桥=两社区）对拍；真模型上社区与手标 `meta.modules` 的吻合度（模块度/NMI）。

### GA-4 版本结构 diff（喂 GP "进化"）
- `diff.rs`：两个 `EquationFile`（或 GP patch 前后）→ 带标签节点对齐 → `{added, removed, kept}` 点/边 + 结构距离标量。
- **CLI**：`eqc diff <modelA> <modelB>`（结构层）；GP `eqc evolve` 采纳后可选输出"这次进化的结构 diff"。
- **契约**：`graph_diff`（给 3D 生长动画 + 溯源体系记录"长出了什么"）。
- **验证**：对同一模型加一条方程/参数 → diff 精确报出新增节点/边 + 距离。

### GA-5 3D 坐标（Rust 算，前端渲染的输入）
- `layout3d.rs`：把 `report/layout.rs` 的力导向扩到 3D（z 轴），**确定性**（golden-angle/无 RNG，可复现，同现有 2D 力导向做法）；可被 GA-1/3 指标驱动（模块→簇位、中心性→建议大小、代际→建议色）。
- **契约**：`/api/report` 或新 `/api/layout3d?...` 输出每节点 `{x,y,z}` + 边 + 指标。**坐标在 Rust 算 = 单一真相源不破**。
- **非目标**：渲染本身（见 GA-6）。
- **验证**：坐标确定性（同输入逐位一致）、无 NaN/越界、连通分量不重叠（沿用 2D 力导向的帧约束经验）。

### GA-6 前端 3D 渲染 + GP 生长动画（**单独 Studio 子 arc，本 spec 只列出口**）
- Studio 加 3D 视图（three.js / Svelte 的 threlte）：消费 GA-5 坐标 + GA-3 指标渲染；GP 采纳/进化时按 GA-4 diff 播"长出新枝"动画。
- 立场重申：**2D（现有 SVG 报告/Forrester）仍是默认分析视图**；3D 是总览 + 炫示 + 生长叙事的补充。3D 需 WebGL（重 JS、Studio-only、不进零-JS 离线报告）——可接受，因 Studio 本就是交互 JS。
- 待 GA-1~5 落地、坐标/指标契约稳定后再开工。

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
