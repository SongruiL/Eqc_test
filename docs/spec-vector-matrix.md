# 设计规格：可求值的向量 / 矩阵（值类型升级）

> 状态：**草案，待评审**（2026-06-18）
> 范围：让 EQC 的**求值器与仿真器**能真正计算向量/矩阵——不只是生成 numpy/nalgebra 代码。
> 动机：cohort（花序/叶/分层）本质是向量；现用「宏展开成标量」(方案 A) 导致图上几十个节点、
> 也偏离数学本意。根治办法是把向量作为**一等可求值的值**（方案 B）。

---

## 0. 背景：缺口在哪

检索确认（见 CHANGELOG / 记忆）：

- **AST 有**向量/矩阵节点：`VectorLit/Dot/Cross/VecNorm/VecNormalize`、`MatrixLit/MatMul/Transpose/Det/Inv/Eigenvalues/Trace`。
- **代码生成有**：to_python→numpy、to_rust→nalgebra、to_latex→pmatrix。
- **求值器没有**：`eval` 对这些节点一律 `Unsupported`，且 `eval` 返回类型是 **`f64`**——**装不下向量**。
- **仿真器没有**（它走 eval）。所以 `eqc simulate` / Studio **不能跑向量模型**。
- 连示例 `operators_matrix.eq.yaml` 都是用标量假装向量。

**结论**：编译那半已有，**求值那半要补**。补上后，EQC 的求值器是语义权威，再回头修 codegen 与之对齐（消除现存的 Python/Rust 不一致）。

---

## 1. 目标 / 非目标

**目标**
1. 求值器/仿真器能算**标量、向量、矩阵**；标量行为与现在**逐字节一致**（零回归）。
2. 现有 52 个标量算子**自动**支持逐元素 + 广播（不逐个改算子）。
3. cohort 用**真正的向量变量**表达：草莓果序/叶从几十个标量节点收缩成约十几个向量节点（图上一节点）。
4. 状态量可以是向量，仿真器逐元素积分。
5. 求值与生成代码**语义一致**（eval == codegen），EQC 求值器为权威。

**非目标（本期不做 / 后置）**
- 完整线性代数（matmul/det/inv/特征值）——AST 已有、codegen 已有，**eval 放到 V4 后置**；本期**先把向量做透**（覆盖 cohort）。
- N 维张量、稀疏、自动微分、GPU。
- numpy 式 2D 广播（只支持「标量↔任意形状」广播 + 「同形状」逐元素，足够且不易错）。

---

## 2. 值类型 `Value`

新增（`src/eval`）：

```rust
pub enum Value {
    Scalar(f64),
    Vector(Vec<f64>),                                  // 1D，长度 n
    Matrix { rows: usize, cols: usize, data: Vec<f64> }, // 2D，行主序
}
```

- `Env` 由 `名->f64` 改为 `名->Value`。
- `Expr::eval` 返回 `Result<Value, EvalError>`。
- 兼容垫片：`Value::as_scalar() -> Result<f64>`（非标量报错）；`Expr::eval_scalar(&Env) -> Result<f64>` = `eval()?.as_scalar()`。**所有现有「期望 f64」的调用点改用 `eval_scalar()`**，标量语义与行为不变（关键的低风险手段）。

形状：`shape()` 返回 `Scalar` / `Vec(n)` / `Mat(r,c)`，用于广播判定与错误信息。

---

## 3. 广播：让 52 个标量算子免费支持向量（核心可行性杠杆）

注册表里每个算子仍是 `fn(&[f64]) -> f64`（**不动**）。eval 在调用它之前加一层广播：

1. 对各参数求值得到 `Value`。
2. 求**广播目标形状**：标量可广播到任意形状；非标量必须**同形状**，否则 `ShapeMismatch`。
3. 对目标形状的每个元素位置，取各参数在该位置的标量（标量参数取自身），调 `spec.eval(&[..])`，得到该位置结果。
4. 组装成目标形状的 `Value`；严格模式**逐元素**查非有限。

于是 `add(v,v)`、`mul(标量,v)`（缩放）、`exp(v)`、`sigmoid(v)`、`geq(标量,v)`… **全部自动逐元素**，算子代码零改动。

**约定（与 numpy 对齐，eval 为权威）**：`mul` = 逐元素（Hadamard，对应 numpy `*`）；点积/矩阵乘是**单独算子** `dot`/`matmul`。

---

## 4. 非逐元素算子（需专门求值）

这些不能由标量算子提升，eval 里专门实现：

**归约（向量→标量）** — cohort 聚合靠它（`Σ_q`）：
- 新增 1 个 AST 节点 `Reduce { kind, arg }`，`kind ∈ {Sum, Prod, Mean, Min, Max}`；范数复用已有 `VecNorm`。
- 例：`Σ_q gs_q` = `Reduce(Sum, gs)`，`gs` 为向量。

**向量算子（AST 已有，补 eval）**：`Dot(u,v)→标量`、`Cross(u,v)→3D 向量`、`VecNorm(v)→标量(L2)`、`VecNormalize(v)→向量`。

**构造（AST 已有，补 eval）**：`VectorLit([..])`、`MatrixLit([[..]])`（逐元素求值后组装）。

**cohort 下标向量**：算 `160·p`（叶）这类「逐元素带下标」式子，需要下标向量 `[1,2,…,N]`。**不新增 AST 节点**——由仿真器/加载期把家族下标作为一个**向量常量注入 Env**（如 `fruit__idx = [1,2,3]`），式子写 `mul(160, fruit__idx)`。

**矩阵算子（MatMul/Transpose/Det/Inv/Trace/Eigenvalues）**：本期**不做 eval**（V4 再补，对齐 codegen）。

> 新增 AST 只有 **1 个**（`Reduce`）。新增变体会触发三处穷尽 `match` 报错——但**编译器逐个点出缺的分支**，所以是「编译期强制补全」，低风险、不会漏。

---

## 5. 仿真器（sim）

- 状态量可为向量：`X[n] = X[n-1] + rate[n]` 用 `Value` 的逐元素加；`init` 为向量初值或标量广播。
- 延迟寄存器同理（`Value` 级）。
- 驱动量：本期仍为**标量逐日序列**（气象）；「每个个体一个常数」（如开花日）= **向量参数**（见 §6）。
- 拓扑序、环检测、`DAT` 内置：不变。

---

## 6. schema：声明向量变量 + cohort 变成真向量

- 变量/参数可声明形状：`shape: 3`（向量）或沿用 `cohort: fruit`（**长度取家族 size，但不再宏展开**，而是一个真正长度 3 的向量变量）。
- 向量参数给整组值：`anthesis: { cohort: fruit, values: [40,80,120] }` → `Value::Vector([40,80,120])`（一个变量，而非三个）。
- 向量状态 `init`：`init: 0`（标量广播）或 `init: [0,0,0]`。
- `cohorts:` 段保留（定义家族名/size/index）；家族提供注入的下标向量 `fruit__idx`。
- **宏 `{ref:X,at:q}` / `{idx:q}` / `sum_over` 的去留**：向量化后，`sum_over` 由 `Reduce(Sum, ·)` 取代，`{idx:q}` 由下标向量取代，`at` 仅在确需取单个元素时用（后置一个 `Index` 节点，本期可不做）。建议**保留宏作为兼容糖**，新模型走向量写法。

---

## 7. cohort 迁移（草莓为例）：几十节点 → 十几节点

果序整段向量化（`fruit` 长度 3）：

| 现（标量展开，30+ 节点） | 向量写法（一个向量变量/方程） |
|---|---|
| `active__1..3 = geq(DAT, anthesis__i)` | `active = geq(DAT, anthesis)`（DAT 标量广播；anthesis 向量）|
| `RFG__1..3 = 1/(1+4615.91·exp(-0.011·TF__i))` | `RFG = ...exp(-0.011·TF)...`（逐元素）|
| `GS = add(gs__1,gs__2,gs__3)` | `GS = Reduce(Sum, gs)` |
| `DF__i (state, rate DMF__i)` | `DF`（向量 state，rate `DMF` 向量）|
| `F = add(FF__1..3)` | `F = Reduce(Sum, FF)` |

叶 cohort 同理（`160·p` 用 `leaf__idx`）。图上：`DF`、`TF`、`RFG`… 各**一个向量节点**，混乱从根上消失。

---

## 8. 契约 / 图表 / 单位 / codegen

- **JSON 契约**：`VarJson` 加可选 `shape`（`"scalar"|"vector(3)"|"matrix(r,c)"`）。轨迹 JSON：向量变量的某步值是数组；逐日就是「时间×分量」矩阵。**只增字段，老前端不受影响**。
- **折线图**：向量变量按「每个分量一条线」展开绘制（如 `DF` → `DF[1] DF[2] DF[3]` 三条），或可选画其归约（如 `vsum`）。
- **报告/Forrester**：一个向量变量 = 一个节点（不再展开）。
- **单位**：向量逐元素同量纲，`check_expr` 基本复用（把现有标量传播套到 Value 的元素量纲；本期可先按「整体一个量纲」处理）。
- **codegen 对齐（V4）**：以 eval 为权威修 to_rust（如 nalgebra `*` 非 Hadamard，`mul` 要改 `.component_mul()`）、补全/校正示例。现存 Python(elementwise) 与 Rust(nalgebra `*`) 的不一致一并修。

---

## 9. 一致性保证

属性测试：随机标量/向量输入下，`eval` 结果 == 生成代码运行结果（在定义域内）。先保证 Python（numpy）一致；Rust 在 V4 修齐。这条自动抓出求值与 codegen 的任何分歧（沿用 spec-operator §3.6 的思路）。

---

## 10. 分期推进（每期 `cargo test` 全绿）

- **V0 值类型 + 广播**：`Value`(Scalar|Vector，Matrix 先只存字面量)、`eval->Value`、`eval_scalar` 垫片、广播提升 52 标量算子、`VectorLit` 求值。**标量路径行为不变**（回归测试 + `eqc build/validate examples` 不变）。
- **V1 向量算子**：`Reduce`(sum/prod/mean/min/max)、`Dot/Cross/VecNorm/VecNormalize`、注入下标向量。
- **V2 仿真向量化**：schema 声明向量变量、向量参数/init、sim 逐元素积分向量状态。
- **V3 草莓向量版**：重写 `strawberry_v1` 为向量模型；契约/图表/报告处理向量；图上一节点；与标量版对照数值一致（验收）。
- **V4 矩阵 eval + codegen 对齐**：matmul/transpose/det/inv/trace（eigen 可再后置）；修 to_rust/示例，eval==codegen 属性测试转绿。

---

## 11. 风险与缓解

| 风险 | 缓解 |
|---|---|
| `eval` 返回类型 f64→Value，调用点多 | 加 `eval_scalar()` 垫片，标量调用点机械替换、语义不变；分 V0 一次性做 + 全测试把关 |
| 广播语义错（与 numpy 偏离）→ eval≠codegen | §3 明确约定（标量广播 + 同形状逐元素，不做 2D 广播）；属性测试对拍 numpy |
| 新增 `Reduce` 触发穷尽 match | **编译器强制补全**，不会漏；只 1 个变体 |
| cohort 迁移改动模型写法 | 保留旧宏作兼容糖；新写法并存；V3 用数值对照旧版验收 |
| 矩阵半成品诱发误用 | 矩阵 eval 明确 V4；之前调用 matmul 等仍 `Unsupported`（显式失败，不静默） |
| 严格模式/非有限 | 逐元素查；任一元素非有限即报 `NonFinite`（保留早失败） |

---

## 12. 需你拍板

1. **整体方向**：按本规格做「可求值向量（先）+ 矩阵（后 V4）」，对吗？
2. **范围**：本期**先把向量做透**（覆盖 cohort），矩阵 eval 放 V4——同意吗？还是矩阵也要一起？
3. **cohort 迁移**：草莓 v1 直接重写成**向量版**（旧宏保留作兼容）——可以吗？
4. **`mul` 语义**：定为**逐元素 Hadamard**（点积/矩阵乘另用 `dot`/`matmul`），与 numpy 对齐——认可吗？
5. **codegen 对齐**：以 eval 为权威，V4 修 to_rust 的向量语义（同事的 nalgebra `*` 等）——同意吗？
