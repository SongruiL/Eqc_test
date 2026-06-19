//! 差分进化（Differential Evolution, DE/rand/1/bin）——阶段 1 的优化器（见 spec §5）。
//!
//! 为何 DE：把模型当**黑盒**、只需能评估目标，对作物模型常见的**非光滑 / 阈值 / 分段 /
//! 多峰**鲁棒，不需要梯度。
//!
//! # 确定性（与项目「输出可复现」一致）
//!
//! DE 用随机数 → 这里**手搓一个确定性 PRNG（SplitMix64）**（不引入 `rand` 依赖）。
//! 同 `seed` + 同（确定性的）代价函数 → **逐位可复现**的结果。
//!
//! # 鲁棒
//!
//! 代价函数自己负责把「垃圾候选」（仿真发散/出错）映射成一个很大的有限值
//! （见 [`super::core::WORST_COST`]）；DE 只做比较与选择，天然把它们淘汰。

/// 确定性 PRNG（SplitMix64）。
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// [0, 1) 均匀，53 位尾数。
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// [0, n) 整数（n>0）。
    fn next_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
}

/// DE 配置。
#[derive(Debug, Clone)]
pub struct DeConfig {
    /// 种群规模（< 4 会被抬到 4，DE/rand/1 需要至少 4 个互异个体）。
    pub pop: usize,
    /// 迭代代数。
    pub iters: usize,
    /// 随机种子（定种子 → 可复现）。
    pub seed: u64,
    /// 差分权重 F（典型 0.5~0.9）。
    pub f: f64,
    /// 交叉概率 CR（典型 0.9）。
    pub cr: f64,
}

impl Default for DeConfig {
    fn default() -> Self {
        Self { pop: 30, iters: 100, seed: 42, f: 0.5, cr: 0.9 }
    }
}

/// DE 结果。
#[derive(Debug, Clone)]
pub struct DeResult {
    /// 最优旋钮向量。
    pub best_x: Vec<f64>,
    /// 最优代价（越小越好）。
    pub best_cost: f64,
    /// 每代的「至今最优代价」（收敛轨迹，单调非增）。
    pub history: Vec<f64>,
}

/// 把 `x` 钳进箱形边界。
fn clamp_to(x: &mut [f64], bounds: &[(f64, f64)]) {
    for (xi, &(lo, hi)) in x.iter_mut().zip(bounds) {
        if *xi < lo {
            *xi = lo;
        } else if *xi > hi {
            *xi = hi;
        }
    }
}

/// 在种群里取 3 个互异且都 ≠ `i` 的下标（NP ≥ 4 时必然存在）。
fn pick3(rng: &mut Rng, np: usize, i: usize) -> (usize, usize, usize) {
    let mut a = rng.next_usize(np);
    while a == i {
        a = rng.next_usize(np);
    }
    let mut b = rng.next_usize(np);
    while b == i || b == a {
        b = rng.next_usize(np);
    }
    let mut c = rng.next_usize(np);
    while c == i || c == a || c == b {
        c = rng.next_usize(np);
    }
    (a, b, c)
}

/// 差分进化最小化 `cost`，在箱形边界 `bounds` 内搜索。
///
/// - `bounds`：每维 `(lo, hi)`；维数 = 旋钮数。空 → 只评估一次空候选。
/// - `cost`：候选向量 → 越小越好的标量（应已把垃圾候选映射成大有限值，不要返回 NaN）。
pub fn differential_evolution<F>(bounds: &[(f64, f64)], cfg: &DeConfig, mut cost: F) -> DeResult
where
    F: FnMut(&[f64]) -> f64,
{
    let dim = bounds.len();
    if dim == 0 {
        let c = cost(&[]);
        return DeResult { best_x: Vec::new(), best_cost: c, history: vec![c] };
    }

    let np = cfg.pop.max(4);
    let mut rng = Rng::new(cfg.seed);

    // —— 初始化种群：每维在 [lo, hi] 均匀 ——
    let mut pop: Vec<Vec<f64>> = Vec::with_capacity(np);
    let mut fit: Vec<f64> = Vec::with_capacity(np);
    for _ in 0..np {
        let x: Vec<f64> = bounds
            .iter()
            .map(|&(lo, hi)| lo + (hi - lo) * rng.next_f64())
            .collect();
        let c = cost(&x);
        pop.push(x);
        fit.push(c);
    }

    // 当前最优
    let mut best_idx = argmin(&fit);
    let mut best_x = pop[best_idx].clone();
    let mut best_cost = fit[best_idx];
    let mut history = Vec::with_capacity(cfg.iters + 1);
    history.push(best_cost);

    // —— 迭代 ——
    for _gen in 0..cfg.iters {
        for i in 0..np {
            let (a, b, c) = pick3(&mut rng, np, i);
            // 变异：v = x_a + F·(x_b − x_c)
            let mut trial = vec![0.0; dim];
            let jrand = rng.next_usize(dim); // 保证至少一维来自变异体
            for j in 0..dim {
                let mutated = pop[a][j] + cfg.f * (pop[b][j] - pop[c][j]);
                // 二项交叉
                trial[j] = if rng.next_f64() < cfg.cr || j == jrand {
                    mutated
                } else {
                    pop[i][j]
                };
            }
            clamp_to(&mut trial, bounds);

            // 选择：试验向量不差于目标向量则取代
            let tc = cost(&trial);
            if tc <= fit[i] {
                pop[i] = trial;
                fit[i] = tc;
                if tc <= best_cost {
                    best_cost = tc;
                    best_x = pop[i].clone();
                    best_idx = i;
                }
            }
        }
        history.push(best_cost);
    }
    let _ = best_idx;

    DeResult { best_x, best_cost, history }
}

/// 最小值下标（空切片返回 0）。
fn argmin(v: &[f64]) -> usize {
    let mut idx = 0;
    let mut best = f64::INFINITY;
    for (i, &x) in v.iter().enumerate() {
        if x < best {
            best = x;
            idx = i;
        }
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sphere：f(x)=Σx² ，最小在原点。DE 应收敛到接近 0。
    #[test]
    fn test_sphere_converges() {
        let bounds = vec![(-5.0, 5.0); 3];
        let cfg = DeConfig { pop: 30, iters: 200, seed: 1, ..Default::default() };
        let res = differential_evolution(&bounds, &cfg, |x| x.iter().map(|v| v * v).sum());
        assert!(res.best_cost < 1e-6, "best_cost = {}", res.best_cost);
        for &xi in &res.best_x {
            assert!(xi.abs() < 1e-3, "x = {:?}", res.best_x);
        }
    }

    /// Rosenbrock 2D：最小 0 在 (1,1)。较难（窄谷），给足代数应接近。
    #[test]
    fn test_rosenbrock_converges() {
        let bounds = vec![(-2.0, 2.0); 2];
        let cfg = DeConfig { pop: 50, iters: 600, seed: 2, f: 0.7, cr: 0.9 };
        let res = differential_evolution(&bounds, &cfg, |x| {
            let (a, b) = (x[0], x[1]);
            (1.0 - a).powi(2) + 100.0 * (b - a * a).powi(2)
        });
        assert!(res.best_cost < 1e-3, "best_cost = {}", res.best_cost);
        assert!((res.best_x[0] - 1.0).abs() < 0.05, "x0 = {}", res.best_x[0]);
        assert!((res.best_x[1] - 1.0).abs() < 0.05, "x1 = {}", res.best_x[1]);
    }

    /// 同种子 → 逐位可复现。
    #[test]
    fn test_determinism_same_seed() {
        let bounds = vec![(-5.0, 5.0); 4];
        let cfg = DeConfig { pop: 20, iters: 50, seed: 123, ..Default::default() };
        let f = |x: &[f64]| x.iter().map(|v| (v - 1.5) * (v - 1.5)).sum::<f64>();
        let r1 = differential_evolution(&bounds, &cfg, f);
        let r2 = differential_evolution(&bounds, &cfg, f);
        assert_eq!(r1.best_cost, r2.best_cost);
        assert_eq!(r1.best_x, r2.best_x);
        assert_eq!(r1.history, r2.history);
    }

    /// 不同种子一般给不同搜索路径（健全性）。
    #[test]
    fn test_different_seed_differs() {
        let bounds = vec![(-5.0, 5.0); 3];
        let f = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>();
        let r1 = differential_evolution(&bounds, &DeConfig { seed: 1, iters: 5, ..Default::default() }, f);
        let r2 = differential_evolution(&bounds, &DeConfig { seed: 2, iters: 5, ..Default::default() }, f);
        // 早期（仅 5 代）两条路径几乎必然不同
        assert_ne!(r1.best_x, r2.best_x);
    }

    /// 结果落在边界内；history 单调非增。
    #[test]
    fn test_bounds_and_monotone_history() {
        let bounds = vec![(2.0, 4.0), (-1.0, 1.0)];
        let cfg = DeConfig { pop: 25, iters: 60, seed: 9, ..Default::default() };
        // 最小在 (2, 0)（被边界截断）：f = x0 + x1²
        let res = differential_evolution(&bounds, &cfg, |x| x[0] + x[1] * x[1]);
        assert!(res.best_x[0] >= 2.0 - 1e-9 && res.best_x[0] <= 4.0 + 1e-9);
        assert!(res.best_x[1] >= -1.0 - 1e-9 && res.best_x[1] <= 1.0 + 1e-9);
        assert!((res.best_x[0] - 2.0).abs() < 1e-3);
        assert!(res.best_x[1].abs() < 1e-2);
        for w in res.history.windows(2) {
            assert!(w[1] <= w[0] + 1e-12, "history 非单调: {:?}", res.history);
        }
    }

    /// 维数为 0：只评估一次空候选。
    #[test]
    fn test_zero_dim() {
        let res = differential_evolution(&[], &DeConfig::default(), |_x| 42.0);
        assert_eq!(res.best_cost, 42.0);
        assert!(res.best_x.is_empty());
    }
}
