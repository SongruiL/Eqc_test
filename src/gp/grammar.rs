//! GP 候选形式语法（grammar-guided）—— 每个 🟠 靶点配一套生物学/物理上合理的候选形式族。
//!
//! 设计（docs/spec-genetic-programming.md §5）：语法**按构造保证先验**——
//! `clamp(·,0,1)`/`expit` 保 [0,1] 有界，正系数（`pc`，初值 >0；乘法扰动保号）保单调。
//!
//! **基因组分两部分**（[`Candidate`]）：
//! - `expr`：**骨架**——结构常数（clamp 的 0/1、expit 的 1、pow 的 2）是固定 `Const`，
//!   **可调常数**是命名占位 `Param("__c{i}")`；
//! - `consts`：可调常数的当前值向量（= 进化/标定的"常数基因"；G2 扰动它、G4 memetic-DE 标定它）。
//!
//! 如此扰动只动可调常数、不碰结构常数 → 永不破坏 [0,1]/单调（结构常数 1、0 原样保留）。
//! 候选只引用 `GpContext.inputs` 内变量 + `__c*` 常数 → 受约束、不长野树。

use indexmap::IndexMap;

use crate::ast::Expr;
use crate::optimize::de::Rng;

/// 一个候选：骨架（含 `__c*` 可调常数占位）+ 可调常数值向量。
#[derive(Debug, Clone)]
pub struct Candidate {
    pub expr: Expr,
    pub consts: Vec<f64>,
}

impl Candidate {
    /// 可调常数个数。
    pub fn n_consts(&self) -> usize {
        self.consts.len()
    }
    /// 第 i 个可调常数的占位名（`__c{i}`）。
    pub fn const_name(i: usize) -> String {
        format!("__c{i}")
    }
}

/// 进化上下文：该靶点在范围的输入、输出有界先验、单调先验。
#[derive(Debug, Clone)]
pub struct GpContext {
    /// 候选式可用变量（来自 `gp_target.inputs`；为空时调用方应回填该方程当前 refs）。
    pub inputs: Vec<String>,
    /// 先验：输出有界 [lo, hi]（缺省=不约束有界，如生长曲线的质量）。
    pub output_bounds: Option<[f64; 2]>,
    /// 先验：对某变量单调（"increasing" / "decreasing"）。
    pub monotone: IndexMap<String, String>,
}

impl GpContext {
    fn driver(&self, dir: &str) -> String {
        self.monotone
            .iter()
            .find(|(_, d)| d.as_str() == dir)
            .map(|(v, _)| v.clone())
            .or_else(|| self.inputs.first().cloned())
            .unwrap_or_else(|| "x".to_string())
    }
    fn other(&self, not: &str) -> Option<String> {
        self.inputs.iter().find(|n| n.as_str() != not).cloned()
    }
}

/// 本版支持的语法名。
pub const KNOWN_GRAMMARS: &[&str] = &[
    "monotone_gate",
    "saturating_sink",
    "allocation_fraction",
    "temperature_response",
    "growth_curve",
];

// ---- 采样构造器：分配可调常数（→ Param 占位）、累计其值 ----
struct Builder<'a> {
    rng: &'a mut Rng,
    consts: Vec<f64>,
}
impl<'a> Builder<'a> {
    fn new(rng: &'a mut Rng) -> Self {
        Self { rng, consts: Vec::new() }
    }
    fn raw(&mut self, lo: f64, hi: f64) -> f64 {
        self.rng.next_range(lo, hi)
    }
    /// 登记一个可调常数（值 v）→ 返回其占位 `Param("__c{i}")`。
    fn konst(&mut self, v: f64) -> Expr {
        let i = self.consts.len();
        self.consts.push(v);
        Expr::param(Candidate::const_name(i))
    }
    /// 可调常数（[lo,hi) 采样）。
    fn ec(&mut self, lo: f64, hi: f64) -> Expr {
        let v = self.raw(lo, hi);
        self.konst(v)
    }
    /// 正可调常数（(0,hi]；乘法扰动保号 → 保单调）。
    fn pc(&mut self, hi: f64) -> Expr {
        let v = self.raw(1e-3, hi);
        self.konst(v)
    }
}

// ---- 结构常数/变量小工具 ----
fn c(v: f64) -> Expr {
    Expr::constant(v)
}
fn v(name: &str) -> Expr {
    Expr::var(name)
}
/// expit(z) = 1/(1+exp(−z))：恒在 (0,1)（两个 1 是**结构常数**，不可调）。
fn expit(z: Expr) -> Expr {
    Expr::div(c(1.0), Expr::add(c(1.0), Expr::exp(Expr::neg(z))))
}

/// 从命名语法采样一个合法候选（随机选形式）。未知语法 → None。
pub fn sample(grammar: &str, ctx: &GpContext, rng: &mut Rng) -> Option<Candidate> {
    let n = effective_form_count(grammar, ctx);
    if n == 0 {
        return None;
    }
    let form = rng.next_usize(n);
    sample_form(grammar, form, ctx, rng)
}

/// 采样指定形式（form idx）。骨架由 (grammar, form, ctx) 决定、与常数值无关
/// → 供 provenance 的形式识别（生成各形式的标准骨架来匹配）。
pub fn sample_form(grammar: &str, form: usize, ctx: &GpContext, rng: &mut Rng) -> Option<Candidate> {
    let n = effective_form_count(grammar, ctx);
    if n == 0 {
        return None;
    }
    let form = form.min(n - 1);
    let mut b = Builder::new(rng);
    let expr = match grammar {
        "monotone_gate" => monotone_gate(ctx, &mut b, form),
        "saturating_sink" => saturating_sink(ctx, &mut b, form),
        "allocation_fraction" => allocation_fraction(ctx, &mut b, form),
        "temperature_response" => temperature_response(ctx, &mut b, form),
        "growth_curve" => growth_curve(ctx, &mut b, form),
        _ => return None,
    };
    Some(Candidate { expr, consts: b.consts })
}

/// 某语法在给定 ctx 下的**有效**形式数（依赖输入个数：互作/比值形式需 2 输入）。
pub fn effective_form_count(grammar: &str, ctx: &GpContext) -> usize {
    let two = ctx.inputs.len() >= 2;
    match grammar {
        "monotone_gate" => if two { 3 } else { 2 },
        "saturating_sink" => 3,
        "allocation_fraction" => if two { 3 } else { 2 },
        "temperature_response" => 2,
        "growth_curve" => 3,
        _ => 0,
    }
}

/// 某语法的候选形式数（最大；供 G2 变异枚举/测试）。
pub fn form_count(grammar: &str) -> usize {
    match grammar {
        "monotone_gate" => 3,
        "saturating_sink" => 3,
        "allocation_fraction" => 3,
        "temperature_response" => 2,
        "growth_curve" => 3,
        _ => 0,
    }
}

/// 形式的人类可读名（供 provenance 报告"GP 选了哪种机理形式"）。
pub fn form_name(grammar: &str, form: usize) -> &'static str {
    match (grammar, form) {
        ("monotone_gate", 0) => "linear_ramp",
        ("monotone_gate", 1) => "sigmoid",
        ("monotone_gate", _) => "sigmoid_chill_heat_interaction",
        ("saturating_sink", 0) => "linear_saturation",
        ("saturating_sink", 1) => "sigmoid_decreasing",
        ("saturating_sink", _) => "hill_decreasing",
        ("allocation_fraction", 0) => "constant_fraction",
        ("allocation_fraction", 1) => "logistic_of_state",
        ("allocation_fraction", _) => "sink_strength_ratio",
        ("temperature_response", 0) => "trapezoid_cardinal",
        ("temperature_response", _) => "gaussian_peak",
        ("growth_curve", 0) => "single_logistic",
        ("growth_curve", 1) => "double_logistic",
        ("growth_curve", _) => "gompertz",
        _ => "unknown",
    }
}

// ============ 5 套通用语法 ============

fn monotone_gate(ctx: &GpContext, b: &mut Builder, form: usize) -> Expr {
    let d = ctx.driver("increasing");
    match form {
        0 => {
            // clamp((d − c0)/c1+, 0, 1)
            let inner = Expr::div(Expr::sub(v(&d), b.ec(0.0, 50.0)), b.pc(50.0));
            Expr::clamp(inner, c(0.0), c(1.0))
        }
        1 => expit(Expr::mul(b.pc(1.0), Expr::sub(v(&d), b.ec(0.0, 50.0)))),
        _ => {
            // expit(c1+·(d − c0 − c2+·h))：冷×热互作
            let h = ctx.other(&d).unwrap();
            let inter = Expr::sub(
                Expr::sub(v(&d), b.ec(0.0, 50.0)),
                Expr::mul(b.pc(0.5), v(&h)),
            );
            expit(Expr::mul(b.pc(1.0), inter))
        }
    }
}

fn saturating_sink(ctx: &GpContext, b: &mut Builder, form: usize) -> Expr {
    let x = ctx.driver("decreasing");
    let cap = ctx.other(&x);
    match form {
        0 => {
            // max(0, 1 − x/cap)；cap=第二输入 或 正常数
            let denom = cap.as_deref().map(v).unwrap_or_else(|| b.pc(10.0));
            Expr::max(vec![c(0.0), Expr::sub(c(1.0), Expr::div(v(&x), denom))])
        }
        1 => expit(Expr::mul(b.pc(1.0), Expr::sub(b.ec(0.0, 10.0), v(&x)))),
        _ => {
            // c0+/(c0+ + x)：对 x 单调降。注意两处须**同一** Param → 先登记再复用。
            let k = b.pc(10.0);
            Expr::div(k.clone(), Expr::add(k, v(&x)))
        }
    }
}

fn allocation_fraction(ctx: &GpContext, b: &mut Builder, form: usize) -> Expr {
    let s = ctx.inputs.first().cloned().unwrap_or_else(|| "x".into());
    match form {
        0 => Expr::clamp(b.ec(0.0, 1.0), c(0.0), c(1.0)),
        1 => expit(Expr::mul(b.pc(1.0), Expr::sub(v(&s), b.ec(0.0, 10.0)))),
        _ => {
            let y = ctx.other(&s).unwrap();
            Expr::div(v(&s), Expr::add(v(&s), v(&y)))
        }
    }
}

fn temperature_response(ctx: &GpContext, b: &mut Builder, form: usize) -> Expr {
    let t = ctx.inputs.first().cloned().unwrap_or_else(|| "T".into());
    match form {
        0 => {
            // 梯形：基点有序采样，登记为可调常数（含正的区间宽度，保证分母>0）
            let tb = b.raw(0.0, 8.0);
            let w1 = b.raw(8.0, 16.0);
            let w2 = b.raw(2.0, 8.0);
            let w3 = b.raw(5.0, 12.0);
            let to2 = tb + w1 + w2;
            let tm = to2 + w3;
            let up = Expr::div(Expr::sub(v(&t), b.konst(tb)), b.konst(w1));
            let down = Expr::div(Expr::sub(b.konst(tm), v(&t)), b.konst(w3));
            Expr::clamp(Expr::min(vec![up, c(1.0), down]), c(0.0), c(1.0))
        }
        _ => {
            // 高斯峰 exp(−c1+·(T − Topt)^2)（2 是结构常数）
            let topt = b.raw(18.0, 30.0);
            let sq = Expr::pow(Expr::sub(v(&t), b.konst(topt)), c(2.0));
            Expr::exp(Expr::neg(Expr::mul(b.pc(0.05), sq)))
        }
    }
}

fn growth_curve(ctx: &GpContext, b: &mut Builder, form: usize) -> Expr {
    let tau = ctx.driver("increasing");
    let wmax = ctx.other(&tau).map(|n| v(&n)).unwrap_or_else(|| b.pc(2.0));
    match form {
        0 => Expr::mul(
            wmax,
            expit(Expr::mul(b.pc(0.1), Expr::sub(v(&tau), b.ec(0.0, 800.0)))),
        ),
        1 => {
            // Wmax·(w·S1 + (1−w)·S2)：双 S。**权重 w=expit(wc)∈(0,1)**——这样无论 wc 怎么扰动，
            // w 与 (1−w) 恒为正 → 两 logistic 正权之和仍单调升（避免裸 w 被扰动到 >1 使 1−w 变负）。
            let wc = b.ec(-2.0, 2.0);
            let w = expit(wc);
            let s1 = expit(Expr::mul(b.pc(0.1), Expr::sub(v(&tau), b.ec(0.0, 400.0))));
            let s2 = expit(Expr::mul(b.pc(0.1), Expr::sub(v(&tau), b.ec(400.0, 1000.0))));
            let blend = Expr::add(
                Expr::mul(w.clone(), s1),
                Expr::mul(Expr::sub(c(1.0), w), s2),
            );
            Expr::mul(wmax, blend)
        }
        _ => {
            // Wmax·exp(−c0+·exp(−c1+·τ))：Gompertz
            let inner = Expr::exp(Expr::neg(Expr::mul(b.pc(0.02), v(&tau))));
            Expr::mul(wmax, Expr::exp(Expr::neg(Expr::mul(b.pc(5.0), inner))))
        }
    }
}
