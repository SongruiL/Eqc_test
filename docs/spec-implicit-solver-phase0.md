# 隐式刚性求解器 · Phase 0（引擎地基）

> 状态：**已实现 + V1 验收通过**。对应上层 arc `greenhouse_model/全保真环控模型演化_spec.md` 的 E1/E5a/V1。
> feature-gated 在 `implicit` 之后，默认构建零影响。追踪记忆 `[[eqc-implicit-solver]]`。

## 0. 目标

把 EQC 的动态过程仿真从**显式定步长 Euler**（`sim/mod.rs:477` 硬编码 `X += rate·dt`）扩展出一条**隐式刚性求解**路径，接 [diffsol](https://github.com/martinjrobins/diffsol)（MIT·纯 Rust 核·BDF），用于亚日刚性 ODE（温室气候）。显式路径**逐位不变**——隐式是平级新增，非改造。

## 1. 为什么显式引擎结构上解不了真联立系统

显式 `topo_order`（`sim/mod.rs`）把积分状态量当"依赖其 rate 的可计算节点"。van Henten 三态里速率方程需读状态量**当前值**（`Q_cov = U·(T_air − T_out)`），这会形成 `Integrator(T_air) → rate_T → T_air` 的自依赖 → `SimError::Cycle`。历史上用**手写 `_prev` 延迟寄存器**（读上一步值）破环。

但这只是**显式调度器的假象**：数学上 `dX/dt = f(X, drivers, t)` 是普通刚性 ODE（每态有热容/容量，**非 DAE**——GreenLight 28 态全物理也是纯 ODE，用 `solve_ivp(BDF)` 解，scipy 不解 DAE）。隐式路径把 state 当积分变量（RHS 输入），auxiliaries 从 state 前向算（无环），Newton 解的是**时间离散式** `X_{n+1}=X_n+dt·f(X_{n+1})`，不是 RHS 内的代数环。

## 2. 加载/编译 pass 顺序（隐式增量）

现有：`structure_expand → cohort_expand → deserialize → reclassify_parameters`。隐式在其后、`build_rate_plan` 前插入（作用于**内存克隆**，源文件不动，SSOT）：

```
… → reclassify_parameters
  → fold_prev_for_implicit   (E5a：X_prev → X，删延迟寄存器)
  → [E2 平滑化]              (0b，本阶段未做；基座/crop 全光滑无靶点)
  → build_rate_plan          (state 作源、方程拓扑序、读 rate)
  → simulate_implicit        (逐驱动步 ZOH · diffsol BDF 自适应内解)
```

## 3. 三个核心小件（`src/sim/implicit.rs`）

### 3.1 `fold_prev_for_implicit`（E5a）
克隆模型；对每个 `prev: X` 变量，用现成 `Expr::substitute(prev_name, Var(X))` 把所有方程里的引用折回真态；再 `shift_remove` 删该延迟寄存器。**源文件不改**（显式路径仍靠 `_prev` 正常跑）。折叠后 state 直接进速率路径 = 显式会报 `Cycle` 的真联立系统。

### 3.2 `build_rate_plan`（state 作源）
- state（`is_integrator`）= 输入源，不进拓扑（否则同显式一样成环）。
- 对**方程**做 Kahn 拓扑序：依赖 = `get_variable_refs() ∩ 方程输出`（state/driver/param 是源）。折叠后此图无环。
- driver = 非积分/无方程/非参数的变量；rate = 每个 state 的速率变量名（方程输出 / 参数 / 驱动）。
- 产出 `RatePlan{ states, ordered_eqs(拓扑序 clone), drivers }`（自持有，不绑模型生命周期）。

### 3.3 RHS 闭包 + 通用 FD Jacobian（`advance_segment`）
diffsol 的 `.rhs()`（`ClosureNoJac`）**不实现 `OdeEquationsImplicit`、不能喂 BDF**（spike 实测；修正了"只给 RHS 内部自动 FD"的乐观预期）。正解 `.rhs_implicit(f, g)`：
- `f(x,p,t,y)`：一趟 rate 计划求值——`env.put(state_i, x_i)`、replay `ordered_eqs` 的 `eval_in_with(strict:false)`、`y_i = rate_i`。**复用现成 `eval_in`**。
- `g(x,p,t,v,y)`：**通用单边有限差分** `J·v = (f(x+εv) − f(x))/ε`，`ε=(1+‖x‖∞)·√(machine_eps)`。复用同一 RHS，**与模型无关的样板**（非逐模型解析求导）——diffsol 用单位向量调它逐列组装 FD Jacobian，等价 scipy BDF 内部做法。
- **非严格求值**（`strict:false`）：Newton trial state 常探到病态值（负浓度/过冲），严格模式会把 NaN/Inf 变 `Err` 让 Newton 无法从惩罚值恢复；这里让其传播给 diffsol 步长回退。首个结构性错误记入 `RefCell<Option<EvalError>>`，段末检查。

### 3.4 分段推进（`simulate_implicit`）
逐驱动步 `[t_n, t_{n+1}]`：驱动量按**零阶保持（ZOH）**设为本段常数（+ 注入 `DAT`），构建 diffsol problem（init=上段末态）、`.bdf::<NalgebraLU>()` 自适应内解 `[0,dt]`、`interpolate(dt)` 取末态。这天然把有效最大步长限在 `dt` 内（diffsol builder 无 `max_step`，此为兜底）。常数驱动下逐段续解 = 精确连续解在网格点采样。

## 4. diffsol 集成（`implicit` feature）

`Cargo.toml`：`diffsol = { version = "=0.16.1", default-features=false, features=["nalgebra"], optional=true }`；`implicit = ["cli", "dep:diffsol"]`（隐式住在 `sim` 里，`sim`/`parser` 现门控在 cli 下）。
- 纯 Rust、稠密 LU、**零 C 依赖**（不碰 SUNDIALS/SuiteSparse/LLVM/CUDA）→ 扛离线/中国 rsproxy 镜像。锁 `=0.16.1` 防 0.x breaking。
- spike 实测（`diffsol_spike/`）：R1 纯 Rust 无 C 编出 ✅；`.rhs_implicit`+FD J·v 喂 BDF 解刚性 2 态解析解 <1e-8 ✅；`Rc<RefCell>` 捕获编过跑通 → 闭包不要求 Send+Sync → 串行调用 → `RefCell<Env>` 设计安全 ✅（整段 solve 里 J·v 仅调数次，BDF 跨步复用 Jacobian）。

## 5. V1 验收（显式↔隐式一致性）

- **V1-0 微观**（in-crate `test_micro_stiff_analytic`）：2 态解耦刚性线性系统（k1=1000/k2=1），末值贴合解析解 <1e-6 → 证 diffsol 接线 + FD Jacobian + BDF 端到端在 EQC 内正确。
- **V1 in-crate 一致性**（`test_explicit_converges_to_implicit`）：含手写 `_prev` 的刚性单态热平衡，显式随 dt→0 单调 O(dt) 收敛到隐式，隐式贴合解析解 `T=15−5e^{−20t}`。
- **V1 真实模型**（`examples/v1_greenhouse.rs`）：真 `greenhouse_v1.eq.yaml` 三态，常数驱动，显式 Euler dt 减半误差**精确减半**（O(dt)），最细 dt=0.5s 各态相对误差 ~1e-5 ~ 2e-6 < 1e-3 → **PASS**。
- **强非线性刚性**（`test_robertson_stiff_nonlinear`）：Robertson 基准（双线性 `y2·y3`+二次 `y2²`+刚性比 ~1e10、y1~1 vs y2~1e-5 量级悬殊），质量守恒 <1e-6 + 末值贴文献 → 证 FD Jacobian 在真曲率+量级悬殊下正确（补微观解耦线性覆盖不足）。
- **零回归**：全套 324 测试通过（319 现有 + 5 隐式）；默认（无 feature）构建不含隐式、逐位不变。

## 5.1 对抗复审（双 agent 交叉证伪）结论 + 已修

两独立 agent（数值面 + 代码面）尽力证伪。**核心数值管线确认正确**（非巧合对）：FD Jacobian 逐列用单位向量装配（`use_coloring=false`+稠密，多分量并扰路径不可达）且只作 Newton 加速器（解由精确 RHS 定根，FD 失真只拖慢/失败不错值）；显式↔隐式各自独立贴解析参照。**已修的发现**：
- **BUG-1（真 bug·已修）**：`fold_prev` 原对所有 `_prev` 无差别折叠，但 `_prev` 有第二种用法=对非状态量做离散差分（`DRLG=RLG−RLG_prev`，草莓 6 模型惯用）→ 折叠致差分恒 0 静默错值。**修**：只折 `is_integrator()` 源（state-lag），源非 state 显式 `Err` 拒绝（连带挡链式 `_prev`）。回归测试 `test_fold_prev_rejects_auxiliary_diff_register`。
- **is_finite 段末守门（已修）**：原靠 diffsol 步长下限兜底、`unwrap_or(NAN)` 静默；现段末对 accepted 态 `is_finite` 复核，非有限 loud `Err`。
- **向量速率 loud 报（已修）**：`eval_rhs` 用 `as_scalar()` 的 `NotScalar` 错误替静默 NaN。

## 6. 已知边界 / 延后项

- **E2 平滑化（0b）**：`GH-UVENT/QHEAT` 的 clamp 控制律、`GH-PHIINJ/HEATSP/VENTSP` 的昼夜/季节门（仅 ctrl 变体有非光滑算子）。隐式 Newton 需 RHS C¹ 可导 → 平滑化 pass 后再上 ctrl。基座/crop 全光滑，V1 无需 E2。
- **耦合穿透（0c）**：`simulate_coupled` 的 fast 回路仍是显式 R 步；把隐式引擎穿到 fast 侧、multi-rate 契约改"边界窗口自适应内解 + 网格点采样聚合"，是 0c。
- **时变驱动**：当前逐段 ZOH。真实天气时变驱动下段边界跳变的误差界，随 0b/Phase 1 用 V2（GreenLight 交叉校核）量化。
- **性能**：每驱动步重建 diffsol problem（V1 规模够快）；大模型可优化为共享 problem + 状态重置。
- **FD Jacobian 优化（复审建议·延后）**：单边 FD + 全局 `‖x‖∞` 标度；Robertson 基准（量级差 1e5）已验足够，但极端量级悬殊（>1e5）真实多态系统宜升级**逐分量 `eps_j=(1+|x_j|)·√ε` / 逐分量 `atol` / 中心差分 / 稀疏着色**，防 `StepSizeTooSmall` 型吵闹失败。
- **守恒律核验隐式输出（复审建议·延后）**：`check_balance_laws` 现只对显式 `simulate` 输出跑（CLI）。隐式接线到 CLI/GP/optimize 时应一并对隐式输出核守恒（`SimOutput` 格式通用，直接可喂）。
- **auxiliary 差分寄存器的隐式语义（0b+）**：BUG-1 现为 loud 拒绝；正确语义 = 段初把 auxiliary 值算成本段常数（类比耦合反馈 hold），使 `DRLG=RLG(trial)−RLG(段初)` = 段内增量。接草莓等作物模型走隐式时再实现。
- **时变驱动 in-crate 测试（复审建议·延后）**：复审 A 探针已证时变驱动下段 ZOH 引入可控 O(dt)、非 bug；可把该探针固化进测试套件。
