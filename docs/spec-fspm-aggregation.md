# spec：FSPM 功能层① —— 拓扑聚合算子（风险3，v1，待施工批准）

> 建在已完成的 **FSPM 地基**（[`spec-fspm-foundation.md`](spec-fspm-foundation.md)，风险1 实例身份 + 风险2 NodeResolver）之上，是**功能层第一块**。
> 目标：让"**沿结构拓扑的聚合**（Σ over children / all …）"成为**一等 AST 算子**，并把 cohort 的 `sum_over` 宏收编统一。
> 它是风险4（器官流 = 生长 + Σ流入 − Σ流出）的**语言地基**——没有一等聚合算子，器官流没法干净写。
> 红线沿用：器官实例身份永远**结构化一等**；**L1** = 加载期按静态拓扑 lower 成标量，**引擎不改**。

## 0. 现状核对（已查代码，决定设计）

EQC **已有**两类聚合 AST 算子（`src/ast/expr.rs`）：
- `Expr::Reduce { kind: ReduceKind, arg }` —— 对一个**向量 Value** 归约成标量（vsum/vprod/vmean/vmin/vmax）。
- `Expr::Sum { index, lower, upper, body }` / `Expr::Product { … }` —— **下标区间**的符号求和 Σ_{i=lower}^{upper}。
- `ReduceKind = Sum | Prod | Mean | Min | Max`（已存在）→ **复用作聚合种类的单一来源**。

**缺的、且 cohort 用宏绕过的** = **沿结构拓扑**的聚合（果穗→Σ各果、整株→Σ各叶）：
- cohort 的 `{op: sum_over, over: F, body}` 是**进 AST 之前的 YAML 宏**（`cohort_expand.rs:355` 折成 `add` 链）→ AST / 契约 / 图分析**看不见**它是"聚合"。
- 地基的 `ref of: self/parent/prev/next` 是**单实例引用**（已做）；**集合值聚合**（children/all）还没有。

→ 风险3 = 引入**拓扑聚合**一等 AST 算子（第三类：沿**实例邻域**，区别于向量 Reduce、下标 Sum），并把 cohort `sum_over` lower 到它。

## 1. 已对齐决策

| 抉择 | 决定 | 说明 |
|---|---|---|
| 聚合种类 | 复用 `ReduceKind`，本轮启用 **Sum + Mean** | Prod/Min/Max 留枚举位、按需开 |
| 拓扑邻域（**集合值**） | 本轮 **`children`（contains 下行）+ `all`（某实体全集求和，`of: <entity>`）** | 真 `subtree`（跨实体子树遍历）随 `tree` kind 做；`borne`/`siblings` 留后 |
| 空集语义 | `sum` 空集 = 0；**`mean` 空集（基数=0）加载期 validate 报错**，不设运行时 0/NaN 魔法值 | 见 §3.1 |
| 与单邻居 ref 的关系 | `of: self/parent/prev/next` 是**单实例引用**（地基已做），**不在聚合范围**；聚合只管**集合** | 概念分清，避免混淆 |
| lower 策略 | **L1**：加载期按静态拓扑把聚合**展开成标量 add/mul 链**（集合加载期已知），**引擎不改** | 与地基一致 |
| cohort `sum_over` | **本轮收编** → lower 到新算子（消宏、单一真相源） | 铁回归锚 = 现有模型仿真**逐位不变** |

**为什么 `all` 而非 `subtree`（架构 + 科学论证）**：
- `subtree` 一词混了两种东西：**`all`（某实体全集求和，如冠层总叶面积 = Σ all metamer）** 成本低——复用风险2 已铺的 `organ_groups`/`StructureInfo.instances` 按实体取实例集，加载期白送，且是番茄整株光合（碳源 = 总叶面积 × 光 × LUE）的**刚需**；**真 `subtree`（某根的跨实体后代闭包）** 成本高（拓扑闭包遍历），而番茄是单主茎链、子树≈全株、用不上，真正需要它的是分枝作物（蓝莓/苹果 = `tree` kind，本就留后）。→ 本轮做 `children + all`，真 `subtree` 绑定到 `tree` kind 再做。

## 2. 语法

```yaml
# 穗级总库强 = Σ 各果库强（children 聚合）
- { for: truss, output: truss_sink,
    expression: { agg: sum, over: children, body: { ref: fruit_sink } } }

# 全株冠层总叶面积 = Σ 所有 metamer 叶面积（all / 整株汇总）
- { output: canopy_leaf_area,
    expression: { agg: sum, over: all, of: metamer, body: { ref: leaf_area } } }

# 平均节间长（mean over 全实体集）
- { output: mean_internode,
    expression: { agg: mean, over: all, of: metamer, body: { ref: internode_len } } }
```
- `agg: <kind>`（一等算子；`kind = sum | mean`）；`over: children | all`；`of: <entity>`（`over: all` 时给）。键名 `agg` 是写法（同 `{sum:…}`/`{product:…}` 结构化算子），非 `{op: agg}`。
- `over: children` —— 对当前 `for:` 实例的**直接子实例**（contains 边）聚合。
- `over: all` + `of: <entity>` —— 对某实体**全部实例**聚合，用于**全株汇总**（无 `for:` 的整株共享量也可用）。
- `body` 内 `ref` 的作用域 = **被聚合的那个子实例**（同 cohort body 里 `at` 的角色）；引用整株共享量仍直接写名。

## 3. AST + 加载期 lower

```rust
// src/ast/expr.rs 新增（additive；遍历/codegen/depth/collect_refs 同步补）
Aggregate { kind: ReduceKind, over: TopoSelector, of: Option<String>, body: Box<Expr> },
pub enum TopoSelector { Children, All, Subtree, Borne, Siblings }   // 本轮实现 Children / All
```
- **手写 Deserialize** 加 `YamlExpr::Aggregate`（key=`agg` 的结构化变体，同 `{sum:…}`）+ `into_expr` 解析 kind/over（未知值报错、不静默）（★`Expr` 是手写 deserializer，见 [[eqc-yaml-expr-deserialize-bug]] 的坑：加测试锁）。
- **加载期 lower（`parser/structure_expand.rs`）**：`for: E` 展开每实例时——
  - `over: children` → 查该实例的 children 实例集（contains 边）；
  - `over: all` + `of: F` → 取实体 F 的全部实例（复用 `StructureInfo.instances` / `organ_groups`）；
  - `body` 对每个目标实例化引用 → 折 `add` 链（`sum`）/ `add 链 ÷ count`（`mean`，count = 加载期常数）。
- **不进 `ops` 注册表**：结构化算子（`Sum`/`Product`/`Reduce`）本就不在注册表（注册表只管 52 个标量算子），`agg` 跟随它们；声明层语义由 `Expr::Aggregate` 变体直接承载，契约/分析读变体。`to_python`/`to_rust` 因「加载期 lower 后不该到 codegen」用 `unreachable!` 守约；`to_latex` 显示成 `\sum_{children}(…)` / `\operatorname{mean}`。

### 3.1 空集语义（科学 + 工程 + 架构三面收口）
- **L1 静态 → `mean` 的分母是加载期常数 count**（`children(truss)`=per-count、`all(metamer)`=count），正常态不会 0/0。
- 真正的"空集" = **count=0**，即用户声明了 0 基数实体 = **建模退化/笔误**，应在**加载期 validate 报错**（"mean over 基数 0 的集合"），**不**塞进运行时：
  - 返回 NaN → 沿方程网络**传染**、烂掉整条轨迹、绕过边界检查（仿真大忌）；
  - 静默返回 0 → **科学误导**（把"还没有"读成"值为零"，下游当分母/乘数错得无声）。
  - 加载期报错 = 把不可判的运行时问题前移成可判的声明错误（EQC「加载期单一真相源 + 边界检查」哲学）。
- `sum` 空集 = 0（加法单位元，数学标准，无争议）。
- ⚠️ **语义边界（留后，本轮不内建）**：foundation 是「预分配 + 门控激活」，"未坐果的果"是**存在但 mass=0 的实例**、会进 `mean` 分母 → 把平均拉低。"只对已激活器官求平均" = **条件聚合**（`over: children where active`），涉科学意图、留风险4 再议。本轮 `mean` = 全集简单平均（分母 = 静态 count）。

## 4. cohort 收编（零回归，第3步 ✅ 采 **A·单一折叠源**）

- **施工时的发现**（改了原计划）：cohort 的 `sum_over`/`prod_over` **早已在 YAML 层折成 add/mul 链**，且其 `fold_binary` 与 structure 的那份**重复**；pass 顺序 `expand_structure` **在** `expand_cohorts` **前**，cohort 没法发 `{agg}` 给 structure lower。且两条路的聚合**都在反序列化前 lower 成标量** —— `Expr::Aggregate` 并不进最终 AST（它是声明形态/退化场景的类型）。
- **采 A（安全·单一折叠源）**：抽出 `src/parser/agg_fold.rs`（`fold_binary` / `op_args` / `fold_sum_or_prod`，空集→单位元 sum 0/prod 1）。cohort `sum_over`/`prod_over` 与 structure `sum`/`prod` **共用同一折叠源**，删掉两份重复折叠。**输出逐位不变**：草莓 S8 仿真 `Y=7.558441109220954`（与既知一致）、cohort `sum_over` 单测断言精确 `add(add(…))` 链。
- **B 档（cohort 发 `{agg}` + 反转 pass 顺序让 structure 统一 lower）暂不做**：要反转既定顺序 + 逐位验证所有 cohort 模型，风险高、收益被"cohort 终将迁 structure"摊薄。
- 变量/方程**名 + 轨迹键保持现状**（`name__i`）→ 现有草莓/番茄 cohort 模型仿真**逐位不变**。

## 5. 番茄用例（接地基切片，到单果级）

- `truss_total_sink = Σ_{fruit ∈ children(truss)} fruit_sink`（穗级库强汇总 → 同化物分配）。
- `canopy_leaf_area = Σ_{all metamer} leaf_area`（全株叶面积 → 冠层光截获 → 整株光合）。
- `plant_fruit_load = Σ_{all fruit} 1`（坐果总数）；`mean_fruit_mass = mean_{all fruit} fruit_mass`。
- 均为**风险4（器官流）**与后续碳分配的输入；具体器官级方程由首席科学家 + 文献 + 田间数据供（架构不依赖具体方程）。

## 6. 施工分步（每步 `cargo test --features cli --offline` 绿 + 用户点头再提交）

1. **AST 算子 ✅**：`Expr::Aggregate{kind:ReduceKind, over:TopoSelector{Children/All/…}, of, body}` + 手写 Deserialize `{agg:…}` + 6 处穷尽 match 补全（collect_refs/depth/substitute/to_latex(Σ)/to_python·to_rust(unreachable)）+ 单测 `test_aggregate_yaml`。**lib 293 绿（--features cli）**。（不进 ops 注册表——随 Sum/Reduce。）
2. **加载期 lower**：`structure_expand` 加 children/all 集合解析 + `Aggregate`→add/mul 链展开；`mean` 空集（count=0）加载期报错。端到端番茄 fixture（穗→Σ果、全株 Σ叶）validate + simulate 验证。
3. **cohort 收编 ✅（A·单一折叠源）**：抽 `agg_fold.rs` 单一折叠源，cohort `sum_over/prod_over` + structure `sum/prod` 共用，删两份重复。297 lib 绿 + 草莓 S8 `Y=7.5584411`（既知值，逐位不变）。
4. **契约 additive（可选本轮 / 或并入风险4）**：`Aggregate` 在声明层导出供分析显"聚合关系"。

## 7. 立场与边界

- 本轮**只做聚合的语言 + 加载期 lower**，**不做**风险4 的"沿流积分"（把聚合接进 rate/integrator 是风险4）。
- 留后：真 `subtree`（跨实体子树，随 `tree` kind）、`borne`/`siblings`、`Prod/Min/Max`、**条件聚合**（门控 active）、动态集合（L2 长器官）；几何（风险5）无关。
- 架构不依赖具体科学方程；器官级方程科学正确性由首席科学家 + 文献 + 数据 + 量纲/边界检查 + 受约束 GP/标定兜底。
- 每步一验、绿灯 + 用户点头再提交（项目一贯节奏）。
