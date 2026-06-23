# EQC 受约束遗传编程（Constrained GP）规格

> 状态：设计稿（2026-06-24，与首席科学家讨论后定方向）。待逐阶段评审实现。
> 谱系：v0.1 静态 → v0.2 动态 → v0.3 Studio → v0.4 交互+工具 → v0.5/0.6 优化 → v0.7 标定 → **v0.8 GP（本规格）**。
> 关联：`docs/spec-optimization.md`（目标评估核 = GP 适应度引擎）、`docs/spec-calibration.md`（标定 = 同一外层循环）、`crop-models/理论溯源/`（进化-冻结边界的来源）。

## 1. 目的与立场

GP 是模型自我改进的引擎：当云南田间反馈数据到来，**在已有机理骨架的"假设留白"处进化出可被田间证伪的方程结构**。三条立场：

1. **受约束（constrained）**：只进化标记为假设（🟠）的方程，**冻结**有文献依据（🟢/🔵）的机理基座。GP 在骨架内进化、非从头长树。
2. **进化-冻结边界 = 理论溯源标签**。刚完成的逐方程溯源已把 5 个模型每条方程分了 🟢有依据 / 🔵平移 / 🟠假设，并对多数 🟠 直接标了「GP 候选」。**这份清单就是 GP 的进化靶点**（蓝莓需冷→休眠解除函数〔综述§6.3 GP 首靶〕、器官分配 pr_*、双 S 形式…）。
3. **GP 提出假设、不取代机理**。进化式撞上已知机理形式 = rediscovery（验证）；长出黑箱拟合 = 待田间证伪的假设。GP 输出回流进溯源体系重新分类。与整体论文叙事「忠实机理基座 + 少数新颖耦合」一致。

## 2. 架构契合（复用已有三层）

优化层 spec 已确立：decision-opt / calibration / **GP-fitness 是同一外层循环**（跑模型→打分→搜索），只换旋钮+目标。GP 是**第 3 种搜索类型**，复用：

| 已有件 | GP 复用 |
|--------|---------|
| `sim::simulate` / `eval::Expr::eval_in`（无克隆热路径） | 候选结构的前向仿真 |
| `optimize/core.rs` `prepare`/`run_obs`（目标评估核 + 约束惩罚） | 适应度 = 数据拟合误差 |
| `optimize/objective.rs` 误差算子 `rmse/mae/nse/bias` + 时间归约 `final/at/max/…` | 适应度标量化 |
| `optimize/de.rs` SplitMix64 确定性 PRNG + 种群/选择基础设施 | GP 种群循环、内层参数标定 |
| `units::check_expr`（量纲传播检查） | 候选结构的硬约束过滤 |
| `ops::OperatorSpec` 注册表（算子单一真相源） | 语法的算子库 |
| `ast::Expr`（S-表达式 = 基因组） | **GP 基因组本身，无需新编码** |

**结论**：GP 要新增的只有——树遗传算子、进化掩码（来自溯源标签）、语法/类型约束、parsimony 压力、合成复原验证。评估、基因组、量纲、误差算子全现成。

## 3. 核心设计决策（讨论已定）

- **D1 进化粒度 = 方程级**：以整条 🟠 方程为进化单元（换其 RHS 表达式形式），冻结 🟢/🔵。子树级粒度留后。理由：粗、可解释、够用（蓝莓 GP 靶都是方程级）。
- **D2 语法制导（grammar-guided）+ 类型/量纲约束**：每个 🟠 槽位配一套候选形式语法（编码先验：可用变量、可用算子、单调性、有界、符号）；`units::check_expr` 作硬过滤。不走自由 tree-GP（防 bloat/不可解释/违量纲）。
- **D3 原生 Rust，在现有 AST 上做**：不接 PySR/Operon。保持「S-expr 即基因组即部署产物」、复用 eval/units/objective-core、无重依赖（同 DE 当年手写无 `rand`）、离线无管理员友好。
- **D4 适应度：先 co-evolve 常数、后升级 memetic**：阶段一把候选式里的常数当基因一起进化（便宜，验证管线）；阶段二升级为 memetic——内层 DE/calibrate 标定候选结构的参数，适应度 = 标定后最佳拟合（更可辨识、结果更干净）。
- **D5 多目标 Pareto：拟合精度 vs 复杂度**：防过拟合；复用已有多目标 DE 的 Pareto 前沿+拥挤截断；首席科学家在拐点挑形式。
- **D6 合成数据优先验证**：投数据前用合成数据验证——已知形式生成数据 → GP 从错形式起点复原它（类比标定工具「recover LUE=4.0」end-to-end）。

## 4. `gp_target` 元数据（additive 契约字段）

仿 `measurable`/`stress_factor`/`management` 的加法式契约扩展，零破坏：

```yaml
# 方程上的可选标记
- { id: BB5-DORM, output: dormancy_released, gp_target: {
      grammar: monotone_gate,          # 引用一套语法（§5）
      inputs: [ChillAccum, GDD],       # 可用变量（默认=该方程当前 refs ∪ 同模块在范围变量）
      output_bounds: [0, 1],           # 先验：有界
      monotone: { ChillAccum: increasing },  # 先验：单调
      frozen: false }, ... }
```

- `Equation.gp_target: Option<GpTarget>`（`#[serde(default)]`，缺省=冻结）。
- 契约 `EqJson` 增 `gp_target`（absent 时跳过，`schema_version` 不变）。
- **G0 工作**：把溯源已标的 🟠 GP 靶**回填进真实 `.eq.yaml`**（先蓝莓 BB1 休眠门控、BB3 分配、BB5 双S；其余渐次）。溯源 markdown → 模型元数据，让 EQC 能机读进化靶点。

## 5. 语法（per-slot grammar）

每套语法 = 一个**类型化的候选形式族**，描述该槽位生物学/物理上合理的结构空间。示例（蓝莓休眠解除门 `monotone_gate`）：

```
<gate>   ::= clamp(<expr>, 0, 1)
<expr>   ::= <ramp> | <sigmoid> | <sigmoid_with_interaction>
<ramp>   ::= (ChillAccum − <c>) / <c+>                 # 当前形式（线性 ramp）
<sigmoid>::= 1 / (1 + exp(−<c+>·(ChillAccum − <c>)))   # 候选：S 形
<sig_x>  ::= 1 / (1 + exp(−<c+>·(ChillAccum − <c> − <c+>·GDD)))  # 冷×热互作（综述明示存在、无函数）
<c>      ::= ephemeral constant（co-evolve / memetic 标定）
<c+>     ::= ephemeral constant > 0（强制正，保单调）
```

- 语法保证：①只用 `inputs` 内变量；②`<c+>` 正 → 单调先验；③`clamp(,0,1)` → 有界先验；④`units::check_expr` 过滤量纲。
- 语法库：少数通用语法（`monotone_gate` / `saturating_sink` / `allocation_fraction` / `temperature_response` / `growth_curve`）覆盖多数作物 🟠 靶；新靶可声明新语法。
- 语法表示：内部为产生式规则表（数据驱动，非硬编码每个槽位），生成/变异/交叉都在语法约束下进行。

## 6. 树遗传算子（在 `ast::Expr` 上）

全部在语法 + 量纲约束下产生合法后代（生成后重过滤）：

- **subtree mutation**：在候选树选一非终结点，按语法重生成一棵合法子树替换。
- **crossover**：两候选在**类型兼容**点交换子树（语法非终结符匹配）。
- **grow / shrink**：受控增删（bloat 控制：复杂度上限 + parsimony 压力）。
- **constant perturbation**：ephemeral 常数高斯扰动（co-evolve 阶段）。
- 确定性：复用 `de.rs` 的 SplitMix64（同种子可复现，同 DE/标定一致）。

## 7. GP 主循环（复用 DE 基础设施）

```
init  : 从语法采样 N 个合法候选（含当前形式作种子之一 = 不退化保证）
loop  : for gen in 1..G:
          evaluate : 每候选 → 换进模型该 gp_target 方程 → run_obs(合成/田间) → 误差
          (memetic): 内层 DE 标定候选常数 → 取最佳拟合       # D4 阶段二
          select   : 锦标赛 + 精英保留 + parsimony（复杂度惩罚 / 多目标 Pareto）
          breed    : crossover + mutation + grow/shrink → 下一代（语法+量纲过滤）
out   : 单目标 → 最佳形式；多目标 → (拟合,复杂度) Pareto 前沿
```

- 复用 `optimize/core.rs` 评估核与 `run_obs`；复用多目标 DE 的非支配归档+拥挤截断做 Pareto。
- 先**单槽位**（一次进化一个 🟠 方程，其余冻结）；多槽位（联合/顺序）留 G5。
- 进化前先 `eqc identify` 查该槽位在计划观测下是否可辨识——不可辨识的留白进化了也标不出（连优化层 DoE 故事）。

## 8. 适应度（复用标定）

- 候选结构换进模型 → `sim` → `(rmse <output> <observed>)`（或加权多观测）vs 观测序列。
- D4 阶段一（co-evolve）：常数是基因，结构+常数一起进化。便宜、验证管线。
- D4 阶段二（memetic）：内层 DE 标定常数 → 适应度=标定后最佳拟合。贵但可辨识、干净。
- 稀疏观测复用 `ObservedData`（1-based DAT，只在观测日比较）。

## 9. 验证（合成复原，数据无关）

GP 的「recover」验收（类比标定 LUE=4.0）：
1. 取一模型，把某 gp_target 方程**换成已知形式**（如休眠解除真值 = 某 sigmoid）。
2. 用该真值仿真 → 抽样得合成观测。
3. GP 从**错形式起点**（线性 ramp）进化 → 应复原出 sigmoid 族 + 接近的常数。
4. 报告：复原形式是否匹配真值结构、拟合误差→0、Pareto 前沿是否含真值。

这套现在就能建+跑，不依赖田间数据。

## 10. CLI

`eqc evolve <model> --spec gp.yaml [--drivers w.csv] [--observed obs.csv] [-o result.json]`
（与 `optimize`/`calibrate`/`identify` 并列；spec 含目标槽位、语法、观测、optimizer 配置）。

## 11. 分阶段实施（标「现在能建」/「等数据」）

| 阶段 | 内容 | 数据依赖 | 验收 |
|------|------|----------|------|
| **G0** `gp_target` 元数据 + 进化掩码 | `Equation.gp_target` 字段 + 契约 `EqJson.gp_target` + 把溯源 🟠 靶回填进 .eq.yaml（先蓝莓 3 处） | ✅现在 | export 显示 gp_target；未标方程不变；测试+计数 |
| **G1** 语法 + 类型/量纲约束 | 语法表示（产生式表）+ 5 套通用语法 + `units::check_expr` 硬过滤 + 先验（单调/有界/符号）检查 | ✅现在 | 语法只生成合法（在范围/量纲一致/满足先验）树；单测 |
| **G2** 树遗传算子 | subtree mutation/crossover/grow/shrink/const-perturb，全语法+量纲过滤，SplitMix64 确定性 | ✅现在 | 算子保持合法性；同种子可复现；单测 |
| **G3** GP 主循环 + co-evolve 适应度 + 合成复原 | 单槽位种群循环（复用评估核/run_obs）+ 锦标赛+精英+parsimony + `eqc evolve` CLI + **合成复原验收** | ✅现在（合成数据） | end-to-end 复原已知 sigmoid；误差→0；同种子复现 |
| **G4** Pareto parsimony + memetic | 多目标（拟合 vs 复杂度）Pareto 前沿（复用多目标 DE）+ 内层 DE 标定常数（memetic） | ✅现在（合成） | Pareto 含真值；memetic 比 co-evolve 复原更干净 |
| **G5** 多槽位 + 溯源回流 + Studio + **田间进化** | 多 🟠 联合/顺序进化；进化式回流溯源（匹配已知形式→建议 🟢/🔵，新形式→🟠 假设，自动生成溯源条目草稿）；Studio 面板（Pareto+MathML 公式+采纳）；**云南数据真进化** | G0-G4 ✅现在；**真进化等 2026-07+ 数据** | 多靶进化；溯源条目自动生成；Studio 验证 |

每阶段：独立 commit、`cargo test --features cli` 与 `--features "cli advanced_math"` 双绿、用户评审后再提交（同标定/优化 arc 工作方式）。G3 完成可发 `v0.8`。

## 12. 溯源回流（论文叙事闭环）

GP 输出回流进 `crop-models/理论溯源/`：
- 进化式**匹配已知机理形式**（如 GP 长出 sigmoid 休眠解除）→ rediscovery，建议升级 🟠→🟢/🔵，溯源记「GP 从田间数据独立复原 X 形式 = 机理验证」。
- 进化式**新颖**→ 标 🟠 假设 + GP 来源 + 田间拟合优度，待进一步证伪。
- 自动生成溯源条目草稿（公式 + 分类建议 + why/risk + 拟合徽章），首席科学家复核。

## 13. 风险与护栏

- **过拟合/伪结构**：parsimony 压力（复杂度惩罚/Pareto）、留出验证（held-out observations）、先验约束（单调/有界/符号/量纲）。
- **bloat**：复杂度上限 + shrink 算子 + 多目标复杂度轴。
- **不可解释**：语法制导（候选都是可读机理形式）、冻结机理基座、人在环（GP 提议、科学家+田间处置）。
- **不可辨识的留白**：进化前 `eqc identify` 预筛；连 DoE（哪些处理/观测能约束该槽位）。
- **退化**：当前形式作种子之一 → GP 至少不比现状差。

## 14. 延后 / 开放

- 子树级粒度（D1 留后）；多槽位联合进化的搜索爆炸控制；
- 时变控制律的 GP（连优化 phase 3）；
- GP 与耦合仿真（在耦合模型上进化作物留白）；
- 更快解释器（compile-to-flat / 更快 hash）——GP 评估量大，性能后续优化（roadmap 已列）。
- 真正解锁 = **云南 2026-07+ 田间数据**（同标定：现在备好、合成验证、等数据来真进化）。
</content>
