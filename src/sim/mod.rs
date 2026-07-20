//! 逐日仿真引擎：对 [`EquationFile`] 描述的**动态过程模型**做显式 Euler 时间步进。
//!
//! 这是 B 路线的第二块：B1 给 schema 加了 Forrester 分类与状态量元数据
//! （`class` / `init` / `rate` / `prev`），本模块据此把一个静态方程网络「跑起来」，
//! 沿时间序列逐日积分出状态量轨迹（如累积干物质、累积温度、果实干重）。
//!
//! # 计算模型
//!
//! 每个时间步 `n`（步长 `dt`，缺省 1 = 日步长；可经模型 `meta.dt` 或 `SimInput.dt` 设亚日步长）按依赖拓扑序求值：
//!
//! - **驱动量 driving**：未被任何方程产生、非跨步的输入变量，逐日从 [`SimInput::drivers`] 取值。
//! - **参数 parameter**：来自 `parameters:` 的默认值，可被 [`SimInput::param_overrides`] 覆盖。
//! - **延迟寄存器 prev（半状态量）**：`X[n] = src[n-1]`，首步用 `init`。在步首即可定值
//!   （只依赖上一步），故视为「源」。
//! - **方程辅助/速率量**：普通 `equations:` 表达式，由 [`Expr::eval`] 求值。
//! - **积分状态量 state**：`X[n] = X[n-1] + rate[n]·dt`，`X[-1]` 用 `init`（显式 Euler）。
//!
//! 所有「当前步引用」一律解析为**本步已算出的值（n）**；唯一的跨步值是
//! ① 积分状态量自身的上一步（隐含在 `X[n]=X_prev+rate` 里）② 延迟寄存器（显式 `prev`）。
//! 若某速率方程直接引用了它所驱动的状态量当前值，会形成步内环 → 报 [`SimError::Cycle`]；
//! 这种情况应改为引用一个延迟寄存器（`prev`）来取上一步值。
//!
//! 求值沿用严格模式（除零/NaN/Inf 报错）；这正是过程模型期望的「早失败」。
//!
//! # 内置变量
//!
//! 每步注入一个保留变量 **`DAT`**（days after transplanting/start，从 1 起 = 当前天数），
//! 方程可直接引用做开花/物候门控（如 `active = geq(DAT, anthesis)`），无需手填驱动量。

use std::collections::{HashMap, HashSet, VecDeque};

use indexmap::IndexMap;

use crate::eval::{value_binop, Env, EvalError, Value};
use crate::schema::{BalanceLaw, DataType, EquationFile, VarClass, Variable, VariableType};

/// 仿真错误。
#[derive(Debug, Clone, PartialEq)]
pub enum SimError {
    /// 缺少驱动量的时间序列输入。
    MissingDriver(String),
    /// 驱动量序列长度与步数不一致。
    DriverLengthMismatch { name: String, expected: usize, found: usize },
    /// 跨步变量（state/prev）缺少 `init` 初值。
    MissingInit(String),
    /// `rate` / `prev` 指向了未定义的来源变量。
    UndefinedSource { var: String, source: String },
    /// 步内依赖存在环（含速率方程引用其状态量当前值的情形）。
    Cycle(Vec<String>),
    /// 某变量在本步从未被定值（方程缺失或拼写错误）。
    Unresolved(String),
    /// 某条方程的输出变量未在 `variables:` 中声明。
    UndeclaredOutput(String),
    /// 表达式求值出错。
    Eval { var: String, err: EvalError },
    /// 耦合仿真错误（缺 dt_seconds、R 非整数、接口非标量、慢驱动无链接等）。
    Coupling(String),
    /// 隐式求解器错误（diffsol 构建/求解失败、状态非标量等）。仅 `implicit` feature 用。
    Solver(String),
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimError::MissingDriver(n) => write!(f, "缺少驱动量时间序列: {n}"),
            SimError::DriverLengthMismatch { name, expected, found } => write!(
                f,
                "驱动量 {name} 序列长度 {found} 与步数 {expected} 不一致"
            ),
            SimError::MissingInit(n) => write!(f, "跨步变量 {n} 缺少 init 初值"),
            SimError::UndefinedSource { var, source } => {
                write!(f, "变量 {var} 的来源 {source} 未定义")
            }
            SimError::Cycle(c) => write!(f, "步内依赖存在环: {}", c.join(" -> ")),
            SimError::Unresolved(n) => write!(f, "变量 {n} 在本步未被定值（方程缺失？）"),
            SimError::UndeclaredOutput(n) => write!(f, "方程输出变量 {n} 未在 variables: 中声明"),
            SimError::Eval { var, err } => write!(f, "求值变量 {var} 出错: {err}"),
            SimError::Coupling(m) => write!(f, "耦合仿真: {m}"),
            SimError::Solver(m) => write!(f, "隐式求解器: {m}"),
        }
    }
}

/// 隐式刚性求解器（Phase 0 引擎地基）：接 diffsol 的 BDF，把手写 `_prev` 折叠回真态、
/// 解真联立系统。见 `docs/spec-implicit-solver-phase0.md`。仅 `implicit` feature 编入。
#[cfg(feature = "implicit")]
pub mod implicit;

impl std::error::Error for SimError {}

/// 仿真输入。
#[derive(Debug, Clone, Default)]
pub struct SimInput {
    /// 时间步数。
    pub steps: usize,
    /// 驱动量时间序列：变量名 -> 每步取值（长度须等于 `steps`）。
    pub drivers: HashMap<String, Vec<f64>>,
    /// 参数覆盖：变量名 -> 值（覆盖 `parameters:` 默认值）。
    pub param_overrides: HashMap<String, f64>,
    /// 初值覆盖：状态量/延迟寄存器名 -> 初值（覆盖变量上的 `init:`）。
    pub init_overrides: HashMap<String, f64>,
    /// 时间步长覆盖：`None` = 用模型 `meta.dt`（缺省 1.0，= 日步长）；`Some(x)` = 强制用 x。
    /// 状态量积分 `X[n] = X[n-1] + rate[n]·dt`。亚日动态模型（如温室气候 ODE）需小步长。
    pub dt: Option<f64>,
}

impl SimInput {
    /// 构造一个 `steps` 步的空输入。
    pub fn new(steps: usize) -> Self {
        Self {
            steps,
            drivers: HashMap::new(),
            param_overrides: HashMap::new(),
            init_overrides: HashMap::new(),
            dt: None,
        }
    }

    /// 链式加入一条驱动量序列。
    pub fn driver(mut self, name: impl Into<String>, series: Vec<f64>) -> Self {
        self.drivers.insert(name.into(), series);
        self
    }

    /// 链式覆盖一个参数。
    pub fn param(mut self, name: impl Into<String>, value: f64) -> Self {
        self.param_overrides.insert(name.into(), value);
        self
    }
}

/// 仿真输出：每个变量的逐日轨迹。
#[derive(Debug, Clone, PartialEq)]
pub struct SimOutput {
    /// 步数。
    pub steps: usize,
    /// 变量名 -> 逐日轨迹（保留 `variables:` 声明顺序）。
    pub trajectories: IndexMap<String, Vec<f64>>,
}

impl SimOutput {
    /// 取某变量的完整轨迹。
    pub fn series(&self, name: &str) -> Option<&[f64]> {
        self.trajectories.get(name).map(|v| v.as_slice())
    }

    /// 取某变量的末值（最后一步）。
    pub fn final_value(&self, name: &str) -> Option<f64> {
        self.trajectories.get(name).and_then(|v| v.last().copied())
    }
}

/// 一条守恒律的逐步残差核算（`|Δstock − dt·净流量/cap|` 的最大值）。
#[derive(Debug, Clone, Copy)]
pub struct BalanceResidual {
    /// 逐步最大绝对残差。
    pub max_resid: f64,
    /// 取到最大残差的步索引。
    pub argstep: usize,
    /// 相对残差 = max_resid / |stock 末值|（末值为 0 时记 0）。
    pub rel: f64,
}

/// 一条守恒律的核算结果（供 CLI `--check-balance` 诊断打印 + GP 硬过滤复用；**本函数不打印、不 IO**）。
#[derive(Debug, Clone)]
pub struct BalanceLawCheck {
    /// 守恒律名。
    pub name: String,
    /// 存量变量名。
    pub stock: String,
    /// 可选容量变量名。
    pub cap: Option<String>,
    /// 容差上限。
    pub tol: f64,
    /// 可核算时 = 残差；缺变量无法核算时 = None。
    pub residual: Option<BalanceResidual>,
    /// 无法核算的原因（存量/源汇/cap 缺轨迹）；可核算时 None。文案即 CLI 诊断的「⚠ …」后半句。
    pub skip_reason: Option<String>,
    /// 守恒是否通过：可核算且 `max_resid ≤ tol`。**缺变量跳过 = false**（与 CLI 旧 `any_fail` 同义）。
    pub ok: bool,
    /// §8：本条走的口径。`true` = 瞬时结构残差（`|rate−net/cap|`·solver-无关·找到权威 rate 变量）；
    /// `false` = 有限差分回退（`|Δstock−dt·net|`·仅显式机器零·状态量无 `rate:` 声明时）。
    pub structural: bool,
}

/// 守恒律逐步最大残差（F5c）。**★步对齐**：轨迹里 `state[n]=state[n-1]+dt·rate[n]`，且 `rate[n]`
/// 与源/汇 auxiliary 同步用 `state[n-1]`、记在【同一行 n】→「进入第 n 步的存量变化 Δstock[n]」对应
/// 【n 处】净流量 `net[n]`。差一步对齐会把「相邻步通量之差」误报成不守恒（早季通量爬升尤甚）。
/// 返回 `(max|残差|, 该步)`。
pub fn balance_residual(stock: &[f64], net: &[f64], dt: f64) -> (f64, usize) {
    let mut max_resid = 0.0f64;
    let mut argstep = 0usize;
    let n = stock.len().min(net.len());
    for t in 0..n.saturating_sub(1) {
        let resid = ((stock[t + 1] - stock[t]) - dt * net[t + 1]).abs();
        if resid > max_resid {
            max_resid = resid;
            argstep = t + 1;
        }
    }
    (max_resid, argstep)
}

/// **§8 瞬时结构残差**（守恒徽章新口径）：`max_t |rate_recorded[t] − net_eff[t]|`，
/// 其中 `net_eff = (Σsources − Σsinks)/cap`、`rate_recorded` = 状态量的已记录速率变量。
///
/// 验的是「**速率方程 ≡ 守恒律声明**」（authoring 守恒）：rate 与 net_eff 同一时刻、同一批
/// 记录值求值，方程照守恒律写时**按定义机器零、与积分器无关**（显式 Euler / 隐式 BDF 皆过）。
/// 相较 [`balance_residual`] 的有限差分口径 `|Δstock − dt·net|`（硬编码显式 Euler 步、隐式 BDF
/// 段内自适应多步时因求积错配误判超容差），本口径把「方程是否守恒」与「积分器是否忠实执行」解耦：
/// 前者=本律，后者=V1 双路径 / V6。漏一项汇 / 加平衡表外项立刻现形（rate ≠ net_eff）。
pub fn structural_residual(rate: &[f64], net_eff: &[f64]) -> (f64, usize) {
    let mut max_resid = 0.0f64;
    let mut argstep = 0usize;
    let n = rate.len().min(net_eff.len());
    for t in 0..n {
        let resid = (rate[t] - net_eff[t]).abs();
        if resid > max_resid {
            max_resid = resid;
            argstep = t;
        }
    }
    (max_resid, argstep)
}

/// §8：解析守恒律 `stock` 的**权威速率变量名**——来自状态量的 `rate:` 声明（SSOT），**非命名约定**。
/// 标量态：直查 `file.variables[stock].rate`；cohort 展开态 `base__i`：拆末尾 `__i` 后缀、查 base 的
/// rate 再接回（`FOM_N__1` → base `FOM_N` 的 rate `rate_FOMN` → `rate_FOMN__1`）。stock 非状态量/无
/// `rate:` → None（守恒律核算回退有限差分）。这样不同模型任意 rate 命名（`rate_CBuf`/`bal_rate`…）都对。
pub fn resolve_stock_rate(file: &EquationFile, stock: &str) -> Option<String> {
    if let Some(v) = file.variables.get(stock) {
        return v.rate.clone();
    }
    if let Some(idx) = stock.rfind("__") {
        let (base, suffix) = (&stock[..idx], &stock[idx..]);
        if let Some(v) = file.variables.get(base) {
            return v.rate.as_ref().map(|r| format!("{r}{suffix}"));
        }
    }
    None
}

/// 按 `meta.balance` 声明逐条核守恒律。**§8：守恒徽章 = 瞬时结构残差** `|rate − (Σ源−Σ汇)/cap|`
/// （验「速率方程 ≡ 守恒律声明」·solver-无关·显式/隐式 BDF 皆机器零；rate 变量经 [`resolve_stock_rate`]
/// 从 `variable.rate` 权威解析）；stock 无权威 rate 变量时回退有限差分 `|Δstock − dt·net|`（仅显式机器零，
/// 隐式会因求积错配假报——`structural=false` 标记之）。`cap` 缺省≡1。**纯核算、不打印**：CLI
/// `--check-balance` 消费它做诊断输出，GP 候选硬过滤（Tier3）也消费它——单一真相源。
/// 缺存量/源汇/cap 轨迹的守恒律记 `skip_reason` + `ok=false`（与旧 CLI `any_fail` 逐字节等义）。
pub fn check_balance_laws(
    laws: &[BalanceLaw],
    out: &SimOutput,
    dt: f64,
    file: &EquationFile,
) -> Vec<BalanceLawCheck> {
    let mut results = Vec::with_capacity(laws.len());
    for law in laws {
        let mk_skip = |reason: String| BalanceLawCheck {
            name: law.name.clone(),
            stock: law.stock.clone(),
            cap: law.cap.clone(),
            tol: law.tol,
            residual: None,
            skip_reason: Some(reason),
            ok: false,
            structural: false,
        };
        let stock = match out.trajectories.get(&law.stock) {
            Some(s) => s,
            None => {
                results.push(mk_skip(format!("存量 {} 不在轨迹（跳过）", law.stock)));
                continue;
            }
        };
        let collect = |names: &[String]| -> Option<Vec<&Vec<f64>>> {
            names.iter().map(|n| out.trajectories.get(n)).collect()
        };
        let (srcs, snks) = match (collect(&law.sources), collect(&law.sinks)) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                results.push(mk_skip("源/汇变量缺失（跳过）".to_string()));
                continue;
            }
        };
        // 逐步净流量 net[t] = Σsources[t] − Σsinks[t]
        let net: Vec<f64> = (0..out.steps)
            .map(|t| {
                srcs.iter().map(|s| s.get(t).copied().unwrap_or(0.0)).sum::<f64>()
                    - snks.iter().map(|s| s.get(t).copied().unwrap_or(0.0)).sum::<f64>()
            })
            .collect();
        // cap：可选「有效容量」变量（缺省 cap≡1）。逐步 net/cap 后再核算。
        let net_eff: Vec<f64> = match &law.cap {
            None => net,
            Some(capname) => match out.trajectories.get(capname) {
                Some(capser) => net
                    .iter()
                    .enumerate()
                    .map(|(t, &nt)| {
                        let c = capser.get(t).copied().unwrap_or(1.0);
                        if c != 0.0 {
                            nt / c
                        } else {
                            nt
                        }
                    })
                    .collect(),
                None => {
                    results.push(mk_skip(format!("cap 变量 {capname} 不在轨迹（跳过）")));
                    continue;
                }
            },
        };
        // §8：优先瞬时结构残差（solver-无关·验速率方程≡守恒律声明）。速率变量经 resolve_stock_rate
        // 从 variable.rate 权威解析（非命名约定·任意 rate 命名都对）；无 rate 声明则回退有限差分（显式机器零）。
        let (max_resid, argstep, used_structural) = match resolve_stock_rate(file, &law.stock)
            .as_deref()
            .and_then(|rn| out.trajectories.get(rn))
        {
            Some(rate_series) => {
                let (r, a) = structural_residual(rate_series, &net_eff);
                (r, a, true)
            }
            None => {
                let (r, a) = balance_residual(stock, &net_eff, dt);
                (r, a, false)
            }
        };
        let scale = stock.last().map(|x| x.abs()).unwrap_or(0.0);
        let rel = if scale > 0.0 { max_resid / scale } else { 0.0 };
        let ok = max_resid <= law.tol;
        results.push(BalanceLawCheck {
            name: law.name.clone(),
            stock: law.stock.clone(),
            cap: law.cap.clone(),
            tol: law.tol,
            residual: Some(BalanceResidual { max_resid, argstep, rel }),
            skip_reason: None,
            ok,
            structural: used_structural,
        });
    }
    results
}

/// 步内可计算节点的种类。
enum Node<'a> {
    /// 普通方程：输出变量 = 表达式。
    Equation(&'a crate::ast::Expr),
    /// 积分状态量：值 = 上一步值 + rate 当前值；`init` 为首步前的值。
    Integrator { rate: &'a str, init: f64 },
}

/// 一条「步内计算」（带变量名），拓扑序排列。供 [`simulate`] 与 `eqc build` 代码生成器共用。
#[derive(Clone, Copy)]
pub enum PlanStep<'a> {
    /// 方程：`name = expr`。
    Equation { name: &'a str, expr: &'a crate::ast::Expr },
    /// 积分状态量：`X[n] = X[n-1] + rate[n]`，首步 `X[n-1] = init`。
    Integrator { name: &'a str, rate: &'a str, init: f64 },
}

/// **步进计划**：把动态模型编译成「逐日时间步进要做的事」——拓扑序的步内计算、
/// 延迟寄存器、驱动量清单。是 [`simulate`]（树遍历引擎）与 `eqc build`（生成独立仿真器）
/// 之间的**单一真相源**：两者消费同一份计划 → 生成的代码与引擎逐步一致。
pub struct SimPlan<'a> {
    /// 拓扑序的步内计算（方程 + 积分状态量）。
    pub steps: Vec<PlanStep<'a>>,
    /// 延迟寄存器：`(name, 来源, init)`，`X[n] = src[n-1]`，首步 = `init`。
    pub delays: Vec<(&'a str, &'a str, f64)>,
    /// 驱动量名（无方程、非跨步的输入；须由外部按步提供时间序列）。
    pub drivers: Vec<&'a str>,
}

/// 把模型编译成步进计划：归类变量（积分/延迟/方程/驱动）+ 步内拓扑排序。
/// `init` 取自变量上的 `init:`（运行期可再被 [`SimInput::init_overrides`] 覆盖）。
pub fn build_plan(file: &EquationFile) -> Result<SimPlan<'_>, SimError> {
    // 方程输出 -> 表达式
    let mut eq_of: HashMap<&str, &crate::ast::Expr> = HashMap::new();
    for eq in &file.equations {
        if !file.variables.contains_key(&eq.output) {
            return Err(SimError::UndeclaredOutput(eq.output.clone()));
        }
        eq_of.insert(eq.output.as_str(), &eq.expression);
    }

    // 步内可计算节点：方程输出 ∪ 积分状态量。延迟寄存器/驱动/参数为「源」。
    let mut nodes: Vec<(&str, Node)> = Vec::new();
    let mut node_idx: HashMap<&str, usize> = HashMap::new();
    let mut delays: Vec<(&str, &str, f64)> = Vec::new();
    for (name, var) in &file.variables {
        let n = name.as_str();
        if var.is_integrator() {
            let rate = var.rate.as_deref().unwrap();
            let init = var.init.ok_or_else(|| SimError::MissingInit(name.clone()))?;
            if !file.variables.contains_key(rate) && !file.parameters.contains_key(rate) {
                return Err(SimError::UndefinedSource { var: name.clone(), source: rate.to_string() });
            }
            node_idx.insert(n, nodes.len());
            nodes.push((n, Node::Integrator { rate, init }));
        } else if var.is_delay() {
            let src = var.prev.as_deref().unwrap();
            let init = var.init.ok_or_else(|| SimError::MissingInit(name.clone()))?;
            if !file.variables.contains_key(src) && !file.parameters.contains_key(src) {
                return Err(SimError::UndefinedSource { var: name.clone(), source: src.to_string() });
            }
            delays.push((n, src, init));
        } else if let Some(expr) = eq_of.get(n) {
            node_idx.insert(n, nodes.len());
            nodes.push((n, Node::Equation(expr)));
        }
        // 其余 = 驱动量
    }

    // 驱动量 = 既无方程、又非跨步者
    let delay_names: HashSet<&str> = delays.iter().map(|(n, _, _)| *n).collect();
    let mut drivers: Vec<&str> = Vec::new();
    for (name, _var) in &file.variables {
        let n = name.as_str();
        if !node_idx.contains_key(n) && !delay_names.contains(n) {
            drivers.push(n);
        }
    }

    let order = topo_order(&nodes, &node_idx)?;
    let steps = order
        .iter()
        .map(|&i| {
            let name = nodes[i].0;
            match &nodes[i].1 {
                Node::Equation(expr) => PlanStep::Equation { name, expr: *expr },
                Node::Integrator { rate, init } => {
                    PlanStep::Integrator { name, rate: *rate, init: *init }
                }
            }
        })
        .collect();

    Ok(SimPlan { steps, delays, drivers })
}

/// **E5b（显式向 `_prev` 自动插入）**：对 `_prev`-free 源自动检测 rate→state 步内环、
/// 插入延迟寄存器（`<state>_prev`），并把「喂速率」的方程里对状态量的直读改写为读上一步值。
///
/// 这是 [`fold_prev_for_implicit`](crate::sim::implicit::fold_prev_for_implicit) 的**镜像**：
/// 隐式路径把手写 `_prev` **折回**真态（解真联立系统）；显式路径反过来，对作者写的真联立
/// 方程自动**补** `_prev` 破环（前向 Euler 用上一步状态算速率）。让作者按 P5「写真联立方程、
/// 不手维护 `_prev`」，两种求解模式各自变换。
///
/// # 语义（前向 Euler）
/// - **喂速率的方程**（反向可达任一 `rate` 变量的方程）里对状态量 `X` 的直读 → `X_prev`。
/// - **纯诊断**（不喂任何速率、在状态更新后算）保留本步值——与手写约定一致（温室三态的
///   `es`/`chi_sat`/`RH` 用本步 `T_air`/`H_air`，无环）。
///
/// # no-op 保证（双约定兼容）
/// 若模型已能拓扑排序（含手写 `_prev` 者、或天然无环者）→ **原样克隆返回**（不插不改）。
/// 只有 `build_plan` 报 [`SimError::Cycle`] 时才介入；介入后仍有环（如两辅助量互引的**真环**，
/// 非 `_prev` 可破）→ 传播 `Cycle`。
///
/// # 只建「状态量」延迟寄存器
/// 只为破 rate→state 环建 `prev: <state>` 寄存器；**绝不**创建「源是 auxiliary」的差分寄存器
/// （那是建模选择、须作者手写，如 `DRLG = RLG − RLG_prev`）——与 E5a 的拒绝逻辑对称。
///
/// # 命名守卫（loud fail）
/// 生成名 `<X>_prev` 若已被占用且非「prev: X」的匹配寄存器（另一态的 `_prev`、方程输出、参数、
/// 甚至同名状态）→ 返回 [`SimError::Solver`]，拒绝静默复用/覆盖/双声明。这同时挡住「状态恰好叫
/// `<Y>_prev`」时顺序 `substitute` 的变量捕获（目标名=源状态时落守卫）。
///
/// # E5a↔E5b 往返范围（诚实边界）
/// 「E5a 折叠 → E5b 重建 = 逐位一致」仅对**诊断不含状态后向差分**的模型成立。若原模型有
/// `dX = X − X_prev`（`X_prev` 源是状态、作者要上一步值做差分），E5a `fold_prev_for_implicit`
/// 会把它折成 `X − X = 0`（信息在 E5a 阶段丢失、非本函数可恢复）。全物理基座的诊断读**本步**
/// 状态（无后向差分），在范围内。
pub fn insert_prev_for_explicit(file: &EquationFile) -> Result<EquationFile, SimError> {
    // 已能拓扑排序 → no-op（手写 `_prev` / 天然无环）。只有真·rate→state 步内环才介入。
    match build_plan(file) {
        Ok(_) => return Ok(file.clone()),
        Err(SimError::Cycle(_)) => {}
        // MissingInit/UndefinedSource/UndeclaredOutput 等真错误非 E5b 职责，直接上抛。
        Err(e) => return Err(e),
    }

    let mut out = file.clone();

    // 状态量（声明 rate 的 integrator）与方程输出集合。
    let states: HashSet<String> = out
        .variables
        .iter()
        .filter(|(_, v)| v.is_integrator())
        .map(|(n, _)| n.clone())
        .collect();
    let eq_outputs: HashSet<String> = out.equations.iter().map(|e| e.output.clone()).collect();
    // 各方程的变量引用（改写前的原始结构）。
    let refs_of: HashMap<String, Vec<String>> = out
        .equations
        .iter()
        .map(|e| (e.output.clone(), e.expression.get_variable_refs()))
        .collect();

    // R = 「喂速率」的方程输出集：从每个 `rate` 变量出发沿方程依赖反向可达，止于非方程输出
    // （=状态量/驱动/参数=源）。前向 Euler 里 R 中每条方程都在某速率的计算锥内。
    let mut feeds_rate: HashSet<String> = HashSet::new();
    let mut work: Vec<String> = out
        .variables
        .values()
        .filter_map(|v| v.rate.clone())
        .filter(|r| eq_outputs.contains(r))
        .collect();
    while let Some(o) = work.pop() {
        if !feeds_rate.insert(o.clone()) {
            continue;
        }
        if let Some(rs) = refs_of.get(&o) {
            for r in rs {
                if eq_outputs.contains(r) && !feeds_rate.contains(r) {
                    work.push(r.clone());
                }
            }
        }
    }

    // 扫描「喂速率」方程里被直读的状态量 = 需建 `_prev` 寄存器（先只收集、不改）。
    let mut regs_needed: Vec<String> = Vec::new();
    for eq in &out.equations {
        if !feeds_rate.contains(&eq.output) {
            continue;
        }
        if let Some(rs) = refs_of.get(&eq.output) {
            for x in rs {
                if states.contains(x) && !regs_needed.iter().any(|s| s == x) {
                    regs_needed.push(x.clone());
                }
            }
        }
    }

    // **命名新鲜性/冲突守卫（改写前先查，loud fail）**：`<X>_prev` 已存在时——仅当它正是
    // 「prev: X」的延迟寄存器才复用（兼容部分手写）；被别的变量占用（另一态的 `_prev`、方程
    // 输出、甚至同名状态）或被参数占用 → 拒绝静默复用/覆盖/双声明，报错要求改名。这一守卫也
    // 挡住「状态恰好叫 `<Y>_prev`」引发的顺序 substitute 变量捕获（目标名=某源状态 → 落守卫）。
    for x in &regs_needed {
        let pname = format!("{x}_prev");
        if let Some(existing) = out.variables.get(pname.as_str()) {
            if existing.prev.as_deref() != Some(x.as_str()) {
                return Err(SimError::Solver(format!(
                    "E5b 破环需为状态量 '{x}' 建延迟寄存器 '{pname}'，但该名已被占用且非 \
                     'prev: {x}' 的延迟寄存器——拒绝静默复用/覆盖（会致错值）。请重命名冲突变量。"
                )));
            }
        } else if out.parameters.contains_key(pname.as_str()) {
            return Err(SimError::Solver(format!(
                "E5b 破环需为状态量 '{x}' 建延迟寄存器 '{pname}'，但该名已是参数——拒绝造成 \
                 参数/变量 同名双声明。请重命名冲突参数。"
            )));
        }
    }

    // 改写「喂速率」方程里对状态量的直读 → `<state>_prev`（守卫已过 → 目标名新鲜/匹配、无捕获）。
    for eq in &mut out.equations {
        if !feeds_rate.contains(&eq.output) {
            continue;
        }
        if let Some(rs) = refs_of.get(&eq.output) {
            for x in rs {
                if states.contains(x) {
                    eq.expression =
                        eq.expression.substitute(x, &crate::ast::Expr::Var(format!("{x}_prev")));
                }
            }
        }
    }

    // 新建状态量延迟寄存器（守卫确认过的匹配寄存器已存在则跳过：兼容部分手写 `_prev`）。
    for x in &regs_needed {
        let pname = format!("{x}_prev");
        if out.variables.contains_key(pname.as_str()) {
            continue;
        }
        let (unit, init) = {
            let sv = out.variables.get(x.as_str()).expect("state var must exist");
            (sv.unit.clone(), sv.init)
        };
        let reg = Variable {
            var_type: VariableType::Intermediate,
            dtype: DataType::Float,
            unit,
            description: None,
            label: None,
            measurable: false,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class: Some(VarClass::SemiState),
            init,
            rate: None,
            prev: Some(x.clone()),
            instance: None,
        };
        out.variables.insert(pname, reg);
    }

    // 校验：介入后若仍有环（非 rate→state 的真环，如两辅助量互引）→ 传播 `Cycle`。
    build_plan(&out)?;
    Ok(out)
}

/// **单模型步进器**：持有一个模型的步进计划 + 跨步状态（env / prev / 参数），可被外部
/// **按步驱动**（每步提供驱动量值），步后读任意变量当前值。`simulate`（单模型）与
/// `simulate_coupled`（多速率耦合）共用它 → 耦合每步与单模型逐步**一致**（单一真相源）。
///
/// 每步语义与原 `simulate` 循环体逐位一致：DAT → 驱动 → 延迟寄存器 → 拓扑序方程/积分
/// → 首步向量延迟形状修正 → 快照 prev。
pub struct Stepper<'a> {
    file: &'a EquationFile,
    plan: SimPlan<'a>,
    params: HashMap<&'a str, Value>,
    init_overrides: HashMap<String, f64>,
    dt: f64,
    env: Env,
    prev: HashMap<String, Value>,
    n: usize,
}

impl<'a> Stepper<'a> {
    /// 新建步进器：编译计划、装入参数（向量参数→Vector，标量→可被覆盖）。
    pub fn new(
        file: &'a EquationFile,
        dt: f64,
        param_overrides: &HashMap<String, f64>,
        init_overrides: &HashMap<String, f64>,
    ) -> Result<Self, SimError> {
        let plan = build_plan(file)?;
        let mut params: HashMap<&str, Value> = HashMap::new();
        for (pname, p) in &file.parameters {
            let v = match &p.values {
                Some(vals) => Value::Vector(vals.clone()),
                None => Value::Scalar(param_overrides.get(pname).copied().unwrap_or(p.default)),
            };
            params.insert(pname.as_str(), v);
        }
        let mut env = Env::new();
        for (pname, v) in &params {
            env.put(pname, v.clone());
        }
        Ok(Self {
            file,
            plan,
            params,
            init_overrides: init_overrides.clone(),
            dt,
            env,
            prev: HashMap::new(),
            n: 0,
        })
    }

    /// 该模型需要外部每步供值的驱动量名。
    pub fn drivers(&self) -> &[&'a str] {
        &self.plan.drivers
    }

    /// 当前已完成的步数（= 下一步的 0 基下标；步后递增）。
    pub fn step_index(&self) -> usize {
        self.n
    }

    /// 推进一步。`get_driver(name)` 供本步每个驱动量的标量值（缺 → `MissingDriver`）。
    /// 步后用 [`Stepper::get`] 读本步任意变量的值。
    pub fn step(&mut self, get_driver: impl Fn(&str) -> Option<f64>) -> Result<(), SimError> {
        let n = self.n;
        // DAT = 第几天（1 起）
        self.env.put("DAT", (n + 1) as f64);
        // 驱动量（标量）
        for &d in &self.plan.drivers {
            let val = get_driver(d).ok_or_else(|| SimError::MissingDriver(d.to_string()))?;
            self.env.put(d, val);
        }
        // 延迟寄存器：X[n] = src[n-1]（首步用 init 标量广播）
        for &(name, src, init) in &self.plan.delays {
            let init0 = self.init_overrides.get(name).copied().unwrap_or(init);
            let v = if n == 0 {
                Value::Scalar(init0)
            } else {
                self.prev
                    .get(src)
                    .cloned()
                    .or_else(|| self.params.get(src).cloned())
                    .ok_or_else(|| SimError::Unresolved(name.to_string()))?
            };
            self.env.put(name, v);
        }
        // 拓扑序求值方程与积分状态量（PlanStep 是 Copy，按下标取出即释放对 self.plan 的借用，
        // 不与 self.env 可变借用冲突、也不每步分配）。
        let dt = self.dt;
        for i in 0..self.plan.steps.len() {
            let step = self.plan.steps[i];
            match step {
                PlanStep::Equation { name, expr } => {
                    let v = expr
                        .eval_in(&mut self.env)
                        .map_err(|err| SimError::Eval { var: name.to_string(), err })?;
                    self.env.put(name, v);
                }
                PlanStep::Integrator { name, rate, init } => {
                    let init0 = self.init_overrides.get(name).copied().unwrap_or(init);
                    let prev_val = if n == 0 {
                        Value::Scalar(init0)
                    } else {
                        self.prev
                            .get(name)
                            .cloned()
                            .ok_or_else(|| SimError::Unresolved(name.to_string()))?
                    };
                    let rate_val =
                        self.env.get(rate).ok_or_else(|| SimError::Unresolved(rate.to_string()))?;
                    let x = value_binop(&prev_val, &rate_val, |a, b| a + b * dt)
                        .map_err(|err| SimError::Eval { var: name.to_string(), err })?;
                    self.env.put(name, x);
                }
            }
        }
        // 首步：向量延迟寄存器的标量 init 广播到来源形状（只修记录形状，不改数值）
        if n == 0 {
            for &(name, src, init) in &self.plan.delays {
                let init0 = self.init_overrides.get(name).copied().unwrap_or(init);
                if let Some(src_val) = self.env.get(src) {
                    let shaped = value_binop(&Value::Scalar(init0), &src_val, |a, _| a)
                        .map_err(|err| SimError::Eval { var: name.to_string(), err })?;
                    self.env.put(name, shaped);
                }
            }
        }
        // 快照本步全部声明变量 → prev（供下一步积分/延迟）
        let mut cur: HashMap<String, Value> = HashMap::new();
        for name in self.file.variables.keys() {
            let v = self.env.get(name).ok_or_else(|| SimError::Unresolved(name.clone()))?;
            cur.insert(name.clone(), v);
        }
        self.prev = cur;
        self.n += 1;
        Ok(())
    }

    /// 读当前步某声明变量的 Value（步后调用）。
    pub fn get(&self, name: &str) -> Option<Value> {
        self.env.get(name)
    }
}

/// 对一个动态模型做逐日仿真。
///
/// 单模块求值：跨模块 `source` 耦合不在此处展开——任何未被方程产生、非跨步的
/// 输入变量都必须由 [`SimInput::drivers`] 提供。**薄封装在 [`Stepper`] 上**（耦合仿真共用）。
pub fn simulate(file: &EquationFile, input: &SimInput) -> Result<SimOutput, SimError> {
    // 时间步长：SimInput.dt 覆盖 > 模型 meta.dt（缺省 1.0=日步长）。状态量积分用 X+=rate·dt。
    let dt = input.dt.unwrap_or(file.meta.dt);
    // E5b（显式向 `_prev` 自动插入）：对 `_prev`-free 源自动补延迟寄存器破 rate→state 步内环；
    // 已可拓扑排序者（含手写 `_prev`、天然无环）原样返回 → 现有模型逐位不变、零回归。
    let sim_file = insert_prev_for_explicit(file)?;
    let mut stepper = Stepper::new(&sim_file, dt, &input.param_overrides, &input.init_overrides)?;

    // 校验驱动量：每个驱动量须有长度=steps 的时间序列（保持原错误语义）。
    for &dn in stepper.drivers() {
        match input.drivers.get(dn) {
            None => return Err(SimError::MissingDriver(dn.to_string())),
            Some(series) if series.len() != input.steps => {
                return Err(SimError::DriverLengthMismatch {
                    name: dn.to_string(),
                    expected: input.steps,
                    found: series.len(),
                })
            }
            Some(_) => {}
        }
    }

    let mut traj: IndexMap<String, Vec<f64>> = IndexMap::new();
    for n in 0..input.steps {
        stepper.step(|d| input.drivers.get(d).map(|s| s[n]))?;
        for name in sim_file.variables.keys() {
            let v = stepper.get(name).ok_or_else(|| SimError::Unresolved(name.clone()))?;
            flatten_into(&mut traj, name, &v);
        }
    }

    Ok(SimOutput { steps: input.steps, trajectories: traj })
}

// ============================================================================
// 耦合仿真（C1：多速率、单向）—— 见 docs/spec-coupled-simulation.md
// ============================================================================

/// 快→慢聚合算子。`mean`=慢步内时均；`integral`=时间积分 `Σx·dt_fast·scale`；`last`=慢步末值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    Mean,
    Integral,
    Last,
}

impl Agg {
    /// 解析 `mean`/`integral`/`last`（未知 → None）。
    pub fn parse(s: &str) -> Option<Agg> {
        match s.trim() {
            "mean" => Some(Agg::Mean),
            "integral" => Some(Agg::Integral),
            "last" => Some(Agg::Last),
            _ => None,
        }
    }
}

/// 一条快→慢链接：慢模型驱动 `to` ← 快模型输出 `from`，按 `agg` 聚合，再乘 `scale`（单位换算）。
/// 例：`Sr ← Q_sun, integral, scale=1e-6`（W/m²·s→MJ/m²）；`T ← T_air, mean, scale=1`。
#[derive(Debug, Clone)]
pub struct CoupledLink {
    pub to: String,
    pub from: String,
    pub agg: Agg,
    pub scale: f64,
}

/// 一条慢→快反馈（C2 双向）：快模型输入 `to` ← 慢模型输出 `from`，乘 `scale`（单位换算）。
/// **滞后一慢步**：本慢步内快模型用慢模型**上一步**的值（常数 hold；首步用 `init`）——把引擎的
/// `_prev` 破环哲学抬到耦合界面，无步内代数环。例：温室 `phi_ass ← 作物 assim_flux_inst`。
#[derive(Debug, Clone)]
pub struct FeedbackLink {
    pub to: String,
    pub from: String,
    pub scale: f64,
    /// 首慢步（作物尚未跑过）的 hold 值。
    pub init: f64,
}

/// 耦合仿真输入（C1：两模型、单向 快→慢）。
pub struct CoupledInput<'a> {
    /// 快模型（如温室，小 dt）。
    pub fast: &'a EquationFile,
    /// 慢模型（如作物，大 dt）。
    pub slow: &'a EquationFile,
    /// 快→慢链接（慢模型每个驱动量都须被某条链接覆盖）。
    pub links: Vec<CoupledLink>,
    /// 慢→快反馈（C2 双向，滞后一慢步；空 = 单向 C1）。其 `to` 是快模型输入，由反馈供值
    /// （故不必出现在 `fast_drivers` 里）。
    pub feedback: Vec<FeedbackLink>,
    /// 快模型室外驱动，每条长度须 = `slow_steps · R`。
    pub fast_drivers: HashMap<String, Vec<f64>>,
    /// 慢步数（作物天数）。
    pub slow_steps: usize,
    pub fast_params: HashMap<String, f64>,
    pub slow_params: HashMap<String, f64>,
    pub fast_init: HashMap<String, f64>,
    pub slow_init: HashMap<String, f64>,
    /// 0c：快模型（刚性温室）在耦合回路里走隐式 BDF（否则显式 Euler 在亚日 dt 发散）。
    /// 缺省 false = 显式（现有行为·bit-identical）；true 需 `implicit` feature。
    pub fast_implicit: bool,
}

impl<'a> CoupledInput<'a> {
    /// 便捷构造（只给两模型 + 链接 + 快驱动 + 步数；覆盖项空）。
    pub fn new(
        fast: &'a EquationFile,
        slow: &'a EquationFile,
        links: Vec<CoupledLink>,
        fast_drivers: HashMap<String, Vec<f64>>,
        slow_steps: usize,
    ) -> Self {
        Self {
            fast,
            slow,
            links,
            feedback: Vec::new(),
            fast_drivers,
            slow_steps,
            fast_params: HashMap::new(),
            slow_params: HashMap::new(),
            fast_init: HashMap::new(),
            slow_init: HashMap::new(),
            fast_implicit: false,
        }
    }
}

/// **快模型引擎**（0c）：耦合 fast 回路的步进抽象——显式 [`Stepper`]（现有·bit-identical）或
/// 隐式 [`implicit::ImplicitStepper`]（刚性温室在回路里走 BDF）。couple 回路只用 `drivers`/`step`/`get`
/// 三能力；显式变体转发到**同一** `Stepper`（零行为·非另一份实现）。
enum FastEngineKind<'a> {
    Explicit(Stepper<'a>),
    #[cfg(feature = "implicit")]
    Implicit(crate::sim::implicit::ImplicitStepper),
}

struct FastEngine<'a> {
    kind: FastEngineKind<'a>,
    /// 驱动量名（构造时从引擎拷出·供 couple 校验·不进数值路径）。
    driver_names: Vec<String>,
}

impl<'a> FastEngine<'a> {
    fn explicit(s: Stepper<'a>) -> Self {
        let driver_names = s.drivers().iter().map(|d| d.to_string()).collect();
        Self { kind: FastEngineKind::Explicit(s), driver_names }
    }
    #[cfg(feature = "implicit")]
    fn implicit(s: crate::sim::implicit::ImplicitStepper) -> Self {
        let driver_names = s.drivers().to_vec();
        Self { kind: FastEngineKind::Implicit(s), driver_names }
    }
    fn drivers(&self) -> &[String] {
        &self.driver_names
    }
    fn step(&mut self, get_driver: impl Fn(&str) -> Option<f64>) -> Result<(), SimError> {
        match &mut self.kind {
            FastEngineKind::Explicit(s) => s.step(get_driver),
            #[cfg(feature = "implicit")]
            FastEngineKind::Implicit(s) => s.step(get_driver),
        }
    }
    fn get(&self, name: &str) -> Option<Value> {
        match &self.kind {
            FastEngineKind::Explicit(s) => s.get(name),
            #[cfg(feature = "implicit")]
            FastEngineKind::Implicit(s) => s.get(name),
        }
    }
}

/// 按 `implicit` 标志建快模型引擎：false→显式 Stepper（现有·bit-identical）；true→隐式（需 feature）。
fn build_fast_engine<'a>(
    fast: &'a EquationFile,
    params: &HashMap<String, f64>,
    init: &HashMap<String, f64>,
    implicit: bool,
) -> Result<FastEngine<'a>, SimError> {
    if implicit {
        #[cfg(feature = "implicit")]
        {
            // 平滑 eps=0.05 与单模型 `simulate --implicit` 生产路径一致（run_simulate_implicit）。
            let opts = crate::sim::implicit::ImplicitOpts {
                smooth_eps: Some(0.05),
                ..Default::default()
            };
            return Ok(FastEngine::implicit(crate::sim::implicit::ImplicitStepper::new(
                fast,
                fast.meta.dt,
                params,
                init,
                opts,
            )?));
        }
        #[cfg(not(feature = "implicit"))]
        {
            return Err(SimError::Coupling(
                "耦合快模型走隐式需 `cargo build --features implicit` 构建（默认不含隐式 BDF）".into(),
            ));
        }
    }
    Ok(FastEngine::explicit(Stepper::new(fast, fast.meta.dt, params, init)?))
}

/// 耦合仿真输出。
pub struct CoupledOutput {
    /// 慢步数。
    pub slow_steps: usize,
    /// 每慢步的快步数 R = dt_slow_秒 / dt_fast_秒。
    pub r: usize,
    /// 慢模型（作物）逐步轨迹。
    pub slow: SimOutput,
    /// 快模型（温室）轨迹，**日均聚合到慢分辨率**（D6；标量变量取慢步内时均）。
    /// A/B 看反馈对温室气候（如 CO₂）的影响即用它。
    pub fast: SimOutput,
    /// 每慢步喂给慢模型的聚合驱动值（= 等效的离线 aggregate CSV，便于核对/插图）。
    pub fed_drivers: IndexMap<String, Vec<f64>>,
}

/// 多速率耦合仿真（C1：单向 快→慢，无反馈）。
///
/// 每慢步：跑 R 个快步（快模型按室外驱动推进、累加链接聚合）→ 聚合收尾得慢模型本步驱动
/// → 慢模型推进一步。复用 [`Stepper`]，故耦合每步与单模型 [`simulate`] 逐步一致。
pub fn simulate_coupled(input: &CoupledInput) -> Result<CoupledOutput, SimError> {
    let fast = input.fast;
    let slow = input.slow;

    // —— 时间尺度：各模型自描述 dt_seconds；R = dt_slow_秒 / dt_fast_秒（须为正整数）——
    let dtf_s = fast.meta.dt_seconds.ok_or_else(|| {
        SimError::Coupling(format!("快模型 {} 缺 meta.dt_seconds（耦合需统一到秒）", fast.meta.id))
    })?;
    let dts_s = slow.meta.dt_seconds.ok_or_else(|| {
        SimError::Coupling(format!("慢模型 {} 缺 meta.dt_seconds", slow.meta.id))
    })?;
    if dtf_s <= 0.0 || dts_s <= 0.0 {
        return Err(SimError::Coupling("dt_seconds 必须为正".into()));
    }
    if dts_s < dtf_s {
        return Err(SimError::Coupling(format!(
            "慢模型 dt_seconds={dts_s} 应 ≥ 快模型 dt_seconds={dtf_s}（fast/slow 角色弄反？）"
        )));
    }
    let r_f = dts_s / dtf_s;
    let r = r_f.round() as usize;
    if r == 0 || (r_f - r as f64).abs() > 1e-9 {
        return Err(SimError::Coupling(format!(
            "dt_slow/dt_fast = {dts_s}/{dtf_s} = {r_f} 非整数，多速率需整数倍"
        )));
    }
    let total_fast = input.slow_steps * r;

    // —— 校验：慢模型每个驱动量须被 link 覆盖，或由共享室外驱动(weather CSV)供给 ——
    // 原设计假设「慢模型气候全来自快模型」（温室→作物单向级联）；泛化后支持**对等双向耦合**：
    // 两模型都读同一份室外气象（如 apple↔soil：apple 要 T/Sr、soil 要 Rain/Es_pot·都在共享 weather 里），
    // 慢模型未被 link 覆盖的外生 driving 从共享 weather 取（fast_drivers 即整份 weather，fast 只取自己需要的列）。
    // 见 get_slow_driver 的 weather fallback。温室场景慢模型 driving 全被 link 覆盖→走原路径、行为不变。
    let link_tos: HashSet<&str> = input.links.iter().map(|l| l.to.as_str()).collect();
    {
        let slow_stepper = Stepper::new(slow, slow.meta.dt, &input.slow_params, &input.slow_init)?;
        for &d in slow_stepper.drivers() {
            if link_tos.contains(d) {
                continue; // 被 link 覆盖：由快模型聚合供值
            }
            // 未被 link 覆盖 → 须由共享室外驱动(weather)供给（对等双向耦合）
            match input.fast_drivers.get(d) {
                None => {
                    return Err(SimError::Coupling(format!(
                        "慢模型驱动量 '{d}' 既无耦合链接、也不在共享室外驱动中（请加 --link 接到快模型输出，或在 weather CSV 加 '{d}' 列）"
                    )))
                }
                Some(col) => {
                    // weather fallback 仅支持 R=1（同速率对等耦合）。R>1 时慢步取窗口首值 v[s·R] 与 link
                    // 的窗口均值(Agg::Mean)语义不一致（日内步长气象取首行=错的日代表值）→ 强制此类驱动走 link。
                    if r > 1 {
                        return Err(SimError::Coupling(format!(
                            "慢模型驱动量 '{d}' 走共享 weather，但 R={r}>1：weather fallback 仅支持 R=1(同速率对等耦合)；多速率下请把 '{d}' 用 --link 接快模型输出"
                        )));
                    }
                    // 长度校验：慢步 s∈[0,slow_steps) 取 v[s]（R=1）→ 须 col.len() ≥ slow_steps，否则 v[s·R] 越界 panic。
                    if col.len() < input.slow_steps {
                        return Err(SimError::DriverLengthMismatch {
                            name: d.to_string(),
                            expected: input.slow_steps,
                            found: col.len(),
                        });
                    }
                }
            }
        }
    }
    // —— 反馈（慢→快，C2 双向，滞后一慢步）：初值 hold + 快模型输入校验 ——
    // 0c：按 fast_implicit 选显式 Stepper（bit-identical）或隐式 ImplicitStepper（刚性温室在回路走 BDF）。
    let mut fast_engine =
        build_fast_engine(fast, &input.fast_params, &input.fast_init, input.fast_implicit)?;
    let fb_targets: HashSet<&str> = input.feedback.iter().map(|f| f.to.as_str()).collect();
    let mut fb: HashMap<String, f64> = input.feedback.iter().map(|f| (f.to.clone(), f.init)).collect();
    // 反馈 to 必须是快模型的输入（驱动量），否则反馈值无处可喂
    {
        let fast_driver_set: HashSet<&str> =
            fast_engine.drivers().iter().map(|s| s.as_str()).collect();
        for f in &input.feedback {
            if !fast_driver_set.contains(f.to.as_str()) {
                return Err(SimError::Coupling(format!(
                    "反馈目标 '{}' 不是快模型 {} 的输入（驱动量）",
                    f.to, fast.meta.id
                )));
            }
            // 反馈目标不得与共享 weather 列同名：快步取值 fast_drivers.get(d) 优先于 fb.get(d)，
            // 撞名会让反馈被 weather 静默覆盖（反馈链路失效）→ 早失败。
            if input.fast_drivers.contains_key(f.to.as_str()) {
                return Err(SimError::Coupling(format!(
                    "反馈目标 '{}' 与共享室外驱动(weather)列同名 → 反馈会被 weather 静默覆盖；请重命名其一",
                    f.to
                )));
            }
        }
    }
    // —— 校验：快模型每个驱动量要么有室外序列（长度 total_fast），要么由反馈供值 ——
    for d in fast_engine.drivers() {
        let d = d.as_str();
        if fb_targets.contains(d) {
            continue; // 由反馈供值
        }
        match input.fast_drivers.get(d) {
            None => return Err(SimError::MissingDriver(d.to_string())),
            Some(s) if s.len() != total_fast => {
                return Err(SimError::DriverLengthMismatch {
                    name: d.to_string(),
                    expected: total_fast,
                    found: s.len(),
                })
            }
            Some(_) => {}
        }
    }

    let mut slow_stepper = Stepper::new(slow, slow.meta.dt, &input.slow_params, &input.slow_init)?;
    let mut fed: IndexMap<String, Vec<f64>> = IndexMap::new();
    let mut slow_traj: IndexMap<String, Vec<f64>> = IndexMap::new();
    // 快模型轨迹（日均聚合）：标量变量累加求均
    let fast_vars: Vec<String> = fast.variables.keys().cloned().collect();
    let mut fast_traj: IndexMap<String, Vec<f64>> = IndexMap::new();

    for s in 0..input.slow_steps {
        // 累加器：mean/integral 用 sum，last 用末值
        let mut acc = vec![0.0f64; input.links.len()];
        let mut last = vec![0.0f64; input.links.len()];
        // 快模型变量日均累加（标量）
        let mut fast_sum = vec![0.0f64; fast_vars.len()];
        let mut fast_is_scalar = vec![true; fast_vars.len()];
        for f in 0..r {
            let gi = s * r + f;
            // 快模型输入：室外驱动 取本快步序列值；反馈目标 取 hold 值（本慢步内常数）
            fast_engine.step(|d| {
                input.fast_drivers.get(d).map(|v| v[gi]).or_else(|| fb.get(d).copied())
            })?;
            for (li, link) in input.links.iter().enumerate() {
                let v = fast_engine
                    .get(&link.from)
                    .ok_or_else(|| SimError::Coupling(format!("快模型无接口输出 '{}'", link.from)))?;
                let x = match v {
                    Value::Scalar(x) => x,
                    _ => {
                        return Err(SimError::Coupling(format!(
                            "接口输出 '{}' 不是标量（耦合链接只支持标量）",
                            link.from
                        )))
                    }
                };
                match link.agg {
                    Agg::Mean | Agg::Integral => acc[li] += x,
                    Agg::Last => last[li] = x,
                }
            }
            // 累加快模型各标量变量（供日均）
            for (i, name) in fast_vars.iter().enumerate() {
                if fast_is_scalar[i] {
                    match fast_engine.get(name) {
                        Some(Value::Scalar(x)) => fast_sum[i] += x,
                        _ => fast_is_scalar[i] = false,
                    }
                }
            }
        }
        // 快模型日均 → 轨迹（仅标量变量）
        for (i, name) in fast_vars.iter().enumerate() {
            if fast_is_scalar[i] {
                fast_traj.entry(name.clone()).or_default().push(fast_sum[i] / r as f64);
            }
        }
        // 聚合收尾 → 本慢步慢模型驱动值
        let finalize = |li: usize| -> f64 {
            let link = &input.links[li];
            match link.agg {
                Agg::Mean => (acc[li] / r as f64) * link.scale,
                Agg::Integral => acc[li] * dtf_s * link.scale,
                Agg::Last => last[li] * link.scale,
            }
        };
        // 记录喂入值（按链接 to 名）
        for (li, link) in input.links.iter().enumerate() {
            fed.entry(link.to.clone()).or_default().push(finalize(li));
        }
        // 慢模型推进一步：驱动量优先取 link 聚合值；未被 link 覆盖的外生 driving 从共享室外驱动
        // (weather)取 v[s·R]。校验段已保证走此 fallback 的驱动仅在 R=1 出现(v[s·R]=v[s])且列长≥slow_steps
        // →不越界；R>1/短列在校验段已 Err、不会执行到这。
        let get_slow_driver = |name: &str| -> Option<f64> {
            input
                .links
                .iter()
                .position(|l| l.to == name)
                .map(finalize)
                .or_else(|| input.fast_drivers.get(name).map(|v| v[s * r]))
        };
        slow_stepper.step(get_slow_driver)?;
        for name in slow.variables.keys() {
            let v = slow_stepper
                .get(name)
                .ok_or_else(|| SimError::Unresolved(name.clone()))?;
            flatten_into(&mut slow_traj, name, &v);
        }
        // 反馈更新（滞后）：本慢步算完作物 → 更新 hold 值，供**下一**慢步快模型用
        for link in &input.feedback {
            let v = slow_stepper.get(&link.from).ok_or_else(|| {
                SimError::Coupling(format!("反馈来源 '{}' 不是慢模型输出", link.from))
            })?;
            let x = match v {
                Value::Scalar(x) => x,
                _ => {
                    return Err(SimError::Coupling(format!(
                        "反馈来源 '{}' 不是标量",
                        link.from
                    )))
                }
            };
            fb.insert(link.to.clone(), x * link.scale);
        }
    }

    Ok(CoupledOutput {
        slow_steps: input.slow_steps,
        r,
        slow: SimOutput { steps: input.slow_steps, trajectories: slow_traj },
        fast: SimOutput { steps: input.slow_steps, trajectories: fast_traj },
        fed_drivers: fed,
    })
}

/// 把一个变量本步的 Value 展平记入轨迹：标量→`name`；向量→`name[1]`、`name[2]`…；矩阵→`name[r,c]`。
pub(crate) fn flatten_into(traj: &mut IndexMap<String, Vec<f64>>, name: &str, v: &Value) {
    match v {
        Value::Scalar(x) => traj.entry(name.to_string()).or_default().push(*x),
        Value::Vector(d) => {
            for (i, x) in d.iter().enumerate() {
                traj.entry(format!("{name}[{}]", i + 1)).or_default().push(*x);
            }
        }
        Value::Matrix { rows, cols, data } => {
            for r in 0..*rows {
                for c in 0..*cols {
                    traj.entry(format!("{name}[{},{}]", r + 1, c + 1))
                        .or_default()
                        .push(data[r * cols + c]);
                }
            }
        }
    }
}

/// 对步内可计算节点做拓扑排序（Kahn 算法），返回节点下标的求值顺序。
fn topo_order(
    nodes: &[(&str, Node)],
    node_idx: &HashMap<&str, usize>,
) -> Result<Vec<usize>, SimError> {
    let count = nodes.len();
    let mut indeg: Vec<usize> = vec![0; count];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); count]; // dep_idx -> [依赖它的节点下标]

    for (i, (name, node)) in nodes.iter().enumerate() {
        // 收集本节点依赖的「可计算节点」下标
        let mut deps: Vec<usize> = match node {
            Node::Equation(expr) => expr
                .get_variable_refs()
                .into_iter()
                .filter_map(|r| node_idx.get(r.as_str()).copied())
                .collect(),
            // 仅当 rate 本身也是可计算节点时才成依赖（否则它是延迟寄存器/驱动/参数=源）
            Node::Integrator { rate, .. } => {
                node_idx.get(*rate).copied().into_iter().collect()
            }
        };
        deps.sort_unstable();
        deps.dedup();
        for d in deps {
            if d == i {
                // 自依赖（速率引用自身状态量当前值）→ 环
                return Err(SimError::Cycle(vec![name.to_string()]));
            }
            adj[d].push(i);
            indeg[i] += 1;
        }
    }

    // Kahn：按下标升序入队，保持声明顺序，输出可复现
    let mut queue: VecDeque<usize> = (0..count).filter(|&i| indeg[i] == 0).collect();
    let mut order: Vec<usize> = Vec::with_capacity(count);
    while let Some(i) = queue.pop_front() {
        order.push(i);
        for &s in &adj[i] {
            indeg[s] -= 1;
            if indeg[s] == 0 {
                queue.push_back(s);
            }
        }
    }

    if order.len() != count {
        let done: HashSet<usize> = order.iter().copied().collect();
        let remaining: Vec<String> = (0..count)
            .filter(|i| !done.contains(i))
            .map(|i| nodes[i].0.to_string())
            .collect();
        return Err(SimError::Cycle(remaining));
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_model(yaml: &str) -> (TempDir, EquationFile) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("m.eq.yaml");
        std::fs::File::create(&path).unwrap().write_all(yaml.as_bytes()).unwrap();
        let file = parse_file(&path).unwrap();
        (dir, file)
    }

    /// 两个累积器：CT = Σ T（积分驱动量）；TDM = Σ DDM（积分方程量）。
    /// 并用延迟寄存器取上一步 CT 做差分校验。
    #[test]
    fn test_two_accumulators_and_delay() {
        let yaml = r#"
meta: { id: ACC, model: Acc, name_cn: 累积测试 }
parameters:
  LUE: { name_cn: 光能利用率, default: 2.0 }
variables:
  T:   { type: input, class: driving }
  CT:  { type: output, class: state, init: 0.0, rate: T }
  DDM: { type: intermediate, class: rate }
  TDM: { type: output, class: state, init: 100.0, rate: DDM }
  CT_prev: { type: intermediate, init: 0.0, prev: CT }
  dCT: { type: output }
equations:
  - id: E1
    name: 日干物质
    output: DDM
    expression: { op: mul, args: [ { const: 1.0 }, { ref: LUE } ] }
  - id: E2
    name: CT 日增量
    output: dCT
    expression: { op: sub, args: [ { ref: CT }, { ref: CT_prev } ] }
"#;
        let (_d, file) = write_model(yaml);
        let input = SimInput::new(3).driver("T", vec![10.0, 20.0, 30.0]);
        let out = simulate(&file, &input).unwrap();

        // CT = 10, 30, 60
        assert_eq!(out.series("CT").unwrap(), &[10.0, 30.0, 60.0]);
        // DDM = 2 每步；TDM = 100+2, +2, +2 = 102,104,106
        assert_eq!(out.series("TDM").unwrap(), &[102.0, 104.0, 106.0]);
        // dCT = CT[n] - CT[n-1]：首步 CT_prev=init0 → 10-0=10；之后 20、30
        assert_eq!(out.series("dCT").unwrap(), &[10.0, 20.0, 30.0]);
        assert_eq!(out.final_value("CT"), Some(60.0));
    }

    /// 亚日步长 dt：状态量积分 X[n]=X[n-1]+rate[n]·dt。dt=0.5 → 增量减半；dt=1 与旧行为一致。
    #[test]
    fn test_sub_day_timestep() {
        let yaml = r#"
meta: { id: DT, model: DT, name_cn: 步长测试 }
variables:
  T:  { type: input, class: driving }
  CT: { type: output, class: state, init: 0.0, rate: T }
"#;
        let (_d, file) = write_model(yaml);
        // dt=1（默认）：CT = 10, 30, 60
        let out1 = simulate(&file, &SimInput::new(3).driver("T", vec![10.0, 20.0, 30.0])).unwrap();
        assert_eq!(out1.series("CT").unwrap(), &[10.0, 30.0, 60.0]);
        // dt=0.5（SimInput 覆盖）：每步增量减半 → CT = 5, 15, 30
        let mut inp = SimInput::new(3).driver("T", vec![10.0, 20.0, 30.0]);
        inp.dt = Some(0.5);
        let out2 = simulate(&file, &inp).unwrap();
        assert_eq!(out2.series("CT").unwrap(), &[5.0, 15.0, 30.0]);
    }

    /// 模型 `meta.dt` 作默认步长（`SimInput.dt=None` 时生效）；缺省 1.0。
    #[test]
    fn test_meta_dt_default() {
        let yaml = r#"
meta: { id: DT2, model: DT2, name_cn: meta步长, dt: 0.25 }
variables:
  T:  { type: input, class: driving }
  CT: { type: output, class: state, init: 0.0, rate: T }
"#;
        let (_d, file) = write_model(yaml);
        assert_eq!(file.meta.dt, 0.25, "meta.dt 应被解析");
        // dt=0.25：CT = 4·0.25=1，+8·0.25=3
        let out = simulate(&file, &SimInput::new(2).driver("T", vec![4.0, 8.0])).unwrap();
        assert_eq!(out.series("CT").unwrap(), &[1.0, 3.0]);
    }

    /// FSPM 风险4 第2步：器官门控激活 + 每果热龄。
    /// 每果按 θ=(节位−1)·phyllo+(果位−1)·ψ 错峰出现（gate），出现前 age 不累积、ss=0；出现后 age 累积。
    /// 验证 {rank}（第1步）+ 现成算子即可表达器官流的「激活」维度，引擎零改动。
    #[test]
    fn test_fspm_organ_gated_activation() {
        let yaml = r#"
meta: { id: FSPMG, model: FspmGate, name_cn: 门控激活测试, dt: 1.0 }
structure:
  entities:
    metamer: { count: 2, topology: chain }
    fruit:   { per: metamer, count: 2 }
parameters:
  phyllo: { name_cn: 节phyllochron, default: 10.0 }
  psi:    { name_cn: 穗内错峰, default: 2.0 }
  w_app:  { name_cn: 出现ramp宽, default: 1.0 }
  Tbase:  { name_cn: 果发育基温, default: 0.0 }
variables:
  T:    { type: input, class: driving }
  rate_Tsum: { class: rate }
  Tsum: { type: output, class: state, init: 0.0, rate: rate_Tsum }
  theta: { of: fruit, type: output, class: auxiliary }
  gate:  { of: fruit, type: output, class: auxiliary }
  rate_age: { of: fruit, class: rate }
  age:   { of: fruit, type: output, class: state, init: 0.0, rate: rate_age }
  ss:    { of: fruit, type: output, class: auxiliary }
equations:
  - { id: RTS, name: 积温速率, output: rate_Tsum, expression: { ref: T } }
  - { id: TH, name: 出现阈值, for: fruit, output: theta,
      expression: { op: add, args: [
        { op: mul, args: [ { op: sub, args: [ { rank: parent }, { const: 1 } ] }, { ref: phyllo } ] },
        { op: mul, args: [ { op: sub, args: [ { rank: self },   { const: 1 } ] }, { ref: psi } ] } ] } }
  - { id: GT, name: 激活门, for: fruit, output: gate,
      expression: { op: max, args: [ { const: 0.0 }, { op: min, args: [ { const: 1.0 },
        { op: div, args: [ { op: sub, args: [ { ref: Tsum }, { ref: theta, of: self } ] }, { ref: w_app } ] } ] } ] } }
  - { id: AG, name: 热龄速率, for: fruit, output: rate_age,
      expression: { op: mul, args: [ { op: max, args: [ { const: 0.0 }, { op: sub, args: [ { ref: T }, { ref: Tbase } ] } ] }, { ref: gate, of: self } ] } }
  - { id: SS, name: 汇强占位, for: fruit, output: ss,
      expression: { op: mul, args: [ { ref: age, of: self }, { ref: gate, of: self } ] } }
"#;
        let (_d, file) = write_model(yaml);
        let out = simulate(&file, &SimInput::new(4).driver("T", vec![10.0, 10.0, 10.0, 10.0])).unwrap();

        // θ 由 {rank} 端到端折出：1.1=0、2.2=(2−1)·10+(2−1)·2=12
        assert_eq!(out.series("theta__1_1").unwrap(), &[0.0, 0.0, 0.0, 0.0]);
        assert_eq!(out.series("theta__2_2").unwrap(), &[12.0, 12.0, 12.0, 12.0]);
        // Tsum = 10,20,30,40
        assert_eq!(out.series("Tsum").unwrap(), &[10.0, 20.0, 30.0, 40.0]);
        // fruit 1.1（θ=0）：首步即出现 → age 从 step0 累积
        assert_eq!(out.series("age__1_1").unwrap(), &[10.0, 20.0, 30.0, 40.0]);
        // fruit 2.2（θ=12）：Tsum<12 时 gate=0 → age 不动、ss=0；Tsum≥12（step1）后才累积
        assert_eq!(out.series("gate__2_2").unwrap(), &[0.0, 1.0, 1.0, 1.0]);
        assert_eq!(out.series("age__2_2").unwrap(), &[0.0, 10.0, 20.0, 30.0]);
        assert_eq!(out.series("ss__2_2").unwrap(), &[0.0, 10.0, 20.0, 30.0]);
    }

    /// FSPM 风险4 第3步：器官级源-库碳经济（共同池 + 相对汇强 + 每果 Yin beta 潜在生长 + 守恒）。
    /// 4 果（2 节×2）+ 集总营养体；每果汇强 ss=cw(age)·ASR，按 ss/ssp 分配共享池，
    /// 缓冲库排出 = Σ各器官分配（同一聚合）→ 守恒由构造保证。验证：
    ///   ① 质量平衡 d(ΣC)=A_gross−总呼吸 逐步成立；② share∈[0,1]；③ 错峰出现；④ 节位库强=Σ子果。
    #[test]
    fn test_fspm_organ_carbon_economy() {
        let yaml = r#"
meta: { id: TFE, model: TomatoFspmEcon, name_cn: 番茄器官碳经济mini, dt: 1.0 }
structure:
  entities:
    metamer: { count: 2, topology: chain }
    fruit:   { per: metamer, count: 2 }
parameters:
  Tbase:   { name_cn: 发育基温, default: 10.0 }
  phyllo:  { name_cn: 节phyllochron, default: 30.0 }
  psi:     { name_cn: 穗内错峰, default: 8.0 }
  w_app:   { name_cn: 出现ramp, default: 1.0 }
  Tbase_f: { name_cn: 果发育基温, default: 8.0 }
  wmax:    { name_cn: 单果潜在结构干重, default: 1060.0 }
  tm:      { name_cn: beta_tm, default: 149.0 }
  te:      { name_cn: beta_te, default: 481.0 }
  ASR:     { name_cn: 同化物需求, default: 1.2 }
  rg_veg:  { name_cn: 营养体汇强集总, default: 2.0 }
  cm_veg:  { name_cn: 营养体维持系数, default: 0.0005 }
  cm_fr:   { name_cn: 果维持系数, default: 0.0003 }
  cg_veg:  { name_cn: 营养体生长呼吸, default: 0.30 }
  cg_fr:   { name_cn: 果生长呼吸, default: 0.27 }
  TINY:    { name_cn: 护栏, default: 0.000001 }
variables:
  T:       { type: input, class: driving }
  A_gross: { type: input, class: driving }
  rate_Tsum: { class: rate }
  Tsum:    { type: output, class: state, init: 0.0, rate: rate_Tsum }
  theta:   { of: fruit, class: auxiliary }
  gate:    { of: fruit, class: auxiliary }
  rate_age: { of: fruit, class: rate }
  age:     { of: fruit, type: output, class: state, init: 0.0, rate: rate_age }
  cw:      { of: fruit, type: output, class: auxiliary }
  ss:      { of: fruit, type: output, class: auxiliary }
  Rm_fr:   { of: fruit, class: auxiliary }
  A:       { of: fruit, type: output, class: auxiliary }
  rate_C:  { of: fruit, class: rate }
  C:       { of: fruit, type: output, class: state, init: 0.0, rate: rate_C }
  C_prev:  { of: fruit, class: semi_state, init: 0.0, prev: C }
  ss_fruit_tot: { type: output, class: auxiliary }
  Rm_fruit_tot: { class: auxiliary }
  A_fruit_tot:  { class: auxiliary }
  C_fruit_tot:  { type: output, class: auxiliary }
  ssp:     { type: output, class: auxiliary }
  Rm:      { class: auxiliary }
  Anet:    { type: output, class: auxiliary }
  Ap:      { class: auxiliary }
  share:   { type: output, class: auxiliary }
  total_alloc: { class: auxiliary }
  rate_Cbuf: { class: rate }
  Cbuf:    { type: output, class: state, init: 0.0, rate: rate_Cbuf }
  Cbuf_prev: { class: semi_state, init: 0.0, prev: Cbuf }
  A_veg:   { type: output, class: auxiliary }
  Rm_veg:  { class: auxiliary }
  rate_C_veg: { class: rate }
  C_veg:   { type: output, class: state, init: 0.0, rate: rate_C_veg }
  C_veg_prev: { class: semi_state, init: 0.0, prev: C_veg }
  C_total: { type: output, class: auxiliary }
  resp_total: { type: output, class: auxiliary }
  node_sink: { of: metamer, type: output, class: auxiliary }
equations:
  - { id: RTS, name: 积温速率, output: rate_Tsum, expression: { op: max, args: [ {const: 0}, { op: sub, args: [ {ref: T}, {ref: Tbase} ] } ] } }
  - { id: TH, name: 出现阈值, for: fruit, output: theta,
      expression: { op: add, args: [
        { op: mul, args: [ { op: sub, args: [ {rank: parent}, {const: 1} ] }, {ref: phyllo} ] },
        { op: mul, args: [ { op: sub, args: [ {rank: self},   {const: 1} ] }, {ref: psi} ] } ] } }
  - { id: GT, name: 激活门, for: fruit, output: gate,
      expression: { op: max, args: [ {const: 0}, { op: min, args: [ {const: 1},
        { op: div, args: [ { op: sub, args: [ {ref: Tsum}, {ref: theta, of: self} ] }, {ref: w_app} ] } ] } ] } }
  - { id: AG, name: 热龄速率, for: fruit, output: rate_age,
      expression: { op: mul, args: [ { op: max, args: [ {const: 0}, { op: sub, args: [ {ref: T}, {ref: Tbase_f} ] } ] }, {ref: gate, of: self} ] } }
  - { id: CW, name: 单果潜在生长率, for: fruit, output: cw,
      reference: "Yin2003 beta 导数 cw=wmax(2te-tm)(te-t)/(te(te-tm)^2)(t/te)^(tm/(te-tm))",
      expression: { op: max, args: [ {const: 0}, { op: mul, args: [
        { op: mul, args: [
          { op: mul, args: [ {ref: wmax}, { op: sub, args: [ { op: mul, args: [ {const: 2}, {ref: te} ] }, {ref: tm} ] } ] },
          { op: div, args: [ { op: sub, args: [ {ref: te}, {ref: age, of: self} ] },
            { op: mul, args: [ {ref: te}, { op: pow, args: [ { op: sub, args: [ {ref: te}, {ref: tm} ] }, {const: 2} ] } ] } ] } ] },
        { op: pow, args: [ { op: div, args: [ {ref: age, of: self}, {ref: te} ] },
          { op: div, args: [ {ref: tm}, { op: sub, args: [ {ref: te}, {ref: tm} ] } ] } ] } ] } ] } }
  - { id: SS, name: 单果汇强, for: fruit, output: ss, expression: { op: mul, args: [ {ref: cw, of: self}, {ref: ASR} ] } }
  - { id: SSFT, name: 全果汇强和, output: ss_fruit_tot, expression: { agg: sum, over: all, of: fruit, body: { ref: ss } } }
  - { id: NSK, name: 节位库强, for: metamer, output: node_sink, expression: { agg: sum, over: children, body: { ref: ss } } }
  - { id: SSP, name: 整株汇强, output: ssp, expression: { op: add, args: [ {ref: rg_veg}, {ref: ss_fruit_tot} ] } }
  - { id: RMF, name: 单果维持呼吸, for: fruit, output: Rm_fr, expression: { op: mul, args: [ {ref: cm_fr}, {ref: C_prev, of: self} ] } }
  - { id: RMFT, name: 全果维持和, output: Rm_fruit_tot, expression: { agg: sum, over: all, of: fruit, body: { ref: Rm_fr } } }
  - { id: RMV, name: 营养体维持, output: Rm_veg, expression: { op: mul, args: [ {ref: cm_veg}, {ref: C_veg_prev} ] } }
  - { id: RM, name: 总维持呼吸, output: Rm, expression: { op: add, args: [ {ref: Rm_veg}, {ref: Rm_fruit_tot} ] } }
  - { id: ANET, name: 净同化, output: Anet, expression: { op: sub, args: [ {ref: A_gross}, {ref: Rm} ] } }
  - { id: AP, name: 可用池, output: Ap, expression: { op: add, args: [ {ref: Anet}, {ref: Cbuf_prev} ] } }
  - { id: SHR, name: 分配份额, output: share, expression: { op: max, args: [ {const: 0}, { op: min, args: [ {const: 1},
      { op: div, args: [ {ref: Ap}, { op: max, args: [ {ref: ssp}, {ref: TINY} ] } ] } ] } ] } }
  - { id: AFR, name: 单果分配, for: fruit, output: A, expression: { op: mul, args: [ {ref: ss, of: self}, {ref: share} ] } }
  - { id: AVG, name: 营养体分配, output: A_veg, expression: { op: mul, args: [ {ref: rg_veg}, {ref: share} ] } }
  - { id: AFT, name: 全果分配和, output: A_fruit_tot, expression: { agg: sum, over: all, of: fruit, body: { ref: A } } }
  - { id: TAL, name: 总分配, output: total_alloc, expression: { op: add, args: [ {ref: A_veg}, {ref: A_fruit_tot} ] } }
  - { id: RCB, name: 缓冲库速率, output: rate_Cbuf, expression: { op: sub, args: [ {ref: Anet}, {ref: total_alloc} ] } }
  - { id: RCF, name: 单果碳速率, for: fruit, output: rate_C, expression: { op: mul, args: [ {ref: A, of: self}, { op: sub, args: [ {const: 1}, {ref: cg_fr} ] } ] } }
  - { id: RCV, name: 营养体碳速率, output: rate_C_veg, expression: { op: mul, args: [ {ref: A_veg}, { op: sub, args: [ {const: 1}, {ref: cg_veg} ] } ] } }
  - { id: CFT, name: 在株果碳和, output: C_fruit_tot, expression: { agg: sum, over: all, of: fruit, body: { ref: C } } }
  - { id: CTOT, name: 总碳, output: C_total, expression: { op: add, args: [ { op: add, args: [ {ref: Cbuf}, {ref: C_veg} ] }, {ref: C_fruit_tot} ] } }
  - { id: RSP, name: 总呼吸, output: resp_total, expression: { op: add, args: [ {ref: Rm},
      { op: add, args: [ { op: mul, args: [ {ref: cg_veg}, {ref: A_veg} ] }, { op: mul, args: [ {ref: cg_fr}, {ref: A_fruit_tot} ] } ] } ] } }
"#;
        let (_d, file) = write_model(yaml);
        let n = 8usize;
        let out = simulate(
            &file,
            &SimInput::new(n).driver("T", vec![25.0; n]).driver("A_gross", vec![5.0; n]),
        )
        .unwrap();

        // ① 质量守恒：d(ΣC) = A_gross − 总呼吸，逐步成立（C_total[-1]=Σinit=0）
        let ct = out.series("C_total").unwrap();
        let rt = out.series("resp_total").unwrap();
        for i in 0..n {
            let prev = if i == 0 { 0.0 } else { ct[i - 1] };
            let bal = (ct[i] - prev) - (5.0 - rt[i]); // A_gross=5, dt=1
            assert!(bal.abs() < 1e-9, "step {i} 守恒失衡 = {bal}");
        }
        // ② share ∈ [0,1]
        for &x in out.series("share").unwrap() {
            assert!((0.0..=1.0).contains(&x), "share 越界 {x}");
        }
        // ③ 错峰出现：果1.1（θ=0）首步即长，果2.2（θ=38）前两步未出现
        assert!(out.series("age__1_1").unwrap()[0] > 0.0);
        assert_eq!(out.series("age__2_2").unwrap()[0], 0.0);
        assert!(out.series("age__2_2").unwrap()[n - 1] > 0.0, "果2.2 应在后期出现");
        // ④ 节位库强 = Σ子果汇强（over: children）
        let nsk1 = out.series("node_sink__1").unwrap()[n - 1];
        let f11 = out.series("ss__1_1").unwrap()[n - 1];
        let f12 = out.series("ss__1_2").unwrap()[n - 1];
        assert!((nsk1 - (f11 + f12)).abs() < 1e-9, "节位1库强应=子果之和");
        // ⑤ 碳确实在累积
        assert!(ct[n - 1] > ct[0], "总碳应随时间增长");
    }

    /// FSPM 风险4 第3步（交付）：加载真实交付模型 crop-models/tomato/tomato_fspm.eq.yaml，
    /// 跑仿真并核查碳守恒（24 果 + LUE 光源 + 叶/茎集总）。文件缺失（异机/未签出）→ 跳过。
    ///
    /// 注：修法 D（聚合 lower 成扁平 `vsum`/`vprod` over `vector`，深度恒为 1）后，24 果模型在
    /// 默认测试线程栈即可跑——此前 N 深聚合链需 ~4MB 栈、已解决（见 agg_fold）。
    #[test]
    fn test_fspm_tomato_model_file_conserves() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../crop-models/tomato/tomato_fspm.eq.yaml");
        if !path.exists() {
            eprintln!("跳过 test_fspm_tomato_model_file_conserves：未找到 {path:?}");
            return;
        }
        let file = parse_file(&path).unwrap();
        let n = 12usize;
        let out = simulate(
            &file,
            &SimInput::new(n).driver("T", vec![22.0; n]).driver("PAR", vec![400.0; n]),
        )
        .unwrap();
        // 守恒：d(全株总碳) = 总同化 − 总呼吸（i≥1，免 init 求和）
        let ct = out.series("C_total").unwrap();
        let ag = out.series("A_gross").unwrap();
        let rt = out.series("resp_total").unwrap();
        for i in 1..n {
            let bal = (ct[i] - ct[i - 1]) - (ag[i] - rt[i]); // dt=1
            assert!(bal.abs() < 1e-6, "step {i} 守恒失衡 = {bal}");
        }
        // 果实累积、节位库强=Σ子果（24 果实例化 + 聚合端到端）
        assert!(out.series("C_fruit_tot").unwrap()[n - 1] > 0.0, "应有在株果碳");
        let ns1 = out.series("node_sink__1").unwrap()[n - 1];
        let s11 = out.series("ss__1_1").unwrap()[n - 1];
        assert!(ns1 >= s11 - 1e-9, "节位1库强应≥其单果汇强");
    }

    /// V2：向量参数 + 向量状态量逐元素积分；输出展平成 name[i]。
    #[test]
    fn test_vector_state_integration() {
        let yaml = r#"
meta: { id: VEC, model: Vec, name_cn: 向量仿真 }
parameters:
  rates: { name_cn: 各组速率, values: [1.0, 2.0, 3.0] }
variables:
  T:     { type: input, class: driving }
  drive: { type: intermediate, class: rate }
  S:     { type: output, class: state, init: 0.0, rate: drive }
  Stot:  { type: output }
equations:
  - { id: E1, name: 向量速率, output: drive, expression: { op: mul, args: [ { ref: rates }, { ref: T } ] } }
  - { id: E2, name: 求和, output: Stot, expression: { op: vsum, args: [ { ref: S } ] } }
"#;
        let (_d, file) = write_model(yaml);
        let input = SimInput::new(3).driver("T", vec![1.0, 1.0, 1.0]);
        let out = simulate(&file, &input).unwrap();

        // drive=[1,2,3] 每步；S 逐元素积分：[1,2,3] → [2,4,6] → [3,6,9]
        assert_eq!(out.series("S[1]").unwrap(), &[1.0, 2.0, 3.0]);
        assert_eq!(out.series("S[2]").unwrap(), &[2.0, 4.0, 6.0]);
        assert_eq!(out.series("S[3]").unwrap(), &[3.0, 6.0, 9.0]);
        // Stot = Σ S = 6, 12, 18
        assert_eq!(out.series("Stot").unwrap(), &[6.0, 12.0, 18.0]);
        // 向量变量本身不作为单一键（已展平）
        assert!(out.series("S").is_none());
    }

    /// V3 回归：向量延迟寄存器——首步标量 init 广播到来源形状，输出形状跨步一致。
    #[test]
    fn test_vector_delay_register() {
        let yaml = r#"
meta: { id: VD, model: VD, name_cn: 向量延迟 }
parameters:
  base: { name_cn: 速率, values: [1.0, 2.0] }
variables:
  T:        { type: input, class: driving }
  acc:      { type: output, class: state, init: 0.0, rate: base }
  acc_prev: { type: intermediate, class: semi_state, init: 0.0, prev: acc }
  delta:    { type: output }
equations:
  - { id: E1, name: 增量和, output: delta,
      expression: { op: vsum, args: [ { op: sub, args: [ { ref: acc }, { ref: acc_prev } ] } ] } }
"#;
        let (_d, file) = write_model(yaml);
        let input = SimInput::new(3).driver("T", vec![0.0, 0.0, 0.0]);
        let out = simulate(&file, &input).unwrap();
        // acc 逐元素积分 base=[1,2]：[1,2]→[2,4]→[3,6]
        assert_eq!(out.series("acc[1]").unwrap(), &[1.0, 2.0, 3.0]);
        assert_eq!(out.series("acc[2]").unwrap(), &[2.0, 4.0, 6.0]);
        // acc_prev 首步广播 init=0 到向量 [0,0]，之后取上一步 acc → 形状跨步一致（长度=3）
        assert_eq!(out.series("acc_prev[1]").unwrap(), &[0.0, 1.0, 2.0]);
        assert_eq!(out.series("acc_prev[2]").unwrap(), &[0.0, 2.0, 4.0]);
        // delta = Σ(acc - acc_prev) = Σ base = 3 每步
        assert_eq!(out.series("delta").unwrap(), &[3.0, 3.0, 3.0]);
    }

    /// 缺驱动量应报错。
    #[test]
    fn test_missing_driver_errors() {
        let yaml = r#"
meta: { id: M, model: M, name_cn: x }
variables:
  T:  { type: input }
  Y:  { type: output }
equations:
  - id: E1
    name: y
    output: Y
    expression: { op: mul, args: [ { ref: T }, { const: 2 } ] }
"#;
        let (_d, file) = write_model(yaml);
        let input = SimInput::new(2); // 没给 T
        assert_eq!(simulate(&file, &input), Err(SimError::MissingDriver("T".into())));
    }

    /// 耦合仿真（C1）：多速率 + mean/integral 聚合，结果解析可验。
    /// 快模型 dt=10s、R=3；快输出 y=u；慢模型收 ybar=mean(y)、yint=integral(y)=Σy·dt_fast。
    #[test]
    fn test_coupled_multirate_aggregation() {
        let fast_yaml = r#"
meta: { id: FAST, model: F, name_cn: 快, dt: 10, dt_seconds: 10 }
variables:
  u: { type: input, class: driving }
  y: { type: output }
equations:
  - { id: E, name: y, output: y, expression: { op: mul, args: [ { ref: u }, { const: 1 } ] } }
"#;
        let slow_yaml = r#"
meta: { id: SLOW, model: S, name_cn: 慢, dt: 1, dt_seconds: 30 }
variables:
  ybar: { type: input, class: driving }
  yint: { type: input, class: driving }
  chk:  { type: output }
equations:
  - { id: E, name: chk, output: chk, expression: { op: add, args: [ { ref: ybar }, { ref: yint } ] } }
"#;
        let (_df, fast) = write_model(fast_yaml);
        let (_ds, slow) = write_model(slow_yaml);
        let links = vec![
            CoupledLink { to: "ybar".into(), from: "y".into(), agg: Agg::Mean, scale: 1.0 },
            CoupledLink { to: "yint".into(), from: "y".into(), agg: Agg::Integral, scale: 1.0 },
        ];
        let mut drv = HashMap::new();
        drv.insert("u".to_string(), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]); // 2 慢步 × R=3
        let inp = CoupledInput::new(&fast, &slow, links, drv, 2);
        let out = simulate_coupled(&inp).unwrap();

        assert_eq!(out.r, 3); // 30/10
        // 慢步0：y=1,2,3 → mean=2、integral=(1+2+3)·10=60；慢步1：y=4,5,6 → mean=5、integral=150
        assert_eq!(out.fed_drivers["ybar"], vec![2.0, 5.0]);
        assert_eq!(out.fed_drivers["yint"], vec![60.0, 150.0]);
        // 慢模型把它们相加：chk = ybar + yint = 62, 155
        assert_eq!(out.slow.series("chk").unwrap(), &[62.0, 155.0]);
        // 喂入值也进慢模型轨迹（驱动量被记录）
        assert_eq!(out.slow.series("ybar").unwrap(), &[2.0, 5.0]);
    }

    /// 0c：刚性快模型走隐式耦合。快 dy/dt=1000·(e−y)（手写 y_prev 破环供显式）→ k·dt=10000：
    /// 显式 Euler 发散、隐式 BDF 稳定弛豫到 e；且耦合隐式 fast 轨迹 == 单模型 `simulate_implicit`（单一真相源）。
    #[cfg(feature = "implicit")]
    #[test]
    fn test_coupled_fast_implicit_stiff() {
        let fast_yaml = r#"
meta: { id: FAST, model: F, name_cn: 刚性快, dt: 10, dt_seconds: 10 }
variables:
  etgt:   { type: input, class: driving }
  y:      { type: output, class: state, init: 0.0, rate: ry }
  y_prev: { type: output, prev: y, init: 0.0 }
  ry:     { type: output }
equations:
  - { id: R, name: ry, output: ry, expression: { op: mul, args: [ { const: 1000 }, { op: sub, args: [ { ref: etgt }, { ref: y_prev } ] } ] } }
"#;
        let slow_yaml = r#"
meta: { id: SLOW, model: S, name_cn: 慢, dt: 1, dt_seconds: 10 }
variables:
  ybar: { type: input, class: driving }
  chk:  { type: output }
equations:
  - { id: E, name: chk, output: chk, expression: { ref: ybar } }
"#;
        let (_df, fast) = write_model(fast_yaml);
        let (_ds, slow) = write_model(slow_yaml);
        let mklink =
            || vec![CoupledLink { to: "ybar".into(), from: "y".into(), agg: Agg::Mean, scale: 1.0 }];
        let mkdrv = || {
            let mut d = HashMap::new();
            d.insert("etgt".to_string(), vec![1.0, 1.0, 1.0]);
            d
        };
        // 隐式：y 在一步内弛豫到 e=1（刚性）→ chk≈1
        let mut inp = CoupledInput::new(&fast, &slow, mklink(), mkdrv(), 3);
        inp.fast_implicit = true;
        let out = simulate_coupled(&inp).unwrap();
        for &v in out.slow.series("chk").unwrap() {
            assert!((v - 1.0).abs() < 1e-3, "隐式耦合 chk={v} 应≈1（弛豫到 etgt）");
        }
        // 一致性：耦合隐式 fast 轨迹 == 单模型 simulate_implicit（同驱动·单一真相源）
        let si = crate::sim::implicit::simulate_implicit(
            &fast,
            &SimInput::new(3).driver("etgt", vec![1.0, 1.0, 1.0]),
            crate::sim::implicit::ImplicitOpts { smooth_eps: Some(0.05), ..Default::default() },
        )
        .unwrap();
        let (cy, sy) = (out.fast.series("y").unwrap(), si.series("y").unwrap());
        for (a, b) in cy.iter().zip(sy.iter()) {
            assert!((a - b).abs() < 1e-9, "耦合隐式 fast y={a} vs simulate_implicit y={b} 不一致");
        }
        // 对照：显式路径在此刚性下发散（|y|>100）→ 证隐式必要
        let mut inp_e = CoupledInput::new(&fast, &slow, mklink(), mkdrv(), 3);
        inp_e.fast_implicit = false;
        let oe = simulate_coupled(&inp_e).unwrap();
        assert!(
            oe.fast.series("y").unwrap().iter().any(|&v| v.abs() > 100.0),
            "显式 Euler 在 k·dt=10000 应发散（|y|>100）"
        );
    }

    /// 耦合：慢模型有未被链接覆盖的驱动 → 报错；缺 dt_seconds → 报错。
    #[test]
    fn test_coupled_validation_errors() {
        let fast_yaml = r#"
meta: { id: F, model: F, name_cn: x, dt: 1, dt_seconds: 1 }
variables: { u: { type: input }, y: { type: output } }
equations: [ { id: E, name: y, output: y, expression: { ref: u } } ]
"#;
        // 慢模型有两个驱动 a、b，但只给 a 一条链接 → b 无链接报错
        let slow_yaml = r#"
meta: { id: S, model: S, name_cn: x, dt: 1, dt_seconds: 2 }
variables: { a: { type: input }, b: { type: input }, o: { type: output } }
equations: [ { id: E, name: o, output: o, expression: { op: add, args: [ { ref: a }, { ref: b } ] } } ]
"#;
        let (_df, fast) = write_model(fast_yaml);
        let (_ds, slow) = write_model(slow_yaml);
        let links = vec![CoupledLink { to: "a".into(), from: "y".into(), agg: Agg::Mean, scale: 1.0 }];
        let mut drv = HashMap::new();
        drv.insert("u".to_string(), vec![1.0, 2.0]);
        let inp = CoupledInput::new(&fast, &slow, links, drv, 1);
        assert!(matches!(simulate_coupled(&inp), Err(SimError::Coupling(_))));
    }

    /// 耦合 C2：双向滞后反馈——快模型 y=u+g，g 由慢模型 z 反馈（滞后一慢步，首步 init）。
    #[test]
    fn test_coupled_feedback_lagged() {
        let fast_yaml = r#"
meta: { id: F2, model: F, name_cn: 快, dt: 1, dt_seconds: 1 }
variables:
  u: { type: input, class: driving }
  g: { type: input, class: driving }
  y: { type: output }
equations:
  - { id: E, name: y, output: y, expression: { op: add, args: [ { ref: u }, { ref: g } ] } }
"#;
        let slow_yaml = r#"
meta: { id: S2, model: S, name_cn: 慢, dt: 1, dt_seconds: 2 }
variables:
  ybar: { type: input, class: driving }
  z: { type: output }
equations:
  - { id: E, name: z, output: z, expression: { op: mul, args: [ { ref: ybar }, { const: 2 } ] } }
"#;
        let (_df, fast) = write_model(fast_yaml);
        let (_ds, slow) = write_model(slow_yaml);
        let links = vec![CoupledLink { to: "ybar".into(), from: "y".into(), agg: Agg::Mean, scale: 1.0 }];
        let mut drv = HashMap::new();
        drv.insert("u".to_string(), vec![1.0; 6]); // 3 慢步 × R=2
        let mut inp = CoupledInput::new(&fast, &slow, links, drv, 3);
        inp.feedback = vec![FeedbackLink { to: "g".into(), from: "z".into(), scale: 1.0, init: 10.0 }];
        let out = simulate_coupled(&inp).unwrap();

        // 慢步0：g=init=10 → y=11、ybar=11、z=22；慢步1：g=22（上步 z）→ y=23、ybar=23、z=46；
        // 慢步2：g=46 → y=47、ybar=47、z=94。反馈滞后一慢步。
        assert_eq!(out.fed_drivers["ybar"], vec![11.0, 23.0, 47.0]);
        assert_eq!(out.slow.series("z").unwrap(), &[22.0, 46.0, 94.0]);
    }

    /// 耦合泛化：慢模型未被 link 覆盖的外生 driving 从**共享室外驱动(weather)**取（对等双向耦合，
    /// 如 apple↔soil：两模型都读同一份室外气象·apple 要 T/Sr、soil 要 Rain/Es_pot）。
    /// b 不在 links、但在 weather 有列 → 从中取值（v[s·R]），不再报「无耦合链接」。
    #[test]
    fn test_coupled_slow_reads_shared_weather() {
        let fast_yaml = r#"
meta: { id: F, model: F, name_cn: x, dt: 1, dt_seconds: 1 }
variables: { u: { type: input, class: driving }, y: { type: output } }
equations: [ { id: E, name: y, output: y, expression: { ref: u } } ]
"#;
        // 慢模型 driving：a（由 link 从 fast.y 取）+ b（未被 link·从共享 weather 取）
        let slow_yaml = r#"
meta: { id: S, model: S, name_cn: x, dt: 1, dt_seconds: 1 }
variables: { a: { type: input, class: driving }, b: { type: input, class: driving }, o: { type: output } }
equations: [ { id: E, name: o, output: o, expression: { op: add, args: [ { ref: a }, { ref: b } ] } } ]
"#;
        let (_df, fast) = write_model(fast_yaml);
        let (_ds, slow) = write_model(slow_yaml);
        // 只给 a 一条 link；b 无 link 但在 weather 有列 → 从 weather 取、不报错（R=1 对等耦合）
        let links = vec![CoupledLink { to: "a".into(), from: "y".into(), agg: Agg::Mean, scale: 1.0 }];
        let mut drv = HashMap::new();
        drv.insert("u".to_string(), vec![1.0, 2.0]); // fast 驱动（2 慢步 × R=1）
        drv.insert("b".to_string(), vec![10.0, 20.0]); // slow 的共享室外驱动（未被 link）
        let inp = CoupledInput::new(&fast, &slow, links, drv, 2);
        let out = simulate_coupled(&inp).unwrap(); // 不再报 Coupling 错
        // 慢步0：a=mean(y)=1、b=weather[0]=10 → o=11；慢步1：a=2、b=20 → o=22
        assert_eq!(out.slow.series("o").unwrap(), &[11.0, 22.0]);
    }

    /// 耦合泛化护栏：短 weather 列→Err(非 panic)、R>1 走 fallback→Err、feedback 与 weather 撞名→Err。
    #[test]
    fn test_coupled_weather_fallback_guards() {
        let fast_yaml = r#"
meta: { id: F, model: F, name_cn: x, dt: 1, dt_seconds: 1 }
variables: { u: { type: input, class: driving }, y: { type: output } }
equations: [ { id: E, name: y, output: y, expression: { ref: u } } ]
"#;
        let slow_yaml = r#"
meta: { id: S, model: S, name_cn: x, dt: 1, dt_seconds: 1 }
variables: { a: { type: input, class: driving }, b: { type: input, class: driving }, o: { type: output } }
equations: [ { id: E, name: o, output: o, expression: { op: add, args: [ { ref: a }, { ref: b } ] } } ]
"#;
        let (_df, fast) = write_model(fast_yaml);
        let (_ds, slow) = write_model(slow_yaml);
        let mk_link = || vec![CoupledLink { to: "a".into(), from: "y".into(), agg: Agg::Mean, scale: 1.0 }];

        // (1) 短列：b 只有 1 行 < slow_steps=2 → DriverLengthMismatch（不 panic）
        let mut drv = HashMap::new();
        drv.insert("u".to_string(), vec![1.0, 2.0]);
        drv.insert("b".to_string(), vec![10.0]);
        let inp = CoupledInput::new(&fast, &slow, mk_link(), drv, 2);
        assert!(matches!(simulate_coupled(&inp), Err(SimError::DriverLengthMismatch { .. })));

        // (2) R>1（slow dt_seconds=2 → R=2）且 b 走 weather fallback → Coupling Err
        let slow_r2_yaml = r#"
meta: { id: S2, model: S, name_cn: x, dt: 1, dt_seconds: 2 }
variables: { a: { type: input, class: driving }, b: { type: input, class: driving }, o: { type: output } }
equations: [ { id: E, name: o, output: o, expression: { op: add, args: [ { ref: a }, { ref: b } ] } } ]
"#;
        let (_ds2, slow2) = write_model(slow_r2_yaml);
        let mut drv2 = HashMap::new();
        drv2.insert("u".to_string(), vec![1.0, 2.0, 3.0, 4.0]);
        drv2.insert("b".to_string(), vec![10.0, 20.0, 30.0, 40.0]);
        let inp2 = CoupledInput::new(&fast, &slow2, mk_link(), drv2, 2);
        assert!(matches!(simulate_coupled(&inp2), Err(SimError::Coupling(_))));

        // (3) feedback target 与 weather 列撞名（都叫 g）→ Coupling Err（防反馈被 weather 静默覆盖）
        let fast_g_yaml = r#"
meta: { id: FG, model: F, name_cn: x, dt: 1, dt_seconds: 1 }
variables: { u: { type: input, class: driving }, g: { type: input, class: driving }, y: { type: output } }
equations: [ { id: E, name: y, output: y, expression: { op: add, args: [ { ref: u }, { ref: g } ] } } ]
"#;
        let (_dfg, fastg) = write_model(fast_g_yaml);
        let mut drv3 = HashMap::new();
        drv3.insert("u".to_string(), vec![1.0, 2.0]);
        drv3.insert("g".to_string(), vec![5.0, 5.0]);
        drv3.insert("b".to_string(), vec![10.0, 20.0]);
        let mut inp3 = CoupledInput::new(&fastg, &slow, mk_link(), drv3, 2);
        inp3.feedback = vec![FeedbackLink { to: "g".into(), from: "o".into(), scale: 1.0, init: 0.0 }];
        assert!(matches!(simulate_coupled(&inp3), Err(SimError::Coupling(_))));
    }

    /// 速率方程引用自身状态量当前值 = rate→state 步内环。底层 `build_plan`/`topo_order` 仍报环；
    /// 但 `simulate` 经 **E5b** 自动插 `X_prev` 破环 → 正常跑（前向 Euler：R[n]=0.1·X[n−1]，
    /// X[n]=X[n−1]+R·dt，dt=1 → X=[1.1, 1.21]）。
    #[test]
    fn test_e5b_auto_inserts_prev_for_self_ref_rate() {
        let yaml = r#"
meta: { id: C, model: C, name_cn: x }
variables:
  X:  { type: output, class: state, init: 1.0, rate: R }
  R:  { type: intermediate, class: rate }
equations:
  - id: E1
    name: r
    output: R
    expression: { op: mul, args: [ { ref: X }, { const: 0.1 } ] }
"#;
        let (_d, file) = write_model(yaml);
        // 底层机制：原始 `_prev`-free 模型仍被 `build_plan` 判为步内环。
        assert!(matches!(build_plan(&file), Err(SimError::Cycle(_))));
        // E5b：`insert_prev_for_explicit` 补 `X_prev` 延迟寄存器破环。
        let fixed = insert_prev_for_explicit(&file).unwrap();
        assert!(fixed.variables.get("X_prev").map_or(false, |v| v.is_delay()));
        assert!(build_plan(&fixed).is_ok());
        // `simulate` 经 E5b 自动破环 → 正常跑，前向 Euler 轨迹 X=[1.1, 1.21]。
        let input = SimInput::new(2);
        let out = simulate(&file, &input).unwrap();
        let x = out.trajectories.get("X").expect("X 轨迹");
        assert!((x[0] - 1.1).abs() < 1e-12, "X[0]={}", x[0]);
        assert!((x[1] - 1.21).abs() < 1e-12, "X[1]={}", x[1]);
    }

    /// **E5b 金标准（E5a↔E5b 互逆 + 逐位复现）**：带手写 `_prev` 的模型经 E5a 折成 `_prev`-free、
    /// 再经 E5b（`simulate` 内自动）重插 → 显式仿真轨迹与原版**逐位一致**。覆盖三种状态读：
    /// 自态 flux 读（rX 读 X）、跨态 flux 读（rY 读 X）、诊断读本步（diag 读 X/Y 当前值）。
    /// （用 E5a `fold_prev_for_implicit` 做逆变换，故仅 `implicit` 构建编译。）
    #[cfg(feature = "implicit")]
    #[test]
    fn test_e5b_roundtrip_bit_identical() {
        let yaml = r#"
meta: { id: RT, model: RT, name_cn: x, dt: 1 }
variables:
  X:      { type: output, class: state, init: 10.0, rate: rX }
  X_prev: { class: semi_state, init: 10.0, prev: X }
  Y:      { type: output, class: state, init: 3.0, rate: rY }
  Y_prev: { class: semi_state, init: 3.0, prev: Y }
  rX:   { class: rate }
  rY:   { class: rate }
  diag: { type: output, class: auxiliary }
equations:
  - { id: RX, name: rX, output: rX, expression: { op: mul, args: [ { const: 0.2 }, { op: sub, args: [ { const: 5.0 }, { ref: X_prev } ] } ] } }
  - { id: RY, name: rY, output: rY, expression: { op: mul, args: [ { const: 0.1 }, { ref: X_prev } ] } }
  - { id: D,  name: diag, output: diag, expression: { op: add, args: [ { ref: X }, { ref: Y } ] } }
"#;
        let (_d, original) = write_model(yaml);
        // E5a：折成 `_prev`-free（删寄存器、X_prev→X）。
        let stripped = crate::sim::implicit::fold_prev_for_implicit(&original).unwrap();
        assert!(stripped.variables.get("X_prev").is_none(), "E5a 应删 X_prev 寄存器");
        assert!(stripped.variables.get("Y_prev").is_none(), "E5a 应删 Y_prev 寄存器");
        assert!(build_plan(&stripped).is_err(), "折叠后 `_prev`-free 应成步内环");
        // 两路同一驱动跑显式仿真：original（E5b no-op）vs stripped（E5b 重插 `_prev`）。
        let input = SimInput::new(6);
        let out_orig = simulate(&original, &input).unwrap();
        let out_strip = simulate(&stripped, &input).unwrap();
        for var in ["X", "Y", "diag"] {
            assert_eq!(
                out_orig.trajectories.get(var).unwrap(),
                out_strip.trajectories.get(var).unwrap(),
                "{var} 轨迹应逐位一致（E5b 忠实重建手写破环约定）"
            );
        }
    }

    /// 命名守卫（BUG1/BUG2a）：生成名 `X_prev` 已被别的变量占用（此处 X_prev 本身是个状态）
    /// → E5b loud 报错，绝不静默复用/捕获致错值。
    #[test]
    fn test_e5b_rejects_prev_name_taken_by_other_var() {
        let yaml = r#"
meta: { id: NC, model: NC, name_cn: x }
variables:
  X:      { type: output, class: state, init: 10.0, rate: rX }
  X_prev: { type: output, class: state, init: 5.0, rate: rXp }
  rX:  { class: rate }
  rXp: { class: rate }
equations:
  - { id: RX,  name: rX,  output: rX,  expression: { op: mul, args: [ { const: -0.1 }, { ref: X } ] } }
  - { id: RXP, name: rXp, output: rXp, expression: { const: 0.0 } }
"#;
        let (_d, file) = write_model(yaml);
        assert!(
            matches!(insert_prev_for_explicit(&file), Err(SimError::Solver(_))),
            "X_prev 名被状态占用 → 应 loud 报错"
        );
    }

    /// 命名守卫（BUG2c）：生成名 `X_prev` 已是参数 → E5b loud 报错，绝不造 参数/变量 双声明。
    #[test]
    fn test_e5b_rejects_prev_name_taken_by_param() {
        let yaml = r#"
meta: { id: NP, model: NP, name_cn: x }
parameters:
  X_prev: { name_cn: 冲突参数, default: 5.0 }
variables:
  X:  { type: output, class: state, init: 1.0, rate: rX }
  rX: { class: rate }
equations:
  - { id: RX, name: rX, output: rX, expression: { op: mul, args: [ { const: -0.1 }, { ref: X } ] } }
"#;
        let (_d, file) = write_model(yaml);
        assert!(matches!(insert_prev_for_explicit(&file), Err(SimError::Solver(_))));
    }

    /// 复用分支：已有匹配的手写 `X_prev`（prev: X）但 rX 漏用、直读了 X → 成环。
    /// E5b 复用现成寄存器破环（不报错、不重建）。
    #[test]
    fn test_e5b_reuses_matching_prev_register() {
        let yaml = r#"
meta: { id: RU, model: RU, name_cn: x, dt: 1 }
variables:
  X:      { type: output, class: state, init: 10.0, rate: rX }
  X_prev: { class: semi_state, init: 10.0, prev: X }
  rX: { class: rate }
equations:
  - { id: RX, name: rX, output: rX, expression: { op: mul, args: [ { const: -0.1 }, { ref: X } ] } }
"#;
        let (_d, file) = write_model(yaml);
        assert!(matches!(build_plan(&file), Err(SimError::Cycle(_))), "直读 X 应成环");
        let fixed = insert_prev_for_explicit(&file).unwrap();
        assert!(build_plan(&fixed).is_ok(), "E5b 复用 X_prev 破环");
        assert_eq!(fixed.variables.get("X_prev").unwrap().prev.as_deref(), Some("X"));
    }
}
