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
use crate::schema::EquationFile;

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

/// 耦合仿真输入（C1：两模型、单向 快→慢）。
pub struct CoupledInput<'a> {
    /// 快模型（如温室，小 dt）。
    pub fast: &'a EquationFile,
    /// 慢模型（如作物，大 dt）。
    pub slow: &'a EquationFile,
    /// 快→慢链接（慢模型每个驱动量都须被某条链接覆盖）。
    pub links: Vec<CoupledLink>,
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

    // —— 校验：慢模型每个驱动量都须被某条链接覆盖（C1 单向：作物气候全来自温室）——
    let link_tos: HashSet<&str> = input.links.iter().map(|l| l.to.as_str()).collect();
    {
        let slow_stepper = Stepper::new(slow, slow.meta.dt, &input.slow_params, &input.slow_init)?;
        for &d in slow_stepper.drivers() {
            if !link_tos.contains(d) {
                return Err(SimError::Coupling(format!(
                    "慢模型驱动量 '{d}' 没有耦合链接（请在 links 里把它接到某快模型输出）"
                )));
            }
        }
    }
    // —— 校验：快模型室外驱动齐全、长度 = total_fast ——
    let mut fast_stepper = Stepper::new(fast, fast.meta.dt, &input.fast_params, &input.fast_init)?;
    for &d in fast_stepper.drivers() {
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

    for s in 0..input.slow_steps {
        // 累加器：mean/integral 用 sum，last 用末值
        let mut acc = vec![0.0f64; input.links.len()];
        let mut last = vec![0.0f64; input.links.len()];
        for f in 0..r {
            let gi = s * r + f;
            fast_stepper.step(|d| input.fast_drivers.get(d).map(|v| v[gi]))?;
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
        // 慢模型推进一步：驱动量按 to 名取聚合值
        let get_slow_driver = |name: &str| -> Option<f64> {
            input.links.iter().position(|l| l.to == name).map(finalize)
        };
        slow_stepper.step(get_slow_driver)?;
        for name in slow.variables.keys() {
            let v = slow_stepper
                .get(name)
                .ok_or_else(|| SimError::Unresolved(name.clone()))?;
            flatten_into(&mut slow_traj, name, &v);
        }
    }

    Ok(CoupledOutput {
        slow_steps: input.slow_steps,
        r,
        slow: SimOutput { steps: input.slow_steps, trajectories: slow_traj },
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
