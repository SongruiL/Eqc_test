# 耦合仿真技术规范（双向、多速率）

状态：**已定稿**（2026-06-23，首席科学家拍板 D1–D6 全采纳推荐；按 C1→C4 实施）。
目标读者：首席科学家 + 实现者。

---

## 1. 目标与范围

把"温室气候模型 + 作物模型"从**离线两趟管道**（见 §2）升级为 EQC **一次集成的耦合仿真**，且支持**双向反馈**（温室气候 ↔ 作物通量）。一旦耦合仿真进引擎，**耦合优化**（`eqc optimize` 直接搜温室环控、目标读作物产量）几乎顺势而成（§8）。

**范围内**：温室（快、秒级）↔ 作物（慢、日/小时级）的多速率积分；快→慢的气候聚合；慢→快的作物通量回拉；接口建模（温室 `phi_ass`/`phi_trans` 接作物）。
**范围外（本草案不做）**：>2 模型的任意耦合网络（设计上预留，先做 2 模型）；隐式/刚性求解器（仍显式 Euler）；土壤/根系子模型。

**精度立场**：精度由"模型 + 聚合规则 + 接口建模"决定，不由"是否一次集成运行"决定。离线管道已给出可信结果（番茄 T4、蓝莓需冷情景），故本特性的增量价值是：①一次运行的便利 ②**真双向**（作物改变温室气候，离线单向管道做不到）③耦合优化在一个进程/一份声明式 spec 里（§8）。

---

## 2. 现状（基线）

- **耦合视图（DAG step 3，已做）**：纯结构，`couplings:` 在工作区清单声明，serve 内存注入 `source:` 画跨模型边。不跑数。
- **离线管道耦合（已用、已验证）**：`eqc simulate 温室`（dt=10s 全季）→ `aggregate_to_{hourly,daily}.py` 重采样 → 作物驱动 CSV → `eqc simulate 作物`。**两趟、文件中转、单向**。`aggregate_to_*.py` 编码了聚合规则（日均 T、日积分辐射、sub-day 积分需冷）。
- **引擎现状**：`sim::simulate(file,&SimInput)->SimOutput`，显式 Euler，`build_plan(file)->SimPlan`（拓扑序 PlanStep + 延迟 + 驱动名）。每步：DAT → 参数 → 驱动`[name][n]` → 延迟`X[n]=src[n-1]` → 拓扑 eval 方程+积分（`X[n]=prev+rate[n]`）→ 记录+快照 prev。`meta.dt` 是步长（**各模型用各自时间单位**：温室 10=10s，蓝莓 1=1d，番茄 3600=1h）。`_prev` 寄存器破步内环。变量 `source:` 现仅用于 DAG/单位检查，**不参与运行时数据流**（sim 按名从 CSV 读驱动）。

---

## 3. 核心概念：多速率耦合

两模型，dt 不同。**快**模型（温室，dt_f）一个**慢**步（作物，dt_s）内跑 R 个快步：

```
R = dt_s_秒 / dt_f_秒
```

**时间单位必须先统一到秒**（关键细节）：温室 dt=10（秒），蓝莓 dt=1（天=86400s）→ R=8640；番茄 dt=3600（=1h）→ R=360。各模型 dt 在其原生单位，耦合层须知每模型"步长的秒数"。
→ **决策点 D1**：每个被耦合模型在 `meta` 加 `dt_seconds:`（步长折秒），或耦合声明里给 `dt_seconds`。推荐前者（模型自描述时间尺度）。

**慢步数** `S = floor(温室总步数 / R)`；温室需要全程 S·R 个快步的室外天气（即现有温室驱动 CSV）。

---

## 4. 接口与聚合规则（快→慢）

作物某步的驱动 ≠ 某瞬时温室输出，而是该慢步内的**聚合**。聚合算子（链接携带）：

| `agg` | 含义 | 例 |
|---|---|---|
| `mean` | 慢步内快值时均 `(1/R)Σx` | 日均温 T ← T_air |
| `integral` | 时间积分 `Σ x·dt_f`（带单位换算） | 日总辐射 Sr[MJ] ← Q_sun[W/m²] |
| `last` | 取慢步末值 | （少用） |

**非线性 sub-day 量的原则（重要）**：像需冷 `chill=∫f(T(t))dt`（f=需冷三角，非线性，日均 T 算不出，会漏~50%）——**不**让链接携带任意函数，而是**把非线性建成快模型的一个显式输出**（如温室加一个 `chill_rate_inst = triangle(T_air)`），链接只对它做 `integral`。这样链接聚合词表只需 `mean/integral/last`，非线性留在模型里（声明式、归位）。`aggregate_to_daily.py` 的 `chill_daily` 本质就是这么做的。
→ **决策点 D2**：确认"链接只做线性聚合 + 非线性建成快模型输出"这一原则（vs 链接携带 S 表达式聚合器）。推荐前者。

**清单语法（在现有 `couplings:` 上加 `agg`/`unit_*`）**：
```yaml
couplings:
  - id: gh_blueberry_sim
    models: [greenhouse, blueberry]
    links:        # 快→慢：温室 → 作物驱动
      - { to: BLUEBERRY_BB5.T,  from: GREENHOUSE_V1.T_air,        agg: mean }
      - { to: BLUEBERRY_BB5.Sr, from: GREENHOUSE_V1.Q_sun,        agg: integral, unit_out: MJ/m2 }
      - { to: BLUEBERRY_BB5.chill_daily, from: GREENHOUSE_V1.chill_rate_inst, agg: integral }
    feedback:     # 慢→快：作物 → 温室（§5，v1 滞后一慢步）
      - { to: GREENHOUSE_V1.phi_ass,  from: BLUEBERRY_BB5.assim_flux_inst, agg: hold }
      - { to: GREENHOUSE_V1.LAI_crop, from: BLUEBERRY_BB5.LAI,             agg: hold }
```
视图层（step 3）只用 `models`+`links`（画边，忽略 `agg`/`feedback`），仿真层全用——**加法扩展、向后兼容**。

---

## 5. 双向反馈（慢→快）

作物影响温室：光合抽走 CO₂（`phi_ass`）、蒸腾增湿（`phi_trans`）、LAI 改遮光/蒸腾。温室那两个量**本就是给作物留的钩子**（greenhouse_v1：`phi_ass`="v1占位,步骤D接作物"；`phi_trans`="待精细化"）——双向 = 把它们从自算占位**变成由作物喂的输入**。

**矛盾点**：作物状态（生物量/LAI）日更，但温室要的是**快步级**的作物通量。两条路线：

- **v1 — 滞后日反馈（推荐先做）**：温室在一整个慢步内，用作物**上一慢步**的接口值（日均光合通量、LAI）作常数（`agg: hold`）。日末作物推进一步、算出新接口值，**下一天**温室才用到。即把引擎现有的 `_prev` 破环哲学**抬到耦合界面**：反馈滞后 ≤1 慢步。LAI 等慢变量，一天滞后误差小。**无步内代数环 → 稳定、显式、改动小。**
- **v2 — 紧耦合（后续）**：去掉滞后。两选一：(a) 慢步内**迭代**到温室气候与作物通量自洽；(b) 把作物的**快通量部分**（瞬时光合/蒸腾，随瞬时气候+冻结的日状态）每快步求值，日末再积分慢状态。更准、更贵、更复杂。**留 C4。**

→ **决策点 D3**：v1 用滞后日反馈（推荐）确认；紧耦合留后续。

---

## 6. 接口建模（这是"双向"真正的新机理，非纯引擎）

- **温室侧**：`phi_ass` 改成 `type: input`（由作物喂），接进 CO₂ 平衡（已在 `rate_CO2` 里）；`phi_trans` 用作物蒸腾替代/增强（接进 `rate_H`），或喂 LAI 改进温室蒸腾式。
- **作物侧**：暴露 `assim_flux_inst`（瞬时 CO₂ 吸收）、`transp_flux_inst`（瞬时蒸腾）、`LAI` 作接口输出。难点=**单位/量纲与时间尺度换算**：作物日光合 `P_gross`[gCH2O/m²/d] → 温室 CO₂ 平衡要 [ppm/s 或 µmol/m²/s]，须过 CH2O↔CO₂ 化学计量、每地面积→每空气体积、每日→每秒。v1 取"日光合/有效日长"作日内常数通量（近似但抓住反馈量级）。这些换算放在**接口方程**（一小段 coupling-local 方程，或作物/温室里的接口块）。
→ **决策点 D4**：接口换算放哪——(a) 温室/作物模型各加接口变量（落盘进模型），(b) 耦合声明里带一小段 S 表达式接口方程（不落模型）。倾向 (a)（机理归模型，和 phi_ass 钩子一致）。
→ **决策点 D5**：双向 v1 先接哪个反馈——**只 CO₂（phi_ass）** 最干净、单条闭环可验证；还是 CO₂+湿度一起。推荐 CO₂ 先行。

---

## 7. 引擎架构

新增**耦合驱动**（不改单模型 `simulate`，复用其 `build_plan`+每步 eval）：

- `CoupledPlan`：由 N 模型 + links + feedback + 各 `dt_seconds` 推导：快/慢角色、R、快→慢聚合器表、慢→快 hold 表、接口变量校验（links 的 `to` 须是慢模型 Input；feedback 的 `to` 须是快模型 Input）。
- `simulate_coupled(models, coupling, outdoor_drivers, slow_steps) -> CoupledSimOutput`：
  ```
  初始化两模型状态；feedback 值置初值
  for 慢步 s in 0..S:
     清零快聚合器
     for 快步 f in 0..R:
        温室输入 = 室外驱动[s*R+f] + feedback 值（v1：本慢步内冻结）
        温室推进一快步（复用 sim 每步机制）
        累加聚合器（mean 累加、integral 累加·dt_f、非线性走快输出）
     聚合器收尾 → 作物本步驱动
     作物推进一慢步（复用 sim 每步）
     从作物新状态更新 feedback 值（供下一慢步——v1 滞后）
     记录两模型轨迹
  ```
- **复用**：每模型仍走自己的 `build_plan` + per-step eval；新代码 = 外层多速率循环 + 聚合 + 接口装配。
- **输出**：作物按慢分辨率、温室按慢分辨率（聚合后，便于同图）+ 可选温室快细节。`CoupledSimOutput` 复用 `trajectories: IndexMap<String,Vec<f64>>`（键加模型前缀，如 `GH.T_air`/`BB.Yield`）→ CSV/chart/JSON 消费者基本不变。
- **CLI**：`eqc couple <workspace.yaml> --coupling gh_blueberry_sim --weather outdoor.csv [--steps S] -o out.csv`（或 `eqc simulate --coupling`）。
- **Studio**：`/api/simulate?model=<coupling-id>` 对耦合条目改走 `simulate_coupled`（解除 step-3a 的 `coupled_guard`，前提是该耦合声明了 `agg`/`dt_seconds`）。

---

## 8. 耦合优化（顺势而成）

`eqc optimize` 的 eval-core 只要求"一个产出轨迹的前向模型"。把 `simulate_coupled` 包成该前向模型：

- 旋钮 = 温室控制参数（`T_heat_force`/CO₂ 注入设定点/…）、可加作物参数；
- 目标 = 对**作物**轨迹的归约（`(sub (final Yield) (mul energy co2_cost))`）；约束（如 `max(RH)≤90`）、Pareto、prescreen、收敛图、Studio 决策面板**全部复用**。
- 这正是 `optimize_force_de.py` 现在在外面用 Python+离线管道编排的事——搬进 EQC = 一个进程、一份声明式 spec、用 EQC 测过的 DE。
- **为什么非双向不可**：旋钮（CO₂ 注入）的真实边际收益取决于作物回吃 CO₂、蒸腾改通风→改能耗。单向优化会系统性找错最优点；双向才在真系统上搜。

---

## 9. 分期

- **C1 多速率单向（先验证骨架）**：`simulate_coupled` + 快→慢聚合（mean/integral），无反馈。**验证 = 复现离线管道**（温室→aggregate→作物 ≈ 耦合 run，逐位或 <1e-9）。拿已验证的基线给多速率循环+聚合上保险。
- **C2 双向滞后反馈**：加慢→快 hold + 接口建模（温室 `phi_ass` 转 input、作物暴露 `assim_flux_inst`）。验证 = 反馈使温室 CO₂/湿度按预期方向变；守恒 sanity；A/B（开/关反馈）对比。
- **C3 耦合优化**：`simulate_coupled` 接 eval-core；`eqc optimize 耦合模型`；Studio 面板。验证 = 复现 `optimize_force_de.py` 的最优（交叉核对），但进程内+声明式。
- **C4（后续）紧耦合**：步内迭代 或 sub-day 作物快通量，去滞后、提精度（仅当滞后误差被证明要紧时）。

---

## 10. 决策（2026-06-23 已定，全采纳推荐）

- **D1 时间单位**：✅ 每模型 `meta.dt_seconds`（模型自描述其步长折秒；耦合时必需）。
- **D2 聚合词表**：✅ 链接只 `mean/integral/last`；非线性 sub-day 量建成快模型显式输出，链接对其 `integral`。
- **D3 双向方式**：✅ v1 滞后日反馈（`_prev` 哲学抬到耦合界面，无步内代数环）；紧耦合迭代留 C4。
- **D4 接口换算落点**：✅ 模型加接口变量（机理归模型，和 `phi_ass` 钩子一致）。
- **D5 双向 v1 范围**：✅ 先只 CO₂（`phi_ass`，单条闭环可验证）；湿度后续。
- **D6 输出粒度**：✅ 温室聚合到慢分辨率；快细节按需后加。

---

## 11. 验证策略（贯穿）

- **C1 对基线**：耦合 run 复现 `aggregate_to_*.py` 管道（同一室外天气 → 同一作物产量，逐位/近逐位）——这是多速率+聚合正确性的金标准。
- **C2 反馈方向**：开/关 `phi_ass` 反馈，温室 CO₂ 稳态、作物产量按机理方向变（作物吃 CO₂ → 同注入下 CO₂ 更低）；质量守恒。
- **C3 对外部 DE**：耦合优化最优 ≈ `optimize_force_de.py` 最优。
- 全程 `cargo test` 两配置绿；每接口变量/方程标 `reference`、新参数待标定。
