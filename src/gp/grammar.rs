//! GP 候选形式语法（grammar-guided）—— 每个 🟠 靶点配一套生物学/物理上合理的候选形式族。
//!
//! 设计（docs/spec-genetic-programming.md §5）：语法**按构造保证先验**——
//! `clamp(·,0,1)`/`expit` 保 [0,1] 有界，正系数（`pc`，强制 >0）保单调。
//! 采样产出**已填常数的具体 `Expr`**（= 基因组）；G2 变异/G3 标定再动常数。
//! 所有候选只引用 `GpContext.inputs` 内的变量 + 临时常数 → 受约束、不长野树。

use indexmap::IndexMap;

use crate::ast::Expr;
use crate::optimize::de::Rng;

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
    /// 取单调方向为 `dir` 的首个变量；无则回退 `inputs[0]`。
    fn driver(&self, dir: &str) -> String {
        self.monotone
            .iter()
            .find(|(_, d)| d.as_str() == dir)
            .map(|(v, _)| v.clone())
            .or_else(|| self.inputs.first().cloned())
            .unwrap_or_else(|| "x".to_string())
    }
    /// 取一个不等于 `not` 的输入（用于第二变量，如互作项/容量/Wmax）；无则 None。
    fn other(&self, not: &str) -> Option<String> {
        self.inputs.iter().find(|n| n.as_str() != not).cloned()
    }
}

/// 本版支持的语法名。
pub const KNOWN_GRAMMARS: &[&str] = &[
    "monotone_gate",        // 0..1 门控，对驱动量单调升（如休眠解除）
    "saturating_sink",      // 0..1 汇饱和，对状态量单调降（如叶汇饱和）
    "allocation_fraction",  // 0..1 分配比
    "temperature_response", // 0..1 温度因子，单峰
    "growth_curve",         // ≥0 生长曲线，对热龄单调升（如双 S）
];

// ---- Expr 构造小工具 ----
fn c(v: f64) -> Expr {
    Expr::constant(v)
}
fn v(name: &str) -> Expr {
    Expr::var(name)
}
/// expit(z) = 1/(1+exp(−z))：恒在 (0,1)、对 z 单调升。
fn expit(z: Expr) -> Expr {
    Expr::div(c(1.0), Expr::add(c(1.0), Expr::exp(Expr::neg(z))))
}
/// 临时常数（[lo,hi) 采样）。
fn ec(rng: &mut Rng, lo: f64, hi: f64) -> Expr {
    c(rng.next_range(lo, hi))
}
/// 正临时常数（(0,hi]，保单调/斜率为正）。
fn pc(rng: &mut Rng, hi: f64) -> Expr {
    c(rng.next_range(1e-3, hi))
}

/// 从命名语法采样一个合法候选 `Expr`（已填常数）。未知语法 → None。
pub fn sample(grammar: &str, ctx: &GpContext, rng: &mut Rng) -> Option<Expr> {
    match grammar {
        "monotone_gate" => Some(monotone_gate(ctx, rng)),
        "saturating_sink" => Some(saturating_sink(ctx, rng)),
        "allocation_fraction" => Some(allocation_fraction(ctx, rng)),
        "temperature_response" => Some(temperature_response(ctx, rng)),
        "growth_curve" => Some(growth_curve(ctx, rng)),
        _ => None,
    }
}

/// 某语法的候选形式数（供 G2 变异枚举/测试）。
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

// ============ 5 套通用语法 ============

/// 门控：0..1、对驱动量 d 单调升。形式：线性 ramp / sigmoid / 冷×热互作 sigmoid。
fn monotone_gate(ctx: &GpContext, rng: &mut Rng) -> Expr {
    let d = ctx.driver("increasing");
    let n = if ctx.inputs.len() >= 2 { 3 } else { 2 };
    match rng.next_usize(n) {
        0 => {
            // clamp((d − c0)/c1+, 0, 1)
            let inner = Expr::div(Expr::sub(v(&d), ec(rng, 0.0, 50.0)), pc(rng, 50.0));
            Expr::clamp(inner, c(0.0), c(1.0))
        }
        1 => {
            // expit(c1+·(d − c0))
            expit(Expr::mul(pc(rng, 1.0), Expr::sub(v(&d), ec(rng, 0.0, 50.0))))
        }
        _ => {
            // expit(c1+·(d − c0 − c2+·h))：冷×热互作（综述明示存在、无函数）
            let h = ctx.other(&d).unwrap();
            let inter = Expr::sub(
                Expr::sub(v(&d), ec(rng, 0.0, 50.0)),
                Expr::mul(pc(rng, 0.5), v(&h)),
            );
            expit(Expr::mul(pc(rng, 1.0), inter))
        }
    }
}

/// 汇饱和：0..1、对状态量 x 单调降。形式：线性饱和 / sigmoid 降 / Hill 降。
fn saturating_sink(ctx: &GpContext, rng: &mut Rng) -> Expr {
    let x = ctx.driver("decreasing");
    let cap = ctx.other(&x); // 第二输入作容量（如 LAI_pot）
    match rng.next_usize(3) {
        0 => {
            // max(0, 1 − x/cap)；cap=第二输入 或 正常数
            let denom = cap.as_deref().map(v).unwrap_or_else(|| pc(rng, 10.0));
            Expr::max(vec![c(0.0), Expr::sub(c(1.0), Expr::div(v(&x), denom))])
        }
        1 => {
            // expit(c1+·(c0 − x))：对 x 单调降，(0,1)
            expit(Expr::mul(pc(rng, 1.0), Expr::sub(ec(rng, 0.0, 10.0), v(&x))))
        }
        _ => {
            // c0+/(c0+ + x)：对 x 单调降，(0,1]
            let k = pc(rng, 10.0);
            Expr::div(k.clone(), Expr::add(k, v(&x)))
        }
    }
}

/// 分配比：0..1。形式：常数比 / 状态 logistic / 两汇强之比。
fn allocation_fraction(ctx: &GpContext, rng: &mut Rng) -> Expr {
    let s = ctx.inputs.first().cloned().unwrap_or_else(|| "x".into());
    let n = if ctx.inputs.len() >= 2 { 3 } else { 2 };
    match rng.next_usize(n) {
        0 => Expr::clamp(ec(rng, 0.0, 1.0), c(0.0), c(1.0)), // 常数比
        1 => expit(Expr::mul(pc(rng, 1.0), Expr::sub(v(&s), ec(rng, 0.0, 10.0)))),
        _ => {
            // x/(x+y)：两汇强之比，(0,1)
            let y = ctx.other(&s).unwrap();
            Expr::div(v(&s), Expr::add(v(&s), v(&y)))
        }
    }
}

/// 温度响应：0..1、单峰。形式：梯形（基数温度）/ 高斯型峰。
fn temperature_response(ctx: &GpContext, rng: &mut Rng) -> Expr {
    let t = ctx.inputs.first().cloned().unwrap_or_else(|| "T".into());
    match rng.next_usize(2) {
        0 => {
            // 梯形 clamp(min((T−Tb)/(To1−Tb), 1, (Tm−T)/(Tm−To2)), 0, 1)；基点有序
            let tb = rng.next_range(0.0, 8.0);
            let to1 = tb + rng.next_range(8.0, 16.0);
            let to2 = to1 + rng.next_range(2.0, 8.0);
            let tm = to2 + rng.next_range(5.0, 12.0);
            let up = Expr::div(Expr::sub(v(&t), c(tb)), c(to1 - tb));
            let down = Expr::div(Expr::sub(c(tm), v(&t)), c(tm - to2));
            Expr::clamp(Expr::min(vec![up, c(1.0), down]), c(0.0), c(1.0))
        }
        _ => {
            // 高斯型峰 exp(−c1+·(T − Topt)^2)：单峰、(0,1]
            let topt = rng.next_range(18.0, 30.0);
            let sq = Expr::pow(Expr::sub(v(&t), c(topt)), c(2.0));
            Expr::exp(Expr::neg(Expr::mul(pc(rng, 0.05), sq)))
        }
    }
}

/// 生长曲线：≥0、对热龄 τ 单调升、由 Wmax 缩放。形式：单 logistic / 双 S / Gompertz。
fn growth_curve(ctx: &GpContext, rng: &mut Rng) -> Expr {
    let tau = ctx.driver("increasing");
    // Wmax = 第二输入（如 Wmax_fruit）或正常数
    let wmax = ctx.other(&tau).map(|n| v(&n)).unwrap_or_else(|| pc(rng, 2.0));
    match rng.next_usize(3) {
        0 => {
            // Wmax·expit(c1+·(τ − c0))
            Expr::mul(wmax, expit(Expr::mul(pc(rng, 0.1), Expr::sub(v(&tau), ec(rng, 0.0, 800.0)))))
        }
        1 => {
            // Wmax·(w·S1 + (1−w)·S2)：双 S（两 logistic 之和）
            let w = rng.next_range(0.1, 0.9);
            let s1 = expit(Expr::mul(pc(rng, 0.1), Expr::sub(v(&tau), ec(rng, 0.0, 400.0))));
            let s2 = expit(Expr::mul(pc(rng, 0.1), Expr::sub(v(&tau), ec(rng, 400.0, 1000.0))));
            let blend = Expr::add(Expr::mul(c(w), s1), Expr::mul(c(1.0 - w), s2));
            Expr::mul(wmax, blend)
        }
        _ => {
            // Wmax·exp(−c0+·exp(−c1+·τ))：Gompertz，对 τ 单调升、[0,Wmax]
            let inner = Expr::exp(Expr::neg(Expr::mul(pc(rng, 0.02), v(&tau))));
            Expr::mul(wmax, Expr::exp(Expr::neg(Expr::mul(pc(rng, 5.0), inner))))
        }
    }
}
