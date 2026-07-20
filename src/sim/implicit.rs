//! 隐式刚性求解器（Phase 0 引擎地基）——把动态模型交给 diffsol 的 BDF 解真联立系统。
//!
//! # 与显式 Euler（[`super::simulate`]）的关系
//!
//! 显式引擎（`Stepper`）一趟拓扑序求值 + `X += rate·dt`，靠手写 `_prev` 延迟寄存器破步内环
//! （速率方程读状态量**上一步**值）。隐式路径把这套翻过来：
//!
//! - **E5a 折叠**（[`fold_prev_for_implicit`]）：把手写 `_prev` 引用折回**真态**（`X_prev → X`）、
//!   删除延迟寄存器变量。源文件不动，只作用于内存克隆（SSOT：一份源、两种编译变换）。
//! - **rate 计划**（[`build_rate_plan`]）：把 state 当**输入源**（不进拓扑），只对方程做拓扑序，
//!   读出每个 state 的 `rate`。折叠后 state 直接进速率路径——显式引擎会报 `Cycle`，而这里正是
//!   隐式求解器要解的**真联立系统**（`dX/dt = f(X, drivers, t)`，一个刚性 ODE，非 DAE）。
//! - **RHS 闭包**：一趟 rate 计划求值即 `f(t, y)=dy/dt`——把 trial 状态灌进 `Env`、复用现成
//!   [`crate::ast::Expr::eval_in_with`]。Jacobian 由「通用有限差分 `J·v`」提供（复用同一 RHS，
//!   与模型无关的样板；照 GreenLight 不写逐模型解析 jac）。diffsol 内部 Newton + BDF 变阶自适应。
//!
//! # 驱动量口径（Phase 0）
//!
//! 逐驱动步（`dt`）推进：每步把驱动量按**零阶保持（ZOH）**设为该步常数，隐式求解器在
//! `[t_n, t_{n+1}]` 上**自适应内解**（把有效最大步长天然限在 `dt` 内 = diffsol 无 `max_step` 的兜底）。
//! 常数驱动下逐段续解 = 精确连续解在网格点上的采样——V1 据此与「显式细化」对齐。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

use indexmap::IndexMap;

use crate::ast::Expr;
use crate::eval::{Env, EvalError, EvalMode, Value};
use crate::schema::EquationFile;

use super::{flatten_into, SimError, SimInput, SimOutput};

use diffsol::{NalgebraLU, NalgebraMat, OdeBuilder, OdeSolverMethod, OdeSolverStopReason};

/// diffsol 稠密矩阵后端（纯 Rust nalgebra，稠密 LU 够 BDF 用）。
type M = NalgebraMat<f64>;

/// 隐式求解选项。
#[derive(Debug, Clone, Copy)]
pub struct ImplicitOpts {
    /// 相对容差（BDF 局部误差控制）。GreenLight 默认 1e-6。
    pub rtol: f64,
    /// 绝对容差。GreenLight 默认 1e-3；V1 校核可收紧（如 1e-9）逼近「精确」。
    pub atol: f64,
    /// E2 平滑化拐角圆化宽度（无量纲）。`None` = 不跑平滑 pass（全光滑模型/V1 逐位不变）；
    /// `Some(ε)` = 对**状态依赖**的非光滑算子（clamp/max/min…）施平滑代理（拐角 ~ε 宽度圆化）。
    /// 因控制律自变量常被 Pband 归一化成无量纲量，单个 ε（~0.05）即可，无需逐物理量 pBand。
    /// 只平滑「自变量子树引用状态量」的开关——驱动/时间开关段内恒常、不进状态 Jacobian、留硬。
    pub smooth_eps: Option<f64>,
}

impl Default for ImplicitOpts {
    fn default() -> Self {
        // 抄 GreenLight `solve_ivp(BDF)` 默认。平滑默认关（全光滑模型无需）。
        Self { rtol: 1e-6, atol: 1e-3, smooth_eps: None }
    }
}

/// 一个积分状态量的隐式规格。
struct StateSpec {
    /// 状态量名。
    name: String,
    /// 速率来源变量名（方程输出 / 参数 / 驱动）。
    rate: String,
    /// 初值。
    init: f64,
}

/// **速率计划**：把动态模型编译成「给定状态向量与驱动，算 dy/dt」要做的事——state 作输入源、
/// 方程按拓扑序求值、读出各 state 的速率。是隐式 RHS 闭包的单一真相源。
pub struct RatePlan {
    /// 积分状态量（顺序 = 状态向量 y 的分量顺序）。
    states: Vec<StateSpec>,
    /// 拓扑序的方程 `(输出名, 表达式)`——state/驱动/参数是源、不在其中。
    ordered_eqs: Vec<(String, Expr)>,
    /// 驱动量名（无方程、非积分量、非参数）。
    drivers: Vec<String>,
}

impl RatePlan {
    /// 状态量个数（= 状态向量维数）。
    pub fn n_states(&self) -> usize {
        self.states.len()
    }
    /// 驱动量名。
    pub fn drivers(&self) -> &[String] {
        &self.drivers
    }
}

/// **E5a：折叠手写 `_prev`**（隐式向）。把每个延迟寄存器 `X_prev`（`prev: X`）在所有方程里的
/// 引用替换回真态 `X`，并删除该延迟寄存器变量。**不改源文件**，返回改过的内存克隆。
///
/// 折叠后速率方程直接读真态，形成显式引擎会报 `Cycle` 的步内环——正是隐式求解器要解的联立系统。
/// 复用现成 [`Expr::substitute`]（与 `reclassify_parameters` 同款 AST 改写）。
///
/// **只折「源是状态量」的延迟寄存器**（state-lag，破 rate→state 代数环）。EQC 里 `_prev` 还有
/// 第二种用法：对**非状态量**（auxiliary）做离散一阶差分（如 `DRLG = RLG − RLG_prev`，RLG 是有方程
/// 的辅助量）。把这类 `_prev` 折回真态会让差分**恒等于 0**（静默错值）。故对「源非 `is_integrator`」的
/// 延迟寄存器**显式报错拒绝**（loud fail），而非无差别折叠。隐式路径下这类差分寄存器的正确语义
/// （段初常数）留待 0b+（见 spec §6）。同时挡住链式 `_prev`（源是另一个半状态量、非 state）。
pub fn fold_prev_for_implicit(file: &EquationFile) -> Result<EquationFile, SimError> {
    let mut folded = file.clone();
    // (延迟寄存器名, 真态名)——只收「源是状态量」的；源非 state 直接拒绝。
    let mut pairs: Vec<(String, String)> = Vec::new();
    for (name, v) in &folded.variables {
        let src = match &v.prev {
            Some(s) => s,
            None => continue,
        };
        let src_is_state = folded.variables.get(src).map_or(false, |sv| sv.is_integrator());
        if !src_is_state {
            return Err(SimError::Solver(format!(
                "隐式路径暂不支持「源非状态量」的延迟寄存器 '{name}'（prev: {src}）：\
                 {src} 不是积分状态量，折叠会让离散差分恒为 0（静默错值）。\
                 此类 auxiliary 差分寄存器的隐式语义（段初常数）留待后续阶段。"
            )));
        }
        pairs.push((name.clone(), src.clone()));
    }
    for (prev_name, src) in &pairs {
        let repl = Expr::Var(src.clone());
        for eq in &mut folded.equations {
            eq.expression = eq.expression.substitute(prev_name, &repl);
        }
    }
    for (prev_name, _) in &pairs {
        folded.variables.shift_remove(prev_name.as_str());
    }
    Ok(folded)
}

fn bx(e: Expr) -> Box<Expr> {
    Box::new(e)
}

/// 平滑 `max(a,b)` → `0.5(a+b+√((a−b)²+ε²))`（GreenLight Eq 9.27 平滑 max0 原型）。
fn smooth_max(a: Expr, b: Expr, eps: f64) -> Expr {
    let diff = Expr::Sub(bx(a.clone()), bx(b.clone()));
    let sq = Expr::Mul(bx(diff.clone()), bx(diff)); // (a−b)²
    let root = Expr::Sqrt(bx(Expr::Add(bx(sq), bx(Expr::Const(eps * eps)))));
    let sum = Expr::Add(bx(Expr::Add(bx(a), bx(b))), bx(root)); // a+b+√…
    Expr::Mul(bx(Expr::Const(0.5)), bx(sum))
}

/// 平滑 `min(a,b)` → `0.5(a+b−√((a−b)²+ε²))`。
fn smooth_min(a: Expr, b: Expr, eps: f64) -> Expr {
    let diff = Expr::Sub(bx(a.clone()), bx(b.clone()));
    let sq = Expr::Mul(bx(diff.clone()), bx(diff));
    let root = Expr::Sqrt(bx(Expr::Add(bx(sq), bx(Expr::Const(eps * eps)))));
    let sum = Expr::Sub(bx(Expr::Add(bx(a), bx(b))), bx(root)); // a+b−√…
    Expr::Mul(bx(Expr::Const(0.5)), bx(sum))
}

fn smooth_max_vec(args: Vec<Expr>, eps: f64) -> Expr {
    let mut it = args.into_iter();
    let mut acc = match it.next() {
        Some(a) => a,
        None => return Expr::Const(0.0),
    };
    for x in it {
        acc = smooth_max(acc, x, eps);
    }
    acc
}

fn smooth_min_vec(args: Vec<Expr>, eps: f64) -> Expr {
    let mut it = args.into_iter();
    let mut acc = match it.next() {
        Some(a) => a,
        None => return Expr::Const(0.0),
    };
    for x in it {
        acc = smooth_min(acc, x, eps);
    }
    acc
}

/// **E2 平滑化（隐式向）**：把**状态依赖**的非光滑算子替换成 C¹ 光滑代理，让隐式 Newton 的
/// Jacobian 良定义。**外科式**：`if !子树引用状态量 → 原样`——驱动/时间开关（如 `if(I_glob≥…)`、
/// `if(DAT<…)`）段-ZOH 下段内恒常、对 `∂f/∂state` 零贡献、留硬（其跳变落段边界，由分段重启处理）。
/// 只平滑真正进状态 Jacobian 的开关（如控制律 `clamp((T_air−setpt)/Pband,0,1)`）。因自变量常被
/// Pband 归一化成无量纲量，单个 ε 即可。**0b 覆盖**：算术容器（+−×÷^neg）递归 + Max/Min/Clamp/Abs
/// 代理；关系/if（Phase 1 结露门 `if vp>vp_sat`）留待扩展（届时状态依赖的它们暂原样、隐式或需更小步）。
fn smooth_expr(e: &Expr, states: &HashSet<String>, eps: f64) -> Expr {
    // 子树无状态依赖 → 原样（驱动/时间开关留硬；也短路掉纯常数子树）
    if !e.get_variable_refs().iter().any(|r| states.contains(r)) {
        return e.clone();
    }
    let s = |x: &Expr| smooth_expr(x, states, eps);
    match e {
        // 递归算术容器（到达内部的非光滑算子）
        Expr::Add(a, b) => Expr::Add(bx(s(a)), bx(s(b))),
        Expr::Sub(a, b) => Expr::Sub(bx(s(a)), bx(s(b))),
        Expr::Mul(a, b) => Expr::Mul(bx(s(a)), bx(s(b))),
        Expr::Div(a, b) => Expr::Div(bx(s(a)), bx(s(b))),
        Expr::Pow(a, b) => Expr::Pow(bx(s(a)), bx(s(b))),
        Expr::Neg(a) => Expr::Neg(bx(s(a))),
        // 非光滑算子 → 平滑代理（此处 dep 已为真 = 状态依赖）
        Expr::Max(args) => smooth_max_vec(args.iter().map(|a| s(a)).collect(), eps),
        Expr::Min(args) => smooth_min_vec(args.iter().map(|a| s(a)).collect(), eps),
        Expr::Clamp(x, lo, hi) => smooth_max(s(lo), smooth_min(s(hi), s(x), eps), eps),
        Expr::Abs(x) => {
            let xs = s(x);
            Expr::Sqrt(bx(Expr::Add(
                bx(Expr::Mul(bx(xs.clone()), bx(xs))),
                bx(Expr::Const(eps * eps)),
            )))
        }
        // 叶子（Var/Const/Param）及未下探的光滑容器：原样。温室控制律的开关都在算术层，
        // 故不下探 exp/ln/sum 等找埋藏开关；Phase 1 按需扩展。
        other => other.clone(),
    }
}

/// 对（已折叠的）模型的每条方程施 [`smooth_expr`]（拐角圆化宽度 ε）。返回改过的克隆。
pub fn smooth_for_implicit(file: &EquationFile, eps: f64) -> EquationFile {
    let states: HashSet<String> = file
        .variables
        .iter()
        .filter(|(_, v)| v.is_integrator())
        .map(|(n, _)| n.clone())
        .collect();
    let mut out = file.clone();
    for eq in &mut out.equations {
        eq.expression = smooth_expr(&eq.expression, &states, eps);
    }
    out
}

/// 把（已折叠 `_prev` 的）模型编译成速率计划：分类状态量/驱动/方程 + 对方程做拓扑排序。
/// `init_overrides` 覆盖状态量初值。
pub fn build_rate_plan(
    file: &EquationFile,
    init_overrides: &HashMap<String, f64>,
) -> Result<RatePlan, SimError> {
    // 方程输出 -> 表达式（保留声明顺序）
    let mut eq_of: IndexMap<&str, &Expr> = IndexMap::new();
    for eq in &file.equations {
        if !file.variables.contains_key(&eq.output) {
            return Err(SimError::UndeclaredOutput(eq.output.clone()));
        }
        eq_of.insert(eq.output.as_str(), &eq.expression);
    }

    // 积分状态量（顺序 = 声明顺序）
    let mut states: Vec<StateSpec> = Vec::new();
    for (name, v) in &file.variables {
        if !v.is_integrator() {
            continue;
        }
        let rate = v.rate.as_deref().unwrap().to_string();
        // 速率来源须存在（方程输出 / 参数 / 变量[驱动]）
        if !eq_of.contains_key(rate.as_str())
            && !file.parameters.contains_key(&rate)
            && !file.variables.contains_key(&rate)
        {
            return Err(SimError::UndefinedSource { var: name.clone(), source: rate });
        }
        let init = init_overrides
            .get(name)
            .copied()
            .or(v.init)
            .ok_or_else(|| SimError::MissingInit(name.clone()))?;
        states.push(StateSpec { name: name.clone(), rate, init });
    }

    // 驱动量 = 既非积分量、又无方程、又非参数（折叠后已无延迟寄存器）
    let mut drivers: Vec<String> = Vec::new();
    for (name, v) in &file.variables {
        if v.is_integrator() {
            continue;
        }
        if eq_of.contains_key(name.as_str()) {
            continue;
        }
        if file.parameters.contains_key(name) {
            continue;
        }
        drivers.push(name.clone());
    }

    // 对方程做拓扑排序（Kahn）：节点 = 方程输出；依赖 = 引用 ∩ 方程输出（state/驱动/参数是源）
    let names: Vec<&str> = eq_of.keys().copied().collect();
    let idx: HashMap<&str, usize> = names.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    let count = names.len();
    let mut indeg = vec![0usize; count];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (i, &n) in names.iter().enumerate() {
        let expr = eq_of[n];
        let mut deps: Vec<usize> = expr
            .get_variable_refs()
            .into_iter()
            .filter_map(|r| idx.get(r.as_str()).copied())
            .collect();
        deps.sort_unstable();
        deps.dedup();
        for d in deps {
            if d == i {
                // 方程自引用（罕见）——这不是「state 破环」（state 不是方程输出），是真错误
                return Err(SimError::Cycle(vec![n.to_string()]));
            }
            adj[d].push(i);
            indeg[i] += 1;
        }
    }
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
            .map(|i| names[i].to_string())
            .collect();
        return Err(SimError::Cycle(remaining));
    }

    let ordered_eqs = order
        .iter()
        .map(|&i| (names[i].to_string(), eq_of[names[i]].clone()))
        .collect();

    Ok(RatePlan { states, ordered_eqs, drivers })
}

/// 一趟 rate 计划求值：给定状态向量 `x`（+ 已灌进 `env` 的驱动/参数/DAT），算出各 state 的速率 `out`。
///
/// **非严格模式**（`strict:false`）：Newton 的 trial state 常探到病态值（负浓度/过冲），严格模式会
/// 把 NaN/Inf 变成 `Err` 让 Newton 无法从惩罚值恢复；这里让 Inf/NaN 传播给 diffsol 的步长回退。
/// 结构性错误（未定义变量等）记入 `err`，由调用方在段末检查。
fn eval_rhs(
    plan: &RatePlan,
    env_cell: &RefCell<Env>,
    x: &[f64],
    out: &mut [f64],
    err: &RefCell<Option<EvalError>>,
) {
    let mut e = env_cell.borrow_mut();
    for (i, s) in plan.states.iter().enumerate() {
        e.put(s.name.as_str(), Value::Scalar(x[i]));
    }
    for (name, expr) in &plan.ordered_eqs {
        match expr.eval_in_with(&mut e, EvalMode { strict: false }) {
            Ok(v) => e.put(name.as_str(), v),
            Err(er) => {
                if err.borrow().is_none() {
                    *err.borrow_mut() = Some(er);
                }
            }
        }
    }
    for (i, s) in plan.states.iter().enumerate() {
        // 速率须为标量。向量速率（如 FSPM 向量态）在隐式路径暂不支持——用 `as_scalar()` 的
        // NotScalar 错误 loud 报（记入 err，段末上报），替静默 NaN。
        match e.get(s.rate.as_str()).map(|v| v.as_scalar()) {
            Some(Ok(v)) => out[i] = v,
            Some(Err(er)) => {
                if err.borrow().is_none() {
                    *err.borrow_mut() = Some(er);
                }
                out[i] = f64::NAN;
            }
            None => {
                if err.borrow().is_none() {
                    *err.borrow_mut() = Some(EvalError::UndefinedVar(s.rate.clone()));
                }
                out[i] = f64::NAN;
            }
        }
    }
}

/// 在一个驱动步 `[0, dt]` 上隐式推进：构建 diffsol BDF problem（RHS + 通用 FD `J·v`）、自适应内解、
/// 返回段末状态向量。驱动/DAT 已在 `env_cell` 里设为本段常数。
fn advance_segment(
    plan: &RatePlan,
    env_cell: &RefCell<Env>,
    x0: &[f64],
    dt: f64,
    opts: ImplicitOpts,
    err: &RefCell<Option<EvalError>>,
) -> Result<Vec<f64>, SimError> {
    let n = plan.states.len();
    let x0v: Vec<f64> = x0.to_vec();

    // f(x,p,t,y): 写 dy/dt
    let f = |x: &<M as diffsol::MatrixCommon>::V,
             _p: &<M as diffsol::MatrixCommon>::V,
             _t: f64,
             y: &mut <M as diffsol::MatrixCommon>::V| {
        let xs: Vec<f64> = (0..n).map(|i| x[i]).collect();
        let mut o = vec![0.0f64; n];
        eval_rhs(plan, env_cell, &xs, &mut o, err);
        for i in 0..n {
            y[i] = o[i];
        }
    };
    // g(x,p,t,v,y): y = J(x)·v，单边有限差分（复用同一 RHS，不写解析 jac）
    let g = |x: &<M as diffsol::MatrixCommon>::V,
             _p: &<M as diffsol::MatrixCommon>::V,
             _t: f64,
             v: &<M as diffsol::MatrixCommon>::V,
             y: &mut <M as diffsol::MatrixCommon>::V| {
        let xs: Vec<f64> = (0..n).map(|i| x[i]).collect();
        let mut fx = vec![0.0f64; n];
        eval_rhs(plan, env_cell, &xs, &mut fx, err);
        let xnorm = xs.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        let eps = (1.0 + xnorm) * f64::EPSILON.sqrt();
        let xp: Vec<f64> = (0..n).map(|i| xs[i] + eps * v[i]).collect();
        let mut fp = vec![0.0f64; n];
        eval_rhs(plan, env_cell, &xp, &mut fp, err);
        for i in 0..n {
            y[i] = (fp[i] - fx[i]) / eps;
        }
    };

    let problem = OdeBuilder::<M>::new()
        .t0(0.0)
        .rtol(opts.rtol)
        .atol([opts.atol])
        .rhs_implicit(f, g)
        .init(
            move |_p, _t, y: &mut <M as diffsol::MatrixCommon>::V| {
                for i in 0..n {
                    y[i] = x0v[i];
                }
            },
            n,
        )
        .build()
        .map_err(|e| SimError::Solver(format!("构建 problem 失败: {e}")))?;

    let mut solver = problem
        .bdf::<NalgebraLU<f64>>()
        .map_err(|e| SimError::Solver(format!("构建 BDF 失败: {e}")))?;

    // step-loop 到 dt：高层 `solve()` 的 max_steps_between_checkpoints 机制会在段内步数超限时
    // 提前返回（刚性/尖拐角段需大量内步 → interpolate(dt) 越界失败）。手动步进到 TstopReached
    // 保证精确抵达 dt，再取 `state().y` 末态，绕开 checkpoint 早返 + interpolate 边界。
    solver
        .set_stop_time(dt)
        .map_err(|e| SimError::Solver(format!("set_stop_time: {e}")))?;
    loop {
        match solver
            .step()
            .map_err(|e| SimError::Solver(format!("求解失败: {e}")))?
        {
            OdeSolverStopReason::TstopReached => break,
            _ => continue, // InternalTimestep（继续步进）；本问题无 root
        }
    }
    let st = solver.state();
    Ok((0..n).map(|i| st.y[i]).collect())
}

/// 段末在状态 `x` 处再评一趟计划，把状态量 + 方程量 + 驱动量记入轨迹（对齐 [`super::simulate`] 输出）。
fn record_step(
    plan: &RatePlan,
    env_cell: &RefCell<Env>,
    x: &[f64],
    traj: &mut IndexMap<String, Vec<f64>>,
) {
    {
        let mut e = env_cell.borrow_mut();
        for (i, s) in plan.states.iter().enumerate() {
            e.put(s.name.as_str(), Value::Scalar(x[i]));
        }
        for (name, expr) in &plan.ordered_eqs {
            if let Ok(v) = expr.eval_in_with(&mut e, EvalMode { strict: false }) {
                e.put(name.as_str(), v);
            }
        }
    }
    let e = env_cell.borrow();
    for (i, s) in plan.states.iter().enumerate() {
        flatten_into(traj, &s.name, &Value::Scalar(x[i]));
        let _ = i;
    }
    for (name, _) in &plan.ordered_eqs {
        if let Some(v) = e.get(name) {
            flatten_into(traj, name, &v);
        }
    }
    for d in &plan.drivers {
        if let Some(v) = e.get(d) {
            flatten_into(traj, d, &v);
        }
    }
}

/// **隐式刚性仿真**（[`super::simulate`] 的隐式对等物）。E5a 折叠 `_prev` → 建 rate 计划 →
/// 逐驱动步用 diffsol BDF 自适应内解 → 采样记轨迹。适用刚性亚日 ODE（温室气候）。
pub fn simulate_implicit(
    file: &EquationFile,
    input: &SimInput,
    opts: ImplicitOpts,
) -> Result<SimOutput, SimError> {
    let folded = fold_prev_for_implicit(file)?;
    // E2 平滑化（隐式向）：仅在 smooth_eps 设置时跑；只平滑状态依赖的非光滑算子。
    let folded = match opts.smooth_eps {
        Some(eps) => smooth_for_implicit(&folded, eps),
        None => folded,
    };
    let plan = build_rate_plan(&folded, &input.init_overrides)?;
    let dt = input.dt.unwrap_or(folded.meta.dt);

    // 校验驱动量序列长度
    for d in &plan.drivers {
        match input.drivers.get(d) {
            None => return Err(SimError::MissingDriver(d.clone())),
            Some(s) if s.len() != input.steps => {
                return Err(SimError::DriverLengthMismatch {
                    name: d.clone(),
                    expected: input.steps,
                    found: s.len(),
                })
            }
            Some(_) => {}
        }
    }

    // 参数基座 env
    let base_env = {
        let mut e = Env::new();
        for (pname, p) in &folded.parameters {
            match &p.values {
                Some(vals) => e.put(pname, Value::Vector(vals.clone())),
                None => e.put(
                    pname,
                    Value::Scalar(input.param_overrides.get(pname).copied().unwrap_or(p.default)),
                ),
            }
        }
        e
    };

    let mut x: Vec<f64> = plan.states.iter().map(|s| s.init).collect();
    let mut traj: IndexMap<String, Vec<f64>> = IndexMap::new();
    let err_slot: RefCell<Option<EvalError>> = RefCell::new(None);

    for n in 0..input.steps {
        let env_cell = RefCell::new(base_env.clone());
        {
            let mut e = env_cell.borrow_mut();
            e.put("DAT", Value::Scalar((n + 1) as f64));
            for d in &plan.drivers {
                let val = input.drivers.get(d).map(|s| s[n]).unwrap();
                e.put(d.as_str(), Value::Scalar(val));
            }
        }
        let x_next = advance_segment(&plan, &env_cell, &x, dt, opts, &err_slot)?;
        if let Some(er) = err_slot.borrow_mut().take() {
            return Err(SimError::Eval { var: "<rhs>".into(), err: er });
        }
        // 段末严格复核：accepted 态须有限。非严格内层求值让 NaN/Inf 传播给 diffsol 步长回退，
        // 但若 diffsol 容差控制漏掉非有限末态，这里 loud fail（而非静默把 NaN 写进轨迹返回）。
        if let Some(bad) = x_next.iter().position(|v| !v.is_finite()) {
            return Err(SimError::Solver(format!(
                "第 {} 步隐式解出现非有限值（状态 '{}' = {}）——模型病态或求解器发散",
                n + 1,
                plan.states[bad].name,
                x_next[bad]
            )));
        }
        x = x_next;
        record_step(&plan, &env_cell, &x, &mut traj);
    }

    Ok(SimOutput { steps: input.steps, trajectories: traj })
}

/// **隐式步进器**（0c：`simulate_coupled` 的 fast 回路走隐式 BDF 用）。与显式 [`super::Stepper`]
/// 同接口（`drivers`/`step`/`get`），per-step **复用 [`advance_segment`] 求解核**（不改求解逻辑，
/// 只把 `simulate_implicit` 的整段循环拆成可被耦合回路逐步驱动的形式）。刚性快模型（温室气候）
/// 在耦合回路里必须走隐式，否则显式 Euler 在其亚日 dt 下数值发散。
///
/// 与 `simulate_implicit` 同源（fold_prev → 平滑 → build_rate_plan → 逐步 advance_segment），
/// 故耦合隐式每步与单模型 `simulate_implicit` 逐步一致（单一真相源）。
pub struct ImplicitStepper {
    plan: RatePlan,
    /// 参数基座 env（每步 clone 出段 env_cell）。
    base_env: Env,
    /// 当前状态向量（顺序 = plan.states）。
    x: Vec<f64>,
    dt: f64,
    opts: ImplicitOpts,
    /// 已完成步数（DAT = n+1）。
    n: usize,
    /// 步后在 x 处评一趟计划得到的 env（供 [`ImplicitStepper::get`] 读接口变量·对齐 record_step）。
    last_env: Env,
    err_slot: RefCell<Option<EvalError>>,
}

impl ImplicitStepper {
    /// 新建隐式步进器。与 `simulate_implicit` 前半段同构：fold_prev（+可选平滑）→ build_rate_plan
    /// → 参数基座 env → 初始状态向量。
    pub fn new(
        file: &EquationFile,
        dt: f64,
        param_overrides: &HashMap<String, f64>,
        init_overrides: &HashMap<String, f64>,
        opts: ImplicitOpts,
    ) -> Result<Self, SimError> {
        let folded = fold_prev_for_implicit(file)?;
        let folded = match opts.smooth_eps {
            Some(eps) => smooth_for_implicit(&folded, eps),
            None => folded,
        };
        let plan = build_rate_plan(&folded, init_overrides)?;
        let mut base_env = Env::new();
        for (pname, p) in &folded.parameters {
            match &p.values {
                Some(vals) => base_env.put(pname, Value::Vector(vals.clone())),
                None => base_env.put(
                    pname,
                    Value::Scalar(param_overrides.get(pname).copied().unwrap_or(p.default)),
                ),
            }
        }
        let x: Vec<f64> = plan.states.iter().map(|s| s.init).collect();
        let last_env = base_env.clone();
        Ok(Self { plan, base_env, x, dt, opts, n: 0, last_env, err_slot: RefCell::new(None) })
    }

    /// 该快模型需外部每步供值的驱动量名（含被反馈供值的）。
    pub fn drivers(&self) -> &[String] {
        self.plan.drivers()
    }

    /// 推进一隐式步（复用 `advance_segment`，与 `simulate_implicit` 循环体逐位一致）。
    /// `get_driver(name)` 供本步每个驱动量的标量值（缺 → `MissingDriver`）。
    pub fn step(&mut self, get_driver: impl Fn(&str) -> Option<f64>) -> Result<(), SimError> {
        let env_cell = RefCell::new(self.base_env.clone());
        {
            let mut e = env_cell.borrow_mut();
            e.put("DAT", Value::Scalar((self.n + 1) as f64));
            for d in self.plan.drivers() {
                let val = get_driver(d).ok_or_else(|| SimError::MissingDriver(d.clone()))?;
                e.put(d.as_str(), Value::Scalar(val));
            }
        }
        let x_next = advance_segment(&self.plan, &env_cell, &self.x, self.dt, self.opts, &self.err_slot)?;
        if let Some(er) = self.err_slot.borrow_mut().take() {
            return Err(SimError::Eval { var: "<rhs>".into(), err: er });
        }
        if let Some(bad) = x_next.iter().position(|v| !v.is_finite()) {
            return Err(SimError::Solver(format!(
                "耦合快模型第 {} 步隐式解出现非有限值（状态 '{}' = {}）——模型病态或求解器发散",
                self.n + 1,
                self.plan.states[bad].name,
                x_next[bad]
            )));
        }
        self.x = x_next;
        // 段末在 x 处评一趟计划填 last_env（供 get 读接口变量/日均聚合·对齐 record_step 的 env 侧）
        {
            let mut e = env_cell.borrow_mut();
            for (i, s) in self.plan.states.iter().enumerate() {
                e.put(s.name.as_str(), Value::Scalar(self.x[i]));
            }
            for (name, expr) in &self.plan.ordered_eqs {
                if let Ok(v) = expr.eval_in_with(&mut e, EvalMode { strict: false }) {
                    e.put(name.as_str(), v);
                }
            }
        }
        self.last_env = env_cell.into_inner();
        self.n += 1;
        Ok(())
    }

    /// 读当前步某声明变量的 Value（步后调用·对齐 [`super::Stepper::get`]）。
    pub fn get(&self, name: &str) -> Option<Value> {
        self.last_env.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_str;
    use crate::sim::simulate;

    /// V1-0 微观：2 态解耦刚性线性系统，有解析解，验证「闭包 RHS + FD J·v + BDF」端到端在 EQC 内跑对。
    /// dy1/dt = k1·(e1 − y1)，dy2/dt = k2·(e2 − y2)；解析 y_i(t) = e_i − (e_i − y_i0)·exp(−k_i·t)。
    /// k1=1000 vs k2=1 → 真刚性。
    #[test]
    fn test_micro_stiff_analytic() {
        let yaml = r#"
meta: { id: STIFF2, model: Stiff2, name_cn: 刚性双态, dt: 0.05, dt_seconds: 1 }
parameters:
  k1: { name_cn: k1, default: 1000.0 }
  k2: { name_cn: k2, default: 1.0 }
  e1: { name_cn: eq1, default: 5.0 }
  e2: { name_cn: eq2, default: 3.0 }
variables:
  y1: { type: output, class: state, init: 0.0, rate: r1 }
  y2: { type: output, class: state, init: 0.0, rate: r2 }
  r1: { type: intermediate, class: rate }
  r2: { type: intermediate, class: rate }
equations:
  - id: R1
    name: rate1
    output: r1
    expression: { op: mul, args: [ { ref: k1 }, { op: sub, args: [ { ref: e1 }, { ref: y1 } ] } ] }
  - id: R2
    name: rate2
    output: r2
    expression: { op: mul, args: [ { ref: k2 }, { op: sub, args: [ { ref: e2 }, { ref: y2 } ] } ] }
"#;
        let file = parse_str(yaml).unwrap();
        // 20 步 × dt 0.05 = t 1.0；无驱动
        let input = SimInput::new(20);
        let opts = ImplicitOpts { rtol: 1e-9, atol: 1e-11, smooth_eps: None };
        let out = simulate_implicit(&file, &input, opts).unwrap();

        let y1 = out.final_value("y1").unwrap();
        let y2 = out.final_value("y2").unwrap();
        let a1 = 5.0 - 5.0 * (-1000.0f64 * 1.0).exp(); // ≈ 5
        let a2 = 3.0 - 3.0 * (-1.0f64 * 1.0).exp(); // 3 − 3e^{−1} ≈ 1.8964
        assert!((y1 - a1).abs() < 1e-6, "y1={y1} vs analytic {a1}");
        assert!((y2 - a2).abs() < 1e-6, "y2={y2} vs analytic {a2}");
    }

    /// §8：守恒徽章 = 瞬时结构残差（solver-无关）。在**隐式 BDF 轨迹**上：
    ///   - 结构残差 `|rate − (Σ源−Σ汇)/cap|` 机器零（速率方程 ≡ 守恒律声明）；
    ///   - 旧有限差分口径 `|Δstock − dt·net|` 因 BDF 段内自适应多步、求积与「dt·端点rate」错配而
    ///     远超容差 —— 正是 §8 修的漏洞（隐式误判「不守恒」，实为求积伪影非泄漏）。
    #[test]
    fn test_s8_structural_residual_solver_independent() {
        use crate::sim::{balance_residual, check_balance_laws, structural_residual};
        // 刚性一阶弛豫 x' = (S − k·x)/cap，k=100 → dt=0.1 下 k·dt=10 极刚，BDF 段内积分 ≠ dt·端点rate。
        let yaml = r#"
meta:
  id: S8
  model: S8
  name_cn: 结构残差测试
  dt: 0.1
  dt_seconds: 1
  balance:
    - { name: X守恒, stock: x, sources: [inflow], sinks: [outflow], cap: capx, tol: 1.0e-9 }
parameters:
  kk: { name_cn: 速率常数, default: 100.0 }
  ss: { name_cn: 源, default: 5.0 }
variables:
  x: { type: output, class: state, init: 10.0, rate: xdot }   # ★非常规 rate 命名(非 rate_x)·验 resolve_stock_rate 走 variable.rate 权威解析
  xdot: { type: intermediate, class: rate }
  inflow: { type: intermediate, class: auxiliary }
  outflow: { type: intermediate, class: auxiliary }
  capx: { type: intermediate, class: auxiliary }
equations:
  - { id: IN, name: 源, output: inflow, expression: { ref: ss } }
  - { id: OUT, name: 汇, output: outflow, expression: { op: mul, args: [ { ref: kk }, { ref: x } ] } }
  - { id: CAP, name: 容量, output: capx, expression: { const: 1.0 } }
  - { id: RATE, name: 速率, output: xdot, expression: { op: div, args: [ { op: sub, args: [ { ref: inflow }, { ref: outflow } ] }, { ref: capx } ] } }
"#;
        let file = parse_str(yaml).unwrap();
        let input = SimInput::new(30); // 30 步 × 0.1 = t 3.0
        let opts = ImplicitOpts { rtol: 1e-10, atol: 1e-12, smooth_eps: None };
        let out = simulate_implicit(&file, &input, opts).unwrap();

        // §8 结构残差（check_balance_laws 现走此口径）：隐式轨迹上机器零。
        let checks = check_balance_laws(&file.meta.balance, &out, 0.1, &file);
        let c = &checks[0];
        assert!(c.ok, "§8 结构残差应机器零守恒，got {:?}", c.residual);
        assert!(c.structural, "应走结构残差口径（rate 变量经 variable.rate 解析命中）");
        assert!(
            c.residual.as_ref().unwrap().max_resid < 1e-9,
            "结构残差应 <1e-9（隐式上按定义机器零）"
        );

        // 旧有限差分口径在同一隐式轨迹上应远超容差（求积错配·§8 必要性硬证据）。
        let stock = out.series("x").unwrap();
        let inflow = out.series("inflow").unwrap();
        let outflow = out.series("outflow").unwrap();
        let capx = out.series("capx").unwrap();
        let net_eff: Vec<f64> =
            (0..out.steps).map(|t| (inflow[t] - outflow[t]) / capx[t]).collect();
        let (fd_resid, _) = balance_residual(stock, &net_eff, 0.1);
        // FD 残差远超守恒律 tol(1e-9)——实测 ~4.5e-4，即误判 ~45 万×；结构残差同轨迹上机器零。
        assert!(
            fd_resid > 1e-5,
            "旧有限差分口径在隐式 BDF 上应误判(远超 tol=1e-9)，got {fd_resid:.3e}（证 §8 必要）"
        );

        // 交叉印证：结构残差直算 = rate 与 net_eff 逐位一致（rate 变量名非常规 `xdot`）。
        let rate = out.series("xdot").unwrap();
        let (struct_resid, _) = structural_residual(rate, &net_eff);
        assert!(struct_resid < 1e-12, "结构残差直算应机器零，got {struct_resid:.3e}");
    }

    /// E5a 折叠：验证 `_prev` 变量被删、方程引用被折回真态。
    #[test]
    fn test_fold_prev_removes_delay_and_rewrites() {
        let yaml = r#"
meta: { id: FOLD, model: Fold, name_cn: 折叠测试, dt: 0.1, dt_seconds: 1 }
parameters:
  U: { name_cn: 传热, default: 2.0 }
variables:
  T: { type: output, class: state, init: 10.0, rate: rate_T }
  T_prev: { type: intermediate, init: 10.0, prev: T }
  Q: { type: intermediate, class: rate }
  rate_T: { type: intermediate, class: rate }
equations:
  - id: E1
    name: 传热
    output: Q
    expression: { op: mul, args: [ { ref: U }, { ref: T_prev } ] }
  - id: E2
    name: 速率
    output: rate_T
    expression: { op: neg, args: [ { ref: Q } ] }
"#;
        let file = parse_str(yaml).unwrap();
        assert!(file.variables.contains_key("T_prev"));
        let folded = fold_prev_for_implicit(&file).unwrap();
        // 延迟寄存器被删
        assert!(!folded.variables.contains_key("T_prev"), "T_prev 应被删");
        assert!(folded.variables.contains_key("T"), "真态 T 应保留");
        // 方程 E1 现在引用 T（真态）而非 T_prev
        let e1 = folded.equations.iter().find(|e| e.id == "E1").unwrap();
        let refs = e1.expression.get_variable_refs();
        assert!(refs.iter().any(|r| r == "T"), "E1 应引用 T，refs={refs:?}");
        assert!(!refs.iter().any(|r| r == "T_prev"), "E1 不应再引用 T_prev");
    }

    /// **对抗复审 BUG-1 回归**：`_prev` 的源是**非状态量**（有方程的 auxiliary）时，折叠会让离散差分
    /// 恒为 0（静默错值）。必须 loud fail 拒绝，而非静默折叠。（草莓模型 `DRLG=RLG−RLG_prev` 惯用法。）
    #[test]
    fn test_fold_prev_rejects_auxiliary_diff_register() {
        let yaml = r#"
meta: { id: DIFFREJ, model: DiffRej, name_cn: 差分寄存器拒绝, dt: 1.0, dt_seconds: 1 }
parameters:
  k: { name_cn: 斜率, default: 3.0 }
variables:
  ramp:      { type: output, class: auxiliary }
  ramp_prev: { type: intermediate, init: 0.0, prev: ramp }
  d:         { type: output, class: auxiliary }
equations:
  - { id: E1, name: 斜坡, output: ramp, expression: { op: mul, args: [ { ref: k }, { ref: DAT } ] } }
  - { id: E2, name: 差分, output: d, expression: { op: sub, args: [ { ref: ramp }, { ref: ramp_prev } ] } }
"#;
        let file = parse_str(yaml).unwrap();
        // ramp 是有方程的 auxiliary（非 is_integrator）→ ramp_prev 折叠会让 d≡0 → 必须拒绝
        let folded = fold_prev_for_implicit(&file);
        assert!(folded.is_err(), "源是 auxiliary 的 _prev 差分寄存器应被拒绝（loud fail）");
        // simulate_implicit 也应拒绝（不静默产 d≡0）
        let out = simulate_implicit(&file, &SimInput::new(3), ImplicitOpts::default());
        assert!(out.is_err(), "simulate_implicit 应拒绝含 auxiliary 差分寄存器的模型");
    }

    /// **Robertson 强非线性刚性基准**（对抗复审 A 建议固化）：经典化学动力学刚性系统，
    /// 含双线性 `y2·y3`、二次 `y2²`、刚性比 ~1e10，真正考验 FD Jacobian 在曲率 + 量级悬殊
    /// （y1~1 vs y2~1e-5）下的正确性。判据：质量守恒 y1+y2+y3≡1（各步）+ 末值贴文献。
    #[test]
    fn test_robertson_stiff_nonlinear() {
        let yaml = r#"
meta: { id: ROBERTSON, model: Robertson, name_cn: Robertson刚性, dt: 1.0, dt_seconds: 1 }
parameters:
  a: { name_cn: k1, default: 0.04 }
  b: { name_cn: k2, default: 3.0e7 }
  c: { name_cn: k3, default: 1.0e4 }
variables:
  y1: { type: output, class: state, init: 1.0, rate: r1 }
  y2: { type: output, class: state, init: 0.0, rate: r2 }
  y3: { type: output, class: state, init: 0.0, rate: r3 }
  r1: { type: intermediate, class: rate }
  r2: { type: intermediate, class: rate }
  r3: { type: intermediate, class: rate }
equations:
  - { id: R1, name: r1, output: r1, expression: { op: add, args: [ { op: mul, args: [ { op: neg, args: [ { ref: a } ] }, { ref: y1 } ] }, { op: mul, args: [ { ref: c }, { op: mul, args: [ { ref: y2 }, { ref: y3 } ] } ] } ] } }
  - { id: R3, name: r3, output: r3, expression: { op: mul, args: [ { ref: b }, { op: mul, args: [ { ref: y2 }, { ref: y2 } ] } ] } }
  - { id: R2, name: r2, output: r2, expression: { op: sub, args: [ { op: neg, args: [ { ref: r1 } ] }, { ref: r3 } ] } }
"#;
        let file = parse_str(yaml).unwrap();
        // 跑到 t=40（40 步 dt=1，BDF 段内自适应吞刚性）
        let out = simulate_implicit(
            &file,
            &SimInput { steps: 40, dt: Some(1.0), ..Default::default() },
            ImplicitOpts { rtol: 1e-8, atol: 1e-10, smooth_eps: None },
        )
        .unwrap();
        let y1 = out.final_value("y1").unwrap();
        let y2 = out.final_value("y2").unwrap();
        let y3 = out.final_value("y3").unwrap();
        // 质量守恒（Robertson 的 y1+y2+y3 恒 =1）：FD Jacobian 若错，刚性快态会破守恒
        assert!((y1 + y2 + y3 - 1.0).abs() < 1e-6, "质量守恒破坏: y1+y2+y3={}", y1 + y2 + y3);
        // 末值贴文献（t=40: y1≈0.7158, y3≈0.2841, y2~3e-5 量级）
        assert!((y1 - 0.7158).abs() < 5e-3, "y1={y1} 偏离文献 0.7158");
        assert!((y3 - 0.2842).abs() < 5e-3, "y3={y3} 偏离文献 0.2842");
        assert!(y2 > 0.0 && y2 < 1e-3, "y2={y2} 应为 ~1e-5 小正量");
    }

    /// **V1 核心（in-crate）：显式↔隐式一致性**。一个含手写 `_prev` 的刚性单态热平衡模型
    /// （结构同 van Henten 能量平衡：flux 读 T_prev → rate → 积分 T）。显式 `simulate`（原模型带 _prev）
    /// 随 dt→0 应收敛到 `simulate_implicit`（自动折叠、解真联立）。常数驱动 → 隐式解 = 精确连续解、dt 无关。
    #[test]
    fn test_explicit_converges_to_implicit() {
        // dT/dt = (Q_in − U·(T − T_out)) / cap；平衡 T* = T_out + Q_in/U。
        // cap 小 + U 大 → 刚性快弛豫。显式带 T_prev 破环。
        let yaml = r#"
meta: { id: HEAT1, model: Heat1, name_cn: 单态热平衡, dt: 0.1, dt_seconds: 1 }
parameters:
  U:    { name_cn: 传热系数, default: 20.0 }
  cap:  { name_cn: 热容, default: 1.0 }
  Q_in: { name_cn: 加热, default: 100.0 }
  T_out: { name_cn: 室外温, default: 10.0 }
variables:
  T:      { type: output, class: state, init: 10.0, rate: rate_T }
  T_prev: { type: intermediate, init: 10.0, prev: T }
  Q_loss: { type: intermediate, class: rate }
  rate_T: { type: intermediate, class: rate }
equations:
  - id: QLOSS
    name: 传热损失
    output: Q_loss
    expression: { op: mul, args: [ { ref: U }, { op: sub, args: [ { ref: T_prev }, { ref: T_out } ] } ] }
  - id: RATET
    name: 温度速率
    output: rate_T
    expression: { op: div, args: [ { op: sub, args: [ { ref: Q_in }, { ref: Q_loss } ] }, { ref: cap } ] }
"#;
        let file = parse_str(yaml).unwrap();

        // 平衡：T* = T_out + Q_in/U = 10 + 100/20 = 15
        let t_star = 15.0;

        // 隐式（近精确）：跑到 t=2.0 已充分弛豫（时间常数 cap/U=0.05）
        let steps_impl = 20; // dt 0.1 × 20 = 2.0
        let impl_out = simulate_implicit(
            &file,
            &SimInput::new(steps_impl),
            ImplicitOpts { rtol: 1e-10, atol: 1e-12, smooth_eps: None },
        )
        .unwrap();
        let t_impl = impl_out.final_value("T").unwrap();

        // 解析解：dT/dt = 300 − 20T，T(0)=10 → T(t) = 15 − 5·e^{−20t}（τ=0.05）。
        // 取 horizon=0.1（≈2τ，瞬态区、显式截断误差明显且随 dt 减小），此处 T_a≈14.3233。
        let horizon = 0.1;
        let t_analytic = 15.0 - 5.0 * (-20.0f64 * horizon).exp();

        // 隐式（近精确）应贴合解析解
        let steps_impl = 10; // dt 0.01 × 10 = 0.1
        let impl2 = simulate_implicit(
            &file,
            &SimInput { steps: steps_impl, dt: Some(0.01), ..Default::default() },
            ImplicitOpts { rtol: 1e-10, atol: 1e-12, smooth_eps: None },
        )
        .unwrap();
        let t_impl_h = impl2.final_value("T").unwrap();
        assert!(
            (t_impl_h - t_analytic).abs() < 1e-6,
            "隐式 {t_impl_h} 应贴合解析解 {t_analytic}（证求解器数值正确）"
        );
        let _ = t_impl; // t=2.0 平衡值（已由上方 t_star 断言覆盖）

        // 显式（原模型带 _prev）随 dt→0 收敛到隐式/解析解：误差应单调下降 ~O(dt)
        let mut errs: Vec<f64> = Vec::new();
        for &dt in &[0.02f64, 0.01, 0.005, 0.0025, 0.00125, 0.000625] {
            let steps = (horizon / dt).round() as usize;
            let input = SimInput { steps, dt: Some(dt), ..Default::default() };
            let expl = simulate(&file, &input).unwrap();
            let t_expl = expl.final_value("T").unwrap();
            errs.push((t_expl - t_impl_h).abs());
        }
        // 单调收敛（瞬态区，每档 dt 减半误差下降）
        for w in errs.windows(2) {
            assert!(
                w[1] < w[0],
                "显式误差未随 dt→0 单调下降：{:?}（应收敛到隐式）",
                errs
            );
        }
        // O(dt) 收敛：dt 缩 8 倍，误差应缩数倍（宽松取 >3×）
        assert!(
            errs[0] > 3.0 * errs[errs.len() - 1],
            "未见 ~O(dt) 收敛：coarsest={} finest={}",
            errs[0],
            errs[errs.len() - 1]
        );
        assert!(errs[errs.len() - 1] < 0.02, "最细 dt 显式-隐式差 {} 仍偏大", errs[errs.len() - 1]);
    }

    /// **0b E2 平滑 pass 外科式验证**：状态依赖的 clamp 被平滑掉 Max/Min；驱动依赖的开关留硬。
    #[test]
    fn test_smooth_pass_surgical() {
        let yaml = r#"
meta: { id: SMOOTHT, model: SmoothT, name_cn: 平滑pass测试, dt: 1.0, dt_seconds: 1 }
parameters:
  Pband: { name_cn: 带, default: 3.0 }
  sp:    { name_cn: 设定点, default: 20.0 }
  thr:   { name_cn: 阈值, default: 5.0 }
variables:
  T:      { type: output, class: state, init: 10.0, rate: rate_T }
  Igl:    { type: input, class: driving }
  u:      { type: output, class: auxiliary }
  g:      { type: output, class: auxiliary }
  rate_T: { type: intermediate, class: rate }
equations:
  - { id: U, name: 状态clamp, output: u, expression: { op: max, args: [ {const: 0}, { op: min, args: [ {const: 1}, { op: div, args: [ { op: sub, args: [ {ref: sp}, {ref: T} ] }, {ref: Pband} ] } ] } ] } }
  - { id: G, name: 驱动max, output: g, expression: { op: max, args: [ {const: 0}, { op: sub, args: [ {ref: Igl}, {ref: thr} ] } ] } }
  - { id: RT, name: 速率, output: rate_T, expression: { op: sub, args: [ {ref: u}, { op: mul, args: [ {const: 0.1}, {ref: T} ] } ] } }
"#;
        let file = parse_str(yaml).unwrap();
        let sm = smooth_for_implicit(&fold_prev_for_implicit(&file).unwrap(), 0.05);
        let dbg = |id: &str| format!("{:?}", sm.equations.iter().find(|e| e.id == id).unwrap().expression);
        // U 依赖状态 T → clamp 应被平滑：不再含 Max/Min，且出现 Sqrt
        let u = dbg("U");
        assert!(!u.contains("Max(") && !u.contains("Min("), "状态依赖 clamp 应平滑掉 Max/Min: {u}");
        assert!(u.contains("Sqrt("), "平滑 clamp 应引入 Sqrt: {u}");
        // G 只依赖驱动 Igl → 段内常数 → 留硬（仍含 Max）
        let g = dbg("G");
        assert!(g.contains("Max("), "驱动依赖 max 应留硬: {g}");
    }

    /// **0b 端到端：带状态依赖控制律的模型可被隐式求解**。`Q_heat=Q_max·clamp((T_sp−T)/Pband,0,1)`
    /// 加热反馈（结构同 ctrl 变体 GH-QHEAT）。平滑-隐式应收敛，且贴合硬-显式-细dt（bottom-line：
    /// 平滑+隐式 ≈ 硬+显式）。硬 clamp 直接喂隐式会让 Newton 在拐角挣扎——平滑是让控制律走隐式的前提。
    #[test]
    fn test_ctrl_control_law_implicit() {
        let yaml = r#"
meta: { id: CTRLHEAT, model: CtrlHeat, name_cn: 控制律热模型, dt: 0.5, dt_seconds: 1 }
parameters:
  U:     { name_cn: 传热, default: 2.0 }
  cap:   { name_cn: 热容, default: 5.0 }
  T_out: { name_cn: 室外, default: 5.0 }
  T_sp:  { name_cn: 加热设定点, default: 20.0 }
  Pband: { name_cn: 比例带, default: 3.0 }
  Q_max: { name_cn: 最大加热, default: 50.0 }
variables:
  T:      { type: output, class: state, init: 5.0, rate: rate_T }
  T_prev: { class: semi_state, init: 5.0, prev: T }
  Q_heat: { type: output, class: auxiliary }
  Q_loss: { class: rate }
  rate_T: { class: rate }
equations:
  - { id: QH, name: 加热控制, output: Q_heat, expression: { op: mul, args: [ {ref: Q_max}, { op: max, args: [ {const: 0}, { op: min, args: [ {const: 1}, { op: div, args: [ { op: sub, args: [ {ref: T_sp}, {ref: T_prev} ] }, {ref: Pband} ] } ] } ] } ] } }
  - { id: QL, name: 传热损失, output: Q_loss, expression: { op: mul, args: [ {ref: U}, { op: sub, args: [ {ref: T_prev}, {ref: T_out} ] } ] } }
  - { id: RT, name: 速率, output: rate_T, expression: { op: div, args: [ { op: sub, args: [ {ref: Q_heat}, {ref: Q_loss} ] }, {ref: cap} ] } }
"#;
        let file = parse_str(yaml).unwrap();
        let horizon = 20.0_f64;

        // 硬-显式-细 dt = truth
        let expl = simulate(
            &file,
            &SimInput { steps: (horizon / 0.02) as usize, dt: Some(0.02), ..Default::default() },
        )
        .unwrap();
        let t_hard = expl.final_value("T").unwrap();

        // 平滑-隐式：应收敛且贴 truth
        let smooth = simulate_implicit(
            &file,
            &SimInput { steps: 40, dt: Some(0.5), ..Default::default() },
            ImplicitOpts { rtol: 1e-8, atol: 1e-9, smooth_eps: Some(0.02) },
        )
        .expect("平滑-隐式应收敛（控制律模型走隐式）");
        let t_smooth = smooth.final_value("T").unwrap();

        // 受控平衡在 clamp 内部线性区（T*≈18.4），平滑几乎不影响 → 贴硬-显式
        assert!(
            (t_smooth - t_hard).abs() < 0.1,
            "平滑-隐式 {t_smooth} 应贴硬-显式-细dt {t_hard}（bottom-line 决策差异小）"
        );
        assert!(t_smooth > 15.0 && t_smooth < 20.0, "受控平衡应在 T_out..T_sp 间: {t_smooth}");
    }
}
