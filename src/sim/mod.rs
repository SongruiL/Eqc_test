//! 逐日仿真引擎：对 [`EquationFile`] 描述的**动态过程模型**做显式 Euler 时间步进。
//!
//! 这是 B 路线的第二块：B1 给 schema 加了 Forrester 分类与状态量元数据
//! （`class` / `init` / `rate` / `prev`），本模块据此把一个静态方程网络「跑起来」，
//! 沿时间序列逐日积分出状态量轨迹（如累积干物质、累积温度、果实干重）。
//!
//! # 计算模型
//!
//! 每个时间步 `n`（日步长 dt=1）按依赖拓扑序求值：
//!
//! - **驱动量 driving**：未被任何方程产生、非跨步的输入变量，逐日从 [`SimInput::drivers`] 取值。
//! - **参数 parameter**：来自 `parameters:` 的默认值，可被 [`SimInput::param_overrides`] 覆盖。
//! - **延迟寄存器 prev（半状态量）**：`X[n] = src[n-1]`，首步用 `init`。在步首即可定值
//!   （只依赖上一步），故视为「源」。
//! - **方程辅助/速率量**：普通 `equations:` 表达式，由 [`Expr::eval`] 求值。
//! - **积分状态量 state**：`X[n] = X[n-1] + rate[n]`，`X[-1]` 用 `init`。
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

use crate::eval::{Env, EvalError};
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
        }
    }
}

impl std::error::Error for SimError {}

/// 仿真输入。
#[derive(Debug, Clone, Default)]
pub struct SimInput {
    /// 时间步数（天数）。
    pub steps: usize,
    /// 驱动量时间序列：变量名 -> 每步取值（长度须等于 `steps`）。
    pub drivers: HashMap<String, Vec<f64>>,
    /// 参数覆盖：变量名 -> 值（覆盖 `parameters:` 默认值）。
    pub param_overrides: HashMap<String, f64>,
}

impl SimInput {
    /// 构造一个 `steps` 步的空输入。
    pub fn new(steps: usize) -> Self {
        Self { steps, drivers: HashMap::new(), param_overrides: HashMap::new() }
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

/// 对一个动态模型做逐日仿真。
///
/// 单模块求值：跨模块 `source` 耦合不在此处展开——任何未被方程产生、非跨步的
/// 输入变量都必须由 [`SimInput::drivers`] 提供。
pub fn simulate(file: &EquationFile, input: &SimInput) -> Result<SimOutput, SimError> {
    // —— 1. 归类变量 ——
    // 方程输出 -> 表达式
    let mut eq_of: HashMap<&str, &crate::ast::Expr> = HashMap::new();
    for eq in &file.equations {
        if !file.variables.contains_key(&eq.output) {
            return Err(SimError::UndeclaredOutput(eq.output.clone()));
        }
        eq_of.insert(eq.output.as_str(), &eq.expression);
    }

    // 步内可计算节点：方程输出 ∪ 积分状态量。延迟寄存器/驱动/参数为「源」，步首预置。
    // 用 Vec 保持声明顺序；拓扑排序按下标进行。
    let mut nodes: Vec<(&str, Node)> = Vec::new();
    let mut node_idx: HashMap<&str, usize> = HashMap::new();
    // 延迟寄存器：(name, prev_src, init)
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
        // 其余（无方程、非跨步）= 驱动量，留待步首从 drivers 取。
    }

    // —— 2. 校验驱动量 ——
    // 驱动量 = 变量里既无方程、又非跨步者。
    let delay_names: HashSet<&str> = delays.iter().map(|(n, _, _)| *n).collect();
    let mut driver_names: Vec<&str> = Vec::new();
    for (name, _var) in &file.variables {
        let n = name.as_str();
        if node_idx.contains_key(n) || delay_names.contains(n) {
            continue;
        }
        // 是驱动量：必须有时间序列
        driver_names.push(n);
        match input.drivers.get(n) {
            None => return Err(SimError::MissingDriver(name.clone())),
            Some(series) if series.len() != input.steps => {
                return Err(SimError::DriverLengthMismatch {
                    name: name.clone(),
                    expected: input.steps,
                    found: series.len(),
                })
            }
            Some(_) => {}
        }
    }

    // —— 3. 步内拓扑排序（Kahn）——
    // 依赖：方程节点依赖其表达式里属于「可计算节点」的变量引用；
    //       积分节点依赖其 rate（若 rate 也是可计算节点）。
    let order = topo_order(&nodes, &node_idx)?;

    // —— 4. 逐步求值 ——
    // 参数值表（常量，每步相同）
    let mut params: HashMap<&str, f64> = HashMap::new();
    for (pname, p) in &file.parameters {
        let v = input.param_overrides.get(pname).copied().unwrap_or(p.default);
        params.insert(pname.as_str(), v);
    }

    // 轨迹容器，按 variables: 声明顺序
    let mut traj: IndexMap<String, Vec<f64>> = IndexMap::new();
    for name in file.variables.keys() {
        traj.insert(name.clone(), Vec::with_capacity(input.steps));
    }

    for n in 0..input.steps {
        let mut env = Env::new();
        // 4a-0. 内置只读变量 DAT = 第几天（1 起）。供物候/开花门控直接引用，无需手填驱动量。
        env.set("DAT", (n + 1) as f64);
        // 4a. 参数
        for (pname, v) in &params {
            env.set(*pname, *v);
        }
        // 4b. 驱动量
        for d in &driver_names {
            let v = input.drivers[*d][n];
            env.set(*d, v);
        }
        // 4c. 延迟寄存器：X[n] = src[n-1]（首步用 init）
        for (name, src, init) in &delays {
            let v = if n == 0 {
                *init
            } else {
                prev_value(&traj, src, &params)
                    .ok_or_else(|| SimError::Unresolved((*name).to_string()))?
            };
            env.set(*name, v);
        }
        // 4d. 按拓扑序求值方程与积分状态量
        for &idx in &order {
            let (name, node) = &nodes[idx];
            match node {
                Node::Equation(expr) => {
                    // V0：仿真器仍为标量（向量化在 V2）；非标量结果在此显式失败。
                    let v = expr
                        .eval_scalar(&env)
                        .map_err(|err| SimError::Eval { var: (*name).to_string(), err })?;
                    env.set(*name, v);
                }
                Node::Integrator { rate, init } => {
                    let prev = if n == 0 {
                        *init
                    } else {
                        traj.get(*name).unwrap()[n - 1]
                    };
                    let r = env
                        .get_scalar(rate)
                        .ok_or_else(|| SimError::Unresolved((*rate).to_string()))?;
                    env.set(*name, prev + r);
                }
            }
        }
        // 4e. 记录本步所有变量值（V0 标量轨迹）
        for (name, series) in traj.iter_mut() {
            let v = env
                .get_scalar(name)
                .ok_or_else(|| SimError::Unresolved(name.clone()))?;
            series.push(v);
        }
    }

    Ok(SimOutput { steps: input.steps, trajectories: traj })
}

/// 取某来源变量「上一步」的值：先查已记录轨迹，再退回参数（常量）。
fn prev_value(traj: &IndexMap<String, Vec<f64>>, src: &str, params: &HashMap<&str, f64>) -> Option<f64> {
    if let Some(series) = traj.get(src) {
        return series.last().copied();
    }
    params.get(src).copied()
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
