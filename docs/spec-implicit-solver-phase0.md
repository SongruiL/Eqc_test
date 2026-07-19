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
逐驱动步 `[t_n, t_{n+1}]`：驱动量按**零阶保持（ZOH）**设为本段常数（+ 注入 `DAT`），构建 diffsol problem（init=上段末态）、`.bdf::<NalgebraLU>()` 自适应内解 `[0,dt]`、取段末态。这天然把有效最大步长限在 `dt` 内（diffsol builder 无 `max_step`，此为兜底）。常数驱动下逐段续解 = 精确连续解在网格点采样。段末 `is_finite` 守门。
> **★ 用 step-loop 而非高层 `solve()`**（0b 由 ctrl 尖拐角段挖出）：diffsol 高层 `solve(final_time)` 有 `max_steps_between_checkpoints` 机制，段内步数超限会**静默提前返回**（`interpolate(dt)` 越界报错）。改为 `set_stop_time(dt)` + `step()` 循环到 `TstopReached`、取 `state().y` 末态——保证精确抵达 `dt`，绕开 checkpoint 早返与 interpolate 边界。刚性/尖拐角段的稳健性关键。

### 3.5 E2 平滑化（`smooth_for_implicit` / `smooth_expr`，0b）
隐式 Newton 需 RHS **C¹ 可导**（Jacobian 良定义、步控稳）；非光滑算子（clamp/max/min/if…）产生阶跃/折点 Jacobian。E2 = 一个 AST pass，把**状态依赖**的非光滑算子换成 C¹ 平滑代理（`max(a,b)→0.5(a+b+√((a−b)²+ε²))`、`min` 对称、Clamp/Abs 同族）。
- **★外科式（段-ZOH 洞察）**：段内驱动/`DAT` 恒常 → 只依赖它们的开关（`if(I_glob≥…)`、`if(DAT<…)`）段内是常数、对 `∂f/∂state` 零贡献 → **留硬**（跳变落段边界由分段重启处理）。只平滑「自变量子树引用状态量」的开关（`if !refs∩states → 原样`）。ctrl 变体 5 条非光滑里**只 2 条 clamp（GH-UVENT/QHEAT，依赖 T_air）需平滑**。
- **★单个无量纲 ε**：clamp 自变量 `(T_air−setpt)/Pband` 已归一化 → 单个 ε（拐角圆化宽度，profile 参数 `smooth_eps`，默认建议 0.05）即可，无需 spec Q5 的逐物理量 pBand。物理带 Pband 本就在模型里。
- **覆盖**：算术容器（+−×÷^neg）递归 + Max/Min/Clamp/Abs 代理；关系/if（Phase 1 结露门 `if vp>vp_sat`）留待扩展。**只在 `ImplicitOpts.smooth_eps=Some(ε)` 时跑**；全光滑模型（V1 基座/crop）不跑 pass、逐位不变。

## 4. diffsol 集成（`implicit` feature）

`Cargo.toml`：`diffsol = { version = "=0.16.1", default-features=false, features=["nalgebra"], optional=true }`；`implicit = ["cli", "dep:diffsol"]`（隐式住在 `sim` 里，`sim`/`parser` 现门控在 cli 下）。
- 纯 Rust、稠密 LU、**零 C 依赖**（不碰 SUNDIALS/SuiteSparse/LLVM/CUDA）→ 扛离线/中国 rsproxy 镜像。锁 `=0.16.1` 防 0.x breaking。
- spike 实测（`diffsol_spike/`）：R1 纯 Rust 无 C 编出 ✅；`.rhs_implicit`+FD J·v 喂 BDF 解刚性 2 态解析解 <1e-8 ✅；`Rc<RefCell>` 捕获编过跑通 → 闭包不要求 Send+Sync → 串行调用 → `RefCell<Env>` 设计安全 ✅（整段 solve 里 J·v 仅调数次，BDF 跨步复用 Jacobian）。

## 5. V1 验收（显式↔隐式一致性）

- **V1-0 微观**（in-crate `test_micro_stiff_analytic`）：2 态解耦刚性线性系统（k1=1000/k2=1），末值贴合解析解 <1e-6 → 证 diffsol 接线 + FD Jacobian + BDF 端到端在 EQC 内正确。
- **V1 in-crate 一致性**（`test_explicit_converges_to_implicit`）：含手写 `_prev` 的刚性单态热平衡，显式随 dt→0 单调 O(dt) 收敛到隐式，隐式贴合解析解 `T=15−5e^{−20t}`。
- **V1 真实模型**（`examples/v1_greenhouse.rs`）：真 `greenhouse_v1.eq.yaml` 三态，常数驱动，显式 Euler dt 减半误差**精确减半**（O(dt)），最细 dt=0.5s 各态相对误差 ~1e-5 ~ 2e-6 < 1e-3 → **PASS**。
- **强非线性刚性**（`test_robertson_stiff_nonlinear`）：Robertson 基准（双线性 `y2·y3`+二次 `y2²`+刚性比 ~1e10、y1~1 vs y2~1e-5 量级悬殊），质量守恒 <1e-6 + 末值贴文献 → 证 FD Jacobian 在真曲率+量级悬殊下正确（补微观解耦线性覆盖不足）。
- **零回归**：全套 326 测试通过（319 现有 + 7 隐式）；默认（无 feature）构建不含隐式、逐位不变。

### 5.2 0b 验收（E2 平滑化 + ctrl 走隐式 + V4）
- **平滑 pass 外科式**（`test_smooth_pass_surgical`）：状态依赖 clamp 被平滑掉 Max/Min（引入 Sqrt）；驱动依赖 max 留硬。
- **控制律走隐式**（`test_ctrl_control_law_implicit`）：`Q_heat=Q_max·clamp((T_sp−T)/Pband,0,1)` 反馈加热模型，平滑-隐式收敛且贴硬-显式-细dt（bottom-line 决策差异 <0.1℃）。
- **V4 决策差异 ε-扫描**（`examples/v4_ctrl.rs`，真实 ctrl 变体，冷天加热 engage）：**ΔT_air末（点值=干净平滑误差信号）随 ε→0 呈 ~O(ε²) 收敛**（0.1→0.01：1.6e-2→1.6e-4℃）；加热决策 <1%（~0.4% 是 dt 求积伪影非平滑误差）；phantom venting 是 max0 关断残留（微小已标注）。**ε≈0.05 甜点**（决策差异可忽略 + 收敛稳）。

## 5.1 对抗复审（双 agent 交叉证伪）结论 + 已修

两独立 agent（数值面 + 代码面）尽力证伪。**核心数值管线确认正确**（非巧合对）：FD Jacobian 逐列用单位向量装配（`use_coloring=false`+稠密，多分量并扰路径不可达）且只作 Newton 加速器（解由精确 RHS 定根，FD 失真只拖慢/失败不错值）；显式↔隐式各自独立贴解析参照。**已修的发现**：
- **BUG-1（真 bug·已修）**：`fold_prev` 原对所有 `_prev` 无差别折叠，但 `_prev` 有第二种用法=对非状态量做离散差分（`DRLG=RLG−RLG_prev`，草莓 6 模型惯用）→ 折叠致差分恒 0 静默错值。**修**：只折 `is_integrator()` 源（state-lag），源非 state 显式 `Err` 拒绝（连带挡链式 `_prev`）。回归测试 `test_fold_prev_rejects_auxiliary_diff_register`。
- **is_finite 段末守门（已修）**：原靠 diffsol 步长下限兜底、`unwrap_or(NAN)` 静默；现段末对 accepted 态 `is_finite` 复核，非有限 loud `Err`。
- **向量速率 loud 报（已修）**：`eval_rhs` 用 `as_scalar()` 的 `NotScalar` 错误替静默 NaN。

## 6. 已知边界 / 延后项

- ~~**E2 平滑化（0b）**~~ **✅ 已完成**（见 §3.5 / §5.2）：外科式只平滑状态依赖开关（ctrl 只 2 条 clamp）+ 单无量纲 ε。关系/if 的平滑（Phase 1 结露门）留待扩展。
- **耦合穿透（0c）**：`simulate_coupled` 的 fast 回路仍是显式 R 步；把隐式引擎穿到 fast 侧、multi-rate 契约改"边界窗口自适应内解 + 网格点采样聚合"，是 0c。**延后到 Phase 2/3**（真有刚性温室↔作物耦合生产场景时；Phase 1 独立交叉校核不需耦合）。做法=零运行时行为的 `FastEngine` 抽取（显式 `Stepper` 成一个 impl、bit-identical），非破坏式重写；不碰模型 SSOT。
- **时变驱动**：当前逐段 ZOH。真实天气时变驱动下段边界跳变的误差界，随 0b/Phase 1 用 V2（GreenLight 交叉校核）量化。
- **性能**：每驱动步重建 diffsol problem（V1 规模够快）；大模型可优化为共享 problem + 状态重置。
- **FD Jacobian 优化（复审建议·延后）**：单边 FD + 全局 `‖x‖∞` 标度；Robertson 基准（量级差 1e5）已验足够，但极端量级悬殊（>1e5）真实多态系统宜升级**逐分量 `eps_j=(1+|x_j|)·√ε` / 逐分量 `atol` / 中心差分 / 稀疏着色**，防 `StepSizeTooSmall` 型吵闹失败。
- **守恒律核验隐式输出（复审建议·延后）**：`check_balance_laws` 现只对显式 `simulate` 输出跑（CLI）。隐式接线到 CLI/GP/optimize 时应一并对隐式输出核守恒（`SimOutput` 格式通用，直接可喂）。
- **auxiliary 差分寄存器的隐式语义（0b+）**：BUG-1 现为 loud 拒绝；正确语义 = 段初把 auxiliary 值算成本段常数（类比耦合反馈 hold），使 `DRLG=RLG(trial)−RLG(段初)` = 段内增量。接草莓等作物模型走隐式时再实现。
- **时变驱动 in-crate 测试（复审建议·延后）**：复审 A 探针已证时变驱动下段 ZOH 引入可控 O(dt)、非 bug；可把该探针固化进测试套件。
