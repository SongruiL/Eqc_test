# 模型的图论分析 —— 理论学习笔记

> 给 EQC「模型图论分析 + 3D 拓扑」arc 打地基用的理论参考。目标读者：数学功底够、想深挖的人。
> 全程对着 EQC 落地（EQC = 把方程编译成 AST/DAG 的单一真相源建模工具）。
> 配套实现见 [`spec-graph-analysis.md`](spec-graph-analysis.md)。

---

## 0. 先把名字理清（一张地图）

底层是**图论（graph theory）**。在它之上，对「方程系统」有两套味道不同的应用，**严谨度和用途都不一样**：

| 层 | 名称 | 对象 | 回答什么 | 严谨度 |
|---|---|---|---|---|
| ① | **结构分析（structural analysis）** | 变量-方程**二部图** | 适定性、求解顺序、代数环、可辨识性 | 硬（必要条件、有定理） |
| ② | **网络分析 / 网络科学（network analysis）** | 依赖**有向图** | 枢纽、社区、距离、版本差异 | 软（描述性指标，驱动可视化/探索） |

「图论分析」是两者的口语合称；写论文时分开叫「结构分析」「网络分析」。
EQC 现有的 2D 图（节点=参数/变量/方程，边=引用）是一张**依赖图（dependency / computational graph）**——网络分析直接用它；结构分析则要先把它**重组成二部图**。

---

## 1. 变量-方程二部图（结构分析的核心对象）

### 1.1 定义

**二部图** G = (E ∪ V, A)：
- 节点分两类：**方程** E、**变量** V；
- 边 (e, v) ∈ A ⇔ **变量 v 在方程 e 中出现**（结构上依赖，不管系数大小）；
- 「二部」= 边只连「方程↔变量」，方程之间、变量之间无边。

它**等价于关联矩阵（incidence matrix）** M：行=方程、列=变量、M[e,v]=1 ⇔ v 出现在 e。
于是 **图算法 ≡ 稀疏 0/1 矩阵的结构运算**——通往线性代数与稀疏矩阵理论的桥。

### 1.2 与 EQC 依赖图的关系（关键区别）

- EQC 的 DAG 是**有向**的：每个方程靠 `output:` 指向它"解出"的那个变量。
- 二部图是**无向**的：只记"谁在谁里出现"，**不预设谁解谁**。
- 二者关系：**给二部图选定一个"匹配"（每方程配一个它含的变量当输出），再定向，就得到 EQC 的有向 DAG。** 换句话说：**EQC 的 `output:` 字段 = 作者手工指定的一个匹配。** 二部图把这层"人为定向"剥掉，让你能分析**结构本身**（而不是某个特定 output 指派）。
- 构造成本几乎为零：EQC 的 `collect_refs` 已经在抽每个方程引用了哪些变量——那就是二部图的边。

### 1.3 小例子（贯穿全文）

参数 `a,b`、驱动量 `x`、方程：
```
eq1:  y = a·x
eq2:  z = y + b
eq3:  w = z·y
```
二部图（方程 ↔ 出现的变量，无向）：
```
eq1 — {a, x, y}
eq2 — {b, y, z}
eq3 — {y, z, w}
```
关联矩阵：
```
        a  b  x  y  z  w
eq1 [   1  .  1  1  .  . ]
eq2 [   .  1  .  1  1  . ]
eq3 [   .  .  .  1  1  1 ]
```

---

## 2. 结构分析的硬理论

### 2.1 匹配（matching）→ 适定性

- **匹配** = 一组两两不共端点的边。
- **最大匹配** = 边数最多的匹配；**完美匹配（perfect matching）**= 覆盖一侧全部节点（每个方程都配到一个各不相同的变量）。
- **定理直觉（适定性的结构必要条件）**：方程数 = 变量数 **且**存在完美匹配 ⇒ 系统**结构非奇异**（structurally nonsingular）。找不到完美匹配 ⇒ 结构奇异 = 某处**过定/欠定**。
- 例子里 {eq1,eq2,eq3} 各配 {y,z,w}（正是 `output`），是完美匹配 → 这三个方程对这三个变量结构适定；`{a,b,x}` 无方程匹配 → 是**自由变量**。
- 算法：**Hopcroft–Karp**，O(E·√V)，基于增广路径。
- ⚠️ **结构 ≠ 数值**：结构非奇异是**必要非充分**——结构好的系统仍可能数值奇异（系数恰好抵消）。结构分析是"便宜的先验筛子"，不替代数值检查。

### 2.2 Dulmage–Mendelsohn 分解（DM）→ 自由变量 + 求解顺序 + 代数环

有了最大匹配，**DM 分解**把二部图（= 把关联矩阵行列重排）**唯一地**切三块：

1. **欠定块（underdetermined）**：变量多于方程 → **自由变量**。EQC 里 = **驱动量 + 参数**（例中 `a,b,x`）。
2. **方定块（well-determined / square）**：方程数=变量数，有唯一匹配 → 真正被解出的（例中 `y,z,w`）。
3. **超定块（overdetermined）**：方程多于变量 → 冗余/冲突。

**方定块再细分**：把"方程→它所依赖且也在方定块里的变量"看成有向图，求**强连通分量（SCC）**：
- 无环 ⇒ 每个 SCC 是单点 ⇒ **块下三角** ⇒ 给出**逐步求解顺序**（例：eq1→eq2→eq3，= EQC 拓扑排序）。
- 有环 ⇒ 几个变量缩成一个 SCC 块 ⇒ **代数环：这一块必须联立（隐式）求解**，块之间仍按三角顺序。

> **DM 分解 = EQC 现有「拓扑排序 + 环检测」的严谨完整版**。它一次给出：① 哪些是自由输入（自动分驱动/参数 vs 输出）② 块三角求解顺序 ③ 代数环精确落在哪个块（→ 只那块要隐式，其余照常显式，接 [`spec`] 与隐式求解器缺口）④ 模型是否过/欠定（写漏/写重方程的结构 bug，跑前就能报）。

把 `eq3` 后再加 `eq4: y = w − 1`，出现 `y→z→w→y` 环 → SCC `{y,z,w}` 缩成一块 → DM 告诉你"这三个要联立解，其余不变"。

### 2.3 微分指标与指标约简（DAE，前瞻）

动态/微分代数系统（DAE）里，二部图 + 匹配能跑 **Pantelides 算法**：算**微分指标（differential index）**、做**指标约简**（决定哪些约束方程要对时间微分、补哪些"哑代数变量"），把高指标 DAE 降成可数值积分的形式。EQC 现在是显式 Euler（指标低、用不到），但等做隐式/刚性求解器时，这是工业级建模工具（Modelica 编译器等）的标准武器，现成可借。

### 2.4 结构可辨识性（structural identifiability）→ 直接喂标定 arc

**问题**：给定"哪些变量可测"（EQC 的 `measurable` 字段），某参数**在结构上**能否从这些观测被唯一确定（与具体数据无关）？

图论直觉（**便宜的必要条件筛子**）：
- 参数 p 若**在图上无任何路径**通到任何可测输出 ⇒ **结构不可辨识**（数据再多也定不出）。
- 两参数若**沿完全相同的路径集**影响相同输出、无法被任何观测区分 ⇒ **结构混淆（confounded）**：只能定其组合（如乘积），不能各自定。EQC 已实测过这类现象（如 `Y=F·Pd/1000` 使 Y 对 Pd ~不变、Kc 在 CO₂≡参考点时不可辨识）。

这是 `eqc identify`（现为数值敏感性 + 等效性相关）的**图论/先验版**——更便宜、更早、互补。
> ⚠️ 严谨边界：**完整**的结构可辨识性判定用**微分代数**（Lie 导数、特征集），见 SIAN / StructuralIdentifiability.jl。图法只给**必要条件**（可达性、混淆候选），不给充分判定；定位为快速筛 + 可视化，重判定仍交数值/代数方法。

---

## 3. 网络分析（通用图度量，软但有用）

对象 = EQC 现成的依赖有向图。这些指标**描述性**强、便宜、直观，且**正好驱动 3D 布局**。

### 3.1 中心性（centrality）—— 谁重要
- **度中心性**：连了多少边（入度=被多少方程用，出度=用了多少量）。
- **介数中心性（betweenness）**：多少条最短路经过它 = 信息/依赖的瓶颈、枢纽。算法见 Brandes (2001)，O(VE)。
- **特征向量 / PageRank**：被重要节点指向就重要（递归定义）。
- EQC 用途：找**枢纽变量**（万物汇聚的关键状态量/积分量）→ 3D 里**按中心性定节点大小**。

### 3.2 距离 / 路径
- 最短路长度；DAG 上可达性。
- 两参数的"距离"≈ 它们的**共同下游**（一起影响哪些输出）= §2.4 结构混淆的探索版。

### 3.3 社区发现 / 模块度（modularity）
- 把图切成"内部边密、簇间边疏"的社区。**模块度 Q**（Newman 2006）量化切得好不好。
- 算法：**Louvain**（Blondel 2008，快、贪心）、**Leiden**（Traag 2019，修 Louvain 的连通性缺陷）。
- EQC 用途：**自动发现/验证子模块**（现在 `meta.modules` 手标，图法能客观聚类对照）→ 3D 里**按模块分层/分色**。

### 3.4 图编辑距离（graph edit distance, GED）/ 结构 diff
- 把图 A 变成图 B 的最小编辑（增删点/边）代价 = 两图的结构距离。精确 GED 是 NP-难，但**有标签对齐的版本对比**可退化成集合差（增/删了哪些节点/边），便宜且够用。
- EQC 用途：**量化模型版本（GP/手改前后）的结构演化**——"变了多少、在哪长出新枝"。**这是"进化规律"最严谨的落点**，也是 3D **生长动画**的数据源。

### 3.5 DAG 专有指标
- 最长路 = 计算深度；DAG 宽度；层数。已在 `report/layout.rs` 的分层布局里用过。

> ⚠️ 诚实复述：网络指标是**描述性**的，价值在于**绑定到具体问题**（别停在"算了一堆好看的数"）。所以优先级：**结构分析（第 2 节）> 网络分析（第 3 节）**；后者当探索 + 可视化驱动。

---

## 4. 与 EQC 的对照（我们离地基有多近）

| 地基要的 | EQC 现有 | 差距 |
|---|---|---|
| 变量-方程二部图 | `collect_refs` 已抽引用关系（DAG 用） | 只差"无向地"重组成二部图 |
| 匹配 | `output:` = 一个手工匹配 | 差自动求/校验匹配（Hopcroft–Karp） |
| DM 分解 | 拓扑排序 + 环检测（`dag/`、`validator/cycle_detector`） | 差 DM 三块切分 + SCC 块三角 |
| 结构可辨识性 | `eqc identify`（数值敏感性） | 差图论版（可达性/混淆，互补） |
| 中心性/社区/GED | 无 | 全新，但都是标准算法（petgraph 有 SCC 等） |
| 3D 坐标 | `report/layout.rs`（2D 力导向/分层/Forrester） | 扩成 3D 力导向、契约多吐 {x,y,z} |

**结论**：地基 = 把这两套理论沉淀成 `src/` 里几个**纯 Rust、可单测、数据无关**的图算法模块，对接已有 identify / GP / report，吐出指标 + 3D 坐标喂前端。EQC 单一真相源架构让这一切几乎免费（AST 即图）。

---

## 5. 延伸阅读（按主题）

**结构分析 / 方程系统**
- Dulmage, A.L. & Mendelsohn, N.S. (1958). *Coverings of bipartite graphs.* Canadian J. Math. —— DM 分解之源。
- Pantelides, C.C. (1988). *The consistent initialization of differential-algebraic systems.* SIAM J. Sci. Stat. Comput. —— 指标约简。
- Cellier, F.E. & Kofman, E. (2006). *Continuous System Simulation.* Springer. —— 结构分析、tearing、Pantelides 的教科书章节（建模仿真视角，最贴 EQC）。
- Hopcroft, J. & Karp, R. (1973). *An n^{5/2} algorithm for maximum matchings in bipartite graphs.* SIAM J. Comput.
- （工程参照）Modelica 编译器（OpenModelica/Dymola）如何用「二部图 + 匹配 + DM + Pantelides」做方程排序与指标约简——业界标准流程。

**结构可辨识性**
- Bellman, R. & Åström, K.J. (1970). *On structural identifiability.* Math. Biosciences. —— 概念之源。
- Villaverde, A.F. 等近年关于 ODE 模型可辨识性的综述；工具 **SIAN**、**StructuralIdentifiability.jl**（微分代数法，给充分判定）。

**网络科学 / 网络分析**
- Newman, M.E.J. (2018). *Networks* (2nd ed.). Oxford. —— 网络科学权威教材（中心性、社区、距离全覆盖）。
- Newman, M.E.J. (2006). *Modularity and community structure in networks.* PNAS.
- Blondel, V.D. 等 (2008). *Fast unfolding of communities in large networks.* J. Stat. Mech. —— Louvain。
- Traag, V.A. 等 (2019). *From Louvain to Leiden.* Scientific Reports.
- Brandes, U. (2001). *A faster algorithm for betweenness centrality.* J. Math. Sociology.

**可视化（为 3D 部分）**
- Munzner, T. (2014). *Visualization Analysis and Design.* CRC. —— 为什么 node-link 图 2D 通常胜 3D（遮挡/深度歧义），3D 何时才值得。

**工具**
- `petgraph`（Rust，EQC 已用）：SCC（Tarjan/Kosaraju）、拓扑序、最短路等现成。
