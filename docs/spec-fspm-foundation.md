# spec：FSPM 地基 —— 器官实例身份 + 拓扑（v1，决策已对齐，待施工批准）

> EQC 从 Functional 模型演进到 **FSPM（功能-结构植物模型）** 的**第一张地基 spec**。只解决
> [`audit-single-source-and-fspm-prereqs.md`](audit-single-source-and-fspm-prereqs.md) 的**风险1（实例身份无单一真相源）**
> + **风险2（NodeResolver 需植入实例维度）**。计算语义（拓扑聚合算子、器官流）与几何/3D 动画是**后续层**，建在本地基上。
> **红线**：器官实例身份永远是**结构化一等数据**，绝不退回字符串后缀（`leaf__3`/`leaf[3]`）。

## 0. 已对齐决策（首席科学家拍板）

| 抉择 | 决定 | 含义 |
|---|---|---|
| A 架构 fork | **L1：身份保留 + 静态结构** | 加载期按结构实例化成"带身份标签的标量方程"，复用现成标量引擎；器官数固定（预分配 + 门控激活）。动态长器官（L2）留后、且本设计对其前向兼容。 |
| B 表示模型 | **(a) 通用实体+拓扑底座 + (b) metamer 链糖** | 底座通用（能长出分枝/克隆/MTG）；链作番茄的好写糖。 |
| 第一靶 | **番茄，分辨率到单果** | 主茎节间 → 叶 + 果穗 → 单果。蓝莓/草莓/苹果等后续作物靠"加拓扑种类"扩展、非返工。 |
| cohort 去向 | **保留为向后兼容糖，加载期 lower 到结构（身份保留版）** | 现有 cohort 模型零改动继续跑，自动获得身份保留；消掉"两套并行多重性"。 |

**3D 形态/动画（风险5）单列**：0–3 步只产出**结构数据**（哪个器官、怎么连），真实植株几何 + 渲染引擎选型（three.js 自建 vs 接 OpenAlea 类）到风险5 专门讨论。

---

## 1. 数据模型：`structure:` 段

新增顶层段 `structure:`（与 `cohorts:` 同属加载期处理，无则模型行为完全不变）。

### 1.1 实体（entity）
```yaml
structure:
  entities:
    metamer:                       # 实体类型名
      count: 12                    # 绝对实例数（静态）
      topology: chain              # 内置链拓扑：metamer#i --succession--> metamer#(i+1)
    truss:
      borne_on: metamer            # 由 metamer 横生（branching/bears 边）
      at: { every: 3, from: 3 }    # 第 3、6、9、12 节各 1 个果穗（共 4）
    fruit:
      per: truss                   # 每个 truss 含 M 个 fruit（decomposition/contains 边）
      count: 6
```
- `count`（绝对）或 `per: <父实体> + count`（每父实例 K 个）确定基数。
- 实例 id = 路径：`metamer#3`、`truss#3`（生在 metamer#3）、`fruit#3.2`（truss#3 的第 2 果）。

### 1.2 拓扑种类（kinds）
| kind | 边 | 本期 | 语义 |
|---|---|---|---|
| `chain` | `E#i → E#(i+1)` | **实现** | 同类型逐实例连续（茎节succession） |
| `per`（隐含） | `parent → child` | **实现** | 分解/包含（果穗含果、节间含叶） |
| `bears`/`borne_on` | `parent#p → child#?` | **实现（按 `at` 规则）** | 横生侧器官（节生果穗） |
| `tree` / `branch` | 任意父子树 | **定义、不实现** | 多年生木质分枝（蓝莓/苹果） |
| `clonal` | 网络 | **定义、不实现** | 匍匐茎→子株（草莓） |
| 多尺度（MTG） | 跨尺度分解 | **不实现** | 轴/生长单元层级，远期 |

> 本期只实现 `chain + per + bears`（够番茄到单果）；其余拓扑种类**在 schema 里留好枚举位**，加一种作物 = 实现一种 kind，不动底座。

---

## 2. 变量 / 方程的实例注解

```yaml
variables:
  # 按实体索引：每个 metamer 实例一份（身份保留，非 __i 串）
  leaf_area:   { of: metamer, class: state, init: 0.0, rate: leaf_growth }
  leaf_growth: { of: metamer, class: rate }
  fruit_mass:  { of: fruit,   class: state, init: 0.0, rate: fruit_growth }
  fruit_growth:{ of: fruit,   class: rate }
  # 无 of: 的变量 = 整株共享标量（如气温 T、全株同化物池 assimilate）

equations:
  # for: 量词化——对该实体每个实例成立；ref 的 of 指明取哪个实例
  - { for: metamer, output: leaf_growth,
      expression: { op: mul, args: [ {ref: LUE}, {ref: leaf_area, of: self} ] } }
  - { for: fruit, output: fruit_growth,
      expression: { op: mul, args: [ {ref: sink_strength, of: self}, {ref: assimilate} ] } }
```
- 变量 `of: <entity>` → 按该实体索引（实例化时每实例一份）。无 `of:` = 整株共享。
- 方程 `for: <entity>` → 对每实例展开一份。`ref` 的 `of:`：
  - `of: self`（默认，省略即 self）→ 同实例；
  - `of: parent` / `of: prev`（chain 前驱）/ `of: next` → **拓扑邻居**（本期支持 self/parent/prev/next；任意"Σ over children"是**风险3 后续层**，本期不做，先用现成 cohort `offset`/`sum_over` 兜着已有模型）。
- 引用整株共享变量（无 `of:`）直接写名。

---

## 3. 身份保留实例化（核心机制）

加载期把 `structure:` 展开成**带身份标签的标量** `EquationFile`，**引擎层完全不用改**（仍跑标量），身份作为结构化元数据并行保留。

**Schema 加两个 additive 字段**（不破坏现有模型/契约）：
```rust
// EquationFile 新增：本模型的结构（声明 + 实例化结果，单一真相源）
pub structure: Option<StructureInfo>,   // None = 纯 Functional 模型（行为不变）

pub struct StructureInfo {
    pub entities: Vec<EntityDecl>,                 // 类型 + 基数 + 拓扑种类
    pub instances: Vec<Instance>,                  // 全部实例（id 路径 + 所属实体 + 父）
    pub topology: Vec<TopoEdge>,                   // (from_instance, to_instance, kind)
}
// Variable / Equation 新增：实例身份标签（实例化时填，引擎忽略，下游读）
pub instance: Option<InstanceTag>,      // None = 整株共享 / 非结构量
pub struct InstanceTag { pub entity: String, pub id: String /* "3" / "3.2" */ }
```
- 实例化产物：`leaf_area` of metamer count=12 → 12 个变量，**名仍可为 `leaf_area__3` 形式（引擎键），但权威身份 = `InstanceTag{entity:"metamer", id:"3"}`**。方程 `for: metamer` → 12 份，各带 tag + `of: prev` 等解析成对应实例的引用名。
- **引擎不变**：`sim`/`eval`/`Stepper` 照常按标量名跑；`instance`/`structure` 它们一概不读。
- **下游读身份**：NodeResolver / 图 / 契约 / 视图读 `InstanceTag` + `StructureInfo`，**无需反解字符串** → 风险1 修复。

---

## 4. cohort 收编（向后兼容，零回归）

`cohorts:` 语法保留有效；加载期**不再擦成裸标量**，而是 lower 成 structure：
- `cohorts: { fruit: {size:3, index:q} }` → `entities: { fruit: {count:3, topology:chain} }`。
- 展开的变量/方程**名仍按现状 `name__i`**（保证现有模型轨迹键逐位不变），但补上 `InstanceTag{entity:"fruit", id:"i"}`。
- `offset` → chain 的 prev/next 邻居引用；`sum_over`/`prod_over` → 现状 add/mul 链（风险3 后续层再升级成一等聚合算子）。

**硬约束**：现有草莓/番茄 cohort 模型仿真**逐位不变**（第 1 步的铁回归锚）。

---

## 5. NodeResolver 实例维度（风险2 关键杠杆）

现状 `(模块, 本地名) → "MODULE.name"`。扩展为携带实例身份：
```rust
// resolve 增加可选实例上下文；返回 id 不变，另出结构化身份供折叠/上色
pub fn resolve_inst(&self, module: &str, name: &str, inst: Option<&InstanceTag>) -> ResolvedNode;
pub struct ResolvedNode { pub id: String, pub entity: Option<String>, pub instance: Option<String> }
```
- **一处实现、全图层派生**：bipartite/digraph 节点带上 `entity/instance` → 结构分析能**按实体折叠**（12 个 metamer 收成 1 个超级节点）、按实例上色、画器官子图。
- **顺带收编裂缝**：`dag/builder.rs` 目前自己 `format!("{module}.{name}")` 没走 resolver → 收编进来，否则实例维度会漏。

---

## 6. 契约 additive 字段（前端获得结构）

`ModelJson` 加（`schema_version` 不动，老前端零回归）：
```ts
structure?: {
  entities: { name: string; count: number; topology: string }[]
  instances: { id: string; entity: string; parent?: string }[]
  topology: { from: string; to: string; kind: string }[]
}
// VarJson/EqJson 加 instance?: { entity: string; id: string }
```
前端据此可"按器官折叠/上色/画器官子图"——但**真实 3D 植株形态**是风险5，本期不做。

---

## 7. 番茄结构切片（第一靶，到单果级）

```yaml
structure:
  entities:
    metamer: { count: 12, topology: chain }          # 主茎 12 节
    truss:   { borne_on: metamer, at: {every: 3, from: 3} }  # 第3/6/9/12节 → 4 穗
    fruit:   { per: truss, count: 6 }                # 每穗 6 果 → 24 果
```
- **metamer 级变量**：leaf_area、internode、节点同化物分配。
- **truss 级**：坐果数、穗发育阶段。
- **fruit 级**：单果质量、单果发育阶段、单果库强。
- **整株共享**：气温/光/CO₂ 驱动、全株同化物池、根。
- 实例总数：12 metamer + 4 truss + 24 fruit = 40 结构实例（× 各自变量）。引擎扛得住。
- **复用现有 T3**：T3 已有 Vanthoor 库源 + cohort 果实阶段；本切片 = 把果实 cohort 升进结构 + 补 metamer/leaf/truss 层级。具体器官级方程（碳分配、库强、phyllochron）由首席科学家 + 文献 + 田间数据供（架构不依赖具体方程）。

---

## 8. 施工分步（0–3 地基；4–6 后续层）

- **第 0 步 ✅**：spec 收口 + 番茄切片定（本文档）。
- **第 1 步 ✅（commit `2fd6ba5`/`ba507f7`/`b6206ee`）**：schema `structure:` + `StructureInfo`/`InstanceTag` 字段（1a）+ cohort lower 身份保留（1b，草莓 cohort 仿真逐位不变）+ 加载期 structure 实例化（1c，`structure_expand.rs`，count/chain/per + ref `of: self/prev/next/parent`）。番茄切片解析+实例化+仿真通；292 lib 绿×2 配置。
- **第 2 步 ✅（commit `6ae67f5`）**：NodeResolver 加 `identity`（节点→InstanceTag）+ `organ_groups` 折叠 + `eqc structure` 显「器官结构」。番茄 metamer(4)/fruit(8) + 草莓 v1 leaf(12)/fruit(3) 都被识别。（dag/builder 收编 deferred。）
- **第 3 步 ✅（commit `d35a5e8`）= 验收点**：契约 `ModelJson.structure` + `VarJson.instance`（additive）+ 3D 拓扑图例从契约派生显「🌿 器官结构」（声明一次→视图派生）。本地 demo `tomato_fspm_demo/`。svelte-check 0 + Playwright + 截图确认。
- **★地基（风险1+2）完整闭环。** 余 deferred：3D 按器官上色/折叠完整交互、2D 报告折叠、dag/builder 收编、`borne_on`/tree/clonal。
- **第 4–6 步（后续，各自 spec）**：拓扑聚合算子（风险3）→ 器官流/多源 rate（风险4）→ 真实几何/3D 动画（风险5，含引擎选型讨论）。
- **作物推广（贯穿）**：分枝（蓝莓/苹果）= 实现 `tree` kind；克隆（草莓）= 实现 `clonal` kind；MTG 多尺度 + L2 动态生长 = 远期。

## 9. 立场与边界

- **每步一验、cargo test 绿 + 用户点头再提交**（项目一贯节奏）。
- 本地基**只表示 + 传播结构，不在结构上计算**（计算=风险3/4，几何=风险5）。
- **架构不依赖具体科学方程**：structure 只承载方程；器官级方程的科学正确性由首席科学家 + 文献 + 数据 + EQC 的量纲/边界检查 + 受约束 GP/标定兜底。
- 闭源未披露的同类模型不抄；建 EQC 自己的开源单一真相源 FSPM。
