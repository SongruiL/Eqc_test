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
use crate::schema::{BalanceLaw, EquationFile};

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
        }
    }
}

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

/// 按 `meta.balance` 声明逐条核守恒律：`|Δstock − dt·(Σsources − Σsinks)/cap| ≤ tol`。
/// `cap`（可选「有效容量」变量）缺省≡1。**纯核算、不打印**：CLI `--check-balance` 消费它做诊断输出，
/// GP 候选硬过滤（Tier3）也消费它判「候选是否破守恒」——单一真相源。
/// 缺存量/源汇/cap 轨迹的守恒律记 `skip_reason` + `ok=false`（与旧 CLI `any_fail` 逐字节等义）。
pub fn check_balance_laws(laws: &[BalanceLaw], out: &SimOutput, dt: f64) -> Vec<BalanceLawCheck> {
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
        let (max_resid, argstep) = balance_residual(stock, &net_eff, dt);
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
    let mut stepper = Stepper::new(file, dt, &input.param_overrides, &input.init_overrides)?;

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
        for name in file.variables.keys() {
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
        }
    }
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
    let mut fast_stepper = Stepper::new(fast, fast.meta.dt, &input.fast_params, &input.fast_init)?;
    let fb_targets: HashSet<&str> = input.feedback.iter().map(|f| f.to.as_str()).collect();
    let mut fb: HashMap<String, f64> = input.feedback.iter().map(|f| (f.to.clone(), f.init)).collect();
    // 反馈 to 必须是快模型的输入（驱动量），否则反馈值无处可喂
    {
        let fast_driver_set: HashSet<&str> = fast_stepper.drivers().iter().copied().collect();
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
    for &d in fast_stepper.drivers() {
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
            fast_stepper.step(|d| {
                input.fast_drivers.get(d).map(|v| v[gi]).or_else(|| fb.get(d).copied())
            })?;
            for (li, link) in input.links.iter().enumerate() {
                let v = fast_stepper
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
                    match fast_stepper.get(name) {
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
fn flatten_into(traj: &mut IndexMap<String, Vec<f64>>, name: &str, v: &Value) {
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

    /// 速率方程引用自身状态量当前值 → 步内环。
    #[test]
    fn test_self_referential_rate_is_cycle() {
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
        let input = SimInput::new(2);
        assert!(matches!(simulate(&file, &input), Err(SimError::Cycle(_))));
    }
}
