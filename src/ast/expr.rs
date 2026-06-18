//! 表达式 AST 节点定义（强类型版本）
//!
//! 基于规范文档 3.3 节设计，每个运算符都是独立的枚举变体。
//! 运算符命名遵循 OpenMath CD 规范。

use serde::{Deserialize, Deserializer, Serialize};

// ============================================
// 核心 AST 枚举定义
// ============================================

/// 向量归约种类（[`Expr::Reduce`]）：对一个向量的全部元素归约成一个标量。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReduceKind {
    Sum,
    Prod,
    Mean,
    Min,
    Max,
}

impl ReduceKind {
    /// 规范名（求值错误标签 / 代码生成 / 解析用）。
    pub fn name(&self) -> &'static str {
        match self {
            ReduceKind::Sum => "vsum",
            ReduceKind::Prod => "vprod",
            ReduceKind::Mean => "vmean",
            ReduceKind::Min => "vmin",
            ReduceKind::Max => "vmax",
        }
    }
}

/// 表达式 AST 节点（强类型）
///
/// 遵循规范文档 3.3 节，每个运算符都是独立的变体。
/// 这提供了编译时类型检查，避免字符串匹配的运行时错误。
///
/// 注意：`Deserialize` 不使用默认 derive（默认会期望外部标签格式 `!Add`），
/// 而是手写实现委托给 [`Expr::from_yaml_value`]，以支持文档/示例使用的
/// `{op: add, args: [...]}` map 格式。见本文件底部的 `impl Deserialize for Expr`。
#[derive(Debug, Clone, Serialize)]
pub enum Expr {
    // === 叶子节点 ===
    /// 常量值
    Const(f64),

    /// 变量引用（来自 variables 定义）
    Var(String),

    /// 参数引用（来自 parameters 定义，以 p 开头如 p1, p2）
    Param(String),

    // === 数学常量 ===
    /// 圆周率 π
    Pi,

    /// 自然常数 e
    E,

    // === 算术运算（arith1）===
    /// 加法: a + b
    Add(Box<Expr>, Box<Expr>),

    /// 减法: a - b
    Sub(Box<Expr>, Box<Expr>),

    /// 乘法: a × b
    Mul(Box<Expr>, Box<Expr>),

    /// 除法: a / b
    Div(Box<Expr>, Box<Expr>),

    /// 取负: -a
    Neg(Box<Expr>),

    /// 幂运算: a^b
    Pow(Box<Expr>, Box<Expr>),

    /// 绝对值: |a|
    Abs(Box<Expr>),

    /// 取余: a mod b
    Mod(Box<Expr>, Box<Expr>),

    /// 向上取整: ⌈x⌉
    Ceil(Box<Expr>),

    /// 向下取整: ⌊x⌋
    Floor(Box<Expr>),

    /// 四舍五入: round(x)
    Round(Box<Expr>),

    /// 截断取整: trunc(x)
    Trunc(Box<Expr>),

    /// 符号函数: sign(x) → -1, 0, 1
    Sign(Box<Expr>),

    // === 超越函数（transc1）===
    /// 指数: e^x
    Exp(Box<Expr>),

    /// 自然对数: ln(x)
    Ln(Box<Expr>),

    /// 常用对数: log10(x)
    Log10(Box<Expr>),

    /// 以2为底对数: log2(x)
    Log2(Box<Expr>),

    /// 平方根: √x
    Sqrt(Box<Expr>),

    /// 立方根: ∛x
    Cbrt(Box<Expr>),

    // === 三角函数（transc1）===
    /// 正弦: sin(x)
    Sin(Box<Expr>),

    /// 余弦: cos(x)
    Cos(Box<Expr>),

    /// 正切: tan(x)
    Tan(Box<Expr>),

    /// 反正弦: arcsin(x)
    ASin(Box<Expr>),

    /// 反余弦: arccos(x)
    ACos(Box<Expr>),

    /// 反正切: arctan(x)
    ATan(Box<Expr>),

    /// 二参数反正切: atan2(y, x)
    ATan2(Box<Expr>, Box<Expr>),

    // === 双曲函数 ===
    /// 双曲正弦: sinh(x)
    Sinh(Box<Expr>),

    /// 双曲余弦: cosh(x)
    Cosh(Box<Expr>),

    /// 双曲正切: tanh(x)
    Tanh(Box<Expr>),

    /// 反双曲正弦: asinh(x)
    ASinh(Box<Expr>),

    /// 反双曲余弦: acosh(x)
    ACosh(Box<Expr>),

    /// 反双曲正切: atanh(x)
    ATanh(Box<Expr>),

    // === 聚合函数（fns1）===
    /// 最大值: max(a, b, ...)
    Max(Vec<Expr>),

    /// 最小值: min(a, b, ...)
    Min(Vec<Expr>),

    /// 求和: Σ_{i=lower}^{upper} body
    Sum {
        index: String,
        lower: Box<Expr>,
        upper: Box<Expr>,
        body: Box<Expr>,
    },

    /// 连乘: Π_{i=lower}^{upper} body
    Product {
        index: String,
        lower: Box<Expr>,
        upper: Box<Expr>,
        body: Box<Expr>,
    },

    // === 关系运算（relation1）===
    /// 等于: a == b
    Eq(Box<Expr>, Box<Expr>),

    /// 小于: a < b
    Lt(Box<Expr>, Box<Expr>),

    /// 大于: a > b
    Gt(Box<Expr>, Box<Expr>),

    /// 小于等于: a <= b
    Leq(Box<Expr>, Box<Expr>),

    /// 大于等于: a >= b
    Geq(Box<Expr>, Box<Expr>),

    /// 不等于: a != b
    Neq(Box<Expr>, Box<Expr>),

    // === 逻辑运算（logic1）===
    /// 逻辑与: a && b
    And(Box<Expr>, Box<Expr>),

    /// 逻辑或: a || b
    Or(Box<Expr>, Box<Expr>),

    /// 逻辑非: !a
    Not(Box<Expr>),

    // === 条件表达式 ===
    /// 三元条件: if cond then a else b
    IfThenElse {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },

    /// 分段函数: piecewise
    Piecewise {
        /// (条件, 值) 对列表
        pieces: Vec<(Expr, Expr)>,
        /// 默认值（otherwise）
        otherwise: Box<Expr>,
    },

    // === 扩展分位数函数（需要 advanced_math feature）===
    /// 指数分布分位数: Exp^{-1}(p; λ)
    ExpPpf(Box<Expr>, Box<Expr>),

    /// 伽马分布分位数: Γ^{-1}(p; α, β)
    GammaPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 贝塔分布分位数: B^{-1}(p; α, β)
    BetaPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 威布尔分布分位数: W^{-1}(p; k, λ)
    WeibullPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 对数正态分布分位数: LN^{-1}(p; μ, σ)
    LognormPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 均匀分布分位数: U^{-1}(p; a, b)
    UniformPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 柯西分布分位数: C^{-1}(p; x₀, γ)
    CauchyPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 复数运算扩展（需要 advanced_math feature）===
    /// 复数双曲正弦: sinh(z)
    ComplexSinh(Box<Expr>),

    /// 复数双曲余弦: cosh(z)
    ComplexCosh(Box<Expr>),

    /// 复数双曲正切: tanh(z)
    ComplexTanh(Box<Expr>),

    /// 复数反双曲正弦: asinh(z)
    ComplexAsinh(Box<Expr>),

    /// 复数反双曲余弦: acosh(z)
    ComplexAcosh(Box<Expr>),

    /// 复数反双曲正切: atanh(z)
    ComplexAtanh(Box<Expr>),

    /// 复数反正弦: asin(z)
    ComplexAsin(Box<Expr>),

    /// 复数反余弦: acos(z)
    ComplexAcos(Box<Expr>),

    /// 复数反正切: atan(z)
    ComplexAtan(Box<Expr>),

    // === 数论函数 ===
    /// 最大公约数: gcd(a, b)
    Gcd(Box<Expr>, Box<Expr>),

    /// 最小公倍数: lcm(a, b)
    Lcm(Box<Expr>, Box<Expr>),

    /// 排列数: P(n, k) = n! / (n-k)!
    Permutation(Box<Expr>, Box<Expr>),

    // === 正交多项式（需要 gsl_math feature）===
    /// 勒让德多项式: P_n(x)
    Legendre(Box<Expr>, Box<Expr>),

    /// 关联勒让德多项式: P_l^m(x)
    LegendreAssoc(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 厄米多项式: H_n(x)
    Hermite(Box<Expr>, Box<Expr>),

    /// 拉盖尔多项式: L_n(x)
    Laguerre(Box<Expr>, Box<Expr>),

    /// 关联拉盖尔多项式: L_n^a(x)
    LaguerreAssoc(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 第一类切比雪夫多项式: T_n(x)
    ChebyshevT(Box<Expr>, Box<Expr>),

    /// 第二类切比雪夫多项式: U_n(x)
    ChebyshevU(Box<Expr>, Box<Expr>),

    // === 椭圆积分（需要 gsl_math feature）===
    /// 完全椭圆积分第一类: K(k)
    EllipK(Box<Expr>),

    /// 完全椭圆积分第二类: E(k)
    EllipE(Box<Expr>),

    // === Lambda 表达式（微积分基础）===
    /// Lambda 函数: λvar.body
    Lambda {
        var: String,
        body: Box<Expr>,
    },

    // === 微积分运算（需要 calculus feature）===
    /// 定积分: ∫[a,b] f(x) dx
    Integrate {
        var: String,
        lower: Box<Expr>,
        upper: Box<Expr>,
        body: Box<Expr>,
    },

    /// 导数: df/dx at point
    Derivative {
        var: String,
        body: Box<Expr>,
        at: Box<Expr>,
    },

    /// 极限: lim(x→to) f(x)
    Limit {
        var: String,
        to: Box<Expr>,
        body: Box<Expr>,
    },

    // === 向量运算（需要 matrix feature）===
    /// 向量字面量: [e1, e2, ...]
    VectorLit(Vec<Expr>),

    /// 点积: u · v
    Dot(Box<Expr>, Box<Expr>),

    /// 叉积: u × v（仅3D）
    Cross(Box<Expr>, Box<Expr>),

    /// 向量范数: ‖v‖
    VecNorm(Box<Expr>),

    /// 向量归一化: v / ‖v‖
    VecNormalize(Box<Expr>),

    /// 向量归约: 对一个向量的全部元素归约成标量（Σ/Π/mean/min/max）
    Reduce { kind: ReduceKind, arg: Box<Expr> },

    // === 矩阵运算（需要 matrix feature）===
    /// 矩阵字面量: [[a,b],[c,d]]
    MatrixLit(Vec<Vec<Expr>>),

    /// 矩阵乘法: A × B
    MatMul(Box<Expr>, Box<Expr>),

    /// 转置: A^T
    Transpose(Box<Expr>),

    /// 行列式: det(A)
    Det(Box<Expr>),

    /// 逆矩阵: A^{-1}
    Inv(Box<Expr>),

    /// 特征值: eigenvalues(A)
    Eigenvalues(Box<Expr>),

    /// 迹: tr(A)
    Trace(Box<Expr>),

    /// 矩阵范数: ‖A‖
    MatNorm(Box<Expr>),

    // === 特殊函数（需要 advanced_math feature）===
    /// 伽马函数: Γ(x)
    Gamma(Box<Expr>),

    /// 对数伽马函数: ln(Γ(x))
    Lgamma(Box<Expr>),

    /// 双伽马函数: ψ(x) = Γ'(x)/Γ(x)
    Digamma(Box<Expr>),

    /// 贝塔函数: B(a,b) = Γ(a)Γ(b)/Γ(a+b)
    Beta(Box<Expr>, Box<Expr>),

    /// 对数贝塔函数: ln(B(a,b))
    Lbeta(Box<Expr>, Box<Expr>),

    /// 误差函数: erf(x)
    Erf(Box<Expr>),

    /// 互补误差函数: erfc(x) = 1 - erf(x)
    Erfc(Box<Expr>),

    /// 逆误差函数: erf^{-1}(x)
    Erfinv(Box<Expr>),

    /// 阶乘: n!
    Factorial(Box<Expr>),

    /// 组合数: C(n,k) = n! / (k!(n-k)!)
    Combination(Box<Expr>, Box<Expr>),

    /// 黎曼 zeta 函数: ζ(s)
    Zeta(Box<Expr>),

    // === 贝塞尔函数（需要 gsl_math feature）===
    /// 第一类贝塞尔函数 J₀(x)
    BesselJ0(Box<Expr>),

    /// 第一类贝塞尔函数 J₁(x)
    BesselJ1(Box<Expr>),

    /// 第一类贝塞尔函数 Jₙ(x)
    BesselJn(Box<Expr>, Box<Expr>),

    /// 第二类贝塞尔函数 Y₀(x)
    BesselY0(Box<Expr>),

    /// 第二类贝塞尔函数 Y₁(x)
    BesselY1(Box<Expr>),

    /// 第二类贝塞尔函数 Yₙ(x)
    BesselYn(Box<Expr>, Box<Expr>),

    /// 修正贝塞尔函数 I₀(x)
    BesselI0(Box<Expr>),

    /// 修正贝塞尔函数 I₁(x)
    BesselI1(Box<Expr>),

    /// 修正贝塞尔函数 Iₙ(x)
    BesselIn(Box<Expr>, Box<Expr>),

    /// 修正贝塞尔函数 K₀(x)
    BesselK0(Box<Expr>),

    /// 修正贝塞尔函数 K₁(x)
    BesselK1(Box<Expr>),

    /// 修正贝塞尔函数 Kₙ(x)
    BesselKn(Box<Expr>, Box<Expr>),

    // === 概率分布函数（需要 advanced_math feature）===
    /// 正态分布 PDF: φ(x; μ, σ)
    NormPdf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 正态分布 CDF: Φ(x; μ, σ)
    NormCdf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 正态分布 PPF: Φ^{-1}(p; μ, σ)
    NormPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// t 分布 PDF
    TPdf(Box<Expr>, Box<Expr>),

    /// t 分布 CDF
    TCdf(Box<Expr>, Box<Expr>),

    /// t 分布 PPF
    TPpf(Box<Expr>, Box<Expr>),

    /// 卡方分布 PDF
    Chi2Pdf(Box<Expr>, Box<Expr>),

    /// 卡方分布 CDF
    Chi2Cdf(Box<Expr>, Box<Expr>),

    /// 卡方分布 PPF
    Chi2Ppf(Box<Expr>, Box<Expr>),

    /// F 分布 PDF
    FPdf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// F 分布 CDF
    FCdf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// F 分布 PPF
    FPpf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 泊松分布 PMF
    PoissonPmf(Box<Expr>, Box<Expr>),

    /// 泊松分布 CDF
    PoissonCdf(Box<Expr>, Box<Expr>),

    /// 二项分布 PMF
    BinomialPmf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 二项分布 CDF
    BinomialCdf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 指数分布 PDF
    ExponentialPdf(Box<Expr>, Box<Expr>),

    /// 指数分布 CDF
    ExponentialCdf(Box<Expr>, Box<Expr>),

    // === 复数运算（需要 advanced_math feature）===
    /// 构造复数: a + bi
    Complex(Box<Expr>, Box<Expr>),

    /// 取实部: Re(z)
    Real(Box<Expr>),

    /// 取虚部: Im(z)
    Imag(Box<Expr>),

    /// 共轭: z*
    Conj(Box<Expr>),

    /// 辐角: arg(z)
    Carg(Box<Expr>),

    /// 复数模: |z|
    Cabs(Box<Expr>),

    /// 极坐标构造: r * e^{iθ}
    Polar(Box<Expr>, Box<Expr>),

    // === 基础数学补充 ===
    /// 斜边: √(x²+y²)
    Hypot(Box<Expr>, Box<Expr>),

    /// 三维斜边: √(x²+y²+z²)
    Hypot3(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 限制范围: clamp(x, min, max)
    Clamp(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 复制符号: copysign(x, y)
    Copysign(Box<Expr>, Box<Expr>),

    /// 融合乘加: a*b + c
    Fma(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 任意底对数: log_b(x)
    Logn(Box<Expr>, Box<Expr>),

    /// Sinc 函数: sin(x)/x
    Sinc(Box<Expr>),

    // === 高精度数值函数 ===
    /// e^x - 1（高精度）
    Expm1(Box<Expr>),

    /// ln(1+x)（高精度）
    Log1p(Box<Expr>),

    /// 2^x
    Exp2(Box<Expr>),

    // === 不完全伽马/贝塔函数 ===
    /// 下不完全伽马函数: γ(a, x)
    Gammainc(Box<Expr>, Box<Expr>),

    /// 上不完全伽马函数: Γ(a, x)
    Gammaincc(Box<Expr>, Box<Expr>),

    /// 正则化不完全贝塔函数: I_x(a, b)
    Betainc(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 扩展三角函数 ===
    /// 正割: sec(x) = 1/cos(x)
    Sec(Box<Expr>),

    /// 余割: csc(x) = 1/sin(x)
    Csc(Box<Expr>),

    /// 余切: cot(x) = 1/tan(x)
    Cot(Box<Expr>),

    /// 反正割: arcsec(x)
    Asec(Box<Expr>),

    /// 反余割: arccsc(x)
    Acsc(Box<Expr>),

    /// 反余切: arccot(x)
    Acot(Box<Expr>),

    // === 扩展双曲函数 ===
    /// 双曲正割: sech(x) = 1/cosh(x)
    Sech(Box<Expr>),

    /// 双曲余割: csch(x) = 1/sinh(x)
    Csch(Box<Expr>),

    /// 双曲余切: coth(x) = 1/tanh(x)
    Coth(Box<Expr>),

    /// 反双曲正割: arsech(x)
    Asech(Box<Expr>),

    /// 反双曲余割: arcsch(x)
    Acsch(Box<Expr>),

    /// 反双曲余切: arcoth(x)
    Acoth(Box<Expr>),

    // === Airy 函数（需要 gsl_math）===
    /// Airy 函数 Ai(x)
    AiryAi(Box<Expr>),

    /// Airy 函数 Bi(x)
    AiryBi(Box<Expr>),

    // === 球谐函数（需要 gsl_math）===
    /// 球谐函数: Y_l^m(θ, φ)
    SphericalHarmonic(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    // === Fresnel 积分 ===
    /// Fresnel 正弦积分: S(x)
    FresnelS(Box<Expr>),

    /// Fresnel 余弦积分: C(x)
    FresnelC(Box<Expr>),

    // === 其他特殊函数 ===
    /// Dawson 函数: D(x)
    Dawson(Box<Expr>),

    /// 指数积分: Ei(x)
    ExpInt(Box<Expr>),

    /// 对数积分: li(x)
    LogInt(Box<Expr>),

    /// 正弦积分: Si(x)
    SinInt(Box<Expr>),

    /// 余弦积分: Ci(x)
    CosInt(Box<Expr>),

    // === Lambert W 函数 ===
    /// Lambert W 函数主支: W₀(x)
    LambertW(Box<Expr>),

    /// Lambert W 函数次支: W₋₁(x)
    LambertWm1(Box<Expr>),

    // === 球贝塞尔函数（需要 gsl_math）===
    /// 球贝塞尔函数第一类: jₙ(x)
    SphBesselJ(Box<Expr>, Box<Expr>),

    /// 球贝塞尔函数第二类: yₙ(x)
    SphBesselY(Box<Expr>, Box<Expr>),

    /// 修正球贝塞尔函数第一类: iₙ(x)
    SphBesselI(Box<Expr>, Box<Expr>),

    /// 修正球贝塞尔函数第二类: kₙ(x)
    SphBesselK(Box<Expr>, Box<Expr>),

    // === 超几何函数（需要 gsl_math）===
    /// 合流超几何函数 0F1: ₀F₁(;b;x)
    Hyp0f1(Box<Expr>, Box<Expr>),

    /// 合流超几何函数 1F1: ₁F₁(a;b;x) = M(a,b,x)
    Hyp1f1(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 高斯超几何函数 2F1: ₂F₁(a,b;c;x)
    Hyp2f1(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    // === Kelvin 函数（需要 gsl_math）===
    /// Kelvin 函数 ber(x)
    KelvinBer(Box<Expr>),

    /// Kelvin 函数 bei(x)
    KelvinBei(Box<Expr>),

    /// Kelvin 函数 ker(x)
    KelvinKer(Box<Expr>),

    /// Kelvin 函数 kei(x)
    KelvinKei(Box<Expr>),

    // === 不完全椭圆积分（需要 gsl_math）===
    /// 不完全椭圆积分第一类: F(φ, k)
    EllipF(Box<Expr>, Box<Expr>),

    /// 不完全椭圆积分第二类: E(φ, k)
    EllipEInc(Box<Expr>, Box<Expr>),

    /// 不完全椭圆积分第三类: Π(φ, n, k)
    EllipPi(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 其他特殊函数 ===
    /// Spence 函数 (dilogarithm): Li₂(x)
    Spence(Box<Expr>),

    /// 多伽马函数: ψⁿ(x)
    Polygamma(Box<Expr>, Box<Expr>),

    /// 第一类 Hankel 函数: H₁ⁿ(x)
    Hankel1(Box<Expr>, Box<Expr>),

    /// 第二类 Hankel 函数: H₂ⁿ(x)
    Hankel2(Box<Expr>, Box<Expr>),

    /// Struve 函数 H: Hᵥ(x)
    StruveH(Box<Expr>, Box<Expr>),

    /// 修正 Struve 函数 L: Lᵥ(x)
    StruveL(Box<Expr>, Box<Expr>),

    /// Owen's T 函数: T(h, a)
    OwensT(Box<Expr>, Box<Expr>),

    /// Riemann-Siegel Z 函数: Z(t)
    RiemannSiegelZ(Box<Expr>),

    /// Riemann-Siegel theta 函数: θ(t)
    RiemannSiegelTheta(Box<Expr>),

    // === Jacobi 椭圆函数（需要 gsl_math）===
    /// Jacobi 椭圆函数 sn(u, m)
    JacobiSn(Box<Expr>, Box<Expr>),

    /// Jacobi 椭圆函数 cn(u, m)
    JacobiCn(Box<Expr>, Box<Expr>),

    /// Jacobi 椭圆函数 dn(u, m)
    JacobiDn(Box<Expr>, Box<Expr>),

    // === 广义正交多项式（需要 gsl_math）===
    /// Gegenbauer 多项式 (超球多项式): C_n^α(x)
    Gegenbauer(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Jacobi 多项式: P_n^(α,β)(x)
    JacobiP(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    // === Mathieu 函数（需要 gsl_math）===
    /// Mathieu 特征值 a_n(q)
    MathieuA(Box<Expr>, Box<Expr>),

    /// Mathieu 特征值 b_n(q)
    MathieuB(Box<Expr>, Box<Expr>),

    /// Mathieu 角函数 ce_n(q, x)
    MathieuCe(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Mathieu 角函数 se_n(q, x)
    MathieuSe(Box<Expr>, Box<Expr>, Box<Expr>),

    // === Coulomb 波函数（需要 gsl_math）===
    /// Coulomb 波函数 F_L(η, ρ)
    CoulombF(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Coulomb 波函数 G_L(η, ρ)
    CoulombG(Box<Expr>, Box<Expr>, Box<Expr>),

    // === Wigner 符号（需要 gsl_math）===
    /// Wigner 3j 符号
    Wigner3j(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// Wigner 6j 符号
    Wigner6j(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// Wigner 9j 符号
    Wigner9j(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    // === Jacobi Theta 函数 ===
    /// Jacobi Theta 函数 θ₁(z, q)
    Theta1(Box<Expr>, Box<Expr>),

    /// Jacobi Theta 函数 θ₂(z, q)
    Theta2(Box<Expr>, Box<Expr>),

    /// Jacobi Theta 函数 θ₃(z, q)
    Theta3(Box<Expr>, Box<Expr>),

    /// Jacobi Theta 函数 θ₄(z, q)
    Theta4(Box<Expr>, Box<Expr>),

    // === 抛物柱面函数 ===
    /// 抛物柱面函数 D_v(x)
    Pbdv(Box<Expr>, Box<Expr>),

    /// 抛物柱面函数 V_v(x)
    Pbvv(Box<Expr>, Box<Expr>),

    /// 抛物柱面函数 W(a, x)
    Pbwa(Box<Expr>, Box<Expr>),

    // === 球扁旋转体波函数（长球/扁球）===
    /// 长球波角函数第一类: S1_mn(c, x)
    ProAng1(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// 长球波径向函数第一类: R1_mn(c, x)
    ProRad1(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// 长球波径向函数第二类: R2_mn(c, x)
    ProRad2(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// 扁球波角函数第一类: S1_mn(c, x)
    OblAng1(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// 扁球波径向函数第一类: R1_mn(c, x)
    OblRad1(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// 扁球波径向函数第二类: R2_mn(c, x)
    OblRad2(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    // === 修改的 Fresnel 积分 ===
    /// 修改的 Fresnel 积分 F+(x)
    ModFresnelP(Box<Expr>),

    /// 修改的 Fresnel 积分 F-(x)
    ModFresnelM(Box<Expr>),

    // === Wright 函数 ===
    /// Wright Bessel 函数: J_{ρ,β}(z)
    WrightBessel(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Wright Omega 函数: ω(z)
    WrightOmega(Box<Expr>),

    // === Voigt 函数 ===
    /// Voigt 线型函数: V(x, σ, γ)
    Voigt(Box<Expr>, Box<Expr>, Box<Expr>),

    // === Sigmoid/Logistic 函数 ===
    /// Logit 函数: log(x / (1-x))
    Logit(Box<Expr>),

    /// Expit/Logistic/Sigmoid 函数: 1 / (1 + exp(-x))
    Expit(Box<Expr>),

    // === Box-Cox 变换 ===
    /// Box-Cox 变换: (x^λ - 1) / λ
    BoxCox(Box<Expr>, Box<Expr>),

    /// Box-Cox 变换 (1+x): ((1+x)^λ - 1) / λ
    BoxCox1p(Box<Expr>, Box<Expr>),

    /// 逆 Box-Cox 变换
    InvBoxCox(Box<Expr>, Box<Expr>),

    /// 逆 Box-Cox 变换 (1+x)
    InvBoxCox1p(Box<Expr>, Box<Expr>),

    // === 信息论函数 ===
    /// 熵函数: -x * log(x)
    Entr(Box<Expr>),

    /// 相对熵: x * log(x/y)
    RelEntr(Box<Expr>, Box<Expr>),

    /// KL 散度: x * log(x/y) - x + y
    KlDiv(Box<Expr>, Box<Expr>),

    // === 阶乘扩展 ===
    /// 双阶乘: n!!
    Factorial2(Box<Expr>),

    /// k 阶乘: n!^(k)
    Factorialk(Box<Expr>, Box<Expr>),

    /// 第二类 Stirling 数: S(n, k)
    Stirling2(Box<Expr>, Box<Expr>),

    /// Pochhammer 符号 (上升阶乘): (z)_m
    Poch(Box<Expr>, Box<Expr>),

    // === Carlson 椭圆积分 ===
    /// Carlson RC: RC(x, y)
    EllipRc(Box<Expr>, Box<Expr>),

    /// Carlson RD: RD(x, y, z)
    EllipRd(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Carlson RF: RF(x, y, z)
    EllipRf(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Carlson RG: RG(x, y, z)
    EllipRg(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Carlson RJ: RJ(x, y, z, p)
    EllipRj(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    // === 扩展误差函数 ===
    /// 缩放互补误差函数: exp(x²) * erfc(x)
    Erfcx(Box<Expr>),

    /// 虚误差函数: -i * erf(ix)
    Erfi(Box<Expr>),

    /// 逆互补误差函数: erfc⁻¹(x)
    Erfcinv(Box<Expr>),

    // === 扩展 Gamma 函数 ===
    /// 合流超几何函数 U: U(a, b, x)
    Hyperu(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 倒数 Gamma: 1/Γ(x)
    Rgamma(Box<Expr>),

    /// Gamma 符号函数: sign(Γ(x))
    Gammasgn(Box<Expr>),

    // === 便利函数 ===
    /// 算术-几何平均: AGM(a, b)
    Agm(Box<Expr>, Box<Expr>),

    /// 相对指数: (exp(x) - 1) / x
    Exprel(Box<Expr>),

    /// x * log(y)，x=0 时返回 0
    Xlogy(Box<Expr>, Box<Expr>),

    /// x * log(1+y)，x=0 时返回 0
    Xlog1py(Box<Expr>, Box<Expr>),

    // === Zeta 扩展 ===
    /// Hurwitz zeta: ζ(s, q)
    HurwitzZeta(Box<Expr>, Box<Expr>),

    /// Zeta - 1: ζ(x) - 1
    Zetac(Box<Expr>),

    /// 多重对数: Li_s(z)
    Polylog(Box<Expr>, Box<Expr>),

    // === 缩放贝塞尔函数（指数衰减版本）===
    /// 缩放修正贝塞尔 I₀: i0e(x) = I₀(x) * exp(-|x|)
    BesselI0e(Box<Expr>),

    /// 缩放修正贝塞尔 I₁: i1e(x) = I₁(x) * exp(-|x|)
    BesselI1e(Box<Expr>),

    /// 缩放修正贝塞尔 Iₙ: ive(n, x)
    BesselIne(Box<Expr>, Box<Expr>),

    /// 缩放修正贝塞尔 K₀: k0e(x) = K₀(x) * exp(x)
    BesselK0e(Box<Expr>),

    /// 缩放修正贝塞尔 K₁: k1e(x) = K₁(x) * exp(x)
    BesselK1e(Box<Expr>),

    /// 缩放修正贝塞尔 Kₙ: kve(n, x)
    BesselKne(Box<Expr>, Box<Expr>),

    /// 缩放贝塞尔 Jₙ: jve(n, x)
    BesselJne(Box<Expr>, Box<Expr>),

    /// 缩放贝塞尔 Yₙ: yve(n, x)
    BesselYne(Box<Expr>, Box<Expr>),

    /// 缩放 Hankel 第一类: hankel1e(n, x)
    Hankel1e(Box<Expr>, Box<Expr>),

    /// 缩放 Hankel 第二类: hankel2e(n, x)
    Hankel2e(Box<Expr>, Box<Expr>),

    // === 贝塞尔函数导数 ===
    /// 贝塞尔 J 导数: jvp(n, x) = dJₙ(x)/dx
    BesselJnp(Box<Expr>, Box<Expr>),

    /// 贝塞尔 Y 导数: yvp(n, x)
    BesselYnp(Box<Expr>, Box<Expr>),

    /// 修正贝塞尔 I 导数: ivp(n, x)
    BesselInp(Box<Expr>, Box<Expr>),

    /// 修正贝塞尔 K 导数: kvp(n, x)
    BesselKnp(Box<Expr>, Box<Expr>),

    /// Hankel 1 导数: h1vp(n, x)
    Hankel1p(Box<Expr>, Box<Expr>),

    /// Hankel 2 导数: h2vp(n, x)
    Hankel2p(Box<Expr>, Box<Expr>),

    // === Huber 损失函数 ===
    /// Huber 损失: huber(delta, r)
    Huber(Box<Expr>, Box<Expr>),

    /// 伪 Huber 损失: pseudo_huber(delta, r)
    PseudoHuber(Box<Expr>, Box<Expr>),

    // === Kolmogorov-Smirnov 函数 ===
    /// Kolmogorov 生存函数
    Kolmogorov(Box<Expr>),

    /// Kolmogorov 逆生存函数
    Kolmogi(Box<Expr>),

    /// Smirnov 分布 CDF 补
    Smirnov(Box<Expr>, Box<Expr>),

    /// Smirnov 逆函数
    Smirnovi(Box<Expr>, Box<Expr>),

    // === Faddeeva/复数误差函数 ===
    /// Faddeeva 函数: wofz(z) = exp(-z²) * erfc(-iz)
    Wofz(Box<Expr>),

    // === Dirichlet 核 ===
    /// Dirichlet 核: diric(x, n)
    Diric(Box<Expr>, Box<Expr>),

    // === Tukey lambda 分布 ===
    /// Tukey lambda PPCC: tklmbda(x, lam)
    Tklmbda(Box<Expr>, Box<Expr>),

    // === Gamma/Beta 逆函数 ===
    /// 下不完全 Gamma 逆: gammaincinv(a, y)
    Gammaincinv(Box<Expr>, Box<Expr>),

    /// 上不完全 Gamma 逆: gammainccinv(a, y)
    Gammainccinv(Box<Expr>, Box<Expr>),

    /// 正则化不完全 Beta 逆: betaincinv(a, b, y)
    Betaincinv(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 高精度便利函数 ===
    /// cos(x) - 1（高精度）
    Cosm1(Box<Expr>),

    /// x^y - 1（高精度）
    Powm1(Box<Expr>, Box<Expr>),

    /// 10^x
    Exp10(Box<Expr>),

    /// log(1+x) - x（高精度）
    Log1pmx(Box<Expr>),

    /// 复数 log-gamma: loggamma(z)
    Loggamma(Box<Expr>),

    // === 度数三角函数 ===
    /// cos(x°) 度数余弦
    Cosdg(Box<Expr>),

    /// sin(x°) 度数正弦
    Sindg(Box<Expr>),

    /// tan(x°) 度数正切
    Tandg(Box<Expr>),

    /// cot(x°) 度数余切
    Cotdg(Box<Expr>),

    /// 度分秒转弧度: radian(d, m, s)
    Radian(Box<Expr>, Box<Expr>, Box<Expr>),

    // === Airy 扩展函数 ===
    /// 缩放 Airy 函数 Ai: aie(x)
    AiryAie(Box<Expr>),

    /// 缩放 Airy 函数 Bi: bie(x)
    AiryBie(Box<Expr>),

    /// Airy 导数 Ai': aip(x)
    AiryAip(Box<Expr>),

    /// Airy 导数 Bi': bip(x)
    AiryBip(Box<Expr>),

    /// Airy 积分: itairy(x) -> (apt, bpt, ant, bnt)
    ItAiry(Box<Expr>),

    // === 指数积分扩展 ===
    /// 广义指数积分: expn(n, x)
    Expn(Box<Expr>, Box<Expr>),

    /// 指数积分 E1: exp1(x)
    Exp1(Box<Expr>),

    /// 双曲正弦积分: shi(x)
    Shi(Box<Expr>),

    /// 双曲余弦积分: chi(x)
    Chi(Box<Expr>),

    // === Struve 积分 ===
    /// Struve 函数积分: itstruve0(x)
    ItStruve0(Box<Expr>),

    /// Struve 二次积分: it2struve0(x)
    It2Struve0(Box<Expr>),

    /// 修正 Struve 积分: itmodstruve0(x)
    ItModStruve0(Box<Expr>),

    // === ML/统计扩展 ===
    /// Log sigmoid: log_expit(x) = log(1/(1+exp(-x)))
    LogExpit(Box<Expr>),

    /// Softplus: softplus(x) = log(1+exp(x))
    Softplus(Box<Expr>),

    /// Log ndtr: log_ndtr(x)
    LogNdtr(Box<Expr>),

    // === Beta 补函数 ===
    /// 正则化不完全 Beta 补函数: betaincc(a, b, x)
    Betaincc(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 正则化不完全 Beta 补函数逆: betainccinv(a, b, y)
    Betainccinv(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 数论函数 ===
    /// Bernoulli 数: bernoulli(n)
    Bernoulli(Box<Expr>),

    /// Euler 数: euler(n)
    Euler(Box<Expr>),

    // === 椭圆扩展 ===
    /// 完全椭圆积分 K(1-m): ellipkm1(p)
    EllipKm1(Box<Expr>),

    // === Kelvin 导数 ===
    /// ber 导数: berp(x)
    KelvinBerp(Box<Expr>),

    /// bei 导数: beip(x)
    KelvinBeip(Box<Expr>),

    /// ker 导数: kerp(x)
    KelvinKerp(Box<Expr>),

    /// kei 导数: keip(x)
    KelvinKeip(Box<Expr>),

    // === 贝塞尔积分 ===
    /// 贝塞尔多项式: besselpoly(a, lmb, nu)
    BesselPoly(Box<Expr>, Box<Expr>, Box<Expr>),

    // === Wright Bessel 扩展 ===
    /// Log Wright Bessel: log_wright_bessel(a, b, x)
    LogWrightBessel(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 二项系数扩展 ===
    /// 实数二项系数: binom(x, y)
    Binom(Box<Expr>, Box<Expr>),

    // === 分布函数（CDF/SF/PPF）===
    /// 二项分布 CDF: bdtr(k, n, p)
    Bdtr(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 二项分布 SF: bdtrc(k, n, p)
    Bdtrc(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 二项分布逆 CDF: bdtri(k, n, y)
    Bdtri(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 卡方分布 CDF: chdtr(v, x)
    Chdtr(Box<Expr>, Box<Expr>),

    /// 卡方分布 SF: chdtrc(v, x)
    Chdtrc(Box<Expr>, Box<Expr>),

    /// 卡方分布逆 CDF: chdtri(v, p)
    Chdtri(Box<Expr>, Box<Expr>),

    /// F 分布 CDF: fdtr(dfn, dfd, x)
    Fdtr(Box<Expr>, Box<Expr>, Box<Expr>),

    /// F 分布 SF: fdtrc(dfn, dfd, x)
    Fdtrc(Box<Expr>, Box<Expr>, Box<Expr>),

    /// F 分布逆 CDF: fdtri(dfn, dfd, p)
    Fdtri(Box<Expr>, Box<Expr>, Box<Expr>),

    /// 学生 t 分布 CDF: stdtr(df, t)
    Stdtr(Box<Expr>, Box<Expr>),

    /// 学生 t 分布 SF: stdtrc(df, t)
    Stdtrc(Box<Expr>, Box<Expr>),

    /// 学生 t 分布逆 CDF: stdtrit(df, p)
    Stdtrit(Box<Expr>, Box<Expr>),

    /// 泊松分布 CDF: pdtr(k, m)
    Pdtr(Box<Expr>, Box<Expr>),

    /// 泊松分布 SF: pdtrc(k, m)
    Pdtrc(Box<Expr>, Box<Expr>),

    /// 泊松分布逆 CDF: pdtri(k, y)
    Pdtri(Box<Expr>, Box<Expr>),

    /// Beta 分布 CDF: btdtr(a, b, x)
    Btdtr(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Beta 分布 SF: btdtrc(a, b, x)
    Btdtrc(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Gamma 分布 CDF: gdtr(a, b, x)
    Gdtr(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Gamma 分布 SF: gdtrc(a, b, x)
    Gdtrc(Box<Expr>, Box<Expr>, Box<Expr>),

    // === 积分函数扩展 ===
    /// 正弦积分和余弦积分组合: sici(x) -> (si, ci)
    Sici(Box<Expr>),

    /// 双曲正弦积分和双曲余弦积分组合: shichi(x) -> (shi, chi)
    Shichi(Box<Expr>),

    // === ML 扩展 ===
    /// Softmax: softmax(x)
    Softmax(Box<Expr>),

    /// Log Softmax: log_softmax(x)
    LogSoftmax(Box<Expr>),

    /// Log-Sum-Exp: logsumexp(x)
    Logsumexp(Box<Expr>),

    // === 零点计算（GSL）===
    /// Airy Ai 第 s 个零点: ai_zero(s)
    AiryZeroAi(Box<Expr>),

    /// Airy Bi 第 s 个零点: bi_zero(s)
    AiryZeroBi(Box<Expr>),

    /// Bessel J0 第 s 个零点: bessel_zero_j0(s)
    BesselZeroJ0(Box<Expr>),

    /// Bessel J1 第 s 个零点: bessel_zero_j1(s)
    BesselZeroJ1(Box<Expr>),

    /// Bessel Jν 第 s 个零点: bessel_zero_jnu(nu, s)
    BesselZeroJnu(Box<Expr>, Box<Expr>),

    // === Legendre 扩展（GSL）===
    /// 球谐函数 Legendre: sph_legendre(l, m, x)
    SphLegendre(Box<Expr>, Box<Expr>, Box<Expr>),

    // === Clausen 函数（GSL）===
    /// Clausen 函数: clausen(x)
    Clausen(Box<Expr>),

    // === Debye 函数（GSL）===
    /// Debye 函数 D_n: debye(n, x)
    Debye(Box<Expr>, Box<Expr>),

    // === Synchrotron 函数（GSL）===
    /// Synchrotron 函数 1: synchrotron1(x)
    Synchrotron1(Box<Expr>),

    /// Synchrotron 函数 2: synchrotron2(x)
    Synchrotron2(Box<Expr>),

    // === Transport 函数（GSL）===
    /// Transport 函数: transport(n, x)
    Transport(Box<Expr>, Box<Expr>),

    // === Fermi-Dirac 函数（GSL）===
    /// Fermi-Dirac 函数: fermi_dirac(j, x)
    FermiDirac(Box<Expr>, Box<Expr>),
}

// ============================================
// 构造方法
// ============================================

impl Expr {
    // --- 叶子节点构造 ---

    /// 创建常量节点
    pub fn constant(value: f64) -> Self {
        Self::Const(value)
    }

    /// 创建变量引用
    pub fn var(name: impl Into<String>) -> Self {
        let name = name.into();
        // 自动判断是参数还是变量
        if Self::is_param_name(&name) {
            Self::Param(name)
        } else {
            Self::Var(name)
        }
    }

    /// 创建参数引用
    pub fn param(name: impl Into<String>) -> Self {
        Self::Param(name.into())
    }

    /// 判断名称是否是参数（p1, p2, ... 格式）
    fn is_param_name(name: &str) -> bool {
        name.starts_with('p') && name.len() > 1 && name[1..].chars().all(|c| c.is_ascii_digit())
    }

    // --- 数学常量 ---

    /// 创建 π 常量
    pub fn pi() -> Self {
        Self::Pi
    }

    /// 创建 e 常量
    pub fn e() -> Self {
        Self::E
    }

    // --- 算术运算构造 ---

    /// 创建加法
    #[allow(clippy::should_implement_trait)]
    pub fn add(left: Expr, right: Expr) -> Self {
        Self::Add(Box::new(left), Box::new(right))
    }

    /// 创建减法
    #[allow(clippy::should_implement_trait)]
    pub fn sub(left: Expr, right: Expr) -> Self {
        Self::Sub(Box::new(left), Box::new(right))
    }

    /// 创建乘法
    #[allow(clippy::should_implement_trait)]
    pub fn mul(left: Expr, right: Expr) -> Self {
        Self::Mul(Box::new(left), Box::new(right))
    }

    /// 创建除法
    #[allow(clippy::should_implement_trait)]
    pub fn div(left: Expr, right: Expr) -> Self {
        Self::Div(Box::new(left), Box::new(right))
    }

    /// 创建取负
    #[allow(clippy::should_implement_trait)]
    pub fn neg(arg: Expr) -> Self {
        Self::Neg(Box::new(arg))
    }

    /// 创建幂运算
    pub fn pow(base: Expr, exp: Expr) -> Self {
        Self::Pow(Box::new(base), Box::new(exp))
    }

    /// 创建绝对值
    pub fn abs(arg: Expr) -> Self {
        Self::Abs(Box::new(arg))
    }

    /// 创建取余
    pub fn modulo(a: Expr, b: Expr) -> Self {
        Self::Mod(Box::new(a), Box::new(b))
    }

    /// 创建向上取整
    pub fn ceil(arg: Expr) -> Self {
        Self::Ceil(Box::new(arg))
    }

    /// 创建向下取整
    pub fn floor(arg: Expr) -> Self {
        Self::Floor(Box::new(arg))
    }

    /// 创建四舍五入
    pub fn round(arg: Expr) -> Self {
        Self::Round(Box::new(arg))
    }

    /// 创建截断取整
    pub fn trunc(arg: Expr) -> Self {
        Self::Trunc(Box::new(arg))
    }

    /// 创建符号函数
    pub fn sign(arg: Expr) -> Self {
        Self::Sign(Box::new(arg))
    }

    // --- 超越函数构造 ---

    /// 创建指数函数
    pub fn exp(arg: Expr) -> Self {
        Self::Exp(Box::new(arg))
    }

    /// 创建自然对数
    pub fn ln(arg: Expr) -> Self {
        Self::Ln(Box::new(arg))
    }

    /// 创建常用对数
    pub fn log10(arg: Expr) -> Self {
        Self::Log10(Box::new(arg))
    }

    /// 创建以2为底对数
    pub fn log2(arg: Expr) -> Self {
        Self::Log2(Box::new(arg))
    }

    /// 创建平方根
    pub fn sqrt(arg: Expr) -> Self {
        Self::Sqrt(Box::new(arg))
    }

    /// 创建立方根
    pub fn cbrt(arg: Expr) -> Self {
        Self::Cbrt(Box::new(arg))
    }

    // --- 三角函数构造 ---

    /// 创建正弦
    pub fn sin(arg: Expr) -> Self {
        Self::Sin(Box::new(arg))
    }

    /// 创建余弦
    pub fn cos(arg: Expr) -> Self {
        Self::Cos(Box::new(arg))
    }

    /// 创建正切
    pub fn tan(arg: Expr) -> Self {
        Self::Tan(Box::new(arg))
    }

    /// 创建反正弦
    pub fn asin(arg: Expr) -> Self {
        Self::ASin(Box::new(arg))
    }

    /// 创建反余弦
    pub fn acos(arg: Expr) -> Self {
        Self::ACos(Box::new(arg))
    }

    /// 创建反正切
    pub fn atan(arg: Expr) -> Self {
        Self::ATan(Box::new(arg))
    }

    /// 创建二参数反正切
    pub fn atan2(y: Expr, x: Expr) -> Self {
        Self::ATan2(Box::new(y), Box::new(x))
    }

    // --- 双曲函数构造 ---

    /// 创建双曲正弦
    pub fn sinh(arg: Expr) -> Self {
        Self::Sinh(Box::new(arg))
    }

    /// 创建双曲余弦
    pub fn cosh(arg: Expr) -> Self {
        Self::Cosh(Box::new(arg))
    }

    /// 创建双曲正切
    pub fn tanh(arg: Expr) -> Self {
        Self::Tanh(Box::new(arg))
    }

    /// 创建反双曲正弦
    pub fn asinh(arg: Expr) -> Self {
        Self::ASinh(Box::new(arg))
    }

    /// 创建反双曲余弦
    pub fn acosh(arg: Expr) -> Self {
        Self::ACosh(Box::new(arg))
    }

    /// 创建反双曲正切
    pub fn atanh(arg: Expr) -> Self {
        Self::ATanh(Box::new(arg))
    }

    // --- 聚合函数构造 ---

    /// 创建最大值
    pub fn max(args: Vec<Expr>) -> Self {
        Self::Max(args)
    }

    /// 创建最小值
    pub fn min(args: Vec<Expr>) -> Self {
        Self::Min(args)
    }

    // --- 条件表达式构造 ---

    /// 创建条件表达式
    pub fn if_then_else(cond: Expr, then_branch: Expr, else_branch: Expr) -> Self {
        Self::IfThenElse {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    // --- 扩展分位数函数 ---

    /// 创建指数分布分位数
    pub fn exp_ppf(p: Expr, lambda: Expr) -> Self {
        Self::ExpPpf(Box::new(p), Box::new(lambda))
    }

    /// 创建伽马分布分位数
    pub fn gamma_ppf(p: Expr, alpha: Expr, beta: Expr) -> Self {
        Self::GammaPpf(Box::new(p), Box::new(alpha), Box::new(beta))
    }

    /// 创建贝塔分布分位数
    pub fn beta_ppf(p: Expr, alpha: Expr, beta: Expr) -> Self {
        Self::BetaPpf(Box::new(p), Box::new(alpha), Box::new(beta))
    }

    /// 创建威布尔分布分位数
    pub fn weibull_ppf(p: Expr, k: Expr, lambda: Expr) -> Self {
        Self::WeibullPpf(Box::new(p), Box::new(k), Box::new(lambda))
    }

    /// 创建对数正态分布分位数
    pub fn lognorm_ppf(p: Expr, mu: Expr, sigma: Expr) -> Self {
        Self::LognormPpf(Box::new(p), Box::new(mu), Box::new(sigma))
    }

    /// 创建均匀分布分位数
    pub fn uniform_ppf(p: Expr, a: Expr, b: Expr) -> Self {
        Self::UniformPpf(Box::new(p), Box::new(a), Box::new(b))
    }

    /// 创建柯西分布分位数
    pub fn cauchy_ppf(p: Expr, x0: Expr, gamma: Expr) -> Self {
        Self::CauchyPpf(Box::new(p), Box::new(x0), Box::new(gamma))
    }

    // --- 复数扩展运算 ---

    /// 创建复数双曲正弦
    pub fn complex_sinh(z: Expr) -> Self {
        Self::ComplexSinh(Box::new(z))
    }

    /// 创建复数双曲余弦
    pub fn complex_cosh(z: Expr) -> Self {
        Self::ComplexCosh(Box::new(z))
    }

    /// 创建复数双曲正切
    pub fn complex_tanh(z: Expr) -> Self {
        Self::ComplexTanh(Box::new(z))
    }

    /// 创建复数反双曲正弦
    pub fn complex_asinh(z: Expr) -> Self {
        Self::ComplexAsinh(Box::new(z))
    }

    /// 创建复数反双曲余弦
    pub fn complex_acosh(z: Expr) -> Self {
        Self::ComplexAcosh(Box::new(z))
    }

    /// 创建复数反双曲正切
    pub fn complex_atanh(z: Expr) -> Self {
        Self::ComplexAtanh(Box::new(z))
    }

    /// 创建复数反正弦
    pub fn complex_asin(z: Expr) -> Self {
        Self::ComplexAsin(Box::new(z))
    }

    /// 创建复数反余弦
    pub fn complex_acos(z: Expr) -> Self {
        Self::ComplexAcos(Box::new(z))
    }

    /// 创建复数反正切
    pub fn complex_atan(z: Expr) -> Self {
        Self::ComplexAtan(Box::new(z))
    }

    // --- 数论函数 ---

    /// 创建最大公约数
    pub fn gcd(a: Expr, b: Expr) -> Self {
        Self::Gcd(Box::new(a), Box::new(b))
    }

    /// 创建最小公倍数
    pub fn lcm(a: Expr, b: Expr) -> Self {
        Self::Lcm(Box::new(a), Box::new(b))
    }

    /// 创建排列数
    pub fn permutation(n: Expr, k: Expr) -> Self {
        Self::Permutation(Box::new(n), Box::new(k))
    }

    // --- 正交多项式 ---

    /// 创建勒让德多项式
    pub fn legendre(n: Expr, x: Expr) -> Self {
        Self::Legendre(Box::new(n), Box::new(x))
    }

    /// 创建关联勒让德多项式
    pub fn legendre_assoc(l: Expr, m: Expr, x: Expr) -> Self {
        Self::LegendreAssoc(Box::new(l), Box::new(m), Box::new(x))
    }

    /// 创建厄米多项式
    pub fn hermite(n: Expr, x: Expr) -> Self {
        Self::Hermite(Box::new(n), Box::new(x))
    }

    /// 创建拉盖尔多项式
    pub fn laguerre(n: Expr, x: Expr) -> Self {
        Self::Laguerre(Box::new(n), Box::new(x))
    }

    /// 创建关联拉盖尔多项式
    pub fn laguerre_assoc(n: Expr, a: Expr, x: Expr) -> Self {
        Self::LaguerreAssoc(Box::new(n), Box::new(a), Box::new(x))
    }

    /// 创建第一类切比雪夫多项式
    pub fn chebyshev_t(n: Expr, x: Expr) -> Self {
        Self::ChebyshevT(Box::new(n), Box::new(x))
    }

    /// 创建第二类切比雪夫多项式
    pub fn chebyshev_u(n: Expr, x: Expr) -> Self {
        Self::ChebyshevU(Box::new(n), Box::new(x))
    }

    // --- 椭圆积分 ---

    /// 创建完全椭圆积分第一类
    pub fn ellip_k(k: Expr) -> Self {
        Self::EllipK(Box::new(k))
    }

    /// 创建完全椭圆积分第二类
    pub fn ellip_e(k: Expr) -> Self {
        Self::EllipE(Box::new(k))
    }

    // --- 微积分运算 ---

    /// 创建 Lambda 表达式
    pub fn lambda(var: impl Into<String>, body: Expr) -> Self {
        Self::Lambda {
            var: var.into(),
            body: Box::new(body),
        }
    }

    /// 创建定积分
    pub fn integrate(var: impl Into<String>, lower: Expr, upper: Expr, body: Expr) -> Self {
        Self::Integrate {
            var: var.into(),
            lower: Box::new(lower),
            upper: Box::new(upper),
            body: Box::new(body),
        }
    }

    /// 创建导数
    pub fn derivative(var: impl Into<String>, body: Expr, at: Expr) -> Self {
        Self::Derivative {
            var: var.into(),
            body: Box::new(body),
            at: Box::new(at),
        }
    }

    /// 创建极限
    pub fn limit(var: impl Into<String>, to: Expr, body: Expr) -> Self {
        Self::Limit {
            var: var.into(),
            to: Box::new(to),
            body: Box::new(body),
        }
    }

    // --- 向量运算 ---

    /// 创建向量字面量
    pub fn vector_lit(elements: Vec<Expr>) -> Self {
        Self::VectorLit(elements)
    }

    /// 创建点积
    pub fn dot(a: Expr, b: Expr) -> Self {
        Self::Dot(Box::new(a), Box::new(b))
    }

    /// 创建叉积
    pub fn cross(a: Expr, b: Expr) -> Self {
        Self::Cross(Box::new(a), Box::new(b))
    }

    /// 创建向量范数
    pub fn vec_norm(v: Expr) -> Self {
        Self::VecNorm(Box::new(v))
    }

    /// 创建向量归一化
    pub fn vec_normalize(v: Expr) -> Self {
        Self::VecNormalize(Box::new(v))
    }

    /// 创建向量归约（Σ/Π/mean/min/max）。
    pub fn reduce(kind: ReduceKind, arg: Expr) -> Self {
        Self::Reduce { kind, arg: Box::new(arg) }
    }
    /// 向量求和归约 Σ。
    pub fn vsum(a: Expr) -> Self {
        Self::reduce(ReduceKind::Sum, a)
    }
    /// 向量求积归约 Π。
    pub fn vprod(a: Expr) -> Self {
        Self::reduce(ReduceKind::Prod, a)
    }
    /// 向量均值归约。
    pub fn vmean(a: Expr) -> Self {
        Self::reduce(ReduceKind::Mean, a)
    }
    /// 向量最小值归约。
    pub fn vmin(a: Expr) -> Self {
        Self::reduce(ReduceKind::Min, a)
    }
    /// 向量最大值归约。
    pub fn vmax(a: Expr) -> Self {
        Self::reduce(ReduceKind::Max, a)
    }

    // --- 矩阵运算 ---

    /// 创建矩阵字面量
    pub fn matrix_lit(rows: Vec<Vec<Expr>>) -> Self {
        Self::MatrixLit(rows)
    }

    /// 创建矩阵乘法
    pub fn mat_mul(a: Expr, b: Expr) -> Self {
        Self::MatMul(Box::new(a), Box::new(b))
    }

    /// 创建转置
    pub fn transpose(a: Expr) -> Self {
        Self::Transpose(Box::new(a))
    }

    /// 创建行列式
    pub fn det(a: Expr) -> Self {
        Self::Det(Box::new(a))
    }

    /// 创建逆矩阵
    pub fn inv(a: Expr) -> Self {
        Self::Inv(Box::new(a))
    }

    /// 创建特征值
    pub fn eigenvalues(a: Expr) -> Self {
        Self::Eigenvalues(Box::new(a))
    }

    /// 创建迹
    pub fn trace(a: Expr) -> Self {
        Self::Trace(Box::new(a))
    }

    /// 创建矩阵范数
    pub fn mat_norm(a: Expr) -> Self {
        Self::MatNorm(Box::new(a))
    }

    // --- 特殊函数 ---

    /// 创建伽马函数
    pub fn gamma(x: Expr) -> Self {
        Self::Gamma(Box::new(x))
    }

    /// 创建对数伽马函数
    pub fn lgamma(x: Expr) -> Self {
        Self::Lgamma(Box::new(x))
    }

    /// 创建双伽马函数
    pub fn digamma(x: Expr) -> Self {
        Self::Digamma(Box::new(x))
    }

    /// 创建贝塔函数
    pub fn beta_fn(a: Expr, b: Expr) -> Self {
        Self::Beta(Box::new(a), Box::new(b))
    }

    /// 创建对数贝塔函数
    pub fn lbeta(a: Expr, b: Expr) -> Self {
        Self::Lbeta(Box::new(a), Box::new(b))
    }

    /// 创建误差函数
    pub fn erf(x: Expr) -> Self {
        Self::Erf(Box::new(x))
    }

    /// 创建互补误差函数
    pub fn erfc(x: Expr) -> Self {
        Self::Erfc(Box::new(x))
    }

    /// 创建逆误差函数
    pub fn erfinv(x: Expr) -> Self {
        Self::Erfinv(Box::new(x))
    }

    /// 创建阶乘
    pub fn factorial(n: Expr) -> Self {
        Self::Factorial(Box::new(n))
    }

    /// 创建组合数
    pub fn combination(n: Expr, k: Expr) -> Self {
        Self::Combination(Box::new(n), Box::new(k))
    }

    /// 创建黎曼 zeta 函数
    pub fn zeta(s: Expr) -> Self {
        Self::Zeta(Box::new(s))
    }

    // --- 贝塞尔函数 ---

    /// 创建 J₀(x)
    pub fn bessel_j0(x: Expr) -> Self {
        Self::BesselJ0(Box::new(x))
    }

    /// 创建 J₁(x)
    pub fn bessel_j1(x: Expr) -> Self {
        Self::BesselJ1(Box::new(x))
    }

    /// 创建 Jₙ(x)
    pub fn bessel_jn(n: Expr, x: Expr) -> Self {
        Self::BesselJn(Box::new(n), Box::new(x))
    }

    /// 创建 Y₀(x)
    pub fn bessel_y0(x: Expr) -> Self {
        Self::BesselY0(Box::new(x))
    }

    /// 创建 Y₁(x)
    pub fn bessel_y1(x: Expr) -> Self {
        Self::BesselY1(Box::new(x))
    }

    /// 创建 Yₙ(x)
    pub fn bessel_yn(n: Expr, x: Expr) -> Self {
        Self::BesselYn(Box::new(n), Box::new(x))
    }

    /// 创建 I₀(x)
    pub fn bessel_i0(x: Expr) -> Self {
        Self::BesselI0(Box::new(x))
    }

    /// 创建 I₁(x)
    pub fn bessel_i1(x: Expr) -> Self {
        Self::BesselI1(Box::new(x))
    }

    /// 创建 Iₙ(x)
    pub fn bessel_in(n: Expr, x: Expr) -> Self {
        Self::BesselIn(Box::new(n), Box::new(x))
    }

    /// 创建 K₀(x)
    pub fn bessel_k0(x: Expr) -> Self {
        Self::BesselK0(Box::new(x))
    }

    /// 创建 K₁(x)
    pub fn bessel_k1(x: Expr) -> Self {
        Self::BesselK1(Box::new(x))
    }

    /// 创建 Kₙ(x)
    pub fn bessel_kn(n: Expr, x: Expr) -> Self {
        Self::BesselKn(Box::new(n), Box::new(x))
    }

    // --- 概率分布函数 ---

    /// 创建正态分布 PDF
    pub fn norm_pdf(x: Expr, mu: Expr, sigma: Expr) -> Self {
        Self::NormPdf(Box::new(x), Box::new(mu), Box::new(sigma))
    }

    /// 创建正态分布 CDF
    pub fn norm_cdf(x: Expr, mu: Expr, sigma: Expr) -> Self {
        Self::NormCdf(Box::new(x), Box::new(mu), Box::new(sigma))
    }

    /// 创建正态分布 PPF
    pub fn norm_ppf(p: Expr, mu: Expr, sigma: Expr) -> Self {
        Self::NormPpf(Box::new(p), Box::new(mu), Box::new(sigma))
    }

    /// 创建 t 分布 PDF
    pub fn t_pdf(x: Expr, df: Expr) -> Self {
        Self::TPdf(Box::new(x), Box::new(df))
    }

    /// 创建 t 分布 CDF
    pub fn t_cdf(x: Expr, df: Expr) -> Self {
        Self::TCdf(Box::new(x), Box::new(df))
    }

    /// 创建 t 分布 PPF
    pub fn t_ppf(p: Expr, df: Expr) -> Self {
        Self::TPpf(Box::new(p), Box::new(df))
    }

    /// 创建卡方分布 PDF
    pub fn chi2_pdf(x: Expr, df: Expr) -> Self {
        Self::Chi2Pdf(Box::new(x), Box::new(df))
    }

    /// 创建卡方分布 CDF
    pub fn chi2_cdf(x: Expr, df: Expr) -> Self {
        Self::Chi2Cdf(Box::new(x), Box::new(df))
    }

    /// 创建卡方分布 PPF
    pub fn chi2_ppf(p: Expr, df: Expr) -> Self {
        Self::Chi2Ppf(Box::new(p), Box::new(df))
    }

    /// 创建 F 分布 PDF
    pub fn f_pdf(x: Expr, d1: Expr, d2: Expr) -> Self {
        Self::FPdf(Box::new(x), Box::new(d1), Box::new(d2))
    }

    /// 创建 F 分布 CDF
    pub fn f_cdf(x: Expr, d1: Expr, d2: Expr) -> Self {
        Self::FCdf(Box::new(x), Box::new(d1), Box::new(d2))
    }

    /// 创建 F 分布 PPF
    pub fn f_ppf(p: Expr, d1: Expr, d2: Expr) -> Self {
        Self::FPpf(Box::new(p), Box::new(d1), Box::new(d2))
    }

    /// 创建泊松分布 PMF
    pub fn poisson_pmf(k: Expr, lambda: Expr) -> Self {
        Self::PoissonPmf(Box::new(k), Box::new(lambda))
    }

    /// 创建泊松分布 CDF
    pub fn poisson_cdf(k: Expr, lambda: Expr) -> Self {
        Self::PoissonCdf(Box::new(k), Box::new(lambda))
    }

    /// 创建二项分布 PMF
    pub fn binomial_pmf(k: Expr, n: Expr, p: Expr) -> Self {
        Self::BinomialPmf(Box::new(k), Box::new(n), Box::new(p))
    }

    /// 创建二项分布 CDF
    pub fn binomial_cdf(k: Expr, n: Expr, p: Expr) -> Self {
        Self::BinomialCdf(Box::new(k), Box::new(n), Box::new(p))
    }

    /// 创建指数分布 PDF
    pub fn exponential_pdf(x: Expr, lambda: Expr) -> Self {
        Self::ExponentialPdf(Box::new(x), Box::new(lambda))
    }

    /// 创建指数分布 CDF
    pub fn exponential_cdf(x: Expr, lambda: Expr) -> Self {
        Self::ExponentialCdf(Box::new(x), Box::new(lambda))
    }

    // --- 复数运算 ---

    /// 创建复数
    pub fn complex(re: Expr, im: Expr) -> Self {
        Self::Complex(Box::new(re), Box::new(im))
    }

    /// 取实部
    pub fn real(z: Expr) -> Self {
        Self::Real(Box::new(z))
    }

    /// 取虚部
    pub fn imag(z: Expr) -> Self {
        Self::Imag(Box::new(z))
    }

    /// 共轭
    pub fn conj(z: Expr) -> Self {
        Self::Conj(Box::new(z))
    }

    /// 辐角
    pub fn carg(z: Expr) -> Self {
        Self::Carg(Box::new(z))
    }

    /// 复数模
    pub fn cabs(z: Expr) -> Self {
        Self::Cabs(Box::new(z))
    }

    /// 极坐标构造
    pub fn polar(r: Expr, theta: Expr) -> Self {
        Self::Polar(Box::new(r), Box::new(theta))
    }

    // --- 基础数学补充 ---

    /// 创建斜边
    pub fn hypot(x: Expr, y: Expr) -> Self {
        Self::Hypot(Box::new(x), Box::new(y))
    }

    /// 创建三维斜边
    pub fn hypot3(x: Expr, y: Expr, z: Expr) -> Self {
        Self::Hypot3(Box::new(x), Box::new(y), Box::new(z))
    }

    /// 创建 clamp
    pub fn clamp(x: Expr, min: Expr, max: Expr) -> Self {
        Self::Clamp(Box::new(x), Box::new(min), Box::new(max))
    }

    /// 创建 copysign
    pub fn copysign(x: Expr, y: Expr) -> Self {
        Self::Copysign(Box::new(x), Box::new(y))
    }

    /// 创建融合乘加
    pub fn fma(a: Expr, b: Expr, c: Expr) -> Self {
        Self::Fma(Box::new(a), Box::new(b), Box::new(c))
    }

    /// 创建任意底对数
    pub fn logn(base: Expr, x: Expr) -> Self {
        Self::Logn(Box::new(base), Box::new(x))
    }

    /// 创建 sinc 函数
    pub fn sinc(x: Expr) -> Self {
        Self::Sinc(Box::new(x))
    }

    // --- 高精度数值函数 ---

    /// 创建 e^x - 1
    pub fn expm1(x: Expr) -> Self {
        Self::Expm1(Box::new(x))
    }

    /// 创建 ln(1+x)
    pub fn log1p(x: Expr) -> Self {
        Self::Log1p(Box::new(x))
    }

    /// 创建 2^x
    pub fn exp2(x: Expr) -> Self {
        Self::Exp2(Box::new(x))
    }

    // --- 不完全伽马/贝塔函数 ---

    /// 创建下不完全伽马函数
    pub fn gammainc(a: Expr, x: Expr) -> Self {
        Self::Gammainc(Box::new(a), Box::new(x))
    }

    /// 创建上不完全伽马函数
    pub fn gammaincc(a: Expr, x: Expr) -> Self {
        Self::Gammaincc(Box::new(a), Box::new(x))
    }

    /// 创建正则化不完全贝塔函数
    pub fn betainc(x: Expr, a: Expr, b: Expr) -> Self {
        Self::Betainc(Box::new(x), Box::new(a), Box::new(b))
    }

    // --- 扩展三角函数 ---

    /// 创建正割
    pub fn sec(x: Expr) -> Self {
        Self::Sec(Box::new(x))
    }

    /// 创建余割
    pub fn csc(x: Expr) -> Self {
        Self::Csc(Box::new(x))
    }

    /// 创建余切
    pub fn cot(x: Expr) -> Self {
        Self::Cot(Box::new(x))
    }

    /// 创建反正割
    pub fn asec(x: Expr) -> Self {
        Self::Asec(Box::new(x))
    }

    /// 创建反余割
    pub fn acsc(x: Expr) -> Self {
        Self::Acsc(Box::new(x))
    }

    /// 创建反余切
    pub fn acot(x: Expr) -> Self {
        Self::Acot(Box::new(x))
    }

    // --- 扩展双曲函数 ---

    /// 创建双曲正割
    pub fn sech(x: Expr) -> Self {
        Self::Sech(Box::new(x))
    }

    /// 创建双曲余割
    pub fn csch(x: Expr) -> Self {
        Self::Csch(Box::new(x))
    }

    /// 创建双曲余切
    pub fn coth(x: Expr) -> Self {
        Self::Coth(Box::new(x))
    }

    /// 创建反双曲正割
    pub fn asech(x: Expr) -> Self {
        Self::Asech(Box::new(x))
    }

    /// 创建反双曲余割
    pub fn acsch(x: Expr) -> Self {
        Self::Acsch(Box::new(x))
    }

    /// 创建反双曲余切
    pub fn acoth(x: Expr) -> Self {
        Self::Acoth(Box::new(x))
    }

    // --- Airy 函数 ---

    /// 创建 Airy Ai
    pub fn airy_ai(x: Expr) -> Self {
        Self::AiryAi(Box::new(x))
    }

    /// 创建 Airy Bi
    pub fn airy_bi(x: Expr) -> Self {
        Self::AiryBi(Box::new(x))
    }

    // --- 球谐函数 ---

    /// 创建球谐函数
    pub fn spherical_harmonic(l: Expr, m: Expr, theta: Expr, phi: Expr) -> Self {
        Self::SphericalHarmonic(Box::new(l), Box::new(m), Box::new(theta), Box::new(phi))
    }

    // --- Fresnel 积分 ---

    /// 创建 Fresnel S
    pub fn fresnel_s(x: Expr) -> Self {
        Self::FresnelS(Box::new(x))
    }

    /// 创建 Fresnel C
    pub fn fresnel_c(x: Expr) -> Self {
        Self::FresnelC(Box::new(x))
    }

    // --- 其他特殊函数 ---

    /// 创建 Dawson 函数
    pub fn dawson(x: Expr) -> Self {
        Self::Dawson(Box::new(x))
    }

    /// 创建指数积分
    pub fn exp_int(x: Expr) -> Self {
        Self::ExpInt(Box::new(x))
    }

    /// 创建对数积分
    pub fn log_int(x: Expr) -> Self {
        Self::LogInt(Box::new(x))
    }

    /// 创建正弦积分
    pub fn sin_int(x: Expr) -> Self {
        Self::SinInt(Box::new(x))
    }

    /// 创建余弦积分
    pub fn cos_int(x: Expr) -> Self {
        Self::CosInt(Box::new(x))
    }

    // --- Lambert W 函数 ---

    /// 创建 Lambert W 主支
    pub fn lambertw(x: Expr) -> Self {
        Self::LambertW(Box::new(x))
    }

    /// 创建 Lambert W 次支
    pub fn lambertw_m1(x: Expr) -> Self {
        Self::LambertWm1(Box::new(x))
    }

    // --- 球贝塞尔函数 ---

    /// 创建球贝塞尔 j
    pub fn sph_bessel_j(n: Expr, x: Expr) -> Self {
        Self::SphBesselJ(Box::new(n), Box::new(x))
    }

    /// 创建球贝塞尔 y
    pub fn sph_bessel_y(n: Expr, x: Expr) -> Self {
        Self::SphBesselY(Box::new(n), Box::new(x))
    }

    /// 创建修正球贝塞尔 i
    pub fn sph_bessel_i(n: Expr, x: Expr) -> Self {
        Self::SphBesselI(Box::new(n), Box::new(x))
    }

    /// 创建修正球贝塞尔 k
    pub fn sph_bessel_k(n: Expr, x: Expr) -> Self {
        Self::SphBesselK(Box::new(n), Box::new(x))
    }

    // --- 超几何函数 ---

    /// 创建 0F1
    pub fn hyp0f1(b: Expr, x: Expr) -> Self {
        Self::Hyp0f1(Box::new(b), Box::new(x))
    }

    /// 创建 1F1
    pub fn hyp1f1(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Hyp1f1(Box::new(a), Box::new(b), Box::new(x))
    }

    /// 创建 2F1
    pub fn hyp2f1(a: Expr, b: Expr, c: Expr, x: Expr) -> Self {
        Self::Hyp2f1(Box::new(a), Box::new(b), Box::new(c), Box::new(x))
    }

    // --- Kelvin 函数 ---

    /// 创建 ber
    pub fn kelvin_ber(x: Expr) -> Self {
        Self::KelvinBer(Box::new(x))
    }

    /// 创建 bei
    pub fn kelvin_bei(x: Expr) -> Self {
        Self::KelvinBei(Box::new(x))
    }

    /// 创建 ker
    pub fn kelvin_ker(x: Expr) -> Self {
        Self::KelvinKer(Box::new(x))
    }

    /// 创建 kei
    pub fn kelvin_kei(x: Expr) -> Self {
        Self::KelvinKei(Box::new(x))
    }

    // --- 不完全椭圆积分 ---

    /// 创建不完全椭圆积分第一类
    pub fn ellipf(phi: Expr, k: Expr) -> Self {
        Self::EllipF(Box::new(phi), Box::new(k))
    }

    /// 创建不完全椭圆积分第二类
    pub fn ellipe_inc(phi: Expr, k: Expr) -> Self {
        Self::EllipEInc(Box::new(phi), Box::new(k))
    }

    /// 创建不完全椭圆积分第三类
    pub fn ellippi(phi: Expr, n: Expr, k: Expr) -> Self {
        Self::EllipPi(Box::new(phi), Box::new(n), Box::new(k))
    }

    // --- 其他特殊函数 ---

    /// 创建 Spence 函数
    pub fn spence(x: Expr) -> Self {
        Self::Spence(Box::new(x))
    }

    /// 创建多伽马函数
    pub fn polygamma(n: Expr, x: Expr) -> Self {
        Self::Polygamma(Box::new(n), Box::new(x))
    }

    /// 创建 Hankel 1
    pub fn hankel1(n: Expr, x: Expr) -> Self {
        Self::Hankel1(Box::new(n), Box::new(x))
    }

    /// 创建 Hankel 2
    pub fn hankel2(n: Expr, x: Expr) -> Self {
        Self::Hankel2(Box::new(n), Box::new(x))
    }

    /// 创建 Struve H
    pub fn struve_h(v: Expr, x: Expr) -> Self {
        Self::StruveH(Box::new(v), Box::new(x))
    }

    /// 创建 Struve L
    pub fn struve_l(v: Expr, x: Expr) -> Self {
        Self::StruveL(Box::new(v), Box::new(x))
    }

    /// 创建 Owen's T
    pub fn owens_t(h: Expr, a: Expr) -> Self {
        Self::OwensT(Box::new(h), Box::new(a))
    }

    /// 创建 Riemann-Siegel Z
    pub fn riemann_siegel_z(t: Expr) -> Self {
        Self::RiemannSiegelZ(Box::new(t))
    }

    /// 创建 Riemann-Siegel theta
    pub fn riemann_siegel_theta(t: Expr) -> Self {
        Self::RiemannSiegelTheta(Box::new(t))
    }

    // --- Jacobi 椭圆函数 ---

    /// 创建 Jacobi sn
    pub fn jacobi_sn(u: Expr, m: Expr) -> Self {
        Self::JacobiSn(Box::new(u), Box::new(m))
    }

    /// 创建 Jacobi cn
    pub fn jacobi_cn(u: Expr, m: Expr) -> Self {
        Self::JacobiCn(Box::new(u), Box::new(m))
    }

    /// 创建 Jacobi dn
    pub fn jacobi_dn(u: Expr, m: Expr) -> Self {
        Self::JacobiDn(Box::new(u), Box::new(m))
    }

    // --- 广义正交多项式 ---

    /// 创建 Gegenbauer 多项式
    pub fn gegenbauer(n: Expr, alpha: Expr, x: Expr) -> Self {
        Self::Gegenbauer(Box::new(n), Box::new(alpha), Box::new(x))
    }

    /// 创建 Jacobi 多项式
    pub fn jacobi_p(n: Expr, alpha: Expr, beta: Expr, x: Expr) -> Self {
        Self::JacobiP(Box::new(n), Box::new(alpha), Box::new(beta), Box::new(x))
    }

    // --- Mathieu 函数 ---

    /// 创建 Mathieu a
    pub fn mathieu_a(n: Expr, q: Expr) -> Self {
        Self::MathieuA(Box::new(n), Box::new(q))
    }

    /// 创建 Mathieu b
    pub fn mathieu_b(n: Expr, q: Expr) -> Self {
        Self::MathieuB(Box::new(n), Box::new(q))
    }

    /// 创建 Mathieu ce
    pub fn mathieu_ce(n: Expr, q: Expr, x: Expr) -> Self {
        Self::MathieuCe(Box::new(n), Box::new(q), Box::new(x))
    }

    /// 创建 Mathieu se
    pub fn mathieu_se(n: Expr, q: Expr, x: Expr) -> Self {
        Self::MathieuSe(Box::new(n), Box::new(q), Box::new(x))
    }

    // --- Coulomb 波函数 ---

    /// 创建 Coulomb F
    pub fn coulomb_f(l: Expr, eta: Expr, rho: Expr) -> Self {
        Self::CoulombF(Box::new(l), Box::new(eta), Box::new(rho))
    }

    /// 创建 Coulomb G
    pub fn coulomb_g(l: Expr, eta: Expr, rho: Expr) -> Self {
        Self::CoulombG(Box::new(l), Box::new(eta), Box::new(rho))
    }

    // --- Wigner 符号 ---

    /// 创建 Wigner 3j
    pub fn wigner_3j(j1: Expr, j2: Expr, j3: Expr, m1: Expr, m2: Expr, m3: Expr) -> Self {
        Self::Wigner3j(Box::new(j1), Box::new(j2), Box::new(j3), Box::new(m1), Box::new(m2), Box::new(m3))
    }

    /// 创建 Wigner 6j
    pub fn wigner_6j(j1: Expr, j2: Expr, j3: Expr, j4: Expr, j5: Expr, j6: Expr) -> Self {
        Self::Wigner6j(Box::new(j1), Box::new(j2), Box::new(j3), Box::new(j4), Box::new(j5), Box::new(j6))
    }

    /// 创建 Wigner 9j
    #[allow(clippy::too_many_arguments)]
    pub fn wigner_9j(j1: Expr, j2: Expr, j3: Expr, j4: Expr, j5: Expr, j6: Expr, j7: Expr, j8: Expr, j9: Expr) -> Self {
        Self::Wigner9j(Box::new(j1), Box::new(j2), Box::new(j3), Box::new(j4), Box::new(j5), Box::new(j6), Box::new(j7), Box::new(j8), Box::new(j9))
    }

    // --- Theta 函数 ---

    /// 创建 Theta1
    pub fn theta1(z: Expr, q: Expr) -> Self {
        Self::Theta1(Box::new(z), Box::new(q))
    }

    /// 创建 Theta2
    pub fn theta2(z: Expr, q: Expr) -> Self {
        Self::Theta2(Box::new(z), Box::new(q))
    }

    /// 创建 Theta3
    pub fn theta3(z: Expr, q: Expr) -> Self {
        Self::Theta3(Box::new(z), Box::new(q))
    }

    /// 创建 Theta4
    pub fn theta4(z: Expr, q: Expr) -> Self {
        Self::Theta4(Box::new(z), Box::new(q))
    }

    // --- 抛物柱面函数 ---

    /// 创建抛物柱面 D
    pub fn pbdv(v: Expr, x: Expr) -> Self {
        Self::Pbdv(Box::new(v), Box::new(x))
    }

    /// 创建抛物柱面 V
    pub fn pbvv(v: Expr, x: Expr) -> Self {
        Self::Pbvv(Box::new(v), Box::new(x))
    }

    /// 创建抛物柱面 W
    pub fn pbwa(a: Expr, x: Expr) -> Self {
        Self::Pbwa(Box::new(a), Box::new(x))
    }

    // --- 球扁旋转体波函数 ---

    /// 创建长球波角函数
    pub fn pro_ang1(m: Expr, n: Expr, c: Expr, x: Expr) -> Self {
        Self::ProAng1(Box::new(m), Box::new(n), Box::new(c), Box::new(x))
    }

    /// 创建长球波径向函数第一类
    pub fn pro_rad1(m: Expr, n: Expr, c: Expr, x: Expr) -> Self {
        Self::ProRad1(Box::new(m), Box::new(n), Box::new(c), Box::new(x))
    }

    /// 创建长球波径向函数第二类
    pub fn pro_rad2(m: Expr, n: Expr, c: Expr, x: Expr) -> Self {
        Self::ProRad2(Box::new(m), Box::new(n), Box::new(c), Box::new(x))
    }

    /// 创建扁球波角函数
    pub fn obl_ang1(m: Expr, n: Expr, c: Expr, x: Expr) -> Self {
        Self::OblAng1(Box::new(m), Box::new(n), Box::new(c), Box::new(x))
    }

    /// 创建扁球波径向函数第一类
    pub fn obl_rad1(m: Expr, n: Expr, c: Expr, x: Expr) -> Self {
        Self::OblRad1(Box::new(m), Box::new(n), Box::new(c), Box::new(x))
    }

    /// 创建扁球波径向函数第二类
    pub fn obl_rad2(m: Expr, n: Expr, c: Expr, x: Expr) -> Self {
        Self::OblRad2(Box::new(m), Box::new(n), Box::new(c), Box::new(x))
    }

    // --- 修改的 Fresnel 积分 ---

    /// 创建修改 Fresnel+
    pub fn mod_fresnel_p(x: Expr) -> Self {
        Self::ModFresnelP(Box::new(x))
    }

    /// 创建修改 Fresnel-
    pub fn mod_fresnel_m(x: Expr) -> Self {
        Self::ModFresnelM(Box::new(x))
    }

    // --- Wright 函数 ---

    /// 创建 Wright Bessel
    pub fn wright_bessel(rho: Expr, beta: Expr, z: Expr) -> Self {
        Self::WrightBessel(Box::new(rho), Box::new(beta), Box::new(z))
    }

    /// 创建 Wright Omega
    pub fn wright_omega(z: Expr) -> Self {
        Self::WrightOmega(Box::new(z))
    }

    // --- Voigt ---

    /// 创建 Voigt 函数
    pub fn voigt(x: Expr, sigma: Expr, gamma: Expr) -> Self {
        Self::Voigt(Box::new(x), Box::new(sigma), Box::new(gamma))
    }

    // --- Sigmoid/Logistic ---

    /// 创建 Logit
    pub fn logit(x: Expr) -> Self {
        Self::Logit(Box::new(x))
    }

    /// 创建 Expit
    pub fn expit(x: Expr) -> Self {
        Self::Expit(Box::new(x))
    }

    // --- Box-Cox ---

    /// 创建 Box-Cox
    pub fn boxcox(x: Expr, lmbda: Expr) -> Self {
        Self::BoxCox(Box::new(x), Box::new(lmbda))
    }

    /// 创建 Box-Cox 1p
    pub fn boxcox1p(x: Expr, lmbda: Expr) -> Self {
        Self::BoxCox1p(Box::new(x), Box::new(lmbda))
    }

    /// 创建逆 Box-Cox
    pub fn inv_boxcox(y: Expr, lmbda: Expr) -> Self {
        Self::InvBoxCox(Box::new(y), Box::new(lmbda))
    }

    /// 创建逆 Box-Cox 1p
    pub fn inv_boxcox1p(y: Expr, lmbda: Expr) -> Self {
        Self::InvBoxCox1p(Box::new(y), Box::new(lmbda))
    }

    // --- 信息论 ---

    /// 创建熵
    pub fn entr(x: Expr) -> Self {
        Self::Entr(Box::new(x))
    }

    /// 创建相对熵
    pub fn rel_entr(x: Expr, y: Expr) -> Self {
        Self::RelEntr(Box::new(x), Box::new(y))
    }

    /// 创建 KL 散度
    pub fn kl_div(x: Expr, y: Expr) -> Self {
        Self::KlDiv(Box::new(x), Box::new(y))
    }

    // --- 阶乘扩展 ---

    /// 创建双阶乘
    pub fn factorial2(n: Expr) -> Self {
        Self::Factorial2(Box::new(n))
    }

    /// 创建 k 阶乘
    pub fn factorialk(n: Expr, k: Expr) -> Self {
        Self::Factorialk(Box::new(n), Box::new(k))
    }

    /// 创建 Stirling2
    pub fn stirling2(n: Expr, k: Expr) -> Self {
        Self::Stirling2(Box::new(n), Box::new(k))
    }

    /// 创建 Pochhammer
    pub fn poch(z: Expr, m: Expr) -> Self {
        Self::Poch(Box::new(z), Box::new(m))
    }

    // --- Carlson 椭圆积分 ---

    /// 创建 Carlson RC
    pub fn elliprc(x: Expr, y: Expr) -> Self {
        Self::EllipRc(Box::new(x), Box::new(y))
    }

    /// 创建 Carlson RD
    pub fn elliprd(x: Expr, y: Expr, z: Expr) -> Self {
        Self::EllipRd(Box::new(x), Box::new(y), Box::new(z))
    }

    /// 创建 Carlson RF
    pub fn elliprf(x: Expr, y: Expr, z: Expr) -> Self {
        Self::EllipRf(Box::new(x), Box::new(y), Box::new(z))
    }

    /// 创建 Carlson RG
    pub fn elliprg(x: Expr, y: Expr, z: Expr) -> Self {
        Self::EllipRg(Box::new(x), Box::new(y), Box::new(z))
    }

    /// 创建 Carlson RJ
    pub fn elliprj(x: Expr, y: Expr, z: Expr, p: Expr) -> Self {
        Self::EllipRj(Box::new(x), Box::new(y), Box::new(z), Box::new(p))
    }

    // --- 扩展误差函数 ---

    /// 创建 Erfcx
    pub fn erfcx(x: Expr) -> Self {
        Self::Erfcx(Box::new(x))
    }

    /// 创建 Erfi
    pub fn erfi(x: Expr) -> Self {
        Self::Erfi(Box::new(x))
    }

    /// 创建 Erfcinv
    pub fn erfcinv(x: Expr) -> Self {
        Self::Erfcinv(Box::new(x))
    }

    // --- 扩展 Gamma ---

    /// 创建 Hyperu
    pub fn hyperu(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Hyperu(Box::new(a), Box::new(b), Box::new(x))
    }

    /// 创建 Rgamma
    pub fn rgamma(x: Expr) -> Self {
        Self::Rgamma(Box::new(x))
    }

    /// 创建 Gammasgn
    pub fn gammasgn(x: Expr) -> Self {
        Self::Gammasgn(Box::new(x))
    }

    // --- 便利函数 ---

    /// 创建 AGM
    pub fn agm(a: Expr, b: Expr) -> Self {
        Self::Agm(Box::new(a), Box::new(b))
    }

    /// 创建 Exprel
    pub fn exprel(x: Expr) -> Self {
        Self::Exprel(Box::new(x))
    }

    /// 创建 Xlogy
    pub fn xlogy(x: Expr, y: Expr) -> Self {
        Self::Xlogy(Box::new(x), Box::new(y))
    }

    /// 创建 Xlog1py
    pub fn xlog1py(x: Expr, y: Expr) -> Self {
        Self::Xlog1py(Box::new(x), Box::new(y))
    }

    // --- Zeta 扩展 ---

    /// 创建 Hurwitz Zeta
    pub fn hurwitz_zeta(s: Expr, q: Expr) -> Self {
        Self::HurwitzZeta(Box::new(s), Box::new(q))
    }

    /// 创建 Zetac
    pub fn zetac(x: Expr) -> Self {
        Self::Zetac(Box::new(x))
    }

    /// 创建 Polylog
    pub fn polylog(s: Expr, z: Expr) -> Self {
        Self::Polylog(Box::new(s), Box::new(z))
    }

    // --- 缩放贝塞尔函数 ---

    /// 缩放修正贝塞尔 I₀
    pub fn bessel_i0e(x: Expr) -> Self {
        Self::BesselI0e(Box::new(x))
    }

    /// 缩放修正贝塞尔 I₁
    pub fn bessel_i1e(x: Expr) -> Self {
        Self::BesselI1e(Box::new(x))
    }

    /// 缩放修正贝塞尔 Iₙ
    pub fn bessel_ine(n: Expr, x: Expr) -> Self {
        Self::BesselIne(Box::new(n), Box::new(x))
    }

    /// 缩放修正贝塞尔 K₀
    pub fn bessel_k0e(x: Expr) -> Self {
        Self::BesselK0e(Box::new(x))
    }

    /// 缩放修正贝塞尔 K₁
    pub fn bessel_k1e(x: Expr) -> Self {
        Self::BesselK1e(Box::new(x))
    }

    /// 缩放修正贝塞尔 Kₙ
    pub fn bessel_kne(n: Expr, x: Expr) -> Self {
        Self::BesselKne(Box::new(n), Box::new(x))
    }

    /// 缩放贝塞尔 Jₙ
    pub fn bessel_jne(n: Expr, x: Expr) -> Self {
        Self::BesselJne(Box::new(n), Box::new(x))
    }

    /// 缩放贝塞尔 Yₙ
    pub fn bessel_yne(n: Expr, x: Expr) -> Self {
        Self::BesselYne(Box::new(n), Box::new(x))
    }

    /// 缩放 Hankel 第一类
    pub fn hankel1e(n: Expr, x: Expr) -> Self {
        Self::Hankel1e(Box::new(n), Box::new(x))
    }

    /// 缩放 Hankel 第二类
    pub fn hankel2e(n: Expr, x: Expr) -> Self {
        Self::Hankel2e(Box::new(n), Box::new(x))
    }

    // --- 贝塞尔函数导数 ---

    /// 贝塞尔 J 导数
    pub fn bessel_jnp(n: Expr, x: Expr) -> Self {
        Self::BesselJnp(Box::new(n), Box::new(x))
    }

    /// 贝塞尔 Y 导数
    pub fn bessel_ynp(n: Expr, x: Expr) -> Self {
        Self::BesselYnp(Box::new(n), Box::new(x))
    }

    /// 修正贝塞尔 I 导数
    pub fn bessel_inp(n: Expr, x: Expr) -> Self {
        Self::BesselInp(Box::new(n), Box::new(x))
    }

    /// 修正贝塞尔 K 导数
    pub fn bessel_knp(n: Expr, x: Expr) -> Self {
        Self::BesselKnp(Box::new(n), Box::new(x))
    }

    /// Hankel 1 导数
    pub fn hankel1p(n: Expr, x: Expr) -> Self {
        Self::Hankel1p(Box::new(n), Box::new(x))
    }

    /// Hankel 2 导数
    pub fn hankel2p(n: Expr, x: Expr) -> Self {
        Self::Hankel2p(Box::new(n), Box::new(x))
    }

    // --- Huber 损失函数 ---

    /// Huber 损失
    pub fn huber(delta: Expr, r: Expr) -> Self {
        Self::Huber(Box::new(delta), Box::new(r))
    }

    /// 伪 Huber 损失
    pub fn pseudo_huber(delta: Expr, r: Expr) -> Self {
        Self::PseudoHuber(Box::new(delta), Box::new(r))
    }

    // --- Kolmogorov-Smirnov 函数 ---

    /// Kolmogorov 生存函数
    pub fn kolmogorov(y: Expr) -> Self {
        Self::Kolmogorov(Box::new(y))
    }

    /// Kolmogorov 逆生存函数
    pub fn kolmogi(p: Expr) -> Self {
        Self::Kolmogi(Box::new(p))
    }

    /// Smirnov 分布
    pub fn smirnov(n: Expr, d: Expr) -> Self {
        Self::Smirnov(Box::new(n), Box::new(d))
    }

    /// Smirnov 逆函数
    pub fn smirnovi(n: Expr, p: Expr) -> Self {
        Self::Smirnovi(Box::new(n), Box::new(p))
    }

    // --- Faddeeva 函数 ---

    /// Faddeeva 函数
    pub fn wofz(z: Expr) -> Self {
        Self::Wofz(Box::new(z))
    }

    // --- Dirichlet 核 ---

    /// Dirichlet 核
    pub fn diric(x: Expr, n: Expr) -> Self {
        Self::Diric(Box::new(x), Box::new(n))
    }

    // --- Tukey lambda ---

    /// Tukey lambda PPCC
    pub fn tklmbda(x: Expr, lam: Expr) -> Self {
        Self::Tklmbda(Box::new(x), Box::new(lam))
    }

    // --- Gamma/Beta 逆函数 ---

    /// 下不完全 Gamma 逆
    pub fn gammaincinv(a: Expr, y: Expr) -> Self {
        Self::Gammaincinv(Box::new(a), Box::new(y))
    }

    /// 上不完全 Gamma 逆
    pub fn gammainccinv(a: Expr, y: Expr) -> Self {
        Self::Gammainccinv(Box::new(a), Box::new(y))
    }

    /// 正则化不完全 Beta 逆
    pub fn betaincinv(a: Expr, b: Expr, y: Expr) -> Self {
        Self::Betaincinv(Box::new(a), Box::new(b), Box::new(y))
    }

    // --- 高精度便利函数 ---

    /// cos(x) - 1
    pub fn cosm1(x: Expr) -> Self {
        Self::Cosm1(Box::new(x))
    }

    /// x^y - 1
    pub fn powm1(x: Expr, y: Expr) -> Self {
        Self::Powm1(Box::new(x), Box::new(y))
    }

    /// 10^x
    pub fn exp10(x: Expr) -> Self {
        Self::Exp10(Box::new(x))
    }

    /// log(1+x) - x
    pub fn log1pmx(x: Expr) -> Self {
        Self::Log1pmx(Box::new(x))
    }

    /// 复数 log-gamma
    pub fn loggamma(z: Expr) -> Self {
        Self::Loggamma(Box::new(z))
    }

    // --- 度数三角函数 ---

    /// 度数余弦
    pub fn cosdg(x: Expr) -> Self {
        Self::Cosdg(Box::new(x))
    }

    /// 度数正弦
    pub fn sindg(x: Expr) -> Self {
        Self::Sindg(Box::new(x))
    }

    /// 度数正切
    pub fn tandg(x: Expr) -> Self {
        Self::Tandg(Box::new(x))
    }

    /// 度数余切
    pub fn cotdg(x: Expr) -> Self {
        Self::Cotdg(Box::new(x))
    }

    /// 度分秒转弧度
    pub fn radian(d: Expr, m: Expr, s: Expr) -> Self {
        Self::Radian(Box::new(d), Box::new(m), Box::new(s))
    }

    // --- Airy 扩展 ---

    /// 缩放 Airy Ai
    pub fn airy_aie(x: Expr) -> Self {
        Self::AiryAie(Box::new(x))
    }

    /// 缩放 Airy Bi
    pub fn airy_bie(x: Expr) -> Self {
        Self::AiryBie(Box::new(x))
    }

    /// Airy 导数 Ai'
    pub fn airy_aip(x: Expr) -> Self {
        Self::AiryAip(Box::new(x))
    }

    /// Airy 导数 Bi'
    pub fn airy_bip(x: Expr) -> Self {
        Self::AiryBip(Box::new(x))
    }

    /// Airy 积分
    pub fn itairy(x: Expr) -> Self {
        Self::ItAiry(Box::new(x))
    }

    // --- 指数积分扩展 ---

    /// 广义指数积分
    pub fn expn(n: Expr, x: Expr) -> Self {
        Self::Expn(Box::new(n), Box::new(x))
    }

    /// 指数积分 E1
    pub fn exp1(x: Expr) -> Self {
        Self::Exp1(Box::new(x))
    }

    /// 双曲正弦积分
    pub fn shi(x: Expr) -> Self {
        Self::Shi(Box::new(x))
    }

    /// 双曲余弦积分
    pub fn chi(x: Expr) -> Self {
        Self::Chi(Box::new(x))
    }

    // --- Struve 积分 ---

    /// Struve 积分
    pub fn itstruve0(x: Expr) -> Self {
        Self::ItStruve0(Box::new(x))
    }

    /// Struve 二次积分
    pub fn it2struve0(x: Expr) -> Self {
        Self::It2Struve0(Box::new(x))
    }

    /// 修正 Struve 积分
    pub fn itmodstruve0(x: Expr) -> Self {
        Self::ItModStruve0(Box::new(x))
    }

    // --- ML/统计扩展 ---

    /// Log sigmoid
    pub fn log_expit(x: Expr) -> Self {
        Self::LogExpit(Box::new(x))
    }

    /// Softplus
    pub fn softplus(x: Expr) -> Self {
        Self::Softplus(Box::new(x))
    }

    /// Log ndtr
    pub fn log_ndtr(x: Expr) -> Self {
        Self::LogNdtr(Box::new(x))
    }

    // --- Beta 补函数 ---

    /// Beta 补函数
    pub fn betaincc(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Betaincc(Box::new(a), Box::new(b), Box::new(x))
    }

    /// Beta 补函数逆
    pub fn betainccinv(a: Expr, b: Expr, y: Expr) -> Self {
        Self::Betainccinv(Box::new(a), Box::new(b), Box::new(y))
    }

    // --- 数论函数 ---

    /// Bernoulli 数
    pub fn bernoulli(n: Expr) -> Self {
        Self::Bernoulli(Box::new(n))
    }

    /// Euler 数
    pub fn euler(n: Expr) -> Self {
        Self::Euler(Box::new(n))
    }

    // --- 椭圆扩展 ---

    /// 完全椭圆积分 K(1-m)
    pub fn ellipkm1(p: Expr) -> Self {
        Self::EllipKm1(Box::new(p))
    }

    // --- Kelvin 导数 ---

    /// ber 导数
    pub fn kelvin_berp(x: Expr) -> Self {
        Self::KelvinBerp(Box::new(x))
    }

    /// bei 导数
    pub fn kelvin_beip(x: Expr) -> Self {
        Self::KelvinBeip(Box::new(x))
    }

    /// ker 导数
    pub fn kelvin_kerp(x: Expr) -> Self {
        Self::KelvinKerp(Box::new(x))
    }

    /// kei 导数
    pub fn kelvin_keip(x: Expr) -> Self {
        Self::KelvinKeip(Box::new(x))
    }

    // --- 贝塞尔积分 ---

    /// 贝塞尔多项式
    pub fn besselpoly(a: Expr, lmb: Expr, nu: Expr) -> Self {
        Self::BesselPoly(Box::new(a), Box::new(lmb), Box::new(nu))
    }

    // --- Wright Bessel 扩展 ---

    /// Log Wright Bessel
    pub fn log_wright_bessel(a: Expr, b: Expr, x: Expr) -> Self {
        Self::LogWrightBessel(Box::new(a), Box::new(b), Box::new(x))
    }

    // --- 二项系数扩展 ---

    /// 实数二项系数
    pub fn binom(x: Expr, y: Expr) -> Self {
        Self::Binom(Box::new(x), Box::new(y))
    }

    // --- 分布函数 ---

    /// 二项分布 CDF
    pub fn bdtr(k: Expr, n: Expr, p: Expr) -> Self {
        Self::Bdtr(Box::new(k), Box::new(n), Box::new(p))
    }

    /// 二项分布 SF
    pub fn bdtrc(k: Expr, n: Expr, p: Expr) -> Self {
        Self::Bdtrc(Box::new(k), Box::new(n), Box::new(p))
    }

    /// 二项分布逆 CDF
    pub fn bdtri(k: Expr, n: Expr, y: Expr) -> Self {
        Self::Bdtri(Box::new(k), Box::new(n), Box::new(y))
    }

    /// 卡方分布 CDF
    pub fn chdtr(v: Expr, x: Expr) -> Self {
        Self::Chdtr(Box::new(v), Box::new(x))
    }

    /// 卡方分布 SF
    pub fn chdtrc(v: Expr, x: Expr) -> Self {
        Self::Chdtrc(Box::new(v), Box::new(x))
    }

    /// 卡方分布逆 CDF
    pub fn chdtri(v: Expr, p: Expr) -> Self {
        Self::Chdtri(Box::new(v), Box::new(p))
    }

    /// F 分布 CDF
    pub fn fdtr(dfn: Expr, dfd: Expr, x: Expr) -> Self {
        Self::Fdtr(Box::new(dfn), Box::new(dfd), Box::new(x))
    }

    /// F 分布 SF
    pub fn fdtrc(dfn: Expr, dfd: Expr, x: Expr) -> Self {
        Self::Fdtrc(Box::new(dfn), Box::new(dfd), Box::new(x))
    }

    /// F 分布逆 CDF
    pub fn fdtri(dfn: Expr, dfd: Expr, p: Expr) -> Self {
        Self::Fdtri(Box::new(dfn), Box::new(dfd), Box::new(p))
    }

    /// 学生 t 分布 CDF
    pub fn stdtr(df: Expr, t: Expr) -> Self {
        Self::Stdtr(Box::new(df), Box::new(t))
    }

    /// 学生 t 分布 SF
    pub fn stdtrc(df: Expr, t: Expr) -> Self {
        Self::Stdtrc(Box::new(df), Box::new(t))
    }

    /// 学生 t 分布逆 CDF
    pub fn stdtrit(df: Expr, p: Expr) -> Self {
        Self::Stdtrit(Box::new(df), Box::new(p))
    }

    /// 泊松分布 CDF
    pub fn pdtr(k: Expr, m: Expr) -> Self {
        Self::Pdtr(Box::new(k), Box::new(m))
    }

    /// 泊松分布 SF
    pub fn pdtrc(k: Expr, m: Expr) -> Self {
        Self::Pdtrc(Box::new(k), Box::new(m))
    }

    /// 泊松分布逆 CDF
    pub fn pdtri(k: Expr, y: Expr) -> Self {
        Self::Pdtri(Box::new(k), Box::new(y))
    }

    /// Beta 分布 CDF
    pub fn btdtr(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Btdtr(Box::new(a), Box::new(b), Box::new(x))
    }

    /// Beta 分布 SF
    pub fn btdtrc(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Btdtrc(Box::new(a), Box::new(b), Box::new(x))
    }

    /// Gamma 分布 CDF
    pub fn gdtr(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Gdtr(Box::new(a), Box::new(b), Box::new(x))
    }

    /// Gamma 分布 SF
    pub fn gdtrc(a: Expr, b: Expr, x: Expr) -> Self {
        Self::Gdtrc(Box::new(a), Box::new(b), Box::new(x))
    }

    // --- 积分函数扩展 ---

    /// 正弦积分和余弦积分组合
    pub fn sici(x: Expr) -> Self {
        Self::Sici(Box::new(x))
    }

    /// 双曲正弦积分和双曲余弦积分组合
    pub fn shichi(x: Expr) -> Self {
        Self::Shichi(Box::new(x))
    }

    // --- ML 扩展 ---

    /// Softmax
    pub fn softmax(x: Expr) -> Self {
        Self::Softmax(Box::new(x))
    }

    /// Log Softmax
    pub fn log_softmax(x: Expr) -> Self {
        Self::LogSoftmax(Box::new(x))
    }

    /// Log-Sum-Exp
    pub fn logsumexp(x: Expr) -> Self {
        Self::Logsumexp(Box::new(x))
    }

    // --- 零点计算（GSL）---

    /// Airy Ai 零点
    pub fn airy_zero_ai(s: Expr) -> Self {
        Self::AiryZeroAi(Box::new(s))
    }

    /// Airy Bi 零点
    pub fn airy_zero_bi(s: Expr) -> Self {
        Self::AiryZeroBi(Box::new(s))
    }

    /// Bessel J0 零点
    pub fn bessel_zero_j0(s: Expr) -> Self {
        Self::BesselZeroJ0(Box::new(s))
    }

    /// Bessel J1 零点
    pub fn bessel_zero_j1(s: Expr) -> Self {
        Self::BesselZeroJ1(Box::new(s))
    }

    /// Bessel Jν 零点
    pub fn bessel_zero_jnu(nu: Expr, s: Expr) -> Self {
        Self::BesselZeroJnu(Box::new(nu), Box::new(s))
    }

    // --- Legendre 扩展（GSL）---

    /// 球谐 Legendre
    pub fn sph_legendre(l: Expr, m: Expr, x: Expr) -> Self {
        Self::SphLegendre(Box::new(l), Box::new(m), Box::new(x))
    }

    // --- Clausen 函数（GSL）---

    /// Clausen 函数
    pub fn clausen(x: Expr) -> Self {
        Self::Clausen(Box::new(x))
    }

    // --- Debye 函数（GSL）---

    /// Debye 函数
    pub fn debye(n: Expr, x: Expr) -> Self {
        Self::Debye(Box::new(n), Box::new(x))
    }

    // --- Synchrotron 函数（GSL）---

    /// Synchrotron 函数 1
    pub fn synchrotron1(x: Expr) -> Self {
        Self::Synchrotron1(Box::new(x))
    }

    /// Synchrotron 函数 2
    pub fn synchrotron2(x: Expr) -> Self {
        Self::Synchrotron2(Box::new(x))
    }

    // --- Transport 函数（GSL）---

    /// Transport 函数
    pub fn transport(n: Expr, x: Expr) -> Self {
        Self::Transport(Box::new(n), Box::new(x))
    }

    // --- Fermi-Dirac 函数（GSL）---

    /// Fermi-Dirac 函数
    pub fn fermi_dirac(j: Expr, x: Expr) -> Self {
        Self::FermiDirac(Box::new(j), Box::new(x))
    }
}

// ============================================
// 分析方法
// ============================================

impl Expr {
    /// 获取所有变量引用（不包括参数）
    pub fn get_variable_refs(&self) -> Vec<String> {
        let mut refs = Vec::new();
        self.collect_refs(&mut refs, RefType::Variable);
        refs
    }

    /// 获取所有参数引用
    pub fn get_parameter_refs(&self) -> Vec<String> {
        let mut refs = Vec::new();
        self.collect_refs(&mut refs, RefType::Parameter);
        refs
    }

    /// 获取所有引用（变量 + 参数）
    pub fn get_all_refs(&self) -> Vec<String> {
        let mut refs = Vec::new();
        self.collect_refs(&mut refs, RefType::All);
        refs
    }

    /// 收集引用
    fn collect_refs(&self, refs: &mut Vec<String>, ref_type: RefType) {
        match self {
            // 叶子节点
            Expr::Const(_) | Expr::Pi | Expr::E => {}
            Expr::Reduce { arg, .. } => arg.collect_refs(refs, ref_type),
            Expr::Var(name) => {
                if matches!(ref_type, RefType::Variable | RefType::All) && !refs.contains(name) {
                    refs.push(name.clone());
                }
            }
            Expr::Param(name) => {
                if matches!(ref_type, RefType::Parameter | RefType::All) && !refs.contains(name) {
                    refs.push(name.clone());
                }
            }

            // 一元运算
            Expr::Neg(a)
            | Expr::Abs(a)
            | Expr::Ceil(a)
            | Expr::Floor(a)
            | Expr::Round(a)
            | Expr::Trunc(a)
            | Expr::Sign(a)
            | Expr::Exp(a)
            | Expr::Ln(a)
            | Expr::Log10(a)
            | Expr::Log2(a)
            | Expr::Sqrt(a)
            | Expr::Cbrt(a)
            | Expr::Sin(a)
            | Expr::Cos(a)
            | Expr::Tan(a)
            | Expr::ASin(a)
            | Expr::ACos(a)
            | Expr::ATan(a)
            | Expr::Sinh(a)
            | Expr::Cosh(a)
            | Expr::Tanh(a)
            | Expr::ASinh(a)
            | Expr::ACosh(a)
            | Expr::ATanh(a)
            | Expr::Not(a) => {
                a.collect_refs(refs, ref_type);
            }

            // 二元运算
            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b)
            | Expr::Mod(a, b)
            | Expr::ATan2(a, b)
            | Expr::Eq(a, b)
            | Expr::Lt(a, b)
            | Expr::Gt(a, b)
            | Expr::Leq(a, b)
            | Expr::Geq(a, b)
            | Expr::Neq(a, b)
            | Expr::And(a, b)
            | Expr::Or(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // 多元运算
            Expr::Max(args) | Expr::Min(args) => {
                for arg in args {
                    arg.collect_refs(refs, ref_type);
                }
            }

            // 求和/连乘
            Expr::Sum { lower, upper, body, .. }
            | Expr::Product { lower, upper, body, .. } => {
                lower.collect_refs(refs, ref_type);
                upper.collect_refs(refs, ref_type);
                body.collect_refs(refs, ref_type);
            }

            // 条件表达式
            Expr::IfThenElse { cond, then_branch, else_branch } => {
                cond.collect_refs(refs, ref_type);
                then_branch.collect_refs(refs, ref_type);
                else_branch.collect_refs(refs, ref_type);
            }

            Expr::Piecewise { pieces, otherwise } => {
                for (cond, value) in pieces {
                    cond.collect_refs(refs, ref_type);
                    value.collect_refs(refs, ref_type);
                }
                otherwise.collect_refs(refs, ref_type);
            }

            // 扩展一元运算
            Expr::ComplexSinh(a)
            | Expr::ComplexCosh(a)
            | Expr::ComplexTanh(a)
            | Expr::ComplexAsinh(a)
            | Expr::ComplexAcosh(a)
            | Expr::ComplexAtanh(a)
            | Expr::ComplexAsin(a)
            | Expr::ComplexAcos(a)
            | Expr::ComplexAtan(a)
            | Expr::EllipK(a)
            | Expr::EllipE(a)
            | Expr::VecNorm(a)
            | Expr::VecNormalize(a)
            | Expr::Transpose(a)
            | Expr::Det(a)
            | Expr::Inv(a)
            | Expr::Eigenvalues(a)
            | Expr::Trace(a)
            | Expr::MatNorm(a) => {
                a.collect_refs(refs, ref_type);
            }

            // 扩展二元运算
            Expr::ExpPpf(a, b)
            | Expr::Gcd(a, b)
            | Expr::Lcm(a, b)
            | Expr::Permutation(a, b)
            | Expr::Legendre(a, b)
            | Expr::Hermite(a, b)
            | Expr::Laguerre(a, b)
            | Expr::ChebyshevT(a, b)
            | Expr::ChebyshevU(a, b)
            | Expr::Dot(a, b)
            | Expr::Cross(a, b)
            | Expr::MatMul(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // 三元运算
            Expr::GammaPpf(a, b, c)
            | Expr::BetaPpf(a, b, c)
            | Expr::WeibullPpf(a, b, c)
            | Expr::LognormPpf(a, b, c)
            | Expr::UniformPpf(a, b, c)
            | Expr::CauchyPpf(a, b, c)
            | Expr::LegendreAssoc(a, b, c)
            | Expr::LaguerreAssoc(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // Lambda 和微积分
            Expr::Lambda { body, .. } => {
                body.collect_refs(refs, ref_type);
            }
            Expr::Integrate { lower, upper, body, .. } => {
                lower.collect_refs(refs, ref_type);
                upper.collect_refs(refs, ref_type);
                body.collect_refs(refs, ref_type);
            }
            Expr::Derivative { body, at, .. } => {
                body.collect_refs(refs, ref_type);
                at.collect_refs(refs, ref_type);
            }
            Expr::Limit { to, body, .. } => {
                to.collect_refs(refs, ref_type);
                body.collect_refs(refs, ref_type);
            }

            // 向量/矩阵字面量
            Expr::VectorLit(elements) => {
                for e in elements {
                    e.collect_refs(refs, ref_type);
                }
            }
            Expr::MatrixLit(rows) => {
                for row in rows {
                    for e in row {
                        e.collect_refs(refs, ref_type);
                    }
                }
            }

            // 扩展一元函数
            Expr::Gamma(a) | Expr::Lgamma(a) | Expr::Digamma(a)
            | Expr::Erf(a) | Expr::Erfc(a) | Expr::Erfinv(a)
            | Expr::Factorial(a) | Expr::Zeta(a)
            | Expr::BesselJ0(a) | Expr::BesselJ1(a)
            | Expr::BesselY0(a) | Expr::BesselY1(a)
            | Expr::BesselI0(a) | Expr::BesselI1(a)
            | Expr::BesselK0(a) | Expr::BesselK1(a)
            | Expr::Real(a) | Expr::Imag(a) | Expr::Conj(a)
            | Expr::Carg(a) | Expr::Cabs(a) | Expr::Sinc(a)
            | Expr::Expm1(a) | Expr::Log1p(a) | Expr::Exp2(a)
            | Expr::Sec(a) | Expr::Csc(a) | Expr::Cot(a)
            | Expr::Asec(a) | Expr::Acsc(a) | Expr::Acot(a)
            | Expr::Sech(a) | Expr::Csch(a) | Expr::Coth(a)
            | Expr::Asech(a) | Expr::Acsch(a) | Expr::Acoth(a)
            | Expr::AiryAi(a) | Expr::AiryBi(a)
            | Expr::FresnelS(a) | Expr::FresnelC(a)
            | Expr::Dawson(a) | Expr::ExpInt(a) | Expr::LogInt(a)
            | Expr::SinInt(a) | Expr::CosInt(a)
            | Expr::LambertW(a) | Expr::LambertWm1(a)
            | Expr::KelvinBer(a) | Expr::KelvinBei(a)
            | Expr::KelvinKer(a) | Expr::KelvinKei(a)
            | Expr::Spence(a)
            | Expr::RiemannSiegelZ(a) | Expr::RiemannSiegelTheta(a) => {
                a.collect_refs(refs, ref_type);
            }

            // 新增二元函数
            Expr::JacobiSn(a, b) | Expr::JacobiCn(a, b) | Expr::JacobiDn(a, b)
            | Expr::MathieuA(a, b) | Expr::MathieuB(a, b)
            | Expr::Theta1(a, b) | Expr::Theta2(a, b) | Expr::Theta3(a, b) | Expr::Theta4(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // 新增三元函数
            Expr::Gegenbauer(a, b, c) | Expr::MathieuCe(a, b, c) | Expr::MathieuSe(a, b, c)
            | Expr::CoulombF(a, b, c) | Expr::CoulombG(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // 新增四元函数
            Expr::JacobiP(a, b, c, d) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
                d.collect_refs(refs, ref_type);
            }

            // Wigner 符号（六元和九元）
            Expr::Wigner3j(a, b, c, d, e, f) | Expr::Wigner6j(a, b, c, d, e, f) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
                d.collect_refs(refs, ref_type);
                e.collect_refs(refs, ref_type);
                f.collect_refs(refs, ref_type);
            }

            Expr::Wigner9j(a, b, c, d, e, f, g, h, i) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
                d.collect_refs(refs, ref_type);
                e.collect_refs(refs, ref_type);
                f.collect_refs(refs, ref_type);
                g.collect_refs(refs, ref_type);
                h.collect_refs(refs, ref_type);
                i.collect_refs(refs, ref_type);
            }

            // 抛物柱面函数
            Expr::Pbdv(a, b) | Expr::Pbvv(a, b) | Expr::Pbwa(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // 球扁旋转体波函数（四元）
            Expr::ProAng1(a, b, c, d) | Expr::ProRad1(a, b, c, d) | Expr::ProRad2(a, b, c, d)
            | Expr::OblAng1(a, b, c, d) | Expr::OblRad1(a, b, c, d) | Expr::OblRad2(a, b, c, d) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
                d.collect_refs(refs, ref_type);
            }

            // 修改 Fresnel 和 Wright Omega
            Expr::ModFresnelP(a) | Expr::ModFresnelM(a) | Expr::WrightOmega(a) => {
                a.collect_refs(refs, ref_type);
            }

            // Wright Bessel 和 Voigt
            Expr::WrightBessel(a, b, c) | Expr::Voigt(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // 新增一元函数
            Expr::Logit(a) | Expr::Expit(a) | Expr::Entr(a) | Expr::Factorial2(a)
            | Expr::Erfcx(a) | Expr::Erfi(a) | Expr::Erfcinv(a)
            | Expr::Rgamma(a) | Expr::Gammasgn(a) | Expr::Exprel(a) | Expr::Zetac(a) => {
                a.collect_refs(refs, ref_type);
            }

            // 新增二元函数
            Expr::BoxCox(a, b) | Expr::BoxCox1p(a, b) | Expr::InvBoxCox(a, b) | Expr::InvBoxCox1p(a, b)
            | Expr::RelEntr(a, b) | Expr::KlDiv(a, b)
            | Expr::Factorialk(a, b) | Expr::Stirling2(a, b) | Expr::Poch(a, b)
            | Expr::EllipRc(a, b) | Expr::Agm(a, b) | Expr::Xlogy(a, b) | Expr::Xlog1py(a, b)
            | Expr::HurwitzZeta(a, b) | Expr::Polylog(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // 新增三元函数
            Expr::EllipRd(a, b, c) | Expr::EllipRf(a, b, c) | Expr::EllipRg(a, b, c)
            | Expr::Hyperu(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // 新增四元函数
            Expr::EllipRj(a, b, c, d) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
                d.collect_refs(refs, ref_type);
            }

            // 扩展二元函数
            Expr::Beta(a, b) | Expr::Lbeta(a, b) | Expr::Combination(a, b)
            | Expr::BesselJn(a, b) | Expr::BesselYn(a, b)
            | Expr::BesselIn(a, b) | Expr::BesselKn(a, b)
            | Expr::TPdf(a, b) | Expr::TCdf(a, b) | Expr::TPpf(a, b)
            | Expr::Chi2Pdf(a, b) | Expr::Chi2Cdf(a, b) | Expr::Chi2Ppf(a, b)
            | Expr::PoissonPmf(a, b) | Expr::PoissonCdf(a, b)
            | Expr::ExponentialPdf(a, b) | Expr::ExponentialCdf(a, b)
            | Expr::Complex(a, b) | Expr::Polar(a, b)
            | Expr::Hypot(a, b) | Expr::Copysign(a, b) | Expr::Logn(a, b)
            | Expr::Gammainc(a, b) | Expr::Gammaincc(a, b)
            | Expr::SphBesselJ(a, b) | Expr::SphBesselY(a, b)
            | Expr::SphBesselI(a, b) | Expr::SphBesselK(a, b)
            | Expr::Hyp0f1(a, b)
            | Expr::EllipF(a, b) | Expr::EllipEInc(a, b)
            | Expr::Polygamma(a, b) | Expr::Hankel1(a, b) | Expr::Hankel2(a, b)
            | Expr::StruveH(a, b) | Expr::StruveL(a, b) | Expr::OwensT(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // 扩展三元函数
            Expr::NormPdf(a, b, c) | Expr::NormCdf(a, b, c) | Expr::NormPpf(a, b, c)
            | Expr::Betainc(a, b, c)
            | Expr::Hyp1f1(a, b, c) | Expr::EllipPi(a, b, c)
            | Expr::FPdf(a, b, c) | Expr::FCdf(a, b, c) | Expr::FPpf(a, b, c)
            | Expr::BinomialPmf(a, b, c) | Expr::BinomialCdf(a, b, c)
            | Expr::Hypot3(a, b, c) | Expr::Clamp(a, b, c) | Expr::Fma(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // 四元函数
            Expr::SphericalHarmonic(a, b, c, d) | Expr::Hyp2f1(a, b, c, d) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
                d.collect_refs(refs, ref_type);
            }

            // === 新增运算符：缩放贝塞尔（一元）===
            Expr::BesselI0e(a) | Expr::BesselI1e(a) | Expr::BesselK0e(a) | Expr::BesselK1e(a)
            | Expr::Kolmogorov(a) | Expr::Kolmogi(a) | Expr::Wofz(a)
            | Expr::Cosm1(a) | Expr::Exp10(a) | Expr::Log1pmx(a) | Expr::Loggamma(a)
            | Expr::Cosdg(a) | Expr::Sindg(a) | Expr::Tandg(a) | Expr::Cotdg(a) => {
                a.collect_refs(refs, ref_type);
            }

            // === 新增运算符：缩放贝塞尔/导数（二元）===
            Expr::BesselIne(a, b) | Expr::BesselKne(a, b)
            | Expr::BesselJne(a, b) | Expr::BesselYne(a, b)
            | Expr::Hankel1e(a, b) | Expr::Hankel2e(a, b)
            | Expr::BesselJnp(a, b) | Expr::BesselYnp(a, b)
            | Expr::BesselInp(a, b) | Expr::BesselKnp(a, b)
            | Expr::Hankel1p(a, b) | Expr::Hankel2p(a, b)
            | Expr::Huber(a, b) | Expr::PseudoHuber(a, b)
            | Expr::Smirnov(a, b) | Expr::Smirnovi(a, b)
            | Expr::Diric(a, b) | Expr::Tklmbda(a, b)
            | Expr::Gammaincinv(a, b) | Expr::Gammainccinv(a, b)
            | Expr::Powm1(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            // === 新增运算符：三元 ===
            Expr::Betaincinv(a, b, c) | Expr::Radian(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // === 新增运算符（最终批次）===
            Expr::AiryAie(a) | Expr::AiryBie(a) | Expr::AiryAip(a) | Expr::AiryBip(a)
            | Expr::ItAiry(a) | Expr::Exp1(a) | Expr::Shi(a) | Expr::Chi(a)
            | Expr::ItStruve0(a) | Expr::It2Struve0(a) | Expr::ItModStruve0(a)
            | Expr::LogExpit(a) | Expr::Softplus(a) | Expr::LogNdtr(a)
            | Expr::Bernoulli(a) | Expr::Euler(a) | Expr::EllipKm1(a)
            | Expr::KelvinBerp(a) | Expr::KelvinBeip(a) | Expr::KelvinKerp(a) | Expr::KelvinKeip(a) => {
                a.collect_refs(refs, ref_type);
            }

            Expr::Expn(a, b) | Expr::Binom(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            Expr::Betaincc(a, b, c) | Expr::Betainccinv(a, b, c)
            | Expr::BesselPoly(a, b, c) | Expr::LogWrightBessel(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // === 分布函数 ===
            Expr::Chdtr(a, b) | Expr::Chdtrc(a, b) | Expr::Chdtri(a, b)
            | Expr::Stdtr(a, b) | Expr::Stdtrc(a, b) | Expr::Stdtrit(a, b)
            | Expr::Pdtr(a, b) | Expr::Pdtrc(a, b) | Expr::Pdtri(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            Expr::Bdtr(a, b, c) | Expr::Bdtrc(a, b, c) | Expr::Bdtri(a, b, c)
            | Expr::Fdtr(a, b, c) | Expr::Fdtrc(a, b, c) | Expr::Fdtri(a, b, c)
            | Expr::Btdtr(a, b, c) | Expr::Btdtrc(a, b, c)
            | Expr::Gdtr(a, b, c) | Expr::Gdtrc(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }

            // === 积分/ML 扩展 ===
            Expr::Sici(a) | Expr::Shichi(a)
            | Expr::Softmax(a) | Expr::LogSoftmax(a) | Expr::Logsumexp(a) => {
                a.collect_refs(refs, ref_type);
            }

            // === GSL 扩展 ===
            Expr::AiryZeroAi(a) | Expr::AiryZeroBi(a) | Expr::BesselZeroJ0(a) | Expr::BesselZeroJ1(a)
            | Expr::Clausen(a) | Expr::Synchrotron1(a) | Expr::Synchrotron2(a) => {
                a.collect_refs(refs, ref_type);
            }

            Expr::BesselZeroJnu(a, b) | Expr::Debye(a, b) | Expr::Transport(a, b) | Expr::FermiDirac(a, b) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
            }

            Expr::SphLegendre(a, b, c) => {
                a.collect_refs(refs, ref_type);
                b.collect_refs(refs, ref_type);
                c.collect_refs(refs, ref_type);
            }
        }
    }

    /// 检查表达式中是否包含指定变量
    pub fn contains_ref(&self, name: &str) -> bool {
        self.get_all_refs().contains(&name.to_string())
    }

    /// 计算表达式深度
    pub fn depth(&self) -> usize {
        match self {
            Expr::Const(_) | Expr::Var(_) | Expr::Param(_) | Expr::Pi | Expr::E => 1,

            Expr::Reduce { arg, .. } => 1 + arg.depth(),

            Expr::Neg(a)
            | Expr::Abs(a)
            | Expr::Ceil(a)
            | Expr::Floor(a)
            | Expr::Round(a)
            | Expr::Trunc(a)
            | Expr::Sign(a)
            | Expr::Exp(a)
            | Expr::Ln(a)
            | Expr::Log10(a)
            | Expr::Log2(a)
            | Expr::Sqrt(a)
            | Expr::Cbrt(a)
            | Expr::Sin(a)
            | Expr::Cos(a)
            | Expr::Tan(a)
            | Expr::ASin(a)
            | Expr::ACos(a)
            | Expr::ATan(a)
            | Expr::Sinh(a)
            | Expr::Cosh(a)
            | Expr::Tanh(a)
            | Expr::ASinh(a)
            | Expr::ACosh(a)
            | Expr::ATanh(a)
            | Expr::Not(a) => 1 + a.depth(),

            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b)
            | Expr::Mod(a, b)
            | Expr::ATan2(a, b)
            | Expr::Eq(a, b)
            | Expr::Lt(a, b)
            | Expr::Gt(a, b)
            | Expr::Leq(a, b)
            | Expr::Geq(a, b)
            | Expr::Neq(a, b)
            | Expr::And(a, b)
            | Expr::Or(a, b) => 1 + a.depth().max(b.depth()),

            Expr::Max(args) | Expr::Min(args) => {
                1 + args.iter().map(|a| a.depth()).max().unwrap_or(0)
            }

            Expr::Sum { body, .. } | Expr::Product { body, .. } => 1 + body.depth(),

            Expr::IfThenElse { cond, then_branch, else_branch } => {
                1 + cond.depth().max(then_branch.depth()).max(else_branch.depth())
            }

            Expr::Piecewise { pieces, otherwise } => {
                let pieces_depth = pieces
                    .iter()
                    .map(|(c, v)| c.depth().max(v.depth()))
                    .max()
                    .unwrap_or(0);
                1 + pieces_depth.max(otherwise.depth())
            }

            // 扩展一元运算
            Expr::ComplexSinh(a)
            | Expr::ComplexCosh(a)
            | Expr::ComplexTanh(a)
            | Expr::ComplexAsinh(a)
            | Expr::ComplexAcosh(a)
            | Expr::ComplexAtanh(a)
            | Expr::ComplexAsin(a)
            | Expr::ComplexAcos(a)
            | Expr::ComplexAtan(a)
            | Expr::EllipK(a)
            | Expr::EllipE(a)
            | Expr::VecNorm(a)
            | Expr::VecNormalize(a)
            | Expr::Transpose(a)
            | Expr::Det(a)
            | Expr::Inv(a)
            | Expr::Eigenvalues(a)
            | Expr::Trace(a)
            | Expr::MatNorm(a) => 1 + a.depth(),

            // 扩展二元运算
            Expr::ExpPpf(a, b)
            | Expr::Gcd(a, b)
            | Expr::Lcm(a, b)
            | Expr::Permutation(a, b)
            | Expr::Legendre(a, b)
            | Expr::Hermite(a, b)
            | Expr::Laguerre(a, b)
            | Expr::ChebyshevT(a, b)
            | Expr::ChebyshevU(a, b)
            | Expr::Dot(a, b)
            | Expr::Cross(a, b)
            | Expr::MatMul(a, b) => 1 + a.depth().max(b.depth()),

            // 三元运算
            Expr::GammaPpf(a, b, c)
            | Expr::BetaPpf(a, b, c)
            | Expr::WeibullPpf(a, b, c)
            | Expr::LognormPpf(a, b, c)
            | Expr::UniformPpf(a, b, c)
            | Expr::CauchyPpf(a, b, c)
            | Expr::LegendreAssoc(a, b, c)
            | Expr::LaguerreAssoc(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // Lambda 和微积分
            Expr::Lambda { body, .. } => 1 + body.depth(),
            Expr::Integrate { lower, upper, body, .. } => {
                1 + lower.depth().max(upper.depth()).max(body.depth())
            }
            Expr::Derivative { body, at, .. } => 1 + body.depth().max(at.depth()),
            Expr::Limit { to, body, .. } => 1 + to.depth().max(body.depth()),

            // 向量/矩阵字面量
            Expr::VectorLit(elements) => {
                1 + elements.iter().map(|e| e.depth()).max().unwrap_or(0)
            }
            Expr::MatrixLit(rows) => {
                1 + rows.iter()
                    .flat_map(|r| r.iter())
                    .map(|e| e.depth())
                    .max()
                    .unwrap_or(0)
            }

            // 扩展一元函数
            Expr::Gamma(a) | Expr::Lgamma(a) | Expr::Digamma(a)
            | Expr::Erf(a) | Expr::Erfc(a) | Expr::Erfinv(a)
            | Expr::Factorial(a) | Expr::Zeta(a)
            | Expr::BesselJ0(a) | Expr::BesselJ1(a)
            | Expr::BesselY0(a) | Expr::BesselY1(a)
            | Expr::BesselI0(a) | Expr::BesselI1(a)
            | Expr::BesselK0(a) | Expr::BesselK1(a)
            | Expr::Real(a) | Expr::Imag(a) | Expr::Conj(a)
            | Expr::Carg(a) | Expr::Cabs(a) | Expr::Sinc(a)
            | Expr::Expm1(a) | Expr::Log1p(a) | Expr::Exp2(a)
            | Expr::Sec(a) | Expr::Csc(a) | Expr::Cot(a)
            | Expr::Asec(a) | Expr::Acsc(a) | Expr::Acot(a)
            | Expr::Sech(a) | Expr::Csch(a) | Expr::Coth(a)
            | Expr::Asech(a) | Expr::Acsch(a) | Expr::Acoth(a)
            | Expr::AiryAi(a) | Expr::AiryBi(a)
            | Expr::FresnelS(a) | Expr::FresnelC(a)
            | Expr::Dawson(a) | Expr::ExpInt(a) | Expr::LogInt(a)
            | Expr::SinInt(a) | Expr::CosInt(a)
            | Expr::LambertW(a) | Expr::LambertWm1(a)
            | Expr::KelvinBer(a) | Expr::KelvinBei(a)
            | Expr::KelvinKer(a) | Expr::KelvinKei(a)
            | Expr::Spence(a)
            | Expr::RiemannSiegelZ(a) | Expr::RiemannSiegelTheta(a) => 1 + a.depth(),

            // 新增二元函数
            Expr::JacobiSn(a, b) | Expr::JacobiCn(a, b) | Expr::JacobiDn(a, b)
            | Expr::MathieuA(a, b) | Expr::MathieuB(a, b)
            | Expr::Theta1(a, b) | Expr::Theta2(a, b) | Expr::Theta3(a, b) | Expr::Theta4(a, b) => {
                1 + a.depth().max(b.depth())
            }

            // 新增三元函数
            Expr::Gegenbauer(a, b, c) | Expr::MathieuCe(a, b, c) | Expr::MathieuSe(a, b, c)
            | Expr::CoulombF(a, b, c) | Expr::CoulombG(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // 新增四元函数
            Expr::JacobiP(a, b, c, d) => {
                1 + a.depth().max(b.depth()).max(c.depth()).max(d.depth())
            }

            // Wigner 符号
            Expr::Wigner3j(a, b, c, d, e, f) | Expr::Wigner6j(a, b, c, d, e, f) => {
                1 + a.depth().max(b.depth()).max(c.depth()).max(d.depth()).max(e.depth()).max(f.depth())
            }

            Expr::Wigner9j(a, b, c, d, e, f, g, h, i) => {
                1 + a.depth().max(b.depth()).max(c.depth()).max(d.depth()).max(e.depth()).max(f.depth()).max(g.depth()).max(h.depth()).max(i.depth())
            }

            // 抛物柱面函数
            Expr::Pbdv(a, b) | Expr::Pbvv(a, b) | Expr::Pbwa(a, b) => {
                1 + a.depth().max(b.depth())
            }

            // 球扁旋转体波函数
            Expr::ProAng1(a, b, c, d) | Expr::ProRad1(a, b, c, d) | Expr::ProRad2(a, b, c, d)
            | Expr::OblAng1(a, b, c, d) | Expr::OblRad1(a, b, c, d) | Expr::OblRad2(a, b, c, d) => {
                1 + a.depth().max(b.depth()).max(c.depth()).max(d.depth())
            }

            // 修改 Fresnel 和 Wright Omega
            Expr::ModFresnelP(a) | Expr::ModFresnelM(a) | Expr::WrightOmega(a) => 1 + a.depth(),

            // Wright Bessel 和 Voigt
            Expr::WrightBessel(a, b, c) | Expr::Voigt(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // 新增一元函数
            Expr::Logit(a) | Expr::Expit(a) | Expr::Entr(a) | Expr::Factorial2(a)
            | Expr::Erfcx(a) | Expr::Erfi(a) | Expr::Erfcinv(a)
            | Expr::Rgamma(a) | Expr::Gammasgn(a) | Expr::Exprel(a) | Expr::Zetac(a) => 1 + a.depth(),

            // 新增二元函数
            Expr::BoxCox(a, b) | Expr::BoxCox1p(a, b) | Expr::InvBoxCox(a, b) | Expr::InvBoxCox1p(a, b)
            | Expr::RelEntr(a, b) | Expr::KlDiv(a, b)
            | Expr::Factorialk(a, b) | Expr::Stirling2(a, b) | Expr::Poch(a, b)
            | Expr::EllipRc(a, b) | Expr::Agm(a, b) | Expr::Xlogy(a, b) | Expr::Xlog1py(a, b)
            | Expr::HurwitzZeta(a, b) | Expr::Polylog(a, b) => 1 + a.depth().max(b.depth()),

            // 新增三元函数
            Expr::EllipRd(a, b, c) | Expr::EllipRf(a, b, c) | Expr::EllipRg(a, b, c)
            | Expr::Hyperu(a, b, c) => 1 + a.depth().max(b.depth()).max(c.depth()),

            // 新增四元函数
            Expr::EllipRj(a, b, c, d) => 1 + a.depth().max(b.depth()).max(c.depth()).max(d.depth()),

            // 扩展二元函数
            Expr::Beta(a, b) | Expr::Lbeta(a, b) | Expr::Combination(a, b)
            | Expr::BesselJn(a, b) | Expr::BesselYn(a, b)
            | Expr::BesselIn(a, b) | Expr::BesselKn(a, b)
            | Expr::TPdf(a, b) | Expr::TCdf(a, b) | Expr::TPpf(a, b)
            | Expr::Chi2Pdf(a, b) | Expr::Chi2Cdf(a, b) | Expr::Chi2Ppf(a, b)
            | Expr::PoissonPmf(a, b) | Expr::PoissonCdf(a, b)
            | Expr::ExponentialPdf(a, b) | Expr::ExponentialCdf(a, b)
            | Expr::Complex(a, b) | Expr::Polar(a, b)
            | Expr::Hypot(a, b) | Expr::Copysign(a, b) | Expr::Logn(a, b)
            | Expr::Gammainc(a, b) | Expr::Gammaincc(a, b)
            | Expr::SphBesselJ(a, b) | Expr::SphBesselY(a, b)
            | Expr::SphBesselI(a, b) | Expr::SphBesselK(a, b)
            | Expr::Hyp0f1(a, b)
            | Expr::EllipF(a, b) | Expr::EllipEInc(a, b)
            | Expr::Polygamma(a, b) | Expr::Hankel1(a, b) | Expr::Hankel2(a, b)
            | Expr::StruveH(a, b) | Expr::StruveL(a, b) | Expr::OwensT(a, b) => {
                1 + a.depth().max(b.depth())
            }

            // 扩展三元函数
            Expr::NormPdf(a, b, c) | Expr::NormCdf(a, b, c) | Expr::NormPpf(a, b, c)
            | Expr::Betainc(a, b, c)
            | Expr::Hyp1f1(a, b, c) | Expr::EllipPi(a, b, c)
            | Expr::FPdf(a, b, c) | Expr::FCdf(a, b, c) | Expr::FPpf(a, b, c)
            | Expr::BinomialPmf(a, b, c) | Expr::BinomialCdf(a, b, c)
            | Expr::Hypot3(a, b, c) | Expr::Clamp(a, b, c) | Expr::Fma(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // 四元函数
            Expr::SphericalHarmonic(a, b, c, d) | Expr::Hyp2f1(a, b, c, d) => {
                1 + a.depth().max(b.depth()).max(c.depth()).max(d.depth())
            }

            // === 新增运算符：缩放贝塞尔（一元）===
            Expr::BesselI0e(a) | Expr::BesselI1e(a) | Expr::BesselK0e(a) | Expr::BesselK1e(a)
            | Expr::Kolmogorov(a) | Expr::Kolmogi(a) | Expr::Wofz(a)
            | Expr::Cosm1(a) | Expr::Exp10(a) | Expr::Log1pmx(a) | Expr::Loggamma(a)
            | Expr::Cosdg(a) | Expr::Sindg(a) | Expr::Tandg(a) | Expr::Cotdg(a) => 1 + a.depth(),

            // === 新增运算符：缩放贝塞尔/导数（二元）===
            Expr::BesselIne(a, b) | Expr::BesselKne(a, b)
            | Expr::BesselJne(a, b) | Expr::BesselYne(a, b)
            | Expr::Hankel1e(a, b) | Expr::Hankel2e(a, b)
            | Expr::BesselJnp(a, b) | Expr::BesselYnp(a, b)
            | Expr::BesselInp(a, b) | Expr::BesselKnp(a, b)
            | Expr::Hankel1p(a, b) | Expr::Hankel2p(a, b)
            | Expr::Huber(a, b) | Expr::PseudoHuber(a, b)
            | Expr::Smirnov(a, b) | Expr::Smirnovi(a, b)
            | Expr::Diric(a, b) | Expr::Tklmbda(a, b)
            | Expr::Gammaincinv(a, b) | Expr::Gammainccinv(a, b)
            | Expr::Powm1(a, b) => 1 + a.depth().max(b.depth()),

            // === 新增运算符：三元 ===
            Expr::Betaincinv(a, b, c) | Expr::Radian(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // === 新增运算符（最终批次）===
            Expr::AiryAie(a) | Expr::AiryBie(a) | Expr::AiryAip(a) | Expr::AiryBip(a)
            | Expr::ItAiry(a) | Expr::Exp1(a) | Expr::Shi(a) | Expr::Chi(a)
            | Expr::ItStruve0(a) | Expr::It2Struve0(a) | Expr::ItModStruve0(a)
            | Expr::LogExpit(a) | Expr::Softplus(a) | Expr::LogNdtr(a)
            | Expr::Bernoulli(a) | Expr::Euler(a) | Expr::EllipKm1(a)
            | Expr::KelvinBerp(a) | Expr::KelvinBeip(a) | Expr::KelvinKerp(a) | Expr::KelvinKeip(a) => 1 + a.depth(),

            Expr::Expn(a, b) | Expr::Binom(a, b) => 1 + a.depth().max(b.depth()),

            Expr::Betaincc(a, b, c) | Expr::Betainccinv(a, b, c)
            | Expr::BesselPoly(a, b, c) | Expr::LogWrightBessel(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // === 分布函数 ===
            Expr::Chdtr(a, b) | Expr::Chdtrc(a, b) | Expr::Chdtri(a, b)
            | Expr::Stdtr(a, b) | Expr::Stdtrc(a, b) | Expr::Stdtrit(a, b)
            | Expr::Pdtr(a, b) | Expr::Pdtrc(a, b) | Expr::Pdtri(a, b) => 1 + a.depth().max(b.depth()),

            Expr::Bdtr(a, b, c) | Expr::Bdtrc(a, b, c) | Expr::Bdtri(a, b, c)
            | Expr::Fdtr(a, b, c) | Expr::Fdtrc(a, b, c) | Expr::Fdtri(a, b, c)
            | Expr::Btdtr(a, b, c) | Expr::Btdtrc(a, b, c)
            | Expr::Gdtr(a, b, c) | Expr::Gdtrc(a, b, c) => {
                1 + a.depth().max(b.depth()).max(c.depth())
            }

            // === 积分/ML 扩展 ===
            Expr::Sici(a) | Expr::Shichi(a)
            | Expr::Softmax(a) | Expr::LogSoftmax(a) | Expr::Logsumexp(a) => 1 + a.depth(),

            // === GSL 扩展 ===
            Expr::AiryZeroAi(a) | Expr::AiryZeroBi(a) | Expr::BesselZeroJ0(a) | Expr::BesselZeroJ1(a)
            | Expr::Clausen(a) | Expr::Synchrotron1(a) | Expr::Synchrotron2(a) => 1 + a.depth(),

            Expr::BesselZeroJnu(a, b) | Expr::Debye(a, b) | Expr::Transport(a, b) | Expr::FermiDirac(a, b) => 1 + a.depth().max(b.depth()),

            Expr::SphLegendre(a, b, c) => 1 + a.depth().max(b.depth()).max(c.depth()),
        }
    }
}

/// 引用类型（用于收集时区分）
#[derive(Clone, Copy)]
enum RefType {
    Variable,
    Parameter,
    All,
}

// ============================================
// 代码生成方法
// ============================================

impl Expr {
    /// 转换为 Python 代码
    pub fn to_python(&self, params_prefix: &str) -> String {
        // 注册表快路径：已迁移算子从 ops 注册表生成（单一真相源）。
        // 下方 match 中这些算子的分支已不可达，仅为保持 match 穷尽性而保留，待后续清理。
        if let Some((name, args)) = crate::ops::as_operator(self) {
            if let Some(s) = crate::ops::spec(name) {
                let codes: Vec<String> =
                    args.iter().map(|a| a.to_python(params_prefix)).collect();
                return (s.python)(&codes);
            }
        }
        match self {
            Expr::Const(value) => format!("{}", value),
            Expr::Var(name) => name.clone(),
            Expr::Param(name) => format!("{}.{}", params_prefix, name),
            Expr::Pi => "np.pi".to_string(),
            Expr::E => "np.e".to_string(),
            Expr::Reduce { kind, arg } => {
                let f = match kind {
                    ReduceKind::Sum => "sum",
                    ReduceKind::Prod => "prod",
                    ReduceKind::Mean => "mean",
                    ReduceKind::Min => "min",
                    ReduceKind::Max => "max",
                };
                format!("np.{}({})", f, arg.to_python(params_prefix))
            }

            // 算术运算
            Expr::Add(a, b) => format!("({} + {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Sub(a, b) => format!("({} - {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Mul(a, b) => format!("({} * {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Div(a, b) => format!("({} / {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Neg(a) => format!("(-{})", a.to_python(params_prefix)),
            Expr::Pow(a, b) => format!("({} ** {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Abs(a) => format!("np.abs({})", a.to_python(params_prefix)),
            Expr::Mod(a, b) => format!("np.mod({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Ceil(a) => format!("np.ceil({})", a.to_python(params_prefix)),
            Expr::Floor(a) => format!("np.floor({})", a.to_python(params_prefix)),
            Expr::Round(a) => format!("np.round({})", a.to_python(params_prefix)),
            Expr::Trunc(a) => format!("np.trunc({})", a.to_python(params_prefix)),
            Expr::Sign(a) => format!("np.sign({})", a.to_python(params_prefix)),

            // 超越函数
            Expr::Exp(a) => format!("np.exp({})", a.to_python(params_prefix)),
            Expr::Ln(a) => format!("np.log({})", a.to_python(params_prefix)),
            Expr::Log10(a) => format!("np.log10({})", a.to_python(params_prefix)),
            Expr::Log2(a) => format!("np.log2({})", a.to_python(params_prefix)),
            Expr::Sqrt(a) => format!("np.sqrt({})", a.to_python(params_prefix)),
            Expr::Cbrt(a) => format!("np.cbrt({})", a.to_python(params_prefix)),

            // 三角函数
            Expr::Sin(a) => format!("np.sin({})", a.to_python(params_prefix)),
            Expr::Cos(a) => format!("np.cos({})", a.to_python(params_prefix)),
            Expr::Tan(a) => format!("np.tan({})", a.to_python(params_prefix)),
            Expr::ASin(a) => format!("np.arcsin({})", a.to_python(params_prefix)),
            Expr::ACos(a) => format!("np.arccos({})", a.to_python(params_prefix)),
            Expr::ATan(a) => format!("np.arctan({})", a.to_python(params_prefix)),
            Expr::ATan2(y, x) => format!("np.arctan2({}, {})", y.to_python(params_prefix), x.to_python(params_prefix)),

            // 双曲函数
            Expr::Sinh(a) => format!("np.sinh({})", a.to_python(params_prefix)),
            Expr::Cosh(a) => format!("np.cosh({})", a.to_python(params_prefix)),
            Expr::Tanh(a) => format!("np.tanh({})", a.to_python(params_prefix)),
            Expr::ASinh(a) => format!("np.arcsinh({})", a.to_python(params_prefix)),
            Expr::ACosh(a) => format!("np.arccosh({})", a.to_python(params_prefix)),
            Expr::ATanh(a) => format!("np.arctanh({})", a.to_python(params_prefix)),

            // 聚合函数
            Expr::Max(args) => {
                let args_py: Vec<String> = args.iter().map(|a| a.to_python(params_prefix)).collect();
                if args.len() == 2 {
                    format!("np.maximum({}, {})", args_py[0], args_py[1])
                } else {
                    format!("np.max([{}])", args_py.join(", "))
                }
            }
            Expr::Min(args) => {
                let args_py: Vec<String> = args.iter().map(|a| a.to_python(params_prefix)).collect();
                if args.len() == 2 {
                    format!("np.minimum({}, {})", args_py[0], args_py[1])
                } else {
                    format!("np.min([{}])", args_py.join(", "))
                }
            }

            // 求和
            Expr::Sum { index, lower, upper, body } => {
                format!(
                    "sum({} for {} in range(int({}), int({}) + 1))",
                    body.to_python(params_prefix),
                    index,
                    lower.to_python(params_prefix),
                    upper.to_python(params_prefix)
                )
            }

            // 连乘
            Expr::Product { index, lower, upper, body } => {
                format!(
                    "np.prod([{} for {} in range(int({}), int({}) + 1)])",
                    body.to_python(params_prefix),
                    index,
                    lower.to_python(params_prefix),
                    upper.to_python(params_prefix)
                )
            }

            // 关系运算
            Expr::Eq(a, b) => format!("({} == {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Lt(a, b) => format!("({} < {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Gt(a, b) => format!("({} > {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Leq(a, b) => format!("({} <= {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Geq(a, b) => format!("({} >= {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Neq(a, b) => format!("({} != {})", a.to_python(params_prefix), b.to_python(params_prefix)),

            // 逻辑运算
            Expr::And(a, b) => format!("({} and {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Or(a, b) => format!("({} or {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Not(a) => format!("(not {})", a.to_python(params_prefix)),

            // 条件表达式
            Expr::IfThenElse { cond, then_branch, else_branch } => {
                format!(
                    "({} if {} else {})",
                    then_branch.to_python(params_prefix),
                    cond.to_python(params_prefix),
                    else_branch.to_python(params_prefix)
                )
            }

            Expr::Piecewise { pieces, otherwise } => {
                let mut result = otherwise.to_python(params_prefix);
                for (cond, value) in pieces.iter().rev() {
                    result = format!(
                        "({} if {} else {})",
                        value.to_python(params_prefix),
                        cond.to_python(params_prefix),
                        result
                    );
                }
                result
            }

            // 扩展分位数函数
            Expr::ExpPpf(p, lam) => format!(
                "scipy.stats.expon.ppf({}, scale=1/{})",
                p.to_python(params_prefix), lam.to_python(params_prefix)
            ),
            Expr::GammaPpf(p, a, b) => format!(
                "scipy.stats.gamma.ppf({}, {}, scale={})",
                p.to_python(params_prefix), a.to_python(params_prefix), b.to_python(params_prefix)
            ),
            Expr::BetaPpf(p, a, b) => format!(
                "scipy.stats.beta.ppf({}, {}, {})",
                p.to_python(params_prefix), a.to_python(params_prefix), b.to_python(params_prefix)
            ),
            Expr::WeibullPpf(p, k, lam) => format!(
                "scipy.stats.weibull_min.ppf({}, {}, scale={})",
                p.to_python(params_prefix), k.to_python(params_prefix), lam.to_python(params_prefix)
            ),
            Expr::LognormPpf(p, mu, sig) => format!(
                "scipy.stats.lognorm.ppf({}, {}, scale=np.exp({}))",
                p.to_python(params_prefix), sig.to_python(params_prefix), mu.to_python(params_prefix)
            ),
            Expr::UniformPpf(p, a, b) => format!(
                "scipy.stats.uniform.ppf({}, {}, {})",
                p.to_python(params_prefix), a.to_python(params_prefix), b.to_python(params_prefix)
            ),
            Expr::CauchyPpf(p, x0, g) => format!(
                "scipy.stats.cauchy.ppf({}, {}, {})",
                p.to_python(params_prefix), x0.to_python(params_prefix), g.to_python(params_prefix)
            ),

            // 复数扩展
            Expr::ComplexSinh(z) => format!("np.sinh({})", z.to_python(params_prefix)),
            Expr::ComplexCosh(z) => format!("np.cosh({})", z.to_python(params_prefix)),
            Expr::ComplexTanh(z) => format!("np.tanh({})", z.to_python(params_prefix)),
            Expr::ComplexAsinh(z) => format!("np.arcsinh({})", z.to_python(params_prefix)),
            Expr::ComplexAcosh(z) => format!("np.arccosh({})", z.to_python(params_prefix)),
            Expr::ComplexAtanh(z) => format!("np.arctanh({})", z.to_python(params_prefix)),
            Expr::ComplexAsin(z) => format!("np.arcsin({})", z.to_python(params_prefix)),
            Expr::ComplexAcos(z) => format!("np.arccos({})", z.to_python(params_prefix)),
            Expr::ComplexAtan(z) => format!("np.arctan({})", z.to_python(params_prefix)),

            // 数论函数
            Expr::Gcd(a, b) => format!("math.gcd(int({}), int({}))", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Lcm(a, b) => format!("math.lcm(int({}), int({}))", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Permutation(n, k) => format!("math.perm(int({}), int({}))", n.to_python(params_prefix), k.to_python(params_prefix)),

            // 正交多项式
            Expr::Legendre(n, x) => format!("scipy.special.legendre(int({}))({})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::LegendreAssoc(l, m, x) => format!("scipy.special.lpmv({}, {}, {})", m.to_python(params_prefix), l.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hermite(n, x) => format!("scipy.special.hermite(int({}))({})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Laguerre(n, x) => format!("scipy.special.laguerre(int({}))({})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::LaguerreAssoc(n, a, x) => format!("scipy.special.genlaguerre(int({}), {})({})", n.to_python(params_prefix), a.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::ChebyshevT(n, x) => format!("scipy.special.chebyt(int({}))({})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::ChebyshevU(n, x) => format!("scipy.special.chebyu(int({}))({})", n.to_python(params_prefix), x.to_python(params_prefix)),

            // 椭圆积分
            Expr::EllipK(k) => format!("scipy.special.ellipk({})", k.to_python(params_prefix)),
            Expr::EllipE(k) => format!("scipy.special.ellipe({})", k.to_python(params_prefix)),

            // Lambda 和微积分
            Expr::Lambda { var, body } => format!("lambda {}: {}", var, body.to_python(params_prefix)),
            Expr::Integrate { var, lower, upper, body } => format!(
                "scipy.integrate.quad(lambda {}: {}, {}, {})[0]",
                var, body.to_python(params_prefix), lower.to_python(params_prefix), upper.to_python(params_prefix)
            ),
            Expr::Derivative { var, body, at } => format!(
                "scipy.misc.derivative(lambda {}: {}, {}, dx=1e-8)",
                var, body.to_python(params_prefix), at.to_python(params_prefix)
            ),
            Expr::Limit { var, to, body } => format!(
                "# limit of {} as {} -> {}",
                body.to_python(params_prefix), var, to.to_python(params_prefix)
            ),

            // 向量运算
            Expr::VectorLit(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| e.to_python(params_prefix)).collect();
                format!("np.array([{}])", elems.join(", "))
            }
            Expr::Dot(a, b) => format!("np.dot({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Cross(a, b) => format!("np.cross({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::VecNorm(v) => format!("np.linalg.norm({})", v.to_python(params_prefix)),
            Expr::VecNormalize(v) => format!("({} / np.linalg.norm({}))", v.to_python(params_prefix), v.to_python(params_prefix)),

            // 矩阵运算
            Expr::MatrixLit(rows) => {
                let row_strs: Vec<_> = rows.iter().map(|row| {
                    let elems: Vec<_> = row.iter().map(|e| e.to_python(params_prefix)).collect();
                    format!("[{}]", elems.join(", "))
                }).collect();
                format!("np.array([{}])", row_strs.join(", "))
            }
            Expr::MatMul(a, b) => format!("np.matmul({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Transpose(a) => format!("np.transpose({})", a.to_python(params_prefix)),
            Expr::Det(a) => format!("np.linalg.det({})", a.to_python(params_prefix)),
            Expr::Inv(a) => format!("np.linalg.inv({})", a.to_python(params_prefix)),
            Expr::Eigenvalues(a) => format!("np.linalg.eigvals({})", a.to_python(params_prefix)),
            Expr::Trace(a) => format!("np.trace({})", a.to_python(params_prefix)),
            Expr::MatNorm(a) => format!("np.linalg.norm({})", a.to_python(params_prefix)),

            // 特殊函数
            Expr::Gamma(x) => format!("scipy.special.gamma({})", x.to_python(params_prefix)),
            Expr::Lgamma(x) => format!("scipy.special.gammaln({})", x.to_python(params_prefix)),
            Expr::Digamma(x) => format!("scipy.special.digamma({})", x.to_python(params_prefix)),
            Expr::Beta(a, b) => format!("scipy.special.beta({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Lbeta(a, b) => format!("scipy.special.betaln({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Erf(x) => format!("scipy.special.erf({})", x.to_python(params_prefix)),
            Expr::Erfc(x) => format!("scipy.special.erfc({})", x.to_python(params_prefix)),
            Expr::Erfinv(x) => format!("scipy.special.erfinv({})", x.to_python(params_prefix)),
            Expr::Factorial(n) => format!("math.factorial(int({}))", n.to_python(params_prefix)),
            Expr::Combination(n, k) => format!("math.comb(int({}), int({}))", n.to_python(params_prefix), k.to_python(params_prefix)),
            Expr::Zeta(s) => format!("scipy.special.zeta({})", s.to_python(params_prefix)),

            // 贝塞尔函数
            Expr::BesselJ0(x) => format!("scipy.special.j0({})", x.to_python(params_prefix)),
            Expr::BesselJ1(x) => format!("scipy.special.j1({})", x.to_python(params_prefix)),
            Expr::BesselJn(n, x) => format!("scipy.special.jv({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselY0(x) => format!("scipy.special.y0({})", x.to_python(params_prefix)),
            Expr::BesselY1(x) => format!("scipy.special.y1({})", x.to_python(params_prefix)),
            Expr::BesselYn(n, x) => format!("scipy.special.yv({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselI0(x) => format!("scipy.special.i0({})", x.to_python(params_prefix)),
            Expr::BesselI1(x) => format!("scipy.special.i1({})", x.to_python(params_prefix)),
            Expr::BesselIn(n, x) => format!("scipy.special.iv({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselK0(x) => format!("scipy.special.k0({})", x.to_python(params_prefix)),
            Expr::BesselK1(x) => format!("scipy.special.k1({})", x.to_python(params_prefix)),
            Expr::BesselKn(n, x) => format!("scipy.special.kv({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),

            // 概率分布
            Expr::NormPdf(x, mu, sig) => format!("scipy.stats.norm.pdf({}, {}, {})", x.to_python(params_prefix), mu.to_python(params_prefix), sig.to_python(params_prefix)),
            Expr::NormCdf(x, mu, sig) => format!("scipy.stats.norm.cdf({}, {}, {})", x.to_python(params_prefix), mu.to_python(params_prefix), sig.to_python(params_prefix)),
            Expr::NormPpf(p, mu, sig) => format!("scipy.stats.norm.ppf({}, {}, {})", p.to_python(params_prefix), mu.to_python(params_prefix), sig.to_python(params_prefix)),
            Expr::TPdf(x, df) => format!("scipy.stats.t.pdf({}, {})", x.to_python(params_prefix), df.to_python(params_prefix)),
            Expr::TCdf(x, df) => format!("scipy.stats.t.cdf({}, {})", x.to_python(params_prefix), df.to_python(params_prefix)),
            Expr::TPpf(p, df) => format!("scipy.stats.t.ppf({}, {})", p.to_python(params_prefix), df.to_python(params_prefix)),
            Expr::Chi2Pdf(x, df) => format!("scipy.stats.chi2.pdf({}, {})", x.to_python(params_prefix), df.to_python(params_prefix)),
            Expr::Chi2Cdf(x, df) => format!("scipy.stats.chi2.cdf({}, {})", x.to_python(params_prefix), df.to_python(params_prefix)),
            Expr::Chi2Ppf(p, df) => format!("scipy.stats.chi2.ppf({}, {})", p.to_python(params_prefix), df.to_python(params_prefix)),
            Expr::FPdf(x, d1, d2) => format!("scipy.stats.f.pdf({}, {}, {})", x.to_python(params_prefix), d1.to_python(params_prefix), d2.to_python(params_prefix)),
            Expr::FCdf(x, d1, d2) => format!("scipy.stats.f.cdf({}, {}, {})", x.to_python(params_prefix), d1.to_python(params_prefix), d2.to_python(params_prefix)),
            Expr::FPpf(p, d1, d2) => format!("scipy.stats.f.ppf({}, {}, {})", p.to_python(params_prefix), d1.to_python(params_prefix), d2.to_python(params_prefix)),
            Expr::PoissonPmf(k, lam) => format!("scipy.stats.poisson.pmf({}, {})", k.to_python(params_prefix), lam.to_python(params_prefix)),
            Expr::PoissonCdf(k, lam) => format!("scipy.stats.poisson.cdf({}, {})", k.to_python(params_prefix), lam.to_python(params_prefix)),
            Expr::BinomialPmf(k, n, p) => format!("scipy.stats.binom.pmf({}, {}, {})", k.to_python(params_prefix), n.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::BinomialCdf(k, n, p) => format!("scipy.stats.binom.cdf({}, {}, {})", k.to_python(params_prefix), n.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::ExponentialPdf(x, lam) => format!("scipy.stats.expon.pdf({}, scale=1/{})", x.to_python(params_prefix), lam.to_python(params_prefix)),
            Expr::ExponentialCdf(x, lam) => format!("scipy.stats.expon.cdf({}, scale=1/{})", x.to_python(params_prefix), lam.to_python(params_prefix)),

            // 复数运算
            Expr::Complex(re, im) => format!("complex({}, {})", re.to_python(params_prefix), im.to_python(params_prefix)),
            Expr::Real(z) => format!("({}).real", z.to_python(params_prefix)),
            Expr::Imag(z) => format!("({}).imag", z.to_python(params_prefix)),
            Expr::Conj(z) => format!("np.conj({})", z.to_python(params_prefix)),
            Expr::Carg(z) => format!("np.angle({})", z.to_python(params_prefix)),
            Expr::Cabs(z) => format!("np.abs({})", z.to_python(params_prefix)),
            Expr::Polar(r, theta) => format!("({} * np.exp(1j * {}))", r.to_python(params_prefix), theta.to_python(params_prefix)),

            // 基础数学补充
            Expr::Hypot(x, y) => format!("np.hypot({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Hypot3(x, y, z) => format!("np.sqrt({}**2 + {}**2 + {}**2)", x.to_python(params_prefix), y.to_python(params_prefix), z.to_python(params_prefix)),
            Expr::Clamp(x, min, max) => format!("np.clip({}, {}, {})", x.to_python(params_prefix), min.to_python(params_prefix), max.to_python(params_prefix)),
            Expr::Copysign(x, y) => format!("np.copysign({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Fma(a, b, c) => format!("({} * {} + {})", a.to_python(params_prefix), b.to_python(params_prefix), c.to_python(params_prefix)),
            Expr::Logn(base, x) => format!("np.log({}) / np.log({})", x.to_python(params_prefix), base.to_python(params_prefix)),
            Expr::Sinc(x) => format!("np.sinc({} / np.pi)", x.to_python(params_prefix)),

            // 高精度数值函数
            Expr::Expm1(x) => format!("np.expm1({})", x.to_python(params_prefix)),
            Expr::Log1p(x) => format!("np.log1p({})", x.to_python(params_prefix)),
            Expr::Exp2(x) => format!("np.exp2({})", x.to_python(params_prefix)),

            // 不完全伽马/贝塔函数
            Expr::Gammainc(a, x) => format!("scipy.special.gammainc({}, {})", a.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Gammaincc(a, x) => format!("scipy.special.gammaincc({}, {})", a.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Betainc(x, a, b) => format!("scipy.special.betainc({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),

            // 扩展三角函数
            Expr::Sec(x) => format!("(1 / np.cos({}))", x.to_python(params_prefix)),
            Expr::Csc(x) => format!("(1 / np.sin({}))", x.to_python(params_prefix)),
            Expr::Cot(x) => format!("(1 / np.tan({}))", x.to_python(params_prefix)),
            Expr::Asec(x) => format!("np.arccos(1 / {})", x.to_python(params_prefix)),
            Expr::Acsc(x) => format!("np.arcsin(1 / {})", x.to_python(params_prefix)),
            Expr::Acot(x) => format!("np.arctan(1 / {})", x.to_python(params_prefix)),

            // 扩展双曲函数
            Expr::Sech(x) => format!("(1 / np.cosh({}))", x.to_python(params_prefix)),
            Expr::Csch(x) => format!("(1 / np.sinh({}))", x.to_python(params_prefix)),
            Expr::Coth(x) => format!("(1 / np.tanh({}))", x.to_python(params_prefix)),
            Expr::Asech(x) => format!("np.arccosh(1 / {})", x.to_python(params_prefix)),
            Expr::Acsch(x) => format!("np.arcsinh(1 / {})", x.to_python(params_prefix)),
            Expr::Acoth(x) => format!("np.arctanh(1 / {})", x.to_python(params_prefix)),

            // Airy 函数
            Expr::AiryAi(x) => format!("scipy.special.airy({})[0]", x.to_python(params_prefix)),
            Expr::AiryBi(x) => format!("scipy.special.airy({})[2]", x.to_python(params_prefix)),

            // 球谐函数
            Expr::SphericalHarmonic(l, m, theta, phi) => format!("scipy.special.sph_harm({}, {}, {}, {})", m.to_python(params_prefix), l.to_python(params_prefix), phi.to_python(params_prefix), theta.to_python(params_prefix)),

            // Fresnel 积分
            Expr::FresnelS(x) => format!("scipy.special.fresnel({})[0]", x.to_python(params_prefix)),
            Expr::FresnelC(x) => format!("scipy.special.fresnel({})[1]", x.to_python(params_prefix)),

            // 其他特殊函数
            Expr::Dawson(x) => format!("scipy.special.dawsn({})", x.to_python(params_prefix)),
            Expr::ExpInt(x) => format!("scipy.special.expi({})", x.to_python(params_prefix)),
            Expr::LogInt(x) => format!("scipy.special.expi(np.log({}))", x.to_python(params_prefix)),
            Expr::SinInt(x) => format!("scipy.special.sici({})[0]", x.to_python(params_prefix)),
            Expr::CosInt(x) => format!("scipy.special.sici({})[1]", x.to_python(params_prefix)),

            // Lambert W
            Expr::LambertW(x) => format!("scipy.special.lambertw({}, 0)", x.to_python(params_prefix)),
            Expr::LambertWm1(x) => format!("scipy.special.lambertw({}, -1)", x.to_python(params_prefix)),

            // 球贝塞尔函数
            Expr::SphBesselJ(n, x) => format!("scipy.special.spherical_jn(int({}), {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::SphBesselY(n, x) => format!("scipy.special.spherical_yn(int({}), {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::SphBesselI(n, x) => format!("scipy.special.spherical_in(int({}), {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::SphBesselK(n, x) => format!("scipy.special.spherical_kn(int({}), {})", n.to_python(params_prefix), x.to_python(params_prefix)),

            // 超几何函数
            Expr::Hyp0f1(b, x) => format!("scipy.special.hyp0f1({}, {})", b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hyp1f1(a, b, x) => format!("scipy.special.hyp1f1({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hyp2f1(a, b, c, x) => format!("scipy.special.hyp2f1({}, {}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),

            // Kelvin 函数
            Expr::KelvinBer(x) => format!("scipy.special.ber({})", x.to_python(params_prefix)),
            Expr::KelvinBei(x) => format!("scipy.special.bei({})", x.to_python(params_prefix)),
            Expr::KelvinKer(x) => format!("scipy.special.ker({})", x.to_python(params_prefix)),
            Expr::KelvinKei(x) => format!("scipy.special.kei({})", x.to_python(params_prefix)),

            // 不完全椭圆积分
            Expr::EllipF(phi, k) => format!("scipy.special.ellipkinc({}, {})", phi.to_python(params_prefix), k.to_python(params_prefix)),
            Expr::EllipEInc(phi, k) => format!("scipy.special.ellipeinc({}, {})", phi.to_python(params_prefix), k.to_python(params_prefix)),
            Expr::EllipPi(phi, n, k) => format!("scipy.special.ellip_pi({}, {}, {})", n.to_python(params_prefix), phi.to_python(params_prefix), k.to_python(params_prefix)),

            // 其他特殊函数
            Expr::Spence(x) => format!("scipy.special.spence({})", x.to_python(params_prefix)),
            Expr::Polygamma(n, x) => format!("scipy.special.polygamma(int({}), {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hankel1(n, x) => format!("scipy.special.hankel1({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hankel2(n, x) => format!("scipy.special.hankel2({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::StruveH(v, x) => format!("scipy.special.struve({}, {})", v.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::StruveL(v, x) => format!("scipy.special.modstruve({}, {})", v.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::OwensT(h, a) => format!("scipy.special.owens_t({}, {})", h.to_python(params_prefix), a.to_python(params_prefix)),
            Expr::RiemannSiegelZ(t) => format!("mpmath.siegelz({})", t.to_python(params_prefix)),
            Expr::RiemannSiegelTheta(t) => format!("mpmath.siegeltheta({})", t.to_python(params_prefix)),

            // Jacobi 椭圆函数
            Expr::JacobiSn(u, m) => format!("scipy.special.ellipj({}, {})[0]", u.to_python(params_prefix), m.to_python(params_prefix)),
            Expr::JacobiCn(u, m) => format!("scipy.special.ellipj({}, {})[1]", u.to_python(params_prefix), m.to_python(params_prefix)),
            Expr::JacobiDn(u, m) => format!("scipy.special.ellipj({}, {})[2]", u.to_python(params_prefix), m.to_python(params_prefix)),

            // 广义正交多项式
            Expr::Gegenbauer(n, alpha, x) => format!("scipy.special.gegenbauer(int({}), {})({})", n.to_python(params_prefix), alpha.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::JacobiP(n, alpha, beta, x) => format!("scipy.special.jacobi(int({}), {}, {})({})", n.to_python(params_prefix), alpha.to_python(params_prefix), beta.to_python(params_prefix), x.to_python(params_prefix)),

            // Mathieu 函数
            Expr::MathieuA(n, q) => format!("scipy.special.mathieu_a(int({}), {})", n.to_python(params_prefix), q.to_python(params_prefix)),
            Expr::MathieuB(n, q) => format!("scipy.special.mathieu_b(int({}), {})", n.to_python(params_prefix), q.to_python(params_prefix)),
            Expr::MathieuCe(n, q, x) => format!("scipy.special.mathieu_cem(int({}), {}, {})[0]", n.to_python(params_prefix), q.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::MathieuSe(n, q, x) => format!("scipy.special.mathieu_sem(int({}), {}, {})[0]", n.to_python(params_prefix), q.to_python(params_prefix), x.to_python(params_prefix)),

            // Coulomb 波函数
            Expr::CoulombF(l, eta, rho) => format!("scipy.special.coulomb_f({}, {}, {})", l.to_python(params_prefix), eta.to_python(params_prefix), rho.to_python(params_prefix)),
            Expr::CoulombG(l, eta, rho) => format!("scipy.special.coulomb_g({}, {}, {})", l.to_python(params_prefix), eta.to_python(params_prefix), rho.to_python(params_prefix)),

            // Wigner 符号
            Expr::Wigner3j(j1, j2, j3, m1, m2, m3) => format!("sympy.physics.wigner.wigner_3j({}, {}, {}, {}, {}, {})", j1.to_python(params_prefix), j2.to_python(params_prefix), j3.to_python(params_prefix), m1.to_python(params_prefix), m2.to_python(params_prefix), m3.to_python(params_prefix)),
            Expr::Wigner6j(j1, j2, j3, j4, j5, j6) => format!("sympy.physics.wigner.wigner_6j({}, {}, {}, {}, {}, {})", j1.to_python(params_prefix), j2.to_python(params_prefix), j3.to_python(params_prefix), j4.to_python(params_prefix), j5.to_python(params_prefix), j6.to_python(params_prefix)),
            Expr::Wigner9j(j1, j2, j3, j4, j5, j6, j7, j8, j9) => format!("sympy.physics.wigner.wigner_9j({}, {}, {}, {}, {}, {}, {}, {}, {})", j1.to_python(params_prefix), j2.to_python(params_prefix), j3.to_python(params_prefix), j4.to_python(params_prefix), j5.to_python(params_prefix), j6.to_python(params_prefix), j7.to_python(params_prefix), j8.to_python(params_prefix), j9.to_python(params_prefix)),

            // Theta 函数
            Expr::Theta1(z, q) => format!("mpmath.jtheta(1, {}, {})", z.to_python(params_prefix), q.to_python(params_prefix)),
            Expr::Theta2(z, q) => format!("mpmath.jtheta(2, {}, {})", z.to_python(params_prefix), q.to_python(params_prefix)),
            Expr::Theta3(z, q) => format!("mpmath.jtheta(3, {}, {})", z.to_python(params_prefix), q.to_python(params_prefix)),
            Expr::Theta4(z, q) => format!("mpmath.jtheta(4, {}, {})", z.to_python(params_prefix), q.to_python(params_prefix)),

            // 抛物柱面函数
            Expr::Pbdv(v, x) => format!("scipy.special.pbdv({}, {})[0]", v.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Pbvv(v, x) => format!("scipy.special.pbvv({}, {})[0]", v.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Pbwa(a, x) => format!("scipy.special.pbwa({}, {})[0]", a.to_python(params_prefix), x.to_python(params_prefix)),

            // 球扁旋转体波函数
            Expr::ProAng1(m, n, c, x) => format!("scipy.special.pro_ang1({}, {}, {}, {})[0]", m.to_python(params_prefix), n.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::ProRad1(m, n, c, x) => format!("scipy.special.pro_rad1({}, {}, {}, {})[0]", m.to_python(params_prefix), n.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::ProRad2(m, n, c, x) => format!("scipy.special.pro_rad2({}, {}, {}, {})[0]", m.to_python(params_prefix), n.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::OblAng1(m, n, c, x) => format!("scipy.special.obl_ang1({}, {}, {}, {})[0]", m.to_python(params_prefix), n.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::OblRad1(m, n, c, x) => format!("scipy.special.obl_rad1({}, {}, {}, {})[0]", m.to_python(params_prefix), n.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::OblRad2(m, n, c, x) => format!("scipy.special.obl_rad2({}, {}, {}, {})[0]", m.to_python(params_prefix), n.to_python(params_prefix), c.to_python(params_prefix), x.to_python(params_prefix)),

            // 修改 Fresnel 积分
            Expr::ModFresnelP(x) => format!("scipy.special.modfresnelp({})[0]", x.to_python(params_prefix)),
            Expr::ModFresnelM(x) => format!("scipy.special.modfresnelm({})[0]", x.to_python(params_prefix)),

            // Wright 函数
            Expr::WrightBessel(rho, beta, z) => format!("scipy.special.wright_bessel({}, {}, {})", rho.to_python(params_prefix), beta.to_python(params_prefix), z.to_python(params_prefix)),
            Expr::WrightOmega(z) => format!("scipy.special.wrightomega({})", z.to_python(params_prefix)),

            // Voigt
            Expr::Voigt(x, sigma, gamma) => format!("scipy.special.voigt_profile({}, {}, {})", x.to_python(params_prefix), sigma.to_python(params_prefix), gamma.to_python(params_prefix)),

            // Sigmoid/Logistic
            Expr::Logit(x) => format!("scipy.special.logit({})", x.to_python(params_prefix)),
            Expr::Expit(x) => format!("scipy.special.expit({})", x.to_python(params_prefix)),

            // Box-Cox
            Expr::BoxCox(x, lmbda) => format!("scipy.special.boxcox({}, {})", x.to_python(params_prefix), lmbda.to_python(params_prefix)),
            Expr::BoxCox1p(x, lmbda) => format!("scipy.special.boxcox1p({}, {})", x.to_python(params_prefix), lmbda.to_python(params_prefix)),
            Expr::InvBoxCox(y, lmbda) => format!("scipy.special.inv_boxcox({}, {})", y.to_python(params_prefix), lmbda.to_python(params_prefix)),
            Expr::InvBoxCox1p(y, lmbda) => format!("scipy.special.inv_boxcox1p({}, {})", y.to_python(params_prefix), lmbda.to_python(params_prefix)),

            // 信息论
            Expr::Entr(x) => format!("scipy.special.entr({})", x.to_python(params_prefix)),
            Expr::RelEntr(x, y) => format!("scipy.special.rel_entr({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::KlDiv(x, y) => format!("scipy.special.kl_div({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),

            // 阶乘扩展
            Expr::Factorial2(n) => format!("scipy.special.factorial2(int({}))", n.to_python(params_prefix)),
            Expr::Factorialk(n, k) => format!("scipy.special.factorialk(int({}), int({}))", n.to_python(params_prefix), k.to_python(params_prefix)),
            Expr::Stirling2(n, k) => format!("scipy.special.stirling2(int({}), int({}))", n.to_python(params_prefix), k.to_python(params_prefix)),
            Expr::Poch(z, m) => format!("scipy.special.poch({}, {})", z.to_python(params_prefix), m.to_python(params_prefix)),

            // Carlson 椭圆积分
            Expr::EllipRc(x, y) => format!("scipy.special.elliprc({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::EllipRd(x, y, z) => format!("scipy.special.elliprd({}, {}, {})", x.to_python(params_prefix), y.to_python(params_prefix), z.to_python(params_prefix)),
            Expr::EllipRf(x, y, z) => format!("scipy.special.elliprf({}, {}, {})", x.to_python(params_prefix), y.to_python(params_prefix), z.to_python(params_prefix)),
            Expr::EllipRg(x, y, z) => format!("scipy.special.elliprg({}, {}, {})", x.to_python(params_prefix), y.to_python(params_prefix), z.to_python(params_prefix)),
            Expr::EllipRj(x, y, z, p) => format!("scipy.special.elliprj({}, {}, {}, {})", x.to_python(params_prefix), y.to_python(params_prefix), z.to_python(params_prefix), p.to_python(params_prefix)),

            // 扩展误差函数
            Expr::Erfcx(x) => format!("scipy.special.erfcx({})", x.to_python(params_prefix)),
            Expr::Erfi(x) => format!("scipy.special.erfi({})", x.to_python(params_prefix)),
            Expr::Erfcinv(x) => format!("scipy.special.erfcinv({})", x.to_python(params_prefix)),

            // 扩展 Gamma
            Expr::Hyperu(a, b, x) => format!("scipy.special.hyperu({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Rgamma(x) => format!("scipy.special.rgamma({})", x.to_python(params_prefix)),
            Expr::Gammasgn(x) => format!("scipy.special.gammasgn({})", x.to_python(params_prefix)),

            // 便利函数
            Expr::Agm(a, b) => format!("scipy.special.agm({}, {})", a.to_python(params_prefix), b.to_python(params_prefix)),
            Expr::Exprel(x) => format!("scipy.special.exprel({})", x.to_python(params_prefix)),
            Expr::Xlogy(x, y) => format!("scipy.special.xlogy({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Xlog1py(x, y) => format!("scipy.special.xlog1py({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),

            // Zeta 扩展
            Expr::HurwitzZeta(s, q) => format!("scipy.special.zeta({}, {})", s.to_python(params_prefix), q.to_python(params_prefix)),
            Expr::Zetac(x) => format!("scipy.special.zetac({})", x.to_python(params_prefix)),
            Expr::Polylog(s, z) => format!("mpmath.polylog({}, {})", s.to_python(params_prefix), z.to_python(params_prefix)),

            // === 缩放贝塞尔函数 ===
            Expr::BesselI0e(x) => format!("scipy.special.i0e({})", x.to_python(params_prefix)),
            Expr::BesselI1e(x) => format!("scipy.special.i1e({})", x.to_python(params_prefix)),
            Expr::BesselIne(n, x) => format!("scipy.special.ive({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselK0e(x) => format!("scipy.special.k0e({})", x.to_python(params_prefix)),
            Expr::BesselK1e(x) => format!("scipy.special.k1e({})", x.to_python(params_prefix)),
            Expr::BesselKne(n, x) => format!("scipy.special.kve({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselJne(n, x) => format!("scipy.special.jve({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselYne(n, x) => format!("scipy.special.yve({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hankel1e(n, x) => format!("scipy.special.hankel1e({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hankel2e(n, x) => format!("scipy.special.hankel2e({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),

            // === 贝塞尔函数导数 ===
            Expr::BesselJnp(n, x) => format!("scipy.special.jvp({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselYnp(n, x) => format!("scipy.special.yvp({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselInp(n, x) => format!("scipy.special.ivp({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::BesselKnp(n, x) => format!("scipy.special.kvp({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hankel1p(n, x) => format!("scipy.special.h1vp({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Hankel2p(n, x) => format!("scipy.special.h2vp({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),

            // === Huber 损失 ===
            Expr::Huber(delta, r) => format!("scipy.special.huber({}, {})", delta.to_python(params_prefix), r.to_python(params_prefix)),
            Expr::PseudoHuber(delta, r) => format!("scipy.special.pseudo_huber({}, {})", delta.to_python(params_prefix), r.to_python(params_prefix)),

            // === Kolmogorov-Smirnov ===
            Expr::Kolmogorov(y) => format!("scipy.special.kolmogorov({})", y.to_python(params_prefix)),
            Expr::Kolmogi(p) => format!("scipy.special.kolmogi({})", p.to_python(params_prefix)),
            Expr::Smirnov(n, d) => format!("scipy.special.smirnov({}, {})", n.to_python(params_prefix), d.to_python(params_prefix)),
            Expr::Smirnovi(n, p) => format!("scipy.special.smirnovi({}, {})", n.to_python(params_prefix), p.to_python(params_prefix)),

            // === Faddeeva ===
            Expr::Wofz(z) => format!("scipy.special.wofz({})", z.to_python(params_prefix)),

            // === Dirichlet 核 ===
            Expr::Diric(x, n) => format!("scipy.special.diric({}, {})", x.to_python(params_prefix), n.to_python(params_prefix)),

            // === Tukey lambda ===
            Expr::Tklmbda(x, lam) => format!("scipy.special.tklmbda({}, {})", x.to_python(params_prefix), lam.to_python(params_prefix)),

            // === Gamma/Beta 逆函数 ===
            Expr::Gammaincinv(a, y) => format!("scipy.special.gammaincinv({}, {})", a.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Gammainccinv(a, y) => format!("scipy.special.gammainccinv({}, {})", a.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Betaincinv(a, b, y) => format!("scipy.special.betaincinv({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), y.to_python(params_prefix)),

            // === 高精度便利函数 ===
            Expr::Cosm1(x) => format!("scipy.special.cosm1({})", x.to_python(params_prefix)),
            Expr::Powm1(x, y) => format!("scipy.special.powm1({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Exp10(x) => format!("scipy.special.exp10({})", x.to_python(params_prefix)),
            Expr::Log1pmx(x) => format!("scipy.special.log1pmx({})", x.to_python(params_prefix)),
            Expr::Loggamma(z) => format!("scipy.special.loggamma({})", z.to_python(params_prefix)),

            // === 度数三角函数 ===
            Expr::Cosdg(x) => format!("scipy.special.cosdg({})", x.to_python(params_prefix)),
            Expr::Sindg(x) => format!("scipy.special.sindg({})", x.to_python(params_prefix)),
            Expr::Tandg(x) => format!("scipy.special.tandg({})", x.to_python(params_prefix)),
            Expr::Cotdg(x) => format!("scipy.special.cotdg({})", x.to_python(params_prefix)),
            Expr::Radian(d, m, s) => format!("scipy.special.radian({}, {}, {})", d.to_python(params_prefix), m.to_python(params_prefix), s.to_python(params_prefix)),

            // === Airy 扩展 ===
            Expr::AiryAie(x) => format!("scipy.special.airye({})[0]", x.to_python(params_prefix)),
            Expr::AiryBie(x) => format!("scipy.special.airye({})[2]", x.to_python(params_prefix)),
            Expr::AiryAip(x) => format!("scipy.special.airy({})[1]", x.to_python(params_prefix)),
            Expr::AiryBip(x) => format!("scipy.special.airy({})[3]", x.to_python(params_prefix)),
            Expr::ItAiry(x) => format!("scipy.special.itairy({})", x.to_python(params_prefix)),

            // === 指数积分扩展 ===
            Expr::Expn(n, x) => format!("scipy.special.expn({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Exp1(x) => format!("scipy.special.exp1({})", x.to_python(params_prefix)),
            Expr::Shi(x) => format!("scipy.special.shichi({})[0]", x.to_python(params_prefix)),
            Expr::Chi(x) => format!("scipy.special.shichi({})[1]", x.to_python(params_prefix)),

            // === Struve 积分 ===
            Expr::ItStruve0(x) => format!("scipy.special.itstruve0({})", x.to_python(params_prefix)),
            Expr::It2Struve0(x) => format!("scipy.special.it2struve0({})", x.to_python(params_prefix)),
            Expr::ItModStruve0(x) => format!("scipy.special.itmodstruve0({})", x.to_python(params_prefix)),

            // === ML/统计扩展 ===
            Expr::LogExpit(x) => format!("scipy.special.log_expit({})", x.to_python(params_prefix)),
            Expr::Softplus(x) => format!("scipy.special.softplus({})", x.to_python(params_prefix)),
            Expr::LogNdtr(x) => format!("scipy.special.log_ndtr({})", x.to_python(params_prefix)),

            // === Beta 补函数 ===
            Expr::Betaincc(a, b, x) => format!("scipy.special.betaincc({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Betainccinv(a, b, y) => format!("scipy.special.betainccinv({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), y.to_python(params_prefix)),

            // === 数论函数 ===
            Expr::Bernoulli(n) => format!("scipy.special.bernoulli(int({}))", n.to_python(params_prefix)),
            Expr::Euler(n) => format!("scipy.special.euler(int({}))", n.to_python(params_prefix)),

            // === 椭圆扩展 ===
            Expr::EllipKm1(p) => format!("scipy.special.ellipkm1({})", p.to_python(params_prefix)),

            // === Kelvin 导数 ===
            Expr::KelvinBerp(x) => format!("scipy.special.berp({})", x.to_python(params_prefix)),
            Expr::KelvinBeip(x) => format!("scipy.special.beip({})", x.to_python(params_prefix)),
            Expr::KelvinKerp(x) => format!("scipy.special.kerp({})", x.to_python(params_prefix)),
            Expr::KelvinKeip(x) => format!("scipy.special.keip({})", x.to_python(params_prefix)),

            // === 贝塞尔积分 ===
            Expr::BesselPoly(a, lmb, nu) => format!("scipy.special.besselpoly({}, {}, {})", a.to_python(params_prefix), lmb.to_python(params_prefix), nu.to_python(params_prefix)),

            // === Wright Bessel 扩展 ===
            Expr::LogWrightBessel(a, b, x) => format!("scipy.special.log_wright_bessel({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),

            // === 二项系数扩展 ===
            Expr::Binom(x, y) => format!("scipy.special.binom({}, {})", x.to_python(params_prefix), y.to_python(params_prefix)),

            // === 分布函数 ===
            Expr::Bdtr(k, n, p) => format!("scipy.special.bdtr({}, {}, {})", k.to_python(params_prefix), n.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::Bdtrc(k, n, p) => format!("scipy.special.bdtrc({}, {}, {})", k.to_python(params_prefix), n.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::Bdtri(k, n, y) => format!("scipy.special.bdtri({}, {}, {})", k.to_python(params_prefix), n.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Chdtr(v, x) => format!("scipy.special.chdtr({}, {})", v.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Chdtrc(v, x) => format!("scipy.special.chdtrc({}, {})", v.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Chdtri(v, p) => format!("scipy.special.chdtri({}, {})", v.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::Fdtr(dfn, dfd, x) => format!("scipy.special.fdtr({}, {}, {})", dfn.to_python(params_prefix), dfd.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Fdtrc(dfn, dfd, x) => format!("scipy.special.fdtrc({}, {}, {})", dfn.to_python(params_prefix), dfd.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Fdtri(dfn, dfd, p) => format!("scipy.special.fdtri({}, {}, {})", dfn.to_python(params_prefix), dfd.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::Stdtr(df, t) => format!("scipy.special.stdtr({}, {})", df.to_python(params_prefix), t.to_python(params_prefix)),
            Expr::Stdtrc(df, t) => format!("scipy.special.stdtrc({}, {})", df.to_python(params_prefix), t.to_python(params_prefix)),
            Expr::Stdtrit(df, p) => format!("scipy.special.stdtrit({}, {})", df.to_python(params_prefix), p.to_python(params_prefix)),
            Expr::Pdtr(k, m) => format!("scipy.special.pdtr({}, {})", k.to_python(params_prefix), m.to_python(params_prefix)),
            Expr::Pdtrc(k, m) => format!("scipy.special.pdtrc({}, {})", k.to_python(params_prefix), m.to_python(params_prefix)),
            Expr::Pdtri(k, y) => format!("scipy.special.pdtri({}, {})", k.to_python(params_prefix), y.to_python(params_prefix)),
            Expr::Btdtr(a, b, x) => format!("scipy.special.btdtr({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Btdtrc(a, b, x) => format!("(1.0 - scipy.special.btdtr({}, {}, {}))", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Gdtr(a, b, x) => format!("scipy.special.gdtr({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Gdtrc(a, b, x) => format!("scipy.special.gdtrc({}, {}, {})", a.to_python(params_prefix), b.to_python(params_prefix), x.to_python(params_prefix)),

            // === 积分/ML 扩展 ===
            Expr::Sici(x) => format!("scipy.special.sici({})", x.to_python(params_prefix)),
            Expr::Shichi(x) => format!("scipy.special.shichi({})", x.to_python(params_prefix)),
            Expr::Softmax(x) => format!("scipy.special.softmax({})", x.to_python(params_prefix)),
            Expr::LogSoftmax(x) => format!("scipy.special.log_softmax({})", x.to_python(params_prefix)),
            Expr::Logsumexp(x) => format!("scipy.special.logsumexp({})", x.to_python(params_prefix)),

            // === GSL 扩展 ===
            Expr::AiryZeroAi(s) => format!("scipy.special.ai_zeros(int({}))[0][-1]", s.to_python(params_prefix)),
            Expr::AiryZeroBi(s) => format!("scipy.special.bi_zeros(int({}))[0][-1]", s.to_python(params_prefix)),
            Expr::BesselZeroJ0(s) => format!("scipy.special.jn_zeros(0, int({}))[int({})-1]", s.to_python(params_prefix), s.to_python(params_prefix)),
            Expr::BesselZeroJ1(s) => format!("scipy.special.jn_zeros(1, int({}))[int({})-1]", s.to_python(params_prefix), s.to_python(params_prefix)),
            Expr::BesselZeroJnu(nu, s) => format!("scipy.special.jn_zeros({}, int({}))[int({})-1]", nu.to_python(params_prefix), s.to_python(params_prefix), s.to_python(params_prefix)),
            Expr::SphLegendre(l, m, x) => format!("scipy.special.lpmv({}, {}, {})", m.to_python(params_prefix), l.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Clausen(x) => format!("scipy.special.clausen({})", x.to_python(params_prefix)),
            Expr::Debye(n, x) => format!("scipy.special.debye({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::Synchrotron1(x) => format!("scipy.special.synchrotron1({})", x.to_python(params_prefix)),
            Expr::Synchrotron2(x) => format!("scipy.special.synchrotron2({})", x.to_python(params_prefix)),
            Expr::Transport(n, x) => format!("scipy.special.transport({}, {})", n.to_python(params_prefix), x.to_python(params_prefix)),
            Expr::FermiDirac(j, x) => format!("scipy.special.fdtr({}, {})", j.to_python(params_prefix), x.to_python(params_prefix)),
        }
    }

    /// 转换为 Rust 代码
    pub fn to_rust(&self) -> String {
        // 注册表快路径：已迁移算子从 ops 注册表生成（单一真相源）。
        // 下方 match 中这些算子的分支已不可达，仅为保持 match 穷尽性而保留，待后续清理。
        if let Some((name, args)) = crate::ops::as_operator(self) {
            if let Some(s) = crate::ops::spec(name) {
                let codes: Vec<String> = args.iter().map(|a| a.to_rust()).collect();
                return (s.rust)(&codes);
            }
        }
        match self {
            Expr::Const(value) => format!("{}_f64", value),
            Expr::Var(name) => name.clone(),
            Expr::Param(name) => name.clone(),
            Expr::Pi => "std::f64::consts::PI".to_string(),
            Expr::E => "std::f64::consts::E".to_string(),
            Expr::Reduce { kind, arg } => {
                let m = match kind {
                    ReduceKind::Sum => "sum",
                    ReduceKind::Prod => "product",
                    ReduceKind::Mean => "mean",
                    ReduceKind::Min => "min",
                    ReduceKind::Max => "max",
                };
                format!("({}).{}()", arg.to_rust(), m)
            }

            // 算术运算
            Expr::Add(a, b) => format!("({} + {})", a.to_rust(), b.to_rust()),
            Expr::Sub(a, b) => format!("({} - {})", a.to_rust(), b.to_rust()),
            Expr::Mul(a, b) => format!("({} * {})", a.to_rust(), b.to_rust()),
            Expr::Div(a, b) => format!("({} / {})", a.to_rust(), b.to_rust()),
            Expr::Neg(a) => format!("(-{})", a.to_rust()),
            Expr::Pow(a, b) => format!("{}.powf({})", a.to_rust(), b.to_rust()),
            Expr::Abs(a) => format!("{}.abs()", a.to_rust()),
            Expr::Mod(a, b) => format!("{}.rem_euclid({})", a.to_rust(), b.to_rust()),
            Expr::Ceil(a) => format!("{}.ceil()", a.to_rust()),
            Expr::Floor(a) => format!("{}.floor()", a.to_rust()),
            Expr::Round(a) => format!("{}.round()", a.to_rust()),
            Expr::Trunc(a) => format!("{}.trunc()", a.to_rust()),
            Expr::Sign(a) => format!("{}.signum()", a.to_rust()),

            // 超越函数
            Expr::Exp(a) => format!("{}.exp()", a.to_rust()),
            Expr::Ln(a) => format!("{}.ln()", a.to_rust()),
            Expr::Log10(a) => format!("{}.log10()", a.to_rust()),
            Expr::Log2(a) => format!("{}.log2()", a.to_rust()),
            Expr::Sqrt(a) => format!("{}.sqrt()", a.to_rust()),
            Expr::Cbrt(a) => format!("{}.cbrt()", a.to_rust()),

            // 三角函数
            Expr::Sin(a) => format!("{}.sin()", a.to_rust()),
            Expr::Cos(a) => format!("{}.cos()", a.to_rust()),
            Expr::Tan(a) => format!("{}.tan()", a.to_rust()),
            Expr::ASin(a) => format!("{}.asin()", a.to_rust()),
            Expr::ACos(a) => format!("{}.acos()", a.to_rust()),
            Expr::ATan(a) => format!("{}.atan()", a.to_rust()),
            Expr::ATan2(y, x) => format!("{}.atan2({})", y.to_rust(), x.to_rust()),

            // 双曲函数
            Expr::Sinh(a) => format!("{}.sinh()", a.to_rust()),
            Expr::Cosh(a) => format!("{}.cosh()", a.to_rust()),
            Expr::Tanh(a) => format!("{}.tanh()", a.to_rust()),
            Expr::ASinh(a) => format!("{}.asinh()", a.to_rust()),
            Expr::ACosh(a) => format!("{}.acosh()", a.to_rust()),
            Expr::ATanh(a) => format!("{}.atanh()", a.to_rust()),

            // 聚合函数
            Expr::Max(args) if args.len() == 2 => {
                format!("{}.max({})", args[0].to_rust(), args[1].to_rust())
            }
            Expr::Max(args) => {
                let args_rust: Vec<String> = args.iter().map(|a| a.to_rust()).collect();
                format!("[{}].into_iter().fold(f64::NEG_INFINITY, f64::max)", args_rust.join(", "))
            }
            Expr::Min(args) if args.len() == 2 => {
                format!("{}.min({})", args[0].to_rust(), args[1].to_rust())
            }
            Expr::Min(args) => {
                let args_rust: Vec<String> = args.iter().map(|a| a.to_rust()).collect();
                format!("[{}].into_iter().fold(f64::INFINITY, f64::min)", args_rust.join(", "))
            }

            // 求和
            Expr::Sum { index, lower, upper, body } => {
                format!(
                    "(({} as i64)..=({} as i64)).map(|{}| {}).sum::<f64>()",
                    lower.to_rust(),
                    upper.to_rust(),
                    index,
                    body.to_rust()
                )
            }

            // 连乘
            Expr::Product { index, lower, upper, body } => {
                format!(
                    "(({} as i64)..=({} as i64)).map(|{}| {}).product::<f64>()",
                    lower.to_rust(),
                    upper.to_rust(),
                    index,
                    body.to_rust()
                )
            }

            // 关系运算
            Expr::Eq(a, b) => format!("({} == {})", a.to_rust(), b.to_rust()),
            Expr::Lt(a, b) => format!("({} < {})", a.to_rust(), b.to_rust()),
            Expr::Gt(a, b) => format!("({} > {})", a.to_rust(), b.to_rust()),
            Expr::Leq(a, b) => format!("({} <= {})", a.to_rust(), b.to_rust()),
            Expr::Geq(a, b) => format!("({} >= {})", a.to_rust(), b.to_rust()),
            Expr::Neq(a, b) => format!("({} != {})", a.to_rust(), b.to_rust()),

            // 逻辑运算
            Expr::And(a, b) => format!("({} && {})", a.to_rust(), b.to_rust()),
            Expr::Or(a, b) => format!("({} || {})", a.to_rust(), b.to_rust()),
            Expr::Not(a) => format!("(!{})", a.to_rust()),

            // 条件表达式
            Expr::IfThenElse { cond, then_branch, else_branch } => {
                format!(
                    "if {} {{ {} }} else {{ {} }}",
                    cond.to_rust(),
                    then_branch.to_rust(),
                    else_branch.to_rust()
                )
            }

            Expr::Piecewise { pieces, otherwise } => {
                let mut result = format!("{{ {} }}", otherwise.to_rust());
                for (cond, value) in pieces.iter().rev() {
                    result = format!(
                        "if {} {{ {} }} else {}",
                        cond.to_rust(),
                        value.to_rust(),
                        result
                    );
                }
                result
            }

            // 扩展分位数函数 (使用 statrs)
            Expr::ExpPpf(p, lam) => format!(
                "statrs::distribution::Exp::new({}).unwrap().inverse_cdf({})",
                lam.to_rust(), p.to_rust()
            ),
            Expr::GammaPpf(p, a, b) => format!(
                "statrs::distribution::Gamma::new({}, {}).unwrap().inverse_cdf({})",
                a.to_rust(), b.to_rust(), p.to_rust()
            ),
            Expr::BetaPpf(p, a, b) => format!(
                "statrs::distribution::Beta::new({}, {}).unwrap().inverse_cdf({})",
                a.to_rust(), b.to_rust(), p.to_rust()
            ),
            Expr::WeibullPpf(p, k, lam) => format!(
                "statrs::distribution::Weibull::new({}, {}).unwrap().inverse_cdf({})",
                k.to_rust(), lam.to_rust(), p.to_rust()
            ),
            Expr::LognormPpf(p, mu, sig) => format!(
                "statrs::distribution::LogNormal::new({}, {}).unwrap().inverse_cdf({})",
                mu.to_rust(), sig.to_rust(), p.to_rust()
            ),
            Expr::UniformPpf(p, a, b) => format!(
                "statrs::distribution::Uniform::new({}, {}).unwrap().inverse_cdf({})",
                a.to_rust(), b.to_rust(), p.to_rust()
            ),
            Expr::CauchyPpf(p, x0, g) => format!(
                "statrs::distribution::Cauchy::new({}, {}).unwrap().inverse_cdf({})",
                x0.to_rust(), g.to_rust(), p.to_rust()
            ),

            // 复数扩展 (使用 num_complex)
            Expr::ComplexSinh(z) => format!("({}).sinh()", z.to_rust()),
            Expr::ComplexCosh(z) => format!("({}).cosh()", z.to_rust()),
            Expr::ComplexTanh(z) => format!("({}).tanh()", z.to_rust()),
            Expr::ComplexAsinh(z) => format!("({}).asinh()", z.to_rust()),
            Expr::ComplexAcosh(z) => format!("({}).acosh()", z.to_rust()),
            Expr::ComplexAtanh(z) => format!("({}).atanh()", z.to_rust()),
            Expr::ComplexAsin(z) => format!("({}).asin()", z.to_rust()),
            Expr::ComplexAcos(z) => format!("({}).acos()", z.to_rust()),
            Expr::ComplexAtan(z) => format!("({}).atan()", z.to_rust()),

            // 数论函数 (使用 num_integer)
            Expr::Gcd(a, b) => format!("num_integer::gcd({} as i64, {} as i64) as f64", a.to_rust(), b.to_rust()),
            Expr::Lcm(a, b) => format!("num_integer::lcm({} as i64, {} as i64) as f64", a.to_rust(), b.to_rust()),
            Expr::Permutation(n, k) => format!(
                "((1..=({} as u64)).product::<u64>() / (1..=(({} - {}) as u64)).product::<u64>()) as f64",
                n.to_rust(), n.to_rust(), k.to_rust()
            ),

            // 正交多项式 (使用 GSL)
            Expr::Legendre(n, x) => format!("GSL::sf::legendre_Pl({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::LegendreAssoc(l, m, x) => format!("GSL::sf::legendre_Plm({} as i32, {} as i32, {})", l.to_rust(), m.to_rust(), x.to_rust()),
            Expr::Hermite(n, x) => format!("GSL::sf::hermite({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::Laguerre(n, x) => format!("GSL::sf::laguerre_n({} as i32, 0.0, {})", n.to_rust(), x.to_rust()),
            Expr::LaguerreAssoc(n, a, x) => format!("GSL::sf::laguerre_n({} as i32, {}, {})", n.to_rust(), a.to_rust(), x.to_rust()),
            Expr::ChebyshevT(n, x) => {
                // 切比雪夫多项式使用递推公式
                format!("chebyshev_t({} as i32, {})", n.to_rust(), x.to_rust())
            }
            Expr::ChebyshevU(n, x) => {
                format!("chebyshev_u({} as i32, {})", n.to_rust(), x.to_rust())
            }

            // 椭圆积分 (使用 GSL)
            Expr::EllipK(k) => format!("GSL::sf::ellint_Kcomp({}, GSL::Mode::Default)", k.to_rust()),
            Expr::EllipE(k) => format!("GSL::sf::ellint_Ecomp({}, GSL::Mode::Default)", k.to_rust()),

            // Lambda 和微积分 (使用 peroxide)
            Expr::Lambda { var, body } => format!("|{}| {}", var, body.to_rust()),
            Expr::Integrate { var, lower, upper, body } => format!(
                "peroxide::numerical::integral::integrate(|{}| {}, ({}, {}), peroxide::numerical::integral::Integral::G30K61(1e-10, 20))",
                var, body.to_rust(), lower.to_rust(), upper.to_rust()
            ),
            Expr::Derivative { var, body, at } => format!(
                "peroxide::numerical::utils::derivative(|{}| {}, {}, 1e-8)",
                var, body.to_rust(), at.to_rust()
            ),
            Expr::Limit { var, to, body } => format!(
                "/* limit of {} as {} -> {} */",
                body.to_rust(), var, to.to_rust()
            ),

            // 向量运算 (使用 nalgebra)
            Expr::VectorLit(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| e.to_rust()).collect();
                format!("nalgebra::DVector::from_vec(vec![{}])", elems.join(", "))
            }
            Expr::Dot(a, b) => format!("({}).dot(&({}))", a.to_rust(), b.to_rust()),
            Expr::Cross(a, b) => format!("({}).cross(&({}))", a.to_rust(), b.to_rust()),
            Expr::VecNorm(v) => format!("({}).norm()", v.to_rust()),
            Expr::VecNormalize(v) => format!("({}).normalize()", v.to_rust()),

            // 矩阵运算 (使用 nalgebra)
            Expr::MatrixLit(rows) => {
                let nrows = rows.len();
                let ncols = rows.first().map(|r| r.len()).unwrap_or(0);
                let elems: Vec<_> = rows.iter()
                    .flat_map(|row| row.iter())
                    .map(|e| e.to_rust())
                    .collect();
                format!("nalgebra::DMatrix::from_row_slice({}, {}, &[{}])", nrows, ncols, elems.join(", "))
            }
            Expr::MatMul(a, b) => format!("({}) * ({})", a.to_rust(), b.to_rust()),
            Expr::Transpose(a) => format!("({}).transpose()", a.to_rust()),
            Expr::Det(a) => format!("({}).determinant()", a.to_rust()),
            Expr::Inv(a) => format!("({}).try_inverse().unwrap()", a.to_rust()),
            Expr::Eigenvalues(a) => format!("({}).eigenvalues().unwrap()", a.to_rust()),
            Expr::Trace(a) => format!("({}).trace()", a.to_rust()),
            Expr::MatNorm(a) => format!("({}).norm()", a.to_rust()),

            // 特殊函数
            Expr::Gamma(x) => format!("puruspe::gamma({})", x.to_rust()),
            Expr::Lgamma(x) => format!("({}).ln_gamma().0", x.to_rust()),
            Expr::Digamma(x) => format!("statrs::function::gamma::digamma({})", x.to_rust()),
            Expr::Beta(a, b) => format!("puruspe::beta({}, {})", a.to_rust(), b.to_rust()),
            Expr::Lbeta(a, b) => format!("statrs::function::beta::ln_beta({}, {})", a.to_rust(), b.to_rust()),
            Expr::Erf(x) => format!("puruspe::erf({})", x.to_rust()),
            Expr::Erfc(x) => format!("puruspe::erfc({})", x.to_rust()),
            Expr::Erfinv(x) => format!("statrs::function::erf::erf_inv({})", x.to_rust()),
            Expr::Factorial(n) => format!("(1..=({} as u64)).product::<u64>() as f64", n.to_rust()),
            Expr::Combination(n, k) => format!("(puruspe::gamma({} + 1.0) / (puruspe::gamma({} + 1.0) * puruspe::gamma({} - {} + 1.0)))", n.to_rust(), k.to_rust(), n.to_rust(), k.to_rust()),
            Expr::Zeta(s) => format!("GSL::sf::zeta({})", s.to_rust()),

            // 贝塞尔函数
            Expr::BesselJ0(x) => format!("GSL::sf::bessel_J0({})", x.to_rust()),
            Expr::BesselJ1(x) => format!("GSL::sf::bessel_J1({})", x.to_rust()),
            Expr::BesselJn(n, x) => format!("GSL::sf::bessel_Jn({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::BesselY0(x) => format!("GSL::sf::bessel_Y0({})", x.to_rust()),
            Expr::BesselY1(x) => format!("GSL::sf::bessel_Y1({})", x.to_rust()),
            Expr::BesselYn(n, x) => format!("GSL::sf::bessel_Yn({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::BesselI0(x) => format!("GSL::sf::bessel_I0({})", x.to_rust()),
            Expr::BesselI1(x) => format!("GSL::sf::bessel_I1({})", x.to_rust()),
            Expr::BesselIn(n, x) => format!("GSL::sf::bessel_In({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::BesselK0(x) => format!("GSL::sf::bessel_K0({})", x.to_rust()),
            Expr::BesselK1(x) => format!("GSL::sf::bessel_K1({})", x.to_rust()),
            Expr::BesselKn(n, x) => format!("GSL::sf::bessel_Kn({} as i32, {})", n.to_rust(), x.to_rust()),

            // 概率分布
            Expr::NormPdf(x, mu, sig) => format!("statrs::distribution::Normal::new({}, {}).unwrap().pdf({})", mu.to_rust(), sig.to_rust(), x.to_rust()),
            Expr::NormCdf(x, mu, sig) => format!("statrs::distribution::Normal::new({}, {}).unwrap().cdf({})", mu.to_rust(), sig.to_rust(), x.to_rust()),
            Expr::NormPpf(p, mu, sig) => format!("statrs::distribution::Normal::new({}, {}).unwrap().inverse_cdf({})", mu.to_rust(), sig.to_rust(), p.to_rust()),
            Expr::TPdf(x, df) => format!("statrs::distribution::StudentsT::new(0.0, 1.0, {}).unwrap().pdf({})", df.to_rust(), x.to_rust()),
            Expr::TCdf(x, df) => format!("statrs::distribution::StudentsT::new(0.0, 1.0, {}).unwrap().cdf({})", df.to_rust(), x.to_rust()),
            Expr::TPpf(p, df) => format!("statrs::distribution::StudentsT::new(0.0, 1.0, {}).unwrap().inverse_cdf({})", df.to_rust(), p.to_rust()),
            Expr::Chi2Pdf(x, df) => format!("statrs::distribution::ChiSquared::new({}).unwrap().pdf({})", df.to_rust(), x.to_rust()),
            Expr::Chi2Cdf(x, df) => format!("statrs::distribution::ChiSquared::new({}).unwrap().cdf({})", df.to_rust(), x.to_rust()),
            Expr::Chi2Ppf(p, df) => format!("statrs::distribution::ChiSquared::new({}).unwrap().inverse_cdf({})", df.to_rust(), p.to_rust()),
            Expr::FPdf(x, d1, d2) => format!("statrs::distribution::FisherSnedecor::new({}, {}).unwrap().pdf({})", d1.to_rust(), d2.to_rust(), x.to_rust()),
            Expr::FCdf(x, d1, d2) => format!("statrs::distribution::FisherSnedecor::new({}, {}).unwrap().cdf({})", d1.to_rust(), d2.to_rust(), x.to_rust()),
            Expr::FPpf(p, d1, d2) => format!("statrs::distribution::FisherSnedecor::new({}, {}).unwrap().inverse_cdf({})", d1.to_rust(), d2.to_rust(), p.to_rust()),
            Expr::PoissonPmf(k, lam) => format!("statrs::distribution::Poisson::new({}).unwrap().pmf({} as u64)", lam.to_rust(), k.to_rust()),
            Expr::PoissonCdf(k, lam) => format!("statrs::distribution::Poisson::new({}).unwrap().cdf({})", lam.to_rust(), k.to_rust()),
            Expr::BinomialPmf(k, n, p) => format!("statrs::distribution::Binomial::new({}, {} as u64).unwrap().pmf({} as u64)", p.to_rust(), n.to_rust(), k.to_rust()),
            Expr::BinomialCdf(k, n, p) => format!("statrs::distribution::Binomial::new({}, {} as u64).unwrap().cdf({})", p.to_rust(), n.to_rust(), k.to_rust()),
            Expr::ExponentialPdf(x, lam) => format!("statrs::distribution::Exp::new({}).unwrap().pdf({})", lam.to_rust(), x.to_rust()),
            Expr::ExponentialCdf(x, lam) => format!("statrs::distribution::Exp::new({}).unwrap().cdf({})", lam.to_rust(), x.to_rust()),

            // 复数运算
            Expr::Complex(re, im) => format!("num_complex::Complex::new({}, {})", re.to_rust(), im.to_rust()),
            Expr::Real(z) => format!("({}).re", z.to_rust()),
            Expr::Imag(z) => format!("({}).im", z.to_rust()),
            Expr::Conj(z) => format!("({}).conj()", z.to_rust()),
            Expr::Carg(z) => format!("({}).arg()", z.to_rust()),
            Expr::Cabs(z) => format!("({}).norm()", z.to_rust()),
            Expr::Polar(r, theta) => format!("num_complex::Complex::from_polar({}, {})", r.to_rust(), theta.to_rust()),

            // 基础数学补充
            Expr::Hypot(x, y) => format!("({}).hypot({})", x.to_rust(), y.to_rust()),
            Expr::Hypot3(x, y, z) => format!("(({}).powi(2) + ({}).powi(2) + ({}).powi(2)).sqrt()", x.to_rust(), y.to_rust(), z.to_rust()),
            Expr::Clamp(x, min, max) => format!("({}).clamp({}, {})", x.to_rust(), min.to_rust(), max.to_rust()),
            Expr::Copysign(x, y) => format!("({}).copysign({})", x.to_rust(), y.to_rust()),
            Expr::Fma(a, b, c) => format!("({}).mul_add({}, {})", a.to_rust(), b.to_rust(), c.to_rust()),
            Expr::Logn(base, x) => format!("({}).log({})", x.to_rust(), base.to_rust()),
            Expr::Sinc(x) => format!("(if {} == 0.0 {{ 1.0 }} else {{ ({}).sin() / {} }})", x.to_rust(), x.to_rust(), x.to_rust()),

            // 高精度数值函数
            Expr::Expm1(x) => format!("({}).exp_m1()", x.to_rust()),
            Expr::Log1p(x) => format!("({}).ln_1p()", x.to_rust()),
            Expr::Exp2(x) => format!("({}).exp2()", x.to_rust()),

            // 不完全伽马/贝塔函数
            Expr::Gammainc(a, x) => format!("puruspe::gammainc({}, {})", a.to_rust(), x.to_rust()),
            Expr::Gammaincc(a, x) => format!("puruspe::gammaincc({}, {})", a.to_rust(), x.to_rust()),
            Expr::Betainc(x, a, b) => format!("puruspe::betainc({}, {}, {})", a.to_rust(), b.to_rust(), x.to_rust()),

            // 扩展三角函数
            Expr::Sec(x) => format!("(1.0 / ({}).cos())", x.to_rust()),
            Expr::Csc(x) => format!("(1.0 / ({}).sin())", x.to_rust()),
            Expr::Cot(x) => format!("(1.0 / ({}).tan())", x.to_rust()),
            Expr::Asec(x) => format!("(1.0 / {}).acos()", x.to_rust()),
            Expr::Acsc(x) => format!("(1.0 / {}).asin()", x.to_rust()),
            Expr::Acot(x) => format!("(1.0 / {}).atan()", x.to_rust()),

            // 扩展双曲函数
            Expr::Sech(x) => format!("(1.0 / ({}).cosh())", x.to_rust()),
            Expr::Csch(x) => format!("(1.0 / ({}).sinh())", x.to_rust()),
            Expr::Coth(x) => format!("(1.0 / ({}).tanh())", x.to_rust()),
            Expr::Asech(x) => format!("(1.0 / {}).acosh()", x.to_rust()),
            Expr::Acsch(x) => format!("(1.0 / {}).asinh()", x.to_rust()),
            Expr::Acoth(x) => format!("(1.0 / {}).atanh()", x.to_rust()),

            // Airy 函数
            Expr::AiryAi(x) => format!("GSL::sf::airy_Ai({}, GSL::sf::MODE_DEFAULT)", x.to_rust()),
            Expr::AiryBi(x) => format!("GSL::sf::airy_Bi({}, GSL::sf::MODE_DEFAULT)", x.to_rust()),

            // 球谐函数
            Expr::SphericalHarmonic(l, m, theta, phi) => format!("GSL::sf::legendre_sphPlm({} as i32, {} as i32, ({}).cos()) * num_complex::Complex::from_polar(1.0, {} * {})", l.to_rust(), m.to_rust(), theta.to_rust(), m.to_rust(), phi.to_rust()),

            // Fresnel 积分
            Expr::FresnelS(x) => format!("GSL::sf::sin_pi({} * {} / 2.0)", x.to_rust(), x.to_rust()),
            Expr::FresnelC(x) => format!("GSL::sf::cos_pi({} * {} / 2.0)", x.to_rust(), x.to_rust()),

            // 其他特殊函数
            Expr::Dawson(x) => format!("GSL::sf::dawson({})", x.to_rust()),
            Expr::ExpInt(x) => format!("GSL::sf::expint_Ei({})", x.to_rust()),
            Expr::LogInt(x) => format!("GSL::sf::expint_Ei(({}).ln())", x.to_rust()),
            Expr::SinInt(x) => format!("GSL::sf::Si({})", x.to_rust()),
            Expr::CosInt(x) => format!("GSL::sf::Ci({})", x.to_rust()),

            // Lambert W
            Expr::LambertW(x) => format!("GSL::sf::lambert_W0({})", x.to_rust()),
            Expr::LambertWm1(x) => format!("GSL::sf::lambert_Wm1({})", x.to_rust()),

            // 球贝塞尔函数
            Expr::SphBesselJ(n, x) => format!("GSL::sf::bessel_jl({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::SphBesselY(n, x) => format!("GSL::sf::bessel_yl({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::SphBesselI(n, x) => format!("GSL::sf::bessel_il_scaled({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::SphBesselK(n, x) => format!("GSL::sf::bessel_kl_scaled({} as i32, {})", n.to_rust(), x.to_rust()),

            // 超几何函数
            Expr::Hyp0f1(b, x) => format!("GSL::sf::hyperg_0F1({}, {})", b.to_rust(), x.to_rust()),
            Expr::Hyp1f1(a, b, x) => format!("GSL::sf::hyperg_1F1({}, {}, {})", a.to_rust(), b.to_rust(), x.to_rust()),
            Expr::Hyp2f1(a, b, c, x) => format!("GSL::sf::hyperg_2F1({}, {}, {}, {})", a.to_rust(), b.to_rust(), c.to_rust(), x.to_rust()),

            // Kelvin 函数
            Expr::KelvinBer(x) => format!("GSL::sf::bessel_ber(0, {})", x.to_rust()),
            Expr::KelvinBei(x) => format!("GSL::sf::bessel_bei(0, {})", x.to_rust()),
            Expr::KelvinKer(x) => format!("GSL::sf::bessel_ker(0, {})", x.to_rust()),
            Expr::KelvinKei(x) => format!("GSL::sf::bessel_kei(0, {})", x.to_rust()),

            // 不完全椭圆积分
            Expr::EllipF(phi, k) => format!("GSL::sf::ellint_F({}, {}, GSL::sf::MODE_DEFAULT)", phi.to_rust(), k.to_rust()),
            Expr::EllipEInc(phi, k) => format!("GSL::sf::ellint_E({}, {}, GSL::sf::MODE_DEFAULT)", phi.to_rust(), k.to_rust()),
            Expr::EllipPi(phi, n, k) => format!("GSL::sf::ellint_P({}, {}, {}, GSL::sf::MODE_DEFAULT)", phi.to_rust(), k.to_rust(), n.to_rust()),

            // 其他特殊函数
            Expr::Spence(x) => format!("GSL::sf::dilog({})", x.to_rust()),
            Expr::Polygamma(n, x) => format!("GSL::sf::psi_n({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::Hankel1(n, x) => format!("num_complex::Complex::new(GSL::sf::bessel_Jn({} as i32, {}), GSL::sf::bessel_Yn({} as i32, {}))", n.to_rust(), x.to_rust(), n.to_rust(), x.to_rust()),
            Expr::Hankel2(n, x) => format!("num_complex::Complex::new(GSL::sf::bessel_Jn({} as i32, {}), -GSL::sf::bessel_Yn({} as i32, {}))", n.to_rust(), x.to_rust(), n.to_rust(), x.to_rust()),
            Expr::StruveH(v, x) => format!("/* struve_h({}, {}) - not in GSL */ 0.0", v.to_rust(), x.to_rust()),
            Expr::StruveL(v, x) => format!("/* struve_l({}, {}) - not in GSL */ 0.0", v.to_rust(), x.to_rust()),
            Expr::OwensT(h, a) => format!("statrs::function::owens_t::owens_t({}, {})", h.to_rust(), a.to_rust()),
            Expr::RiemannSiegelZ(t) => format!("/* riemann_siegel_z({}) */ 0.0", t.to_rust()),
            Expr::RiemannSiegelTheta(t) => format!("/* riemann_siegel_theta({}) */ 0.0", t.to_rust()),

            // Jacobi 椭圆函数
            Expr::JacobiSn(u, m) => format!("{{ let (sn, _, _, _) = GSL::sf::elljac_e({}, {}); sn }}", u.to_rust(), m.to_rust()),
            Expr::JacobiCn(u, m) => format!("{{ let (_, cn, _, _) = GSL::sf::elljac_e({}, {}); cn }}", u.to_rust(), m.to_rust()),
            Expr::JacobiDn(u, m) => format!("{{ let (_, _, dn, _) = GSL::sf::elljac_e({}, {}); dn }}", u.to_rust(), m.to_rust()),

            // 广义正交多项式
            Expr::Gegenbauer(n, alpha, x) => format!("GSL::sf::gegenpoly_n({} as i32, {}, {})", n.to_rust(), alpha.to_rust(), x.to_rust()),
            Expr::JacobiP(n, alpha, beta, x) => format!("/* jacobi_p({}, {}, {}, {}) */ 0.0", n.to_rust(), alpha.to_rust(), beta.to_rust(), x.to_rust()),

            // Mathieu 函数
            Expr::MathieuA(n, q) => format!("GSL::sf::mathieu_a({} as i32, {})", n.to_rust(), q.to_rust()),
            Expr::MathieuB(n, q) => format!("GSL::sf::mathieu_b({} as i32, {})", n.to_rust(), q.to_rust()),
            Expr::MathieuCe(n, q, x) => format!("GSL::sf::mathieu_ce({} as i32, {}, {})", n.to_rust(), q.to_rust(), x.to_rust()),
            Expr::MathieuSe(n, q, x) => format!("GSL::sf::mathieu_se({} as i32, {}, {})", n.to_rust(), q.to_rust(), x.to_rust()),

            // Coulomb 波函数
            Expr::CoulombF(l, eta, rho) => format!("GSL::sf::coulomb_wave_F_array({}, 1, {}, {})[0]", l.to_rust(), eta.to_rust(), rho.to_rust()),
            Expr::CoulombG(l, eta, rho) => format!("GSL::sf::coulomb_wave_G_array({}, 1, {}, {})[0]", l.to_rust(), eta.to_rust(), rho.to_rust()),

            // Wigner 符号
            Expr::Wigner3j(j1, j2, j3, m1, m2, m3) => format!("GSL::sf::coupling_3j(({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32)", j1.to_rust(), j2.to_rust(), j3.to_rust(), m1.to_rust(), m2.to_rust(), m3.to_rust()),
            Expr::Wigner6j(j1, j2, j3, j4, j5, j6) => format!("GSL::sf::coupling_6j(({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32)", j1.to_rust(), j2.to_rust(), j3.to_rust(), j4.to_rust(), j5.to_rust(), j6.to_rust()),
            Expr::Wigner9j(j1, j2, j3, j4, j5, j6, j7, j8, j9) => format!("GSL::sf::coupling_9j(({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32, ({} * 2.0) as i32)", j1.to_rust(), j2.to_rust(), j3.to_rust(), j4.to_rust(), j5.to_rust(), j6.to_rust(), j7.to_rust(), j8.to_rust(), j9.to_rust()),

            // Theta 函数
            Expr::Theta1(z, q) => format!("/* theta1({}, {}) */ 0.0", z.to_rust(), q.to_rust()),
            Expr::Theta2(z, q) => format!("/* theta2({}, {}) */ 0.0", z.to_rust(), q.to_rust()),
            Expr::Theta3(z, q) => format!("/* theta3({}, {}) */ 0.0", z.to_rust(), q.to_rust()),
            Expr::Theta4(z, q) => format!("/* theta4({}, {}) */ 0.0", z.to_rust(), q.to_rust()),

            // 抛物柱面函数 (使用 scirs2_special)
            Expr::Pbdv(v, x) => format!("scirs2_special::pbdv({}, {}).0", v.to_rust(), x.to_rust()),
            Expr::Pbvv(v, x) => format!("scirs2_special::pbvv({}, {}).0", v.to_rust(), x.to_rust()),
            Expr::Pbwa(a, x) => format!("scirs2_special::pbwa({}, {}).0", a.to_rust(), x.to_rust()),

            // 球扁旋转体波函数
            Expr::ProAng1(m, n, c, x) => format!("scirs2_special::pro_ang1({}, {}, {}, {}).0", m.to_rust(), n.to_rust(), c.to_rust(), x.to_rust()),
            Expr::ProRad1(m, n, c, x) => format!("scirs2_special::pro_rad1({}, {}, {}, {}).0", m.to_rust(), n.to_rust(), c.to_rust(), x.to_rust()),
            Expr::ProRad2(m, n, c, x) => format!("scirs2_special::pro_rad2({}, {}, {}, {}).0", m.to_rust(), n.to_rust(), c.to_rust(), x.to_rust()),
            Expr::OblAng1(m, n, c, x) => format!("scirs2_special::obl_ang1({}, {}, {}, {}).0", m.to_rust(), n.to_rust(), c.to_rust(), x.to_rust()),
            Expr::OblRad1(m, n, c, x) => format!("scirs2_special::obl_rad1({}, {}, {}, {}).0", m.to_rust(), n.to_rust(), c.to_rust(), x.to_rust()),
            Expr::OblRad2(m, n, c, x) => format!("scirs2_special::obl_rad2({}, {}, {}, {}).0", m.to_rust(), n.to_rust(), c.to_rust(), x.to_rust()),

            // 修改 Fresnel 积分
            Expr::ModFresnelP(x) => format!("scirs2_special::mod_fresnel_plus({}).0", x.to_rust()),
            Expr::ModFresnelM(x) => format!("scirs2_special::mod_fresnelminus({}).0", x.to_rust()),

            // Wright 函数
            Expr::WrightBessel(rho, beta, z) => format!("scirs2_special::wright_bessel({}, {}, {})", rho.to_rust(), beta.to_rust(), z.to_rust()),
            Expr::WrightOmega(z) => format!("scirs2_special::wright_omega_real({})", z.to_rust()),

            // Voigt
            Expr::Voigt(x, sigma, gamma) => format!("scirs2_special::voigt_profile({}, {}, {})", x.to_rust(), sigma.to_rust(), gamma.to_rust()),

            // Sigmoid/Logistic
            Expr::Logit(x) => format!("(({x}).ln() - (1.0 - {x}).ln())", x = x.to_rust()),
            Expr::Expit(x) => format!("(1.0 / (1.0 + (-{}).exp()))", x.to_rust()),

            // Box-Cox
            Expr::BoxCox(x, lmbda) => format!("scirs2_special::boxcox({}, {})", x.to_rust(), lmbda.to_rust()),
            Expr::BoxCox1p(x, lmbda) => format!("scirs2_special::boxcox1p({}, {})", x.to_rust(), lmbda.to_rust()),
            Expr::InvBoxCox(y, lmbda) => format!("scirs2_special::inv_boxcox({}, {})", y.to_rust(), lmbda.to_rust()),
            Expr::InvBoxCox1p(y, lmbda) => format!("scirs2_special::inv_boxcox1p({}, {})", y.to_rust(), lmbda.to_rust()),

            // 信息论
            Expr::Entr(x) => format!("{{ let x = {}; if x > 0.0 {{ -x * x.ln() }} else {{ 0.0 }} }}", x.to_rust()),
            Expr::RelEntr(x, y) => format!("{{ let (x, y) = ({}, {}); if x > 0.0 && y > 0.0 {{ x * (x / y).ln() }} else {{ 0.0 }} }}", x.to_rust(), y.to_rust()),
            Expr::KlDiv(x, y) => format!("{{ let (x, y) = ({}, {}); if x > 0.0 && y > 0.0 {{ x * (x / y).ln() - x + y }} else {{ y }} }}", x.to_rust(), y.to_rust()),

            // 阶乘扩展
            Expr::Factorial2(n) => format!("scirs2_special::factorial2({} as u64)", n.to_rust()),
            Expr::Factorialk(n, k) => format!("scirs2_special::factorialk({} as u64, {} as u64)", n.to_rust(), k.to_rust()),
            Expr::Stirling2(n, k) => format!("scirs2_special::stirling2({} as u64, {} as u64)", n.to_rust(), k.to_rust()),
            Expr::Poch(z, m) => format!("scirs2_special::pochhammer({}, {})", z.to_rust(), m.to_rust()),

            // Carlson 椭圆积分
            Expr::EllipRc(x, y) => format!("scirs2_special::elliprc({}, {})", x.to_rust(), y.to_rust()),
            Expr::EllipRd(x, y, z) => format!("scirs2_special::elliprd({}, {}, {})", x.to_rust(), y.to_rust(), z.to_rust()),
            Expr::EllipRf(x, y, z) => format!("scirs2_special::elliprf({}, {}, {})", x.to_rust(), y.to_rust(), z.to_rust()),
            Expr::EllipRg(x, y, z) => format!("scirs2_special::elliprg({}, {}, {})", x.to_rust(), y.to_rust(), z.to_rust()),
            Expr::EllipRj(x, y, z, p) => format!("scirs2_special::elliprj({}, {}, {}, {})", x.to_rust(), y.to_rust(), z.to_rust(), p.to_rust()),

            // 扩展误差函数
            Expr::Erfcx(x) => format!("({x}.powi(2).exp() * puruspe::erfc({x}))", x = x.to_rust()),
            Expr::Erfi(x) => format!("(2.0 / std::f64::consts::PI.sqrt() * {x}.powi(2).exp() * puruspe::dawson({x}))", x = x.to_rust()),
            Expr::Erfcinv(x) => format!("(-puruspe::erfinv(1.0 - {}))", x.to_rust()),

            // 扩展 Gamma
            Expr::Hyperu(a, b, x) => format!("scirs2_special::hyperu({}, {}, {})", a.to_rust(), b.to_rust(), x.to_rust()),
            Expr::Rgamma(x) => format!("(1.0 / puruspe::gamma({}))", x.to_rust()),
            Expr::Gammasgn(x) => format!("puruspe::gamma({}).signum()", x.to_rust()),

            // 便利函数
            Expr::Agm(a, b) => format!("scirs2_special::agm({}, {})", a.to_rust(), b.to_rust()),
            Expr::Exprel(x) => format!("{{ let x = {}; if x.abs() < 1e-10 {{ 1.0 }} else {{ (x.exp() - 1.0) / x }} }}", x.to_rust()),
            Expr::Xlogy(x, y) => format!("{{ let (x, y) = ({}, {}); if x == 0.0 {{ 0.0 }} else {{ x * y.ln() }} }}", x.to_rust(), y.to_rust()),
            Expr::Xlog1py(x, y) => format!("{{ let (x, y) = ({}, {}); if x == 0.0 {{ 0.0 }} else {{ x * (1.0 + y).ln() }} }}", x.to_rust(), y.to_rust()),

            // Zeta 扩展
            Expr::HurwitzZeta(s, q) => format!("scirs2_special::hurwitz_zeta({}, {})", s.to_rust(), q.to_rust()),
            Expr::Zetac(x) => format!("(scirs2_special::zeta({}) - 1.0)", x.to_rust()),
            Expr::Polylog(s, z) => format!("scirs2_special::polylog({}, {})", s.to_rust(), z.to_rust()),

            // === 缩放贝塞尔函数 ===
            Expr::BesselI0e(x) => format!("{{ let x = {}; puruspe::bessel_i0(x) * (-x.abs()).exp() }}", x.to_rust()),
            Expr::BesselI1e(x) => format!("{{ let x = {}; puruspe::bessel_i1(x) * (-x.abs()).exp() }}", x.to_rust()),
            Expr::BesselIne(n, x) => format!("{{ let (n, x) = ({}, {}); GSL::sf::bessel_Inu_scaled(n, x) }}", n.to_rust(), x.to_rust()),
            Expr::BesselK0e(x) => format!("{{ let x = {}; puruspe::bessel_k0(x) * x.exp() }}", x.to_rust()),
            Expr::BesselK1e(x) => format!("{{ let x = {}; puruspe::bessel_k1(x) * x.exp() }}", x.to_rust()),
            Expr::BesselKne(n, x) => format!("{{ let (n, x) = ({}, {}); GSL::sf::bessel_Knu_scaled(n, x) }}", n.to_rust(), x.to_rust()),
            Expr::BesselJne(n, x) => format!("{{ let (n, x) = ({}, {}); GSL::sf::bessel_Jnu(n, x) * (-x.abs()).exp() }}", n.to_rust(), x.to_rust()),
            Expr::BesselYne(n, x) => format!("{{ let (n, x) = ({}, {}); GSL::sf::bessel_Ynu(n, x) * (-x.abs()).exp() }}", n.to_rust(), x.to_rust()),
            Expr::Hankel1e(n, x) => format!("{{ let (n, x) = ({}, {}); num_complex::Complex64::new(GSL::sf::bessel_Jnu(n, x), GSL::sf::bessel_Ynu(n, x)) * num_complex::Complex64::new(0.0, -x).exp() }}", n.to_rust(), x.to_rust()),
            Expr::Hankel2e(n, x) => format!("{{ let (n, x) = ({}, {}); num_complex::Complex64::new(GSL::sf::bessel_Jnu(n, x), -GSL::sf::bessel_Ynu(n, x)) * num_complex::Complex64::new(0.0, x).exp() }}", n.to_rust(), x.to_rust()),

            // === 贝塞尔函数导数（使用递推关系）===
            Expr::BesselJnp(n, x) => format!("{{ let (n, x) = ({}, {}); 0.5 * (GSL::sf::bessel_Jnu(n - 1.0, x) - GSL::sf::bessel_Jnu(n + 1.0, x)) }}", n.to_rust(), x.to_rust()),
            Expr::BesselYnp(n, x) => format!("{{ let (n, x) = ({}, {}); 0.5 * (GSL::sf::bessel_Ynu(n - 1.0, x) - GSL::sf::bessel_Ynu(n + 1.0, x)) }}", n.to_rust(), x.to_rust()),
            Expr::BesselInp(n, x) => format!("{{ let (n, x) = ({}, {}); 0.5 * (GSL::sf::bessel_Inu(n - 1.0, x) + GSL::sf::bessel_Inu(n + 1.0, x)) }}", n.to_rust(), x.to_rust()),
            Expr::BesselKnp(n, x) => format!("{{ let (n, x) = ({}, {}); -0.5 * (GSL::sf::bessel_Knu(n - 1.0, x) + GSL::sf::bessel_Knu(n + 1.0, x)) }}", n.to_rust(), x.to_rust()),
            Expr::Hankel1p(n, x) => format!("{{ let (n, x) = ({}, {}); let h1 = num_complex::Complex64::new(GSL::sf::bessel_Jnu(n, x), GSL::sf::bessel_Ynu(n, x)); let h1_prev = num_complex::Complex64::new(GSL::sf::bessel_Jnu(n - 1.0, x), GSL::sf::bessel_Ynu(n - 1.0, x)); h1_prev - (n / x) * h1 }}", n.to_rust(), x.to_rust()),
            Expr::Hankel2p(n, x) => format!("{{ let (n, x) = ({}, {}); let h2 = num_complex::Complex64::new(GSL::sf::bessel_Jnu(n, x), -GSL::sf::bessel_Ynu(n, x)); let h2_prev = num_complex::Complex64::new(GSL::sf::bessel_Jnu(n - 1.0, x), -GSL::sf::bessel_Ynu(n - 1.0, x)); h2_prev - (n / x) * h2 }}", n.to_rust(), x.to_rust()),

            // === Huber 损失 ===
            Expr::Huber(delta, r) => format!("{{ let (d, r) = ({}, {}); if r.abs() <= d {{ 0.5 * r * r }} else {{ d * (r.abs() - 0.5 * d) }} }}", delta.to_rust(), r.to_rust()),
            Expr::PseudoHuber(delta, r) => format!("{{ let (d, r) = ({}, {}); d * d * ((1.0 + (r / d).powi(2)).sqrt() - 1.0) }}", delta.to_rust(), r.to_rust()),

            // === Kolmogorov-Smirnov ===
            Expr::Kolmogorov(y) => format!("scirs2_special::kolmogorov({})", y.to_rust()),
            Expr::Kolmogi(p) => format!("scirs2_special::kolmogi({})", p.to_rust()),
            Expr::Smirnov(n, d) => format!("scirs2_special::smirnov({} as usize, {})", n.to_rust(), d.to_rust()),
            Expr::Smirnovi(n, p) => format!("scirs2_special::smirnovi({} as usize, {})", n.to_rust(), p.to_rust()),

            // === Faddeeva (复数误差函数) ===
            Expr::Wofz(z) => format!("scirs2_special::wofz({})", z.to_rust()),

            // === Dirichlet 核 ===
            Expr::Diric(x, n) => format!("{{ let (x, n) = ({}, {} as i32); if (x % (2.0 * std::f64::consts::PI)).abs() < 1e-15 {{ if n % 2 == 1 {{ 1.0 }} else {{ -1.0 }} }} else {{ (n as f64 * x / 2.0).sin() / (n as f64 * (x / 2.0).sin()) }} }}", x.to_rust(), n.to_rust()),

            // === Tukey lambda ===
            Expr::Tklmbda(x, lam) => format!("{{ let (x, lam) = ({}, {}); if lam.abs() < 1e-15 {{ (x - 0.5).signum() * ((0.5 - (x - 0.5).abs()).abs().ln()) }} else {{ (x.powf(lam) - (1.0 - x).powf(lam)) / lam }} }}", x.to_rust(), lam.to_rust()),

            // === Gamma/Beta 逆函数 ===
            Expr::Gammaincinv(a, y) => format!("scirs2_special::gammaincinv({}, {})", a.to_rust(), y.to_rust()),
            Expr::Gammainccinv(a, y) => format!("scirs2_special::gammainccinv({}, {})", a.to_rust(), y.to_rust()),
            Expr::Betaincinv(a, b, y) => format!("scirs2_special::betaincinv({}, {}, {})", a.to_rust(), b.to_rust(), y.to_rust()),

            // === 高精度便利函数 ===
            Expr::Cosm1(x) => format!("{{ let x = {}; -2.0 * (x / 2.0).sin().powi(2) }}", x.to_rust()),
            Expr::Powm1(x, y) => format!("{{ let (x, y) = ({}, {}); x.powf(y) - 1.0 }}", x.to_rust(), y.to_rust()),
            Expr::Exp10(x) => format!("(10.0_f64).powf({})", x.to_rust()),
            Expr::Log1pmx(x) => format!("{{ let x = {}; (1.0 + x).ln() - x }}", x.to_rust()),
            Expr::Loggamma(z) => format!("scirs2_special::loggamma({})", z.to_rust()),

            // === 度数三角函数 ===
            Expr::Cosdg(x) => format!("({} * std::f64::consts::PI / 180.0).cos()", x.to_rust()),
            Expr::Sindg(x) => format!("({} * std::f64::consts::PI / 180.0).sin()", x.to_rust()),
            Expr::Tandg(x) => format!("({} * std::f64::consts::PI / 180.0).tan()", x.to_rust()),
            Expr::Cotdg(x) => format!("(1.0 / ({} * std::f64::consts::PI / 180.0).tan())", x.to_rust()),
            Expr::Radian(d, m, s) => format!("(({} + {} / 60.0 + {} / 3600.0) * std::f64::consts::PI / 180.0)", d.to_rust(), m.to_rust(), s.to_rust()),

            // === Airy 扩展 ===
            Expr::AiryAie(x) => format!("scirs2_special::aie({})", x.to_rust()),
            Expr::AiryBie(x) => format!("scirs2_special::bie({})", x.to_rust()),
            Expr::AiryAip(x) => format!("scirs2_special::aip({})", x.to_rust()),
            Expr::AiryBip(x) => format!("scirs2_special::bip({})", x.to_rust()),
            Expr::ItAiry(x) => format!("scirs2_special::itairy({})", x.to_rust()),

            // === 指数积分扩展 ===
            Expr::Expn(n, x) => format!("scirs2_special::expint({} as i32, {})", n.to_rust(), x.to_rust()),
            Expr::Exp1(x) => format!("scirs2_special::e1({})", x.to_rust()),
            Expr::Shi(x) => format!("scirs2_special::shi({})", x.to_rust()),
            Expr::Chi(x) => format!("scirs2_special::chi({})", x.to_rust()),

            // === Struve 积分 ===
            Expr::ItStruve0(x) => format!("scirs2_special::it_struve0({})", x.to_rust()),
            Expr::It2Struve0(x) => format!("scirs2_special::it2_struve0({})", x.to_rust()),
            Expr::ItModStruve0(x) => format!("scirs2_special::it_mod_struve0({})", x.to_rust()),

            // === ML/统计扩展 ===
            Expr::LogExpit(x) => format!("{{ let x = {}; -((1.0 + (-x).exp()).ln()) }}", x.to_rust()),
            Expr::Softplus(x) => format!("{{ let x = {}; if x > 20.0 {{ x }} else {{ (1.0 + x.exp()).ln() }} }}", x.to_rust()),
            Expr::LogNdtr(x) => format!("scirs2_special::distributions::log_ndtr({})", x.to_rust()),

            // === Beta 补函数 ===
            Expr::Betaincc(a, b, x) => format!("(1.0 - puruspe::betai({}, {}, {}))", x.to_rust(), a.to_rust(), b.to_rust()),
            Expr::Betainccinv(a, b, y) => format!("scirs2_special::betainccinv({}, {}, {})", a.to_rust(), b.to_rust(), y.to_rust()),

            // === 数论函数 ===
            Expr::Bernoulli(n) => format!("scirs2_special::bernoulli_number({} as usize)", n.to_rust()),
            Expr::Euler(n) => format!("scirs2_special::euler_number({} as usize)", n.to_rust()),

            // === 椭圆扩展 ===
            Expr::EllipKm1(p) => format!("scirs2_special::ellipkm1({})", p.to_rust()),

            // === Kelvin 导数 ===
            Expr::KelvinBerp(x) => format!("scirs2_special::berp({})", x.to_rust()),
            Expr::KelvinBeip(x) => format!("scirs2_special::beip({})", x.to_rust()),
            Expr::KelvinKerp(x) => format!("scirs2_special::kerp({})", x.to_rust()),
            Expr::KelvinKeip(x) => format!("scirs2_special::keip({})", x.to_rust()),

            // === 贝塞尔积分 ===
            Expr::BesselPoly(a, lmb, nu) => format!("scirs2_special::besselpoly({}, {}, {})", a.to_rust(), lmb.to_rust(), nu.to_rust()),

            // === Wright Bessel 扩展 ===
            Expr::LogWrightBessel(a, b, x) => format!("scirs2_special::log_wright_bessel({}, {}, {})", a.to_rust(), b.to_rust(), x.to_rust()),

            // === 二项系数扩展 ===
            Expr::Binom(x, y) => format!("scirs2_special::binomial({} as usize, {} as usize) as f64", x.to_rust(), y.to_rust()),

            // === 分布函数 ===
            Expr::Bdtr(k, n, p) => format!("statrs::distribution::Binomial::new({}, {} as u64).unwrap().cdf({} as u64)", p.to_rust(), n.to_rust(), k.to_rust()),
            Expr::Bdtrc(k, n, p) => format!("(1.0 - statrs::distribution::Binomial::new({}, {} as u64).unwrap().cdf({} as u64))", p.to_rust(), n.to_rust(), k.to_rust()),
            Expr::Bdtri(_k, n, y) => format!("statrs::distribution::Binomial::new(0.5, {} as u64).unwrap().inverse_cdf({})", n.to_rust(), y.to_rust()),
            Expr::Chdtr(v, x) => format!("statrs::distribution::ChiSquared::new({}).unwrap().cdf({})", v.to_rust(), x.to_rust()),
            Expr::Chdtrc(v, x) => format!("(1.0 - statrs::distribution::ChiSquared::new({}).unwrap().cdf({}))", v.to_rust(), x.to_rust()),
            Expr::Chdtri(v, p) => format!("statrs::distribution::ChiSquared::new({}).unwrap().inverse_cdf({})", v.to_rust(), p.to_rust()),
            Expr::Fdtr(dfn, dfd, x) => format!("statrs::distribution::FisherSnedecor::new({}, {}).unwrap().cdf({})", dfn.to_rust(), dfd.to_rust(), x.to_rust()),
            Expr::Fdtrc(dfn, dfd, x) => format!("(1.0 - statrs::distribution::FisherSnedecor::new({}, {}).unwrap().cdf({}))", dfn.to_rust(), dfd.to_rust(), x.to_rust()),
            Expr::Fdtri(dfn, dfd, p) => format!("statrs::distribution::FisherSnedecor::new({}, {}).unwrap().inverse_cdf({})", dfn.to_rust(), dfd.to_rust(), p.to_rust()),
            Expr::Stdtr(df, t) => format!("statrs::distribution::StudentsT::new(0.0, 1.0, {}).unwrap().cdf({})", df.to_rust(), t.to_rust()),
            Expr::Stdtrc(df, t) => format!("(1.0 - statrs::distribution::StudentsT::new(0.0, 1.0, {}).unwrap().cdf({}))", df.to_rust(), t.to_rust()),
            Expr::Stdtrit(df, p) => format!("statrs::distribution::StudentsT::new(0.0, 1.0, {}).unwrap().inverse_cdf({})", df.to_rust(), p.to_rust()),
            Expr::Pdtr(k, m) => format!("statrs::distribution::Poisson::new({}).unwrap().cdf({} as f64)", m.to_rust(), k.to_rust()),
            Expr::Pdtrc(k, m) => format!("(1.0 - statrs::distribution::Poisson::new({}).unwrap().cdf({} as f64))", m.to_rust(), k.to_rust()),
            Expr::Pdtri(k, y) => format!("statrs::distribution::Poisson::new({}).unwrap().inverse_cdf({})", k.to_rust(), y.to_rust()),
            Expr::Btdtr(a, b, x) => format!("statrs::distribution::Beta::new({}, {}).unwrap().cdf({})", a.to_rust(), b.to_rust(), x.to_rust()),
            Expr::Btdtrc(a, b, x) => format!("(1.0 - statrs::distribution::Beta::new({}, {}).unwrap().cdf({}))", a.to_rust(), b.to_rust(), x.to_rust()),
            Expr::Gdtr(a, b, x) => format!("statrs::distribution::Gamma::new({}, 1.0/{}).unwrap().cdf({})", a.to_rust(), b.to_rust(), x.to_rust()),
            Expr::Gdtrc(a, b, x) => format!("(1.0 - statrs::distribution::Gamma::new({}, 1.0/{}).unwrap().cdf({}))", a.to_rust(), b.to_rust(), x.to_rust()),

            // === 积分/ML 扩展 ===
            Expr::Sici(x) => format!("scirs2_special::sici({})", x.to_rust()),
            Expr::Shichi(x) => format!("scirs2_special::shichi({})", x.to_rust()),
            Expr::Softmax(x) => format!("{{ let v = {}; let max_v = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max); let exp_v: Vec<f64> = v.iter().map(|x| (x - max_v).exp()).collect(); let sum: f64 = exp_v.iter().sum(); exp_v.iter().map(|x| x / sum).collect::<Vec<f64>>() }}", x.to_rust()),
            Expr::LogSoftmax(x) => format!("{{ let v = {}; let max_v = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max); let lse = max_v + v.iter().map(|x| (x - max_v).exp()).sum::<f64>().ln(); v.iter().map(|x| x - lse).collect::<Vec<f64>>() }}", x.to_rust()),
            Expr::Logsumexp(x) => format!("{{ let v = {}; let max_v = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max); max_v + v.iter().map(|x| (x - max_v).exp()).sum::<f64>().ln() }}", x.to_rust()),

            // === GSL 扩展 ===
            Expr::AiryZeroAi(s) => format!("GSL::sf::airy_zero_Ai({} as u32)", s.to_rust()),
            Expr::AiryZeroBi(s) => format!("GSL::sf::airy_zero_Bi({} as u32)", s.to_rust()),
            Expr::BesselZeroJ0(s) => format!("GSL::sf::bessel_zero_J0({} as u32)", s.to_rust()),
            Expr::BesselZeroJ1(s) => format!("GSL::sf::bessel_zero_J1({} as u32)", s.to_rust()),
            Expr::BesselZeroJnu(nu, s) => format!("GSL::sf::bessel_zero_Jnu({}, {} as u32)", nu.to_rust(), s.to_rust()),
            Expr::SphLegendre(l, m, x) => format!("GSL::sf::legendre_sphPlm({} as i32, {} as i32, {})", l.to_rust(), m.to_rust(), x.to_rust()),
            Expr::Clausen(x) => format!("GSL::sf::clausen({})", x.to_rust()),
            Expr::Debye(n, x) => format!("match {} as i32 {{ 1 => GSL::sf::debye_1({}), 2 => GSL::sf::debye_2({}), 3 => GSL::sf::debye_3({}), 4 => GSL::sf::debye_4({}), 5 => GSL::sf::debye_5({}), _ => GSL::sf::debye_6({}) }}", n.to_rust(), x.to_rust(), x.to_rust(), x.to_rust(), x.to_rust(), x.to_rust(), x.to_rust()),
            Expr::Synchrotron1(x) => format!("GSL::sf::synchrotron_1({})", x.to_rust()),
            Expr::Synchrotron2(x) => format!("GSL::sf::synchrotron_2({})", x.to_rust()),
            Expr::Transport(n, x) => format!("match {} as i32 {{ 2 => GSL::sf::transport_2({}), 3 => GSL::sf::transport_3({}), 4 => GSL::sf::transport_4({}), _ => GSL::sf::transport_5({}) }}", n.to_rust(), x.to_rust(), x.to_rust(), x.to_rust(), x.to_rust()),
            Expr::FermiDirac(j, x) => format!("GSL::sf::fermi_dirac_int({} as i32, {})", j.to_rust(), x.to_rust()),
        }
    }

    /// 转换为 LaTeX 代码
    pub fn to_latex(&self) -> String {
        // 注册表快路径：已迁移算子从 ops 注册表生成（单一真相源）。
        // 下方 match 中这些算子的分支已不可达，仅为保持 match 穷尽性而保留，待后续清理。
        if let Some((name, args)) = crate::ops::as_operator(self) {
            if let Some(s) = crate::ops::spec(name) {
                let codes: Vec<String> = args.iter().map(|a| a.to_latex()).collect();
                return (s.latex)(&codes);
            }
        }
        match self {
            Expr::Const(value) => format!("{}", value),
            Expr::Var(name) => Self::name_to_latex(name),
            Expr::Param(name) => Self::name_to_latex(name),
            Expr::Pi => "\\pi".to_string(),
            Expr::E => "e".to_string(),
            Expr::Reduce { kind, arg } => {
                format!("\\operatorname{{{}}}\\left({}\\right)", kind.name(), arg.to_latex())
            }

            // 算术运算
            Expr::Add(a, b) => format!("{} + {}", a.to_latex(), b.to_latex()),
            Expr::Sub(a, b) => format!("{} - {}", a.to_latex(), b.to_latex()),
            Expr::Mul(a, b) => format!("{} \\times {}", a.to_latex(), b.to_latex()),
            Expr::Div(a, b) => format!("\\frac{{{}}}{{{}}}", a.to_latex(), b.to_latex()),
            Expr::Neg(a) => format!("-{}", a.to_latex()),
            Expr::Pow(a, b) => format!("{}^{{{}}}", a.to_latex(), b.to_latex()),
            Expr::Abs(a) => format!("|{}|", a.to_latex()),
            Expr::Mod(a, b) => format!("{} \\mod {}", a.to_latex(), b.to_latex()),
            Expr::Ceil(a) => format!("\\lceil {} \\rceil", a.to_latex()),
            Expr::Floor(a) => format!("\\lfloor {} \\rfloor", a.to_latex()),
            Expr::Round(a) => format!("\\text{{round}}({})", a.to_latex()),
            Expr::Trunc(a) => format!("\\text{{trunc}}({})", a.to_latex()),
            Expr::Sign(a) => format!("\\text{{sgn}}({})", a.to_latex()),

            // 超越函数
            Expr::Exp(a) => format!("e^{{{}}}", a.to_latex()),
            Expr::Ln(a) => format!("\\ln({})", a.to_latex()),
            Expr::Log10(a) => format!("\\log_{{10}}({})", a.to_latex()),
            Expr::Log2(a) => format!("\\log_{{2}}({})", a.to_latex()),
            Expr::Sqrt(a) => format!("\\sqrt{{{}}}", a.to_latex()),
            Expr::Cbrt(a) => format!("\\sqrt[3]{{{}}}", a.to_latex()),

            // 三角函数
            Expr::Sin(a) => format!("\\sin({})", a.to_latex()),
            Expr::Cos(a) => format!("\\cos({})", a.to_latex()),
            Expr::Tan(a) => format!("\\tan({})", a.to_latex()),
            Expr::ASin(a) => format!("\\arcsin({})", a.to_latex()),
            Expr::ACos(a) => format!("\\arccos({})", a.to_latex()),
            Expr::ATan(a) => format!("\\arctan({})", a.to_latex()),
            Expr::ATan2(y, x) => format!("\\text{{atan2}}({}, {})", y.to_latex(), x.to_latex()),

            // 双曲函数
            Expr::Sinh(a) => format!("\\sinh({})", a.to_latex()),
            Expr::Cosh(a) => format!("\\cosh({})", a.to_latex()),
            Expr::Tanh(a) => format!("\\tanh({})", a.to_latex()),
            Expr::ASinh(a) => format!("\\text{{asinh}}({})", a.to_latex()),
            Expr::ACosh(a) => format!("\\text{{acosh}}({})", a.to_latex()),
            Expr::ATanh(a) => format!("\\text{{atanh}}({})", a.to_latex()),

            // 聚合函数
            Expr::Max(args) => {
                let args_tex: Vec<String> = args.iter().map(|a| a.to_latex()).collect();
                format!("\\max({})", args_tex.join(", "))
            }
            Expr::Min(args) => {
                let args_tex: Vec<String> = args.iter().map(|a| a.to_latex()).collect();
                format!("\\min({})", args_tex.join(", "))
            }

            // 求和
            Expr::Sum { index, lower, upper, body } => {
                format!(
                    "\\sum_{{{}={}}}^{{{}}} {}",
                    index,
                    lower.to_latex(),
                    upper.to_latex(),
                    body.to_latex()
                )
            }

            // 连乘
            Expr::Product { index, lower, upper, body } => {
                format!(
                    "\\prod_{{{}={}}}^{{{}}} {}",
                    index,
                    lower.to_latex(),
                    upper.to_latex(),
                    body.to_latex()
                )
            }

            // 关系运算
            Expr::Eq(a, b) => format!("{} = {}", a.to_latex(), b.to_latex()),
            Expr::Lt(a, b) => format!("{} < {}", a.to_latex(), b.to_latex()),
            Expr::Gt(a, b) => format!("{} > {}", a.to_latex(), b.to_latex()),
            Expr::Leq(a, b) => format!("{} \\leq {}", a.to_latex(), b.to_latex()),
            Expr::Geq(a, b) => format!("{} \\geq {}", a.to_latex(), b.to_latex()),
            Expr::Neq(a, b) => format!("{} \\neq {}", a.to_latex(), b.to_latex()),

            // 逻辑运算
            Expr::And(a, b) => format!("{} \\land {}", a.to_latex(), b.to_latex()),
            Expr::Or(a, b) => format!("{} \\lor {}", a.to_latex(), b.to_latex()),
            Expr::Not(a) => format!("\\neg {}", a.to_latex()),

            // 条件表达式
            Expr::IfThenElse { cond, then_branch, else_branch } => {
                format!(
                    "\\begin{{cases}} {} & \\text{{if }} {} \\\\ {} & \\text{{otherwise}} \\end{{cases}}",
                    then_branch.to_latex(),
                    cond.to_latex(),
                    else_branch.to_latex()
                )
            }

            Expr::Piecewise { pieces, otherwise } => {
                let mut cases = String::new();
                for (cond, value) in pieces {
                    cases.push_str(&format!(
                        "{} & \\text{{if }} {} \\\\ ",
                        value.to_latex(),
                        cond.to_latex()
                    ));
                }
                cases.push_str(&format!("{} & \\text{{otherwise}}", otherwise.to_latex()));
                format!("\\begin{{cases}} {} \\end{{cases}}", cases)
            }

            // 扩展分位数函数
            Expr::ExpPpf(p, lam) => format!("\\text{{Exp}}^{{-1}}({};{})", p.to_latex(), lam.to_latex()),
            Expr::GammaPpf(p, a, b) => format!("\\Gamma^{{-1}}({};{},{})", p.to_latex(), a.to_latex(), b.to_latex()),
            Expr::BetaPpf(p, a, b) => format!("B^{{-1}}({};{},{})", p.to_latex(), a.to_latex(), b.to_latex()),
            Expr::WeibullPpf(p, k, lam) => format!("W^{{-1}}({};{},{})", p.to_latex(), k.to_latex(), lam.to_latex()),
            Expr::LognormPpf(p, mu, sig) => format!("\\text{{LN}}^{{-1}}({};{},{})", p.to_latex(), mu.to_latex(), sig.to_latex()),
            Expr::UniformPpf(p, a, b) => format!("U^{{-1}}({};{},{})", p.to_latex(), a.to_latex(), b.to_latex()),
            Expr::CauchyPpf(p, x0, g) => format!("C^{{-1}}({};{},{})", p.to_latex(), x0.to_latex(), g.to_latex()),

            // 复数扩展
            Expr::ComplexSinh(z) => format!("\\sinh({})", z.to_latex()),
            Expr::ComplexCosh(z) => format!("\\cosh({})", z.to_latex()),
            Expr::ComplexTanh(z) => format!("\\tanh({})", z.to_latex()),
            Expr::ComplexAsinh(z) => format!("\\text{{asinh}}({})", z.to_latex()),
            Expr::ComplexAcosh(z) => format!("\\text{{acosh}}({})", z.to_latex()),
            Expr::ComplexAtanh(z) => format!("\\text{{atanh}}({})", z.to_latex()),
            Expr::ComplexAsin(z) => format!("\\arcsin({})", z.to_latex()),
            Expr::ComplexAcos(z) => format!("\\arccos({})", z.to_latex()),
            Expr::ComplexAtan(z) => format!("\\arctan({})", z.to_latex()),

            // 数论函数
            Expr::Gcd(a, b) => format!("\\gcd({},{})", a.to_latex(), b.to_latex()),
            Expr::Lcm(a, b) => format!("\\text{{lcm}}({},{})", a.to_latex(), b.to_latex()),
            Expr::Permutation(n, k) => format!("P({},{})", n.to_latex(), k.to_latex()),

            // 正交多项式
            Expr::Legendre(n, x) => format!("P_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::LegendreAssoc(l, m, x) => format!("P_{{{}}}^{{{}}}({})", l.to_latex(), m.to_latex(), x.to_latex()),
            Expr::Hermite(n, x) => format!("H_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::Laguerre(n, x) => format!("L_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::LaguerreAssoc(n, a, x) => format!("L_{{{}}}^{{{}}}({})", n.to_latex(), a.to_latex(), x.to_latex()),
            Expr::ChebyshevT(n, x) => format!("T_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::ChebyshevU(n, x) => format!("U_{{{}}}({})", n.to_latex(), x.to_latex()),

            // 椭圆积分
            Expr::EllipK(k) => format!("K({})", k.to_latex()),
            Expr::EllipE(k) => format!("E({})", k.to_latex()),

            // Lambda 和微积分
            Expr::Lambda { var, body } => format!("\\lambda {}.{}", var, body.to_latex()),
            Expr::Integrate { var, lower, upper, body } => format!(
                "\\int_{{{}}}^{{{}}} {} \\, d{}",
                lower.to_latex(), upper.to_latex(), body.to_latex(), var
            ),
            Expr::Derivative { var, body, at } => format!(
                "\\left.\\frac{{d}}{{d{}}}{}\\right|_{{{}={}}}",
                var, body.to_latex(), var, at.to_latex()
            ),
            Expr::Limit { var, to, body } => format!(
                "\\lim_{{{}\\to {}}} {}",
                var, to.to_latex(), body.to_latex()
            ),

            // 向量运算
            Expr::VectorLit(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| e.to_latex()).collect();
                format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", elems.join(" \\\\ "))
            }
            Expr::Dot(a, b) => format!("{} \\cdot {}", a.to_latex(), b.to_latex()),
            Expr::Cross(a, b) => format!("{} \\times {}", a.to_latex(), b.to_latex()),
            Expr::VecNorm(v) => format!("\\|{}\\|", v.to_latex()),
            Expr::VecNormalize(v) => format!("\\hat{{{}}}", v.to_latex()),

            // 矩阵运算
            Expr::MatrixLit(rows) => {
                let row_strs: Vec<_> = rows.iter().map(|row| {
                    let elems: Vec<_> = row.iter().map(|e| e.to_latex()).collect();
                    elems.join(" & ")
                }).collect();
                format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", row_strs.join(" \\\\ "))
            }
            Expr::MatMul(a, b) => format!("{} \\cdot {}", a.to_latex(), b.to_latex()),
            Expr::Transpose(a) => format!("{}^T", a.to_latex()),
            Expr::Det(a) => format!("\\det({})", a.to_latex()),
            Expr::Inv(a) => format!("{}^{{-1}}", a.to_latex()),
            Expr::Eigenvalues(a) => format!("\\lambda({})", a.to_latex()),
            Expr::Trace(a) => format!("\\text{{tr}}({})", a.to_latex()),
            Expr::MatNorm(a) => format!("\\|{}\\|", a.to_latex()),

            // 特殊函数
            Expr::Gamma(x) => format!("\\Gamma({})", x.to_latex()),
            Expr::Lgamma(x) => format!("\\ln\\Gamma({})", x.to_latex()),
            Expr::Digamma(x) => format!("\\psi({})", x.to_latex()),
            Expr::Beta(a, b) => format!("B({}, {})", a.to_latex(), b.to_latex()),
            Expr::Lbeta(a, b) => format!("\\ln B({}, {})", a.to_latex(), b.to_latex()),
            Expr::Erf(x) => format!("\\text{{erf}}({})", x.to_latex()),
            Expr::Erfc(x) => format!("\\text{{erfc}}({})", x.to_latex()),
            Expr::Erfinv(x) => format!("\\text{{erf}}^{{-1}}({})", x.to_latex()),
            Expr::Factorial(n) => format!("{}!", n.to_latex()),
            Expr::Combination(n, k) => format!("\\binom{{{}}}{{{}}}", n.to_latex(), k.to_latex()),
            Expr::Zeta(s) => format!("\\zeta({})", s.to_latex()),

            // 贝塞尔函数
            Expr::BesselJ0(x) => format!("J_0({})", x.to_latex()),
            Expr::BesselJ1(x) => format!("J_1({})", x.to_latex()),
            Expr::BesselJn(n, x) => format!("J_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselY0(x) => format!("Y_0({})", x.to_latex()),
            Expr::BesselY1(x) => format!("Y_1({})", x.to_latex()),
            Expr::BesselYn(n, x) => format!("Y_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselI0(x) => format!("I_0({})", x.to_latex()),
            Expr::BesselI1(x) => format!("I_1({})", x.to_latex()),
            Expr::BesselIn(n, x) => format!("I_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselK0(x) => format!("K_0({})", x.to_latex()),
            Expr::BesselK1(x) => format!("K_1({})", x.to_latex()),
            Expr::BesselKn(n, x) => format!("K_{{{}}}({})", n.to_latex(), x.to_latex()),

            // 概率分布
            Expr::NormPdf(x, mu, sig) => format!("\\phi({};{},{})", x.to_latex(), mu.to_latex(), sig.to_latex()),
            Expr::NormCdf(x, mu, sig) => format!("\\Phi({};{},{})", x.to_latex(), mu.to_latex(), sig.to_latex()),
            Expr::NormPpf(p, mu, sig) => format!("\\Phi^{{-1}}({};{},{})", p.to_latex(), mu.to_latex(), sig.to_latex()),
            Expr::TPdf(x, df) => format!("t_{{{}}}({})", df.to_latex(), x.to_latex()),
            Expr::TCdf(x, df) => format!("T_{{{}}}({})", df.to_latex(), x.to_latex()),
            Expr::TPpf(p, df) => format!("T^{{-1}}_{{{}}}({})", df.to_latex(), p.to_latex()),
            Expr::Chi2Pdf(x, df) => format!("\\chi^2_{{{}}}({})", df.to_latex(), x.to_latex()),
            Expr::Chi2Cdf(x, df) => format!("\\chi^2_{{{},\\text{{cdf}}}}({})", df.to_latex(), x.to_latex()),
            Expr::Chi2Ppf(p, df) => format!("\\chi^{{2,-1}}_{{{}}}({})", df.to_latex(), p.to_latex()),
            Expr::FPdf(x, d1, d2) => format!("F_{{{},{}}}({})", d1.to_latex(), d2.to_latex(), x.to_latex()),
            Expr::FCdf(x, d1, d2) => format!("F_{{{},{},\\text{{cdf}}}}({})", d1.to_latex(), d2.to_latex(), x.to_latex()),
            Expr::FPpf(p, d1, d2) => format!("F^{{-1}}_{{{},{}}}({})", d1.to_latex(), d2.to_latex(), p.to_latex()),
            Expr::PoissonPmf(k, lam) => format!("\\text{{Poisson}}({};{})", k.to_latex(), lam.to_latex()),
            Expr::PoissonCdf(k, lam) => format!("\\text{{Poisson}}_{{\\text{{cdf}}}}({};{})", k.to_latex(), lam.to_latex()),
            Expr::BinomialPmf(k, n, p) => format!("\\text{{Bin}}({};{},{})", k.to_latex(), n.to_latex(), p.to_latex()),
            Expr::BinomialCdf(k, n, p) => format!("\\text{{Bin}}_{{\\text{{cdf}}}}({};{},{})", k.to_latex(), n.to_latex(), p.to_latex()),
            Expr::ExponentialPdf(x, lam) => format!("\\text{{Exp}}({};{})", x.to_latex(), lam.to_latex()),
            Expr::ExponentialCdf(x, lam) => format!("\\text{{Exp}}_{{\\text{{cdf}}}}({};{})", x.to_latex(), lam.to_latex()),

            // 复数运算
            Expr::Complex(re, im) => format!("{} + {}i", re.to_latex(), im.to_latex()),
            Expr::Real(z) => format!("\\Re({})", z.to_latex()),
            Expr::Imag(z) => format!("\\Im({})", z.to_latex()),
            Expr::Conj(z) => format!("\\overline{{{}}}", z.to_latex()),
            Expr::Carg(z) => format!("\\arg({})", z.to_latex()),
            Expr::Cabs(z) => format!("|{}|", z.to_latex()),
            Expr::Polar(r, theta) => format!("{}e^{{i{}}}", r.to_latex(), theta.to_latex()),

            // 基础数学补充
            Expr::Hypot(x, y) => format!("\\sqrt{{{}^2 + {}^2}}", x.to_latex(), y.to_latex()),
            Expr::Hypot3(x, y, z) => format!("\\sqrt{{{}^2 + {}^2 + {}^2}}", x.to_latex(), y.to_latex(), z.to_latex()),
            Expr::Clamp(x, min, max) => format!("\\text{{clamp}}({}, {}, {})", x.to_latex(), min.to_latex(), max.to_latex()),
            Expr::Copysign(x, y) => format!("\\text{{copysign}}({}, {})", x.to_latex(), y.to_latex()),
            Expr::Fma(a, b, c) => format!("{} \\cdot {} + {}", a.to_latex(), b.to_latex(), c.to_latex()),
            Expr::Logn(base, x) => format!("\\log_{{{}}}({})", base.to_latex(), x.to_latex()),
            Expr::Sinc(x) => format!("\\text{{sinc}}({})", x.to_latex()),

            // 高精度数值函数
            Expr::Expm1(x) => format!("e^{{{}}} - 1", x.to_latex()),
            Expr::Log1p(x) => format!("\\ln(1 + {})", x.to_latex()),
            Expr::Exp2(x) => format!("2^{{{}}}", x.to_latex()),

            // 不完全伽马/贝塔函数
            Expr::Gammainc(a, x) => format!("\\gamma({}, {})", a.to_latex(), x.to_latex()),
            Expr::Gammaincc(a, x) => format!("\\Gamma({}, {})", a.to_latex(), x.to_latex()),
            Expr::Betainc(x, a, b) => format!("I_{{{}}}({}, {})", x.to_latex(), a.to_latex(), b.to_latex()),

            // 扩展三角函数
            Expr::Sec(x) => format!("\\sec({})", x.to_latex()),
            Expr::Csc(x) => format!("\\csc({})", x.to_latex()),
            Expr::Cot(x) => format!("\\cot({})", x.to_latex()),
            Expr::Asec(x) => format!("\\text{{arcsec}}({})", x.to_latex()),
            Expr::Acsc(x) => format!("\\text{{arccsc}}({})", x.to_latex()),
            Expr::Acot(x) => format!("\\text{{arccot}}({})", x.to_latex()),

            // 扩展双曲函数
            Expr::Sech(x) => format!("\\text{{sech}}({})", x.to_latex()),
            Expr::Csch(x) => format!("\\text{{csch}}({})", x.to_latex()),
            Expr::Coth(x) => format!("\\coth({})", x.to_latex()),
            Expr::Asech(x) => format!("\\text{{arsech}}({})", x.to_latex()),
            Expr::Acsch(x) => format!("\\text{{arcsch}}({})", x.to_latex()),
            Expr::Acoth(x) => format!("\\text{{arcoth}}({})", x.to_latex()),

            // Airy 函数
            Expr::AiryAi(x) => format!("\\text{{Ai}}({})", x.to_latex()),
            Expr::AiryBi(x) => format!("\\text{{Bi}}({})", x.to_latex()),

            // 球谐函数
            Expr::SphericalHarmonic(l, m, theta, phi) => format!("Y_{{{}}}^{{{}}}({}, {})", l.to_latex(), m.to_latex(), theta.to_latex(), phi.to_latex()),

            // Fresnel 积分
            Expr::FresnelS(x) => format!("S({})", x.to_latex()),
            Expr::FresnelC(x) => format!("C({})", x.to_latex()),

            // 其他特殊函数
            Expr::Dawson(x) => format!("D({})", x.to_latex()),
            Expr::ExpInt(x) => format!("\\text{{Ei}}({})", x.to_latex()),
            Expr::LogInt(x) => format!("\\text{{li}}({})", x.to_latex()),
            Expr::SinInt(x) => format!("\\text{{Si}}({})", x.to_latex()),
            Expr::CosInt(x) => format!("\\text{{Ci}}({})", x.to_latex()),

            // Lambert W
            Expr::LambertW(x) => format!("W_0({})", x.to_latex()),
            Expr::LambertWm1(x) => format!("W_{{-1}}({})", x.to_latex()),

            // 球贝塞尔函数
            Expr::SphBesselJ(n, x) => format!("j_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::SphBesselY(n, x) => format!("y_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::SphBesselI(n, x) => format!("i_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::SphBesselK(n, x) => format!("k_{{{}}}({})", n.to_latex(), x.to_latex()),

            // 超几何函数
            Expr::Hyp0f1(b, x) => format!("{{}}_{0}F_1(;{};{})", b.to_latex(), x.to_latex()),
            Expr::Hyp1f1(a, b, x) => format!("{{}}_{1}F_1({};{};{})", a.to_latex(), b.to_latex(), x.to_latex()),
            Expr::Hyp2f1(a, b, c, x) => format!("{{}}_{2}F_1({},{};{};{})", a.to_latex(), b.to_latex(), c.to_latex(), x.to_latex()),

            // Kelvin 函数
            Expr::KelvinBer(x) => format!("\\text{{ber}}({})", x.to_latex()),
            Expr::KelvinBei(x) => format!("\\text{{bei}}({})", x.to_latex()),
            Expr::KelvinKer(x) => format!("\\text{{ker}}({})", x.to_latex()),
            Expr::KelvinKei(x) => format!("\\text{{kei}}({})", x.to_latex()),

            // 不完全椭圆积分
            Expr::EllipF(phi, k) => format!("F({}, {})", phi.to_latex(), k.to_latex()),
            Expr::EllipEInc(phi, k) => format!("E({}, {})", phi.to_latex(), k.to_latex()),
            Expr::EllipPi(phi, n, k) => format!("\\Pi({}, {}, {})", phi.to_latex(), n.to_latex(), k.to_latex()),

            // 其他特殊函数
            Expr::Spence(x) => format!("\\text{{Li}}_2({})", x.to_latex()),
            Expr::Polygamma(n, x) => format!("\\psi^{{({})}}({})", n.to_latex(), x.to_latex()),
            Expr::Hankel1(n, x) => format!("H^{{(1)}}_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::Hankel2(n, x) => format!("H^{{(2)}}_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::StruveH(v, x) => format!("\\mathbf{{H}}_{{{}}}({})", v.to_latex(), x.to_latex()),
            Expr::StruveL(v, x) => format!("\\mathbf{{L}}_{{{}}}({})", v.to_latex(), x.to_latex()),
            Expr::OwensT(h, a) => format!("T({}, {})", h.to_latex(), a.to_latex()),
            Expr::RiemannSiegelZ(t) => format!("Z({})", t.to_latex()),
            Expr::RiemannSiegelTheta(t) => format!("\\theta({})", t.to_latex()),

            // Jacobi 椭圆函数
            Expr::JacobiSn(u, m) => format!("\\text{{sn}}({}, {})", u.to_latex(), m.to_latex()),
            Expr::JacobiCn(u, m) => format!("\\text{{cn}}({}, {})", u.to_latex(), m.to_latex()),
            Expr::JacobiDn(u, m) => format!("\\text{{dn}}({}, {})", u.to_latex(), m.to_latex()),

            // 广义正交多项式
            Expr::Gegenbauer(n, alpha, x) => format!("C_{{{}}}^{{{}}}({})", n.to_latex(), alpha.to_latex(), x.to_latex()),
            Expr::JacobiP(n, alpha, beta, x) => format!("P_{{{}}}^{{({},{})}}({})", n.to_latex(), alpha.to_latex(), beta.to_latex(), x.to_latex()),

            // Mathieu 函数
            Expr::MathieuA(n, q) => format!("a_{{{}}}({})", n.to_latex(), q.to_latex()),
            Expr::MathieuB(n, q) => format!("b_{{{}}}({})", n.to_latex(), q.to_latex()),
            Expr::MathieuCe(n, q, x) => format!("\\text{{ce}}_{{{}}}({}, {})", n.to_latex(), q.to_latex(), x.to_latex()),
            Expr::MathieuSe(n, q, x) => format!("\\text{{se}}_{{{}}}({}, {})", n.to_latex(), q.to_latex(), x.to_latex()),

            // Coulomb 波函数
            Expr::CoulombF(l, eta, rho) => format!("F_{{{}}}({}, {})", l.to_latex(), eta.to_latex(), rho.to_latex()),
            Expr::CoulombG(l, eta, rho) => format!("G_{{{}}}({}, {})", l.to_latex(), eta.to_latex(), rho.to_latex()),

            // Wigner 符号
            Expr::Wigner3j(j1, j2, j3, m1, m2, m3) => format!("\\begin{{pmatrix}} {} & {} & {} \\\\ {} & {} & {} \\end{{pmatrix}}", j1.to_latex(), j2.to_latex(), j3.to_latex(), m1.to_latex(), m2.to_latex(), m3.to_latex()),
            Expr::Wigner6j(j1, j2, j3, j4, j5, j6) => format!("\\begin{{Bmatrix}} {} & {} & {} \\\\ {} & {} & {} \\end{{Bmatrix}}", j1.to_latex(), j2.to_latex(), j3.to_latex(), j4.to_latex(), j5.to_latex(), j6.to_latex()),
            Expr::Wigner9j(j1, j2, j3, j4, j5, j6, j7, j8, j9) => format!("\\begin{{Bmatrix}} {} & {} & {} \\\\ {} & {} & {} \\\\ {} & {} & {} \\end{{Bmatrix}}", j1.to_latex(), j2.to_latex(), j3.to_latex(), j4.to_latex(), j5.to_latex(), j6.to_latex(), j7.to_latex(), j8.to_latex(), j9.to_latex()),

            // Theta 函数
            Expr::Theta1(z, q) => format!("\\theta_1({}, {})", z.to_latex(), q.to_latex()),
            Expr::Theta2(z, q) => format!("\\theta_2({}, {})", z.to_latex(), q.to_latex()),
            Expr::Theta3(z, q) => format!("\\theta_3({}, {})", z.to_latex(), q.to_latex()),
            Expr::Theta4(z, q) => format!("\\theta_4({}, {})", z.to_latex(), q.to_latex()),

            // 抛物柱面函数
            Expr::Pbdv(v, x) => format!("D_{{{}}}({})", v.to_latex(), x.to_latex()),
            Expr::Pbvv(v, x) => format!("V_{{{}}}({})", v.to_latex(), x.to_latex()),
            Expr::Pbwa(a, x) => format!("W({}, {})", a.to_latex(), x.to_latex()),

            // 球扁旋转体波函数
            Expr::ProAng1(m, n, c, x) => format!("S_{{{}{}}}^{{(1)}}({}, {})", m.to_latex(), n.to_latex(), c.to_latex(), x.to_latex()),
            Expr::ProRad1(m, n, c, x) => format!("R_{{{}{}}}^{{(1)}}({}, {})", m.to_latex(), n.to_latex(), c.to_latex(), x.to_latex()),
            Expr::ProRad2(m, n, c, x) => format!("R_{{{}{}}}^{{(2)}}({}, {})", m.to_latex(), n.to_latex(), c.to_latex(), x.to_latex()),
            Expr::OblAng1(m, n, c, x) => format!("S_{{{}{}}}^{{(1)}}({}, {})", m.to_latex(), n.to_latex(), c.to_latex(), x.to_latex()),
            Expr::OblRad1(m, n, c, x) => format!("R_{{{}{}}}^{{(1)}}({}, {})", m.to_latex(), n.to_latex(), c.to_latex(), x.to_latex()),
            Expr::OblRad2(m, n, c, x) => format!("R_{{{}{}}}^{{(2)}}({}, {})", m.to_latex(), n.to_latex(), c.to_latex(), x.to_latex()),

            // 修改 Fresnel 积分
            Expr::ModFresnelP(x) => format!("F_+({})", x.to_latex()),
            Expr::ModFresnelM(x) => format!("F_-({})", x.to_latex()),

            // Wright 函数
            Expr::WrightBessel(rho, beta, z) => format!("J_{{({}, {})}}({})", rho.to_latex(), beta.to_latex(), z.to_latex()),
            Expr::WrightOmega(z) => format!("\\omega({})", z.to_latex()),

            // Voigt
            Expr::Voigt(x, sigma, gamma) => format!("V({}, {}, {})", x.to_latex(), sigma.to_latex(), gamma.to_latex()),

            // Sigmoid/Logistic
            Expr::Logit(x) => format!("\\text{{logit}}({})", x.to_latex()),
            Expr::Expit(x) => format!("\\sigma({})", x.to_latex()),

            // Box-Cox
            Expr::BoxCox(x, lmbda) => format!("\\text{{boxcox}}({}, {})", x.to_latex(), lmbda.to_latex()),
            Expr::BoxCox1p(x, lmbda) => format!("\\text{{boxcox1p}}({}, {})", x.to_latex(), lmbda.to_latex()),
            Expr::InvBoxCox(y, lmbda) => format!("\\text{{boxcox}}^{{-1}}({}, {})", y.to_latex(), lmbda.to_latex()),
            Expr::InvBoxCox1p(y, lmbda) => format!("\\text{{boxcox1p}}^{{-1}}({}, {})", y.to_latex(), lmbda.to_latex()),

            // 信息论
            Expr::Entr(x) => format!("-{} \\ln({})", x.to_latex(), x.to_latex()),
            Expr::RelEntr(x, y) => format!("{} \\ln\\left(\\frac{{{}}}{{{}}}\\right)", x.to_latex(), x.to_latex(), y.to_latex()),
            Expr::KlDiv(x, y) => format!("D_{{KL}}({} \\| {})", x.to_latex(), y.to_latex()),

            // 阶乘扩展
            Expr::Factorial2(n) => format!("{}!!", n.to_latex()),
            Expr::Factorialk(n, k) => format!("{}!^{{({})}}", n.to_latex(), k.to_latex()),
            Expr::Stirling2(n, k) => format!("S({}, {})", n.to_latex(), k.to_latex()),
            Expr::Poch(z, m) => format!("({})_{{{}}}", z.to_latex(), m.to_latex()),

            // Carlson 椭圆积分
            Expr::EllipRc(x, y) => format!("R_C({}, {})", x.to_latex(), y.to_latex()),
            Expr::EllipRd(x, y, z) => format!("R_D({}, {}, {})", x.to_latex(), y.to_latex(), z.to_latex()),
            Expr::EllipRf(x, y, z) => format!("R_F({}, {}, {})", x.to_latex(), y.to_latex(), z.to_latex()),
            Expr::EllipRg(x, y, z) => format!("R_G({}, {}, {})", x.to_latex(), y.to_latex(), z.to_latex()),
            Expr::EllipRj(x, y, z, p) => format!("R_J({}, {}, {}, {})", x.to_latex(), y.to_latex(), z.to_latex(), p.to_latex()),

            // 扩展误差函数
            Expr::Erfcx(x) => format!("\\text{{erfcx}}({})", x.to_latex()),
            Expr::Erfi(x) => format!("\\text{{erfi}}({})", x.to_latex()),
            Expr::Erfcinv(x) => format!("\\text{{erfc}}^{{-1}}({})", x.to_latex()),

            // 扩展 Gamma
            Expr::Hyperu(a, b, x) => format!("U({}, {}, {})", a.to_latex(), b.to_latex(), x.to_latex()),
            Expr::Rgamma(x) => format!("\\frac{{1}}{{\\Gamma({})}}", x.to_latex()),
            Expr::Gammasgn(x) => format!("\\text{{sgn}}(\\Gamma({}))", x.to_latex()),

            // 便利函数
            Expr::Agm(a, b) => format!("\\text{{agm}}({}, {})", a.to_latex(), b.to_latex()),
            Expr::Exprel(x) => format!("\\frac{{e^{{{}}} - 1}}{{{}}}", x.to_latex(), x.to_latex()),
            Expr::Xlogy(x, y) => format!("{} \\ln({})", x.to_latex(), y.to_latex()),
            Expr::Xlog1py(x, y) => format!("{} \\ln(1 + {})", x.to_latex(), y.to_latex()),

            // Zeta 扩展
            Expr::HurwitzZeta(s, q) => format!("\\zeta({}, {})", s.to_latex(), q.to_latex()),
            Expr::Zetac(x) => format!("\\zeta({}) - 1", x.to_latex()),
            Expr::Polylog(s, z) => format!("\\text{{Li}}_{{{}}}({})", s.to_latex(), z.to_latex()),

            // === 缩放贝塞尔函数 ===
            Expr::BesselI0e(x) => format!("I_0^{{(e)}}({})", x.to_latex()),
            Expr::BesselI1e(x) => format!("I_1^{{(e)}}({})", x.to_latex()),
            Expr::BesselIne(n, x) => format!("I_{{{}}}^{{(e)}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselK0e(x) => format!("K_0^{{(e)}}({})", x.to_latex()),
            Expr::BesselK1e(x) => format!("K_1^{{(e)}}({})", x.to_latex()),
            Expr::BesselKne(n, x) => format!("K_{{{}}}^{{(e)}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselJne(n, x) => format!("J_{{{}}}^{{(e)}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselYne(n, x) => format!("Y_{{{}}}^{{(e)}}({})", n.to_latex(), x.to_latex()),
            Expr::Hankel1e(n, x) => format!("H_{{{}}}^{{(1,e)}}({})", n.to_latex(), x.to_latex()),
            Expr::Hankel2e(n, x) => format!("H_{{{}}}^{{(2,e)}}({})", n.to_latex(), x.to_latex()),

            // === 贝塞尔函数导数 ===
            Expr::BesselJnp(n, x) => format!("J'_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselYnp(n, x) => format!("Y'_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselInp(n, x) => format!("I'_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::BesselKnp(n, x) => format!("K'_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::Hankel1p(n, x) => format!("{{H_{{{}}}^{{(1)}}}}^\\prime({})", n.to_latex(), x.to_latex()),
            Expr::Hankel2p(n, x) => format!("{{H_{{{}}}^{{(2)}}}}^\\prime({})", n.to_latex(), x.to_latex()),

            // === Huber 损失 ===
            Expr::Huber(delta, r) => format!("\\text{{Huber}}_{{{}}}({})", delta.to_latex(), r.to_latex()),
            Expr::PseudoHuber(delta, r) => format!("\\text{{PseudoHuber}}_{{{}}}({})", delta.to_latex(), r.to_latex()),

            // === Kolmogorov-Smirnov ===
            Expr::Kolmogorov(y) => format!("K({})", y.to_latex()),
            Expr::Kolmogi(p) => format!("K^{{-1}}({})", p.to_latex()),
            Expr::Smirnov(n, d) => format!("P_{{{}}}(D_n > {})", n.to_latex(), d.to_latex()),
            Expr::Smirnovi(n, p) => format!("P_{{{}}}^{{-1}}({})", n.to_latex(), p.to_latex()),

            // === Faddeeva ===
            Expr::Wofz(z) => format!("w({})", z.to_latex()),

            // === Dirichlet 核 ===
            Expr::Diric(x, n) => format!("D_{{{}}}({})", n.to_latex(), x.to_latex()),

            // === Tukey lambda ===
            Expr::Tklmbda(x, lam) => format!("T_{{{}}}({})", lam.to_latex(), x.to_latex()),

            // === Gamma/Beta 逆函数 ===
            Expr::Gammaincinv(a, y) => format!("P^{{-1}}({}, {})", a.to_latex(), y.to_latex()),
            Expr::Gammainccinv(a, y) => format!("Q^{{-1}}({}, {})", a.to_latex(), y.to_latex()),
            Expr::Betaincinv(a, b, y) => format!("I^{{-1}}_{{{}}}({}, {})", y.to_latex(), a.to_latex(), b.to_latex()),

            // === 高精度便利函数 ===
            Expr::Cosm1(x) => format!("\\cos({}) - 1", x.to_latex()),
            Expr::Powm1(x, y) => format!("{}^{{{}}} - 1", x.to_latex(), y.to_latex()),
            Expr::Exp10(x) => format!("10^{{{}}}", x.to_latex()),
            Expr::Log1pmx(x) => format!("\\ln(1 + {}) - {}", x.to_latex(), x.to_latex()),
            Expr::Loggamma(z) => format!("\\ln\\Gamma({})", z.to_latex()),

            // === 度数三角函数 ===
            Expr::Cosdg(x) => format!("\\cos({}^\\circ)", x.to_latex()),
            Expr::Sindg(x) => format!("\\sin({}^\\circ)", x.to_latex()),
            Expr::Tandg(x) => format!("\\tan({}^\\circ)", x.to_latex()),
            Expr::Cotdg(x) => format!("\\cot({}^\\circ)", x.to_latex()),
            Expr::Radian(d, m, s) => format!("\\text{{rad}}({}^\\circ {}'{}'')", d.to_latex(), m.to_latex(), s.to_latex()),

            // === Airy 扩展 ===
            Expr::AiryAie(x) => format!("\\text{{Ai}}_e({})", x.to_latex()),
            Expr::AiryBie(x) => format!("\\text{{Bi}}_e({})", x.to_latex()),
            Expr::AiryAip(x) => format!("\\text{{Ai}}'({})", x.to_latex()),
            Expr::AiryBip(x) => format!("\\text{{Bi}}'({})", x.to_latex()),
            Expr::ItAiry(x) => format!("\\int \\text{{Ai}}({})", x.to_latex()),

            // === 指数积分扩展 ===
            Expr::Expn(n, x) => format!("E_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::Exp1(x) => format!("E_1({})", x.to_latex()),
            Expr::Shi(x) => format!("\\text{{Shi}}({})", x.to_latex()),
            Expr::Chi(x) => format!("\\text{{Chi}}({})", x.to_latex()),

            // === Struve 积分 ===
            Expr::ItStruve0(x) => format!("\\int H_0({})", x.to_latex()),
            Expr::It2Struve0(x) => format!("\\iint H_0({})", x.to_latex()),
            Expr::ItModStruve0(x) => format!("\\int L_0({})", x.to_latex()),

            // === ML/统计扩展 ===
            Expr::LogExpit(x) => format!("\\log\\sigma({})", x.to_latex()),
            Expr::Softplus(x) => format!("\\text{{softplus}}({})", x.to_latex()),
            Expr::LogNdtr(x) => format!("\\log\\Phi({})", x.to_latex()),

            // === Beta 补函数 ===
            Expr::Betaincc(a, b, x) => format!("1 - I_{{{}}}({}, {})", x.to_latex(), a.to_latex(), b.to_latex()),
            Expr::Betainccinv(a, b, y) => format!("(1-I)^{{-1}}_{{{}}}({}, {})", y.to_latex(), a.to_latex(), b.to_latex()),

            // === 数论函数 ===
            Expr::Bernoulli(n) => format!("B_{{{}}}", n.to_latex()),
            Expr::Euler(n) => format!("E_{{{}}}", n.to_latex()),

            // === 椭圆扩展 ===
            Expr::EllipKm1(p) => format!("K(1-{})", p.to_latex()),

            // === Kelvin 导数 ===
            Expr::KelvinBerp(x) => format!("\\text{{ber}}'({})", x.to_latex()),
            Expr::KelvinBeip(x) => format!("\\text{{bei}}'({})", x.to_latex()),
            Expr::KelvinKerp(x) => format!("\\text{{ker}}'({})", x.to_latex()),
            Expr::KelvinKeip(x) => format!("\\text{{kei}}'({})", x.to_latex()),

            // === 贝塞尔积分 ===
            Expr::BesselPoly(a, lmb, nu) => format!("\\text{{besselpoly}}({}, {}, {})", a.to_latex(), lmb.to_latex(), nu.to_latex()),

            // === Wright Bessel 扩展 ===
            Expr::LogWrightBessel(a, b, x) => format!("\\log J_{{({}, {})}}({})", a.to_latex(), b.to_latex(), x.to_latex()),

            // === 二项系数扩展 ===
            Expr::Binom(x, y) => format!("\\binom{{{}}}{{{}}}", x.to_latex(), y.to_latex()),

            // === 分布函数 ===
            Expr::Bdtr(k, n, p) => format!("B_{{{}; {}, {}}}(k \\le {})", k.to_latex(), n.to_latex(), p.to_latex(), k.to_latex()),
            Expr::Bdtrc(k, n, p) => format!("1 - B_{{{}; {}, {}}}", k.to_latex(), n.to_latex(), p.to_latex()),
            Expr::Bdtri(k, n, y) => format!("B^{{-1}}_{{{}; {}}}({})", k.to_latex(), n.to_latex(), y.to_latex()),
            Expr::Chdtr(v, x) => format!("\\chi^2_{{{}}}({})", v.to_latex(), x.to_latex()),
            Expr::Chdtrc(v, x) => format!("1 - \\chi^2_{{{}}}({})", v.to_latex(), x.to_latex()),
            Expr::Chdtri(v, p) => format!("(\\chi^2_{{{}}})^{{-1}}({})", v.to_latex(), p.to_latex()),
            Expr::Fdtr(dfn, dfd, x) => format!("F_{{{}, {}}}({})", dfn.to_latex(), dfd.to_latex(), x.to_latex()),
            Expr::Fdtrc(dfn, dfd, x) => format!("1 - F_{{{}, {}}}({})", dfn.to_latex(), dfd.to_latex(), x.to_latex()),
            Expr::Fdtri(dfn, dfd, p) => format!("F^{{-1}}_{{{}, {}}}({})", dfn.to_latex(), dfd.to_latex(), p.to_latex()),
            Expr::Stdtr(df, t) => format!("t_{{{}}}({})", df.to_latex(), t.to_latex()),
            Expr::Stdtrc(df, t) => format!("1 - t_{{{}}}({})", df.to_latex(), t.to_latex()),
            Expr::Stdtrit(df, p) => format!("t^{{-1}}_{{{}}}({})", df.to_latex(), p.to_latex()),
            Expr::Pdtr(k, m) => format!("P_{{{}}}({})", m.to_latex(), k.to_latex()),
            Expr::Pdtrc(k, m) => format!("1 - P_{{{}}}({})", m.to_latex(), k.to_latex()),
            Expr::Pdtri(k, y) => format!("P^{{-1}}_{{{}}}({})", k.to_latex(), y.to_latex()),
            Expr::Btdtr(a, b, x) => format!("I_{{{}}}({}, {})", x.to_latex(), a.to_latex(), b.to_latex()),
            Expr::Btdtrc(a, b, x) => format!("1 - I_{{{}}}({}, {})", x.to_latex(), a.to_latex(), b.to_latex()),
            Expr::Gdtr(a, b, x) => format!("\\Gamma_{{{}}}({}, {})", a.to_latex(), b.to_latex(), x.to_latex()),
            Expr::Gdtrc(a, b, x) => format!("1 - \\Gamma_{{{}}}({}, {})", a.to_latex(), b.to_latex(), x.to_latex()),

            // === 积分/ML 扩展 ===
            Expr::Sici(x) => format!("(\\text{{Si}}({}), \\text{{Ci}}({}))", x.to_latex(), x.to_latex()),
            Expr::Shichi(x) => format!("(\\text{{Shi}}({}), \\text{{Chi}}({}))", x.to_latex(), x.to_latex()),
            Expr::Softmax(x) => format!("\\text{{softmax}}({})", x.to_latex()),
            Expr::LogSoftmax(x) => format!("\\log\\text{{softmax}}({})", x.to_latex()),
            Expr::Logsumexp(x) => format!("\\log\\sum\\exp({})", x.to_latex()),

            // === GSL 扩展 ===
            Expr::AiryZeroAi(s) => format!("a_{{Ai,{}}}", s.to_latex()),
            Expr::AiryZeroBi(s) => format!("a_{{Bi,{}}}", s.to_latex()),
            Expr::BesselZeroJ0(s) => format!("j_{{0,{}}}", s.to_latex()),
            Expr::BesselZeroJ1(s) => format!("j_{{1,{}}}", s.to_latex()),
            Expr::BesselZeroJnu(nu, s) => format!("j_{{{},{}}}", nu.to_latex(), s.to_latex()),
            Expr::SphLegendre(l, m, x) => format!("P_{{{}}}^{{{}}}({})", l.to_latex(), m.to_latex(), x.to_latex()),
            Expr::Clausen(x) => format!("\\text{{Cl}}_2({})", x.to_latex()),
            Expr::Debye(n, x) => format!("D_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::Synchrotron1(x) => format!("F({})", x.to_latex()),
            Expr::Synchrotron2(x) => format!("G({})", x.to_latex()),
            Expr::Transport(n, x) => format!("J_{{{}}}({})", n.to_latex(), x.to_latex()),
            Expr::FermiDirac(j, x) => format!("F_{{{}}}({})", j.to_latex(), x.to_latex()),
        }
    }

    /// 将变量名转换为 LaTeX 格式
    fn name_to_latex(name: &str) -> String {
        // 处理下标：Pmax_l -> P_{max,l}
        if name.contains('_') {
            let parts: Vec<&str> = name.splitn(2, '_').collect();
            format!("{}_{{{}}}", parts[0], parts[1])
        } else {
            name.to_string()
        }
    }
}

// ============================================
// 变换方法
// ============================================

impl Expr {
    /// 深度克隆
    pub fn deep_clone(&self) -> Self {
        self.clone()
    }

    /// 替换变量
    pub fn substitute(&self, var: &str, replacement: &Expr) -> Self {
        match self {
            Expr::Var(name) if name == var => replacement.clone(),
            Expr::Param(name) if name == var => replacement.clone(),

            Expr::Const(_) | Expr::Var(_) | Expr::Param(_) | Expr::Pi | Expr::E => self.clone(),

            Expr::Reduce { kind, arg } => Expr::Reduce {
                kind: *kind,
                arg: Box::new(arg.substitute(var, replacement)),
            },

            // 一元运算
            Expr::Neg(a) => Expr::neg(a.substitute(var, replacement)),
            Expr::Abs(a) => Expr::abs(a.substitute(var, replacement)),
            Expr::Ceil(a) => Expr::ceil(a.substitute(var, replacement)),
            Expr::Floor(a) => Expr::floor(a.substitute(var, replacement)),
            Expr::Round(a) => Expr::round(a.substitute(var, replacement)),
            Expr::Trunc(a) => Expr::trunc(a.substitute(var, replacement)),
            Expr::Sign(a) => Expr::sign(a.substitute(var, replacement)),
            Expr::Exp(a) => Expr::exp(a.substitute(var, replacement)),
            Expr::Ln(a) => Expr::ln(a.substitute(var, replacement)),
            Expr::Log10(a) => Expr::log10(a.substitute(var, replacement)),
            Expr::Log2(a) => Expr::log2(a.substitute(var, replacement)),
            Expr::Sqrt(a) => Expr::sqrt(a.substitute(var, replacement)),
            Expr::Cbrt(a) => Expr::cbrt(a.substitute(var, replacement)),
            Expr::Sin(a) => Expr::sin(a.substitute(var, replacement)),
            Expr::Cos(a) => Expr::cos(a.substitute(var, replacement)),
            Expr::Tan(a) => Expr::tan(a.substitute(var, replacement)),
            Expr::ASin(a) => Expr::asin(a.substitute(var, replacement)),
            Expr::ACos(a) => Expr::acos(a.substitute(var, replacement)),
            Expr::ATan(a) => Expr::atan(a.substitute(var, replacement)),
            Expr::Sinh(a) => Expr::sinh(a.substitute(var, replacement)),
            Expr::Cosh(a) => Expr::cosh(a.substitute(var, replacement)),
            Expr::Tanh(a) => Expr::tanh(a.substitute(var, replacement)),
            Expr::ASinh(a) => Expr::asinh(a.substitute(var, replacement)),
            Expr::ACosh(a) => Expr::acosh(a.substitute(var, replacement)),
            Expr::ATanh(a) => Expr::atanh(a.substitute(var, replacement)),
            Expr::Not(a) => Expr::Not(Box::new(a.substitute(var, replacement))),

            // 二元运算
            Expr::Add(a, b) => Expr::add(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::Sub(a, b) => Expr::sub(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::Mul(a, b) => Expr::mul(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::Div(a, b) => Expr::div(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::Pow(a, b) => Expr::pow(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::Mod(a, b) => Expr::modulo(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::ATan2(a, b) => Expr::atan2(a.substitute(var, replacement), b.substitute(var, replacement)),
            Expr::Eq(a, b) => Expr::Eq(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Lt(a, b) => Expr::Lt(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Gt(a, b) => Expr::Gt(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Leq(a, b) => Expr::Leq(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Geq(a, b) => Expr::Geq(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Neq(a, b) => Expr::Neq(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::And(a, b) => Expr::And(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Or(a, b) => Expr::Or(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 多元运算
            Expr::Max(args) => Expr::max(args.iter().map(|a| a.substitute(var, replacement)).collect()),
            Expr::Min(args) => Expr::min(args.iter().map(|a| a.substitute(var, replacement)).collect()),

            // 求和/连乘
            Expr::Sum { index, lower, upper, body } => Expr::Sum {
                index: index.clone(),
                lower: Box::new(lower.substitute(var, replacement)),
                upper: Box::new(upper.substitute(var, replacement)),
                body: Box::new(body.substitute(var, replacement)),
            },
            Expr::Product { index, lower, upper, body } => Expr::Product {
                index: index.clone(),
                lower: Box::new(lower.substitute(var, replacement)),
                upper: Box::new(upper.substitute(var, replacement)),
                body: Box::new(body.substitute(var, replacement)),
            },

            // 条件表达式
            Expr::IfThenElse { cond, then_branch, else_branch } => Expr::if_then_else(
                cond.substitute(var, replacement),
                then_branch.substitute(var, replacement),
                else_branch.substitute(var, replacement),
            ),

            Expr::Piecewise { pieces, otherwise } => Expr::Piecewise {
                pieces: pieces
                    .iter()
                    .map(|(c, v)| (c.substitute(var, replacement), v.substitute(var, replacement)))
                    .collect(),
                otherwise: Box::new(otherwise.substitute(var, replacement)),
            },

            // 扩展一元运算
            Expr::ComplexSinh(a) => Expr::ComplexSinh(Box::new(a.substitute(var, replacement))),
            Expr::ComplexCosh(a) => Expr::ComplexCosh(Box::new(a.substitute(var, replacement))),
            Expr::ComplexTanh(a) => Expr::ComplexTanh(Box::new(a.substitute(var, replacement))),
            Expr::ComplexAsinh(a) => Expr::ComplexAsinh(Box::new(a.substitute(var, replacement))),
            Expr::ComplexAcosh(a) => Expr::ComplexAcosh(Box::new(a.substitute(var, replacement))),
            Expr::ComplexAtanh(a) => Expr::ComplexAtanh(Box::new(a.substitute(var, replacement))),
            Expr::ComplexAsin(a) => Expr::ComplexAsin(Box::new(a.substitute(var, replacement))),
            Expr::ComplexAcos(a) => Expr::ComplexAcos(Box::new(a.substitute(var, replacement))),
            Expr::ComplexAtan(a) => Expr::ComplexAtan(Box::new(a.substitute(var, replacement))),
            Expr::EllipK(a) => Expr::EllipK(Box::new(a.substitute(var, replacement))),
            Expr::EllipE(a) => Expr::EllipE(Box::new(a.substitute(var, replacement))),
            Expr::VecNorm(a) => Expr::VecNorm(Box::new(a.substitute(var, replacement))),
            Expr::VecNormalize(a) => Expr::VecNormalize(Box::new(a.substitute(var, replacement))),
            Expr::Transpose(a) => Expr::Transpose(Box::new(a.substitute(var, replacement))),
            Expr::Det(a) => Expr::Det(Box::new(a.substitute(var, replacement))),
            Expr::Inv(a) => Expr::Inv(Box::new(a.substitute(var, replacement))),
            Expr::Eigenvalues(a) => Expr::Eigenvalues(Box::new(a.substitute(var, replacement))),
            Expr::Trace(a) => Expr::Trace(Box::new(a.substitute(var, replacement))),
            Expr::MatNorm(a) => Expr::MatNorm(Box::new(a.substitute(var, replacement))),

            // 扩展二元运算
            Expr::ExpPpf(a, b) => Expr::ExpPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Gcd(a, b) => Expr::Gcd(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Lcm(a, b) => Expr::Lcm(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Permutation(a, b) => Expr::Permutation(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Legendre(a, b) => Expr::Legendre(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Hermite(a, b) => Expr::Hermite(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Laguerre(a, b) => Expr::Laguerre(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::ChebyshevT(a, b) => Expr::ChebyshevT(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::ChebyshevU(a, b) => Expr::ChebyshevU(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Dot(a, b) => Expr::Dot(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::Cross(a, b) => Expr::Cross(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
            Expr::MatMul(a, b) => Expr::MatMul(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),

            // 三元运算
            Expr::GammaPpf(a, b, c) => Expr::GammaPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::BetaPpf(a, b, c) => Expr::BetaPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::WeibullPpf(a, b, c) => Expr::WeibullPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::LognormPpf(a, b, c) => Expr::LognormPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::UniformPpf(a, b, c) => Expr::UniformPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::CauchyPpf(a, b, c) => Expr::CauchyPpf(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::LegendreAssoc(a, b, c) => Expr::LegendreAssoc(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),
            Expr::LaguerreAssoc(a, b, c) => Expr::LaguerreAssoc(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
                Box::new(c.substitute(var, replacement)),
            ),

            // Lambda 和微积分
            Expr::Lambda { var: v, body } => {
                if v == var {
                    Expr::Lambda { var: v.clone(), body: body.clone() }
                } else {
                    Expr::Lambda {
                        var: v.clone(),
                        body: Box::new(body.substitute(var, replacement)),
                    }
                }
            }
            Expr::Integrate { var: v, lower, upper, body } => {
                if v == var {
                    Expr::Integrate {
                        var: v.clone(),
                        lower: Box::new(lower.substitute(var, replacement)),
                        upper: Box::new(upper.substitute(var, replacement)),
                        body: body.clone(),
                    }
                } else {
                    Expr::Integrate {
                        var: v.clone(),
                        lower: Box::new(lower.substitute(var, replacement)),
                        upper: Box::new(upper.substitute(var, replacement)),
                        body: Box::new(body.substitute(var, replacement)),
                    }
                }
            }
            Expr::Derivative { var: v, body, at } => {
                if v == var {
                    Expr::Derivative {
                        var: v.clone(),
                        body: body.clone(),
                        at: Box::new(at.substitute(var, replacement)),
                    }
                } else {
                    Expr::Derivative {
                        var: v.clone(),
                        body: Box::new(body.substitute(var, replacement)),
                        at: Box::new(at.substitute(var, replacement)),
                    }
                }
            }
            Expr::Limit { var: v, to, body } => {
                if v == var {
                    Expr::Limit {
                        var: v.clone(),
                        to: Box::new(to.substitute(var, replacement)),
                        body: body.clone(),
                    }
                } else {
                    Expr::Limit {
                        var: v.clone(),
                        to: Box::new(to.substitute(var, replacement)),
                        body: Box::new(body.substitute(var, replacement)),
                    }
                }
            }

            // 向量/矩阵字面量
            Expr::VectorLit(elements) => Expr::VectorLit(
                elements.iter().map(|e| e.substitute(var, replacement)).collect()
            ),
            Expr::MatrixLit(rows) => Expr::MatrixLit(
                rows.iter()
                    .map(|row| row.iter().map(|e| e.substitute(var, replacement)).collect())
                    .collect()
            ),

            // 扩展一元函数
            Expr::Gamma(a) => Expr::Gamma(Box::new(a.substitute(var, replacement))),
            Expr::Lgamma(a) => Expr::Lgamma(Box::new(a.substitute(var, replacement))),
            Expr::Digamma(a) => Expr::Digamma(Box::new(a.substitute(var, replacement))),
            Expr::Erf(a) => Expr::Erf(Box::new(a.substitute(var, replacement))),
            Expr::Erfc(a) => Expr::Erfc(Box::new(a.substitute(var, replacement))),
            Expr::Erfinv(a) => Expr::Erfinv(Box::new(a.substitute(var, replacement))),
            Expr::Factorial(a) => Expr::Factorial(Box::new(a.substitute(var, replacement))),
            Expr::Zeta(a) => Expr::Zeta(Box::new(a.substitute(var, replacement))),
            Expr::BesselJ0(a) => Expr::BesselJ0(Box::new(a.substitute(var, replacement))),
            Expr::BesselJ1(a) => Expr::BesselJ1(Box::new(a.substitute(var, replacement))),
            Expr::BesselY0(a) => Expr::BesselY0(Box::new(a.substitute(var, replacement))),
            Expr::BesselY1(a) => Expr::BesselY1(Box::new(a.substitute(var, replacement))),
            Expr::BesselI0(a) => Expr::BesselI0(Box::new(a.substitute(var, replacement))),
            Expr::BesselI1(a) => Expr::BesselI1(Box::new(a.substitute(var, replacement))),
            Expr::BesselK0(a) => Expr::BesselK0(Box::new(a.substitute(var, replacement))),
            Expr::BesselK1(a) => Expr::BesselK1(Box::new(a.substitute(var, replacement))),
            Expr::Real(a) => Expr::Real(Box::new(a.substitute(var, replacement))),
            Expr::Imag(a) => Expr::Imag(Box::new(a.substitute(var, replacement))),
            Expr::Conj(a) => Expr::Conj(Box::new(a.substitute(var, replacement))),
            Expr::Carg(a) => Expr::Carg(Box::new(a.substitute(var, replacement))),
            Expr::Cabs(a) => Expr::Cabs(Box::new(a.substitute(var, replacement))),
            Expr::Sinc(a) => Expr::Sinc(Box::new(a.substitute(var, replacement))),

            // 扩展二元函数
            Expr::Beta(a, b) => Expr::Beta(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Lbeta(a, b) => Expr::Lbeta(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Combination(a, b) => Expr::Combination(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselJn(a, b) => Expr::BesselJn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselYn(a, b) => Expr::BesselYn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselIn(a, b) => Expr::BesselIn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselKn(a, b) => Expr::BesselKn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::TPdf(a, b) => Expr::TPdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::TCdf(a, b) => Expr::TCdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::TPpf(a, b) => Expr::TPpf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Chi2Pdf(a, b) => Expr::Chi2Pdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Chi2Cdf(a, b) => Expr::Chi2Cdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Chi2Ppf(a, b) => Expr::Chi2Ppf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::PoissonPmf(a, b) => Expr::PoissonPmf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::PoissonCdf(a, b) => Expr::PoissonCdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::ExponentialPdf(a, b) => Expr::ExponentialPdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::ExponentialCdf(a, b) => Expr::ExponentialCdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Complex(a, b) => Expr::Complex(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Polar(a, b) => Expr::Polar(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hypot(a, b) => Expr::Hypot(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Copysign(a, b) => Expr::Copysign(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Logn(a, b) => Expr::Logn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 扩展三元函数
            Expr::NormPdf(a, b, c) => Expr::NormPdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::NormCdf(a, b, c) => Expr::NormCdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::NormPpf(a, b, c) => Expr::NormPpf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::FPdf(a, b, c) => Expr::FPdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::FCdf(a, b, c) => Expr::FCdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::FPpf(a, b, c) => Expr::FPpf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::BinomialPmf(a, b, c) => Expr::BinomialPmf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::BinomialCdf(a, b, c) => Expr::BinomialCdf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Hypot3(a, b, c) => Expr::Hypot3(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Clamp(a, b, c) => Expr::Clamp(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Fma(a, b, c) => Expr::Fma(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // 扩展一元函数
            Expr::Expm1(a) => Expr::Expm1(Box::new(a.substitute(var, replacement))),
            Expr::Log1p(a) => Expr::Log1p(Box::new(a.substitute(var, replacement))),
            Expr::Exp2(a) => Expr::Exp2(Box::new(a.substitute(var, replacement))),
            Expr::Sec(a) => Expr::Sec(Box::new(a.substitute(var, replacement))),
            Expr::Csc(a) => Expr::Csc(Box::new(a.substitute(var, replacement))),
            Expr::Cot(a) => Expr::Cot(Box::new(a.substitute(var, replacement))),
            Expr::Asec(a) => Expr::Asec(Box::new(a.substitute(var, replacement))),
            Expr::Acsc(a) => Expr::Acsc(Box::new(a.substitute(var, replacement))),
            Expr::Acot(a) => Expr::Acot(Box::new(a.substitute(var, replacement))),
            Expr::Sech(a) => Expr::Sech(Box::new(a.substitute(var, replacement))),
            Expr::Csch(a) => Expr::Csch(Box::new(a.substitute(var, replacement))),
            Expr::Coth(a) => Expr::Coth(Box::new(a.substitute(var, replacement))),
            Expr::Asech(a) => Expr::Asech(Box::new(a.substitute(var, replacement))),
            Expr::Acsch(a) => Expr::Acsch(Box::new(a.substitute(var, replacement))),
            Expr::Acoth(a) => Expr::Acoth(Box::new(a.substitute(var, replacement))),
            Expr::AiryAi(a) => Expr::AiryAi(Box::new(a.substitute(var, replacement))),
            Expr::AiryBi(a) => Expr::AiryBi(Box::new(a.substitute(var, replacement))),
            Expr::FresnelS(a) => Expr::FresnelS(Box::new(a.substitute(var, replacement))),
            Expr::FresnelC(a) => Expr::FresnelC(Box::new(a.substitute(var, replacement))),
            Expr::Dawson(a) => Expr::Dawson(Box::new(a.substitute(var, replacement))),
            Expr::ExpInt(a) => Expr::ExpInt(Box::new(a.substitute(var, replacement))),
            Expr::LogInt(a) => Expr::LogInt(Box::new(a.substitute(var, replacement))),
            Expr::SinInt(a) => Expr::SinInt(Box::new(a.substitute(var, replacement))),
            Expr::CosInt(a) => Expr::CosInt(Box::new(a.substitute(var, replacement))),

            // 扩展二元函数
            Expr::Gammainc(a, b) => Expr::Gammainc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Gammaincc(a, b) => Expr::Gammaincc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 扩展三元函数
            Expr::Betainc(a, b, c) => Expr::Betainc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // 四元函数
            Expr::SphericalHarmonic(a, b, c, d) => Expr::SphericalHarmonic(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),
            Expr::Hyp2f1(a, b, c, d) => Expr::Hyp2f1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),

            // 新增一元函数
            Expr::LambertW(a) => Expr::LambertW(Box::new(a.substitute(var, replacement))),
            Expr::LambertWm1(a) => Expr::LambertWm1(Box::new(a.substitute(var, replacement))),
            Expr::KelvinBer(a) => Expr::KelvinBer(Box::new(a.substitute(var, replacement))),
            Expr::KelvinBei(a) => Expr::KelvinBei(Box::new(a.substitute(var, replacement))),
            Expr::KelvinKer(a) => Expr::KelvinKer(Box::new(a.substitute(var, replacement))),
            Expr::KelvinKei(a) => Expr::KelvinKei(Box::new(a.substitute(var, replacement))),
            Expr::Spence(a) => Expr::Spence(Box::new(a.substitute(var, replacement))),
            Expr::RiemannSiegelZ(a) => Expr::RiemannSiegelZ(Box::new(a.substitute(var, replacement))),
            Expr::RiemannSiegelTheta(a) => Expr::RiemannSiegelTheta(Box::new(a.substitute(var, replacement))),

            // 新增二元函数
            Expr::SphBesselJ(a, b) => Expr::SphBesselJ(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::SphBesselY(a, b) => Expr::SphBesselY(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::SphBesselI(a, b) => Expr::SphBesselI(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::SphBesselK(a, b) => Expr::SphBesselK(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hyp0f1(a, b) => Expr::Hyp0f1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::EllipF(a, b) => Expr::EllipF(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::EllipEInc(a, b) => Expr::EllipEInc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Polygamma(a, b) => Expr::Polygamma(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hankel1(a, b) => Expr::Hankel1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hankel2(a, b) => Expr::Hankel2(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::StruveH(a, b) => Expr::StruveH(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::StruveL(a, b) => Expr::StruveL(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::OwensT(a, b) => Expr::OwensT(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 新增三元函数
            Expr::Hyp1f1(a, b, c) => Expr::Hyp1f1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::EllipPi(a, b, c) => Expr::EllipPi(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // Jacobi 椭圆函数
            Expr::JacobiSn(a, b) => Expr::JacobiSn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::JacobiCn(a, b) => Expr::JacobiCn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::JacobiDn(a, b) => Expr::JacobiDn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 广义正交多项式
            Expr::Gegenbauer(a, b, c) => Expr::Gegenbauer(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::JacobiP(a, b, c, d) => Expr::JacobiP(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),

            // Mathieu 函数
            Expr::MathieuA(a, b) => Expr::MathieuA(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::MathieuB(a, b) => Expr::MathieuB(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::MathieuCe(a, b, c) => Expr::MathieuCe(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::MathieuSe(a, b, c) => Expr::MathieuSe(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // Coulomb 波函数
            Expr::CoulombF(a, b, c) => Expr::CoulombF(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::CoulombG(a, b, c) => Expr::CoulombG(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // Wigner 符号
            Expr::Wigner3j(a, b, c, d, e, f) => Expr::Wigner3j(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement)), Box::new(e.substitute(var, replacement)), Box::new(f.substitute(var, replacement))),
            Expr::Wigner6j(a, b, c, d, e, f) => Expr::Wigner6j(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement)), Box::new(e.substitute(var, replacement)), Box::new(f.substitute(var, replacement))),
            Expr::Wigner9j(a, b, c, d, e, f, g, h, i) => Expr::Wigner9j(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement)), Box::new(e.substitute(var, replacement)), Box::new(f.substitute(var, replacement)), Box::new(g.substitute(var, replacement)), Box::new(h.substitute(var, replacement)), Box::new(i.substitute(var, replacement))),

            // Theta 函数
            Expr::Theta1(a, b) => Expr::Theta1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Theta2(a, b) => Expr::Theta2(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Theta3(a, b) => Expr::Theta3(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Theta4(a, b) => Expr::Theta4(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 抛物柱面函数
            Expr::Pbdv(a, b) => Expr::Pbdv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Pbvv(a, b) => Expr::Pbvv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Pbwa(a, b) => Expr::Pbwa(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 球扁旋转体波函数
            Expr::ProAng1(a, b, c, d) => Expr::ProAng1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),
            Expr::ProRad1(a, b, c, d) => Expr::ProRad1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),
            Expr::ProRad2(a, b, c, d) => Expr::ProRad2(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),
            Expr::OblAng1(a, b, c, d) => Expr::OblAng1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),
            Expr::OblRad1(a, b, c, d) => Expr::OblRad1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),
            Expr::OblRad2(a, b, c, d) => Expr::OblRad2(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),

            // 修改 Fresnel 和 Wright Omega
            Expr::ModFresnelP(a) => Expr::ModFresnelP(Box::new(a.substitute(var, replacement))),
            Expr::ModFresnelM(a) => Expr::ModFresnelM(Box::new(a.substitute(var, replacement))),
            Expr::WrightOmega(a) => Expr::WrightOmega(Box::new(a.substitute(var, replacement))),

            // Wright Bessel 和 Voigt
            Expr::WrightBessel(a, b, c) => Expr::WrightBessel(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Voigt(a, b, c) => Expr::Voigt(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // 新增一元函数
            Expr::Logit(a) => Expr::Logit(Box::new(a.substitute(var, replacement))),
            Expr::Expit(a) => Expr::Expit(Box::new(a.substitute(var, replacement))),
            Expr::Entr(a) => Expr::Entr(Box::new(a.substitute(var, replacement))),
            Expr::Factorial2(a) => Expr::Factorial2(Box::new(a.substitute(var, replacement))),
            Expr::Erfcx(a) => Expr::Erfcx(Box::new(a.substitute(var, replacement))),
            Expr::Erfi(a) => Expr::Erfi(Box::new(a.substitute(var, replacement))),
            Expr::Erfcinv(a) => Expr::Erfcinv(Box::new(a.substitute(var, replacement))),
            Expr::Rgamma(a) => Expr::Rgamma(Box::new(a.substitute(var, replacement))),
            Expr::Gammasgn(a) => Expr::Gammasgn(Box::new(a.substitute(var, replacement))),
            Expr::Exprel(a) => Expr::Exprel(Box::new(a.substitute(var, replacement))),
            Expr::Zetac(a) => Expr::Zetac(Box::new(a.substitute(var, replacement))),

            // 新增二元函数
            Expr::BoxCox(a, b) => Expr::BoxCox(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BoxCox1p(a, b) => Expr::BoxCox1p(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::InvBoxCox(a, b) => Expr::InvBoxCox(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::InvBoxCox1p(a, b) => Expr::InvBoxCox1p(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::RelEntr(a, b) => Expr::RelEntr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::KlDiv(a, b) => Expr::KlDiv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Factorialk(a, b) => Expr::Factorialk(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Stirling2(a, b) => Expr::Stirling2(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Poch(a, b) => Expr::Poch(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::EllipRc(a, b) => Expr::EllipRc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Agm(a, b) => Expr::Agm(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Xlogy(a, b) => Expr::Xlogy(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Xlog1py(a, b) => Expr::Xlog1py(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::HurwitzZeta(a, b) => Expr::HurwitzZeta(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Polylog(a, b) => Expr::Polylog(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // 新增三元函数
            Expr::EllipRd(a, b, c) => Expr::EllipRd(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::EllipRf(a, b, c) => Expr::EllipRf(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::EllipRg(a, b, c) => Expr::EllipRg(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Hyperu(a, b, c) => Expr::Hyperu(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // 新增四元函数
            Expr::EllipRj(a, b, c, d) => Expr::EllipRj(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement)), Box::new(d.substitute(var, replacement))),

            // === 缩放贝塞尔函数 ===
            Expr::BesselI0e(a) => Expr::BesselI0e(Box::new(a.substitute(var, replacement))),
            Expr::BesselI1e(a) => Expr::BesselI1e(Box::new(a.substitute(var, replacement))),
            Expr::BesselIne(a, b) => Expr::BesselIne(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselK0e(a) => Expr::BesselK0e(Box::new(a.substitute(var, replacement))),
            Expr::BesselK1e(a) => Expr::BesselK1e(Box::new(a.substitute(var, replacement))),
            Expr::BesselKne(a, b) => Expr::BesselKne(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselJne(a, b) => Expr::BesselJne(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselYne(a, b) => Expr::BesselYne(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hankel1e(a, b) => Expr::Hankel1e(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hankel2e(a, b) => Expr::Hankel2e(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === 贝塞尔函数导数 ===
            Expr::BesselJnp(a, b) => Expr::BesselJnp(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselYnp(a, b) => Expr::BesselYnp(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselInp(a, b) => Expr::BesselInp(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::BesselKnp(a, b) => Expr::BesselKnp(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hankel1p(a, b) => Expr::Hankel1p(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Hankel2p(a, b) => Expr::Hankel2p(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === Huber 损失 ===
            Expr::Huber(a, b) => Expr::Huber(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::PseudoHuber(a, b) => Expr::PseudoHuber(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === Kolmogorov-Smirnov ===
            Expr::Kolmogorov(a) => Expr::Kolmogorov(Box::new(a.substitute(var, replacement))),
            Expr::Kolmogi(a) => Expr::Kolmogi(Box::new(a.substitute(var, replacement))),
            Expr::Smirnov(a, b) => Expr::Smirnov(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Smirnovi(a, b) => Expr::Smirnovi(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === Faddeeva ===
            Expr::Wofz(a) => Expr::Wofz(Box::new(a.substitute(var, replacement))),

            // === Dirichlet 核 ===
            Expr::Diric(a, b) => Expr::Diric(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === Tukey lambda ===
            Expr::Tklmbda(a, b) => Expr::Tklmbda(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === Gamma/Beta 逆函数 ===
            Expr::Gammaincinv(a, b) => Expr::Gammaincinv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Gammainccinv(a, b) => Expr::Gammainccinv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Betaincinv(a, b, c) => Expr::Betaincinv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // === 高精度便利函数 ===
            Expr::Cosm1(a) => Expr::Cosm1(Box::new(a.substitute(var, replacement))),
            Expr::Powm1(a, b) => Expr::Powm1(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Exp10(a) => Expr::Exp10(Box::new(a.substitute(var, replacement))),
            Expr::Log1pmx(a) => Expr::Log1pmx(Box::new(a.substitute(var, replacement))),
            Expr::Loggamma(a) => Expr::Loggamma(Box::new(a.substitute(var, replacement))),

            // === 度数三角函数 ===
            Expr::Cosdg(a) => Expr::Cosdg(Box::new(a.substitute(var, replacement))),
            Expr::Sindg(a) => Expr::Sindg(Box::new(a.substitute(var, replacement))),
            Expr::Tandg(a) => Expr::Tandg(Box::new(a.substitute(var, replacement))),
            Expr::Cotdg(a) => Expr::Cotdg(Box::new(a.substitute(var, replacement))),
            Expr::Radian(a, b, c) => Expr::Radian(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // === Airy 扩展 ===
            Expr::AiryAie(a) => Expr::AiryAie(Box::new(a.substitute(var, replacement))),
            Expr::AiryBie(a) => Expr::AiryBie(Box::new(a.substitute(var, replacement))),
            Expr::AiryAip(a) => Expr::AiryAip(Box::new(a.substitute(var, replacement))),
            Expr::AiryBip(a) => Expr::AiryBip(Box::new(a.substitute(var, replacement))),
            Expr::ItAiry(a) => Expr::ItAiry(Box::new(a.substitute(var, replacement))),

            // === 指数积分扩展 ===
            Expr::Expn(a, b) => Expr::Expn(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Exp1(a) => Expr::Exp1(Box::new(a.substitute(var, replacement))),
            Expr::Shi(a) => Expr::Shi(Box::new(a.substitute(var, replacement))),
            Expr::Chi(a) => Expr::Chi(Box::new(a.substitute(var, replacement))),

            // === Struve 积分 ===
            Expr::ItStruve0(a) => Expr::ItStruve0(Box::new(a.substitute(var, replacement))),
            Expr::It2Struve0(a) => Expr::It2Struve0(Box::new(a.substitute(var, replacement))),
            Expr::ItModStruve0(a) => Expr::ItModStruve0(Box::new(a.substitute(var, replacement))),

            // === ML/统计扩展 ===
            Expr::LogExpit(a) => Expr::LogExpit(Box::new(a.substitute(var, replacement))),
            Expr::Softplus(a) => Expr::Softplus(Box::new(a.substitute(var, replacement))),
            Expr::LogNdtr(a) => Expr::LogNdtr(Box::new(a.substitute(var, replacement))),

            // === Beta 补函数 ===
            Expr::Betaincc(a, b, c) => Expr::Betaincc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Betainccinv(a, b, c) => Expr::Betainccinv(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // === 数论函数 ===
            Expr::Bernoulli(a) => Expr::Bernoulli(Box::new(a.substitute(var, replacement))),
            Expr::Euler(a) => Expr::Euler(Box::new(a.substitute(var, replacement))),

            // === 椭圆扩展 ===
            Expr::EllipKm1(a) => Expr::EllipKm1(Box::new(a.substitute(var, replacement))),

            // === Kelvin 导数 ===
            Expr::KelvinBerp(a) => Expr::KelvinBerp(Box::new(a.substitute(var, replacement))),
            Expr::KelvinBeip(a) => Expr::KelvinBeip(Box::new(a.substitute(var, replacement))),
            Expr::KelvinKerp(a) => Expr::KelvinKerp(Box::new(a.substitute(var, replacement))),
            Expr::KelvinKeip(a) => Expr::KelvinKeip(Box::new(a.substitute(var, replacement))),

            // === 贝塞尔积分 ===
            Expr::BesselPoly(a, b, c) => Expr::BesselPoly(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // === Wright Bessel 扩展 ===
            Expr::LogWrightBessel(a, b, c) => Expr::LogWrightBessel(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // === 二项系数扩展 ===
            Expr::Binom(a, b) => Expr::Binom(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),

            // === 分布函数 ===
            Expr::Bdtr(a, b, c) => Expr::Bdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Bdtrc(a, b, c) => Expr::Bdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Bdtri(a, b, c) => Expr::Bdtri(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Chdtr(a, b) => Expr::Chdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Chdtrc(a, b) => Expr::Chdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Chdtri(a, b) => Expr::Chdtri(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Fdtr(a, b, c) => Expr::Fdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Fdtrc(a, b, c) => Expr::Fdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Fdtri(a, b, c) => Expr::Fdtri(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Stdtr(a, b) => Expr::Stdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Stdtrc(a, b) => Expr::Stdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Stdtrit(a, b) => Expr::Stdtrit(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Pdtr(a, b) => Expr::Pdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Pdtrc(a, b) => Expr::Pdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Pdtri(a, b) => Expr::Pdtri(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Btdtr(a, b, c) => Expr::Btdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Btdtrc(a, b, c) => Expr::Btdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Gdtr(a, b, c) => Expr::Gdtr(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Gdtrc(a, b, c) => Expr::Gdtrc(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),

            // === 积分/ML 扩展 ===
            Expr::Sici(a) => Expr::Sici(Box::new(a.substitute(var, replacement))),
            Expr::Shichi(a) => Expr::Shichi(Box::new(a.substitute(var, replacement))),
            Expr::Softmax(a) => Expr::Softmax(Box::new(a.substitute(var, replacement))),
            Expr::LogSoftmax(a) => Expr::LogSoftmax(Box::new(a.substitute(var, replacement))),
            Expr::Logsumexp(a) => Expr::Logsumexp(Box::new(a.substitute(var, replacement))),

            // === GSL 扩展 ===
            Expr::AiryZeroAi(a) => Expr::AiryZeroAi(Box::new(a.substitute(var, replacement))),
            Expr::AiryZeroBi(a) => Expr::AiryZeroBi(Box::new(a.substitute(var, replacement))),
            Expr::BesselZeroJ0(a) => Expr::BesselZeroJ0(Box::new(a.substitute(var, replacement))),
            Expr::BesselZeroJ1(a) => Expr::BesselZeroJ1(Box::new(a.substitute(var, replacement))),
            Expr::BesselZeroJnu(a, b) => Expr::BesselZeroJnu(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::SphLegendre(a, b, c) => Expr::SphLegendre(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement)), Box::new(c.substitute(var, replacement))),
            Expr::Clausen(a) => Expr::Clausen(Box::new(a.substitute(var, replacement))),
            Expr::Debye(a, b) => Expr::Debye(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::Synchrotron1(a) => Expr::Synchrotron1(Box::new(a.substitute(var, replacement))),
            Expr::Synchrotron2(a) => Expr::Synchrotron2(Box::new(a.substitute(var, replacement))),
            Expr::Transport(a, b) => Expr::Transport(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
            Expr::FermiDirac(a, b) => Expr::FermiDirac(Box::new(a.substitute(var, replacement)), Box::new(b.substitute(var, replacement))),
        }
    }
}

// ============================================
// 自定义 Serde 反序列化
// ============================================

/// YAML 表达式的中间表示（用于反序列化）
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum YamlExpr {
    /// 运算符节点
    Op {
        op: String,
        args: Vec<YamlExpr>,
    },
    /// 引用节点
    Ref {
        #[serde(rename = "ref")]
        name: String,
    },
    /// 常量节点
    Const {
        #[serde(rename = "const")]
        value: f64,
    },
    /// 条件表达式
    IfThenElse {
        #[serde(rename = "if")]
        cond: Box<YamlExpr>,
        #[serde(rename = "then")]
        then_branch: Box<YamlExpr>,
        #[serde(rename = "else")]
        else_branch: Box<YamlExpr>,
    },
    /// 求和表达式
    Sum {
        #[serde(rename = "sum")]
        index: String,
        lower: Box<YamlExpr>,
        upper: Box<YamlExpr>,
        body: Box<YamlExpr>,
    },
    /// 连乘表达式
    Product {
        #[serde(rename = "product")]
        index: String,
        lower: Box<YamlExpr>,
        upper: Box<YamlExpr>,
        body: Box<YamlExpr>,
    },
    /// 分段函数
    Piecewise {
        pieces: Vec<PiecewisePiece>,
        otherwise: Box<YamlExpr>,
    },
}

/// 分段函数的一个分支
#[derive(Debug, Clone, Deserialize)]
struct PiecewisePiece {
    condition: YamlExpr,
    value: YamlExpr,
}

impl YamlExpr {
    /// 转换为强类型 Expr
    fn into_expr(self) -> Result<Expr, String> {
        match self {
            YamlExpr::Const { value } => Ok(Expr::Const(value)),
            YamlExpr::Ref { name } => {
                // 检查是否是特殊常量
                match name.as_str() {
                    "pi" | "PI" => Ok(Expr::Pi),
                    "e" | "E" => Ok(Expr::E),
                    _ => Ok(Expr::var(name)),
                }
            }
            
            YamlExpr::IfThenElse { cond, then_branch, else_branch } => {
                Ok(Expr::if_then_else(
                    cond.into_expr()?,
                    then_branch.into_expr()?,
                    else_branch.into_expr()?,
                ))
            }

            YamlExpr::Sum { index, lower, upper, body } => {
                Ok(Expr::Sum {
                    index,
                    lower: Box::new(lower.into_expr()?),
                    upper: Box::new(upper.into_expr()?),
                    body: Box::new(body.into_expr()?),
                })
            }

            YamlExpr::Product { index, lower, upper, body } => {
                Ok(Expr::Product {
                    index,
                    lower: Box::new(lower.into_expr()?),
                    upper: Box::new(upper.into_expr()?),
                    body: Box::new(body.into_expr()?),
                })
            }

            YamlExpr::Piecewise { pieces, otherwise } => {
                let converted_pieces: Result<Vec<(Expr, Expr)>, String> = pieces
                    .into_iter()
                    .map(|p| Ok((p.condition.into_expr()?, p.value.into_expr()?)))
                    .collect();
                Ok(Expr::Piecewise {
                    pieces: converted_pieces?,
                    otherwise: Box::new(otherwise.into_expr()?),
                })
            }

            YamlExpr::Op { op, args } => {
                let converted: Result<Vec<Expr>, String> =
                    args.into_iter().map(|a| a.into_expr()).collect();
                let args = converted?;

                match op.as_str() {
                    // 常量
                    "pi" => Ok(Expr::Pi),
                    "e" => Ok(Expr::E),

                    // 算术运算
                    "add" => Self::binary_op(args, Expr::add, "add"),
                    "sub" => Self::binary_op(args, Expr::sub, "sub"),
                    "mul" => Self::binary_op(args, Expr::mul, "mul"),
                    "div" => Self::binary_op(args, Expr::div, "div"),
                    "neg" => Self::unary_op(args, Expr::neg, "neg"),
                    "pow" => Self::binary_op(args, Expr::pow, "pow"),
                    "abs" => Self::unary_op(args, Expr::abs, "abs"),
                    "mod" | "rem" => Self::binary_op(args, Expr::modulo, "mod"),
                    "ceil" => Self::unary_op(args, Expr::ceil, "ceil"),
                    "floor" => Self::unary_op(args, Expr::floor, "floor"),
                    "round" => Self::unary_op(args, Expr::round, "round"),
                    "trunc" => Self::unary_op(args, Expr::trunc, "trunc"),
                    "sign" | "signum" => Self::unary_op(args, Expr::sign, "sign"),

                    // 超越函数
                    "exp" => Self::unary_op(args, Expr::exp, "exp"),
                    "ln" | "log" => Self::unary_op(args, Expr::ln, "ln"),
                    "log10" => Self::unary_op(args, Expr::log10, "log10"),
                    "log2" => Self::unary_op(args, Expr::log2, "log2"),
                    "sqrt" => Self::unary_op(args, Expr::sqrt, "sqrt"),
                    "cbrt" => Self::unary_op(args, Expr::cbrt, "cbrt"),

                    // 三角函数
                    "sin" => Self::unary_op(args, Expr::sin, "sin"),
                    "cos" => Self::unary_op(args, Expr::cos, "cos"),
                    "tan" => Self::unary_op(args, Expr::tan, "tan"),
                    "asin" | "arcsin" => Self::unary_op(args, Expr::asin, "asin"),
                    "acos" | "arccos" => Self::unary_op(args, Expr::acos, "acos"),
                    "atan" | "arctan" => Self::unary_op(args, Expr::atan, "atan"),
                    "atan2" => Self::binary_op(args, Expr::atan2, "atan2"),

                    // 双曲函数
                    "sinh" => Self::unary_op(args, Expr::sinh, "sinh"),
                    "cosh" => Self::unary_op(args, Expr::cosh, "cosh"),
                    "tanh" => Self::unary_op(args, Expr::tanh, "tanh"),
                    "asinh" | "arcsinh" => Self::unary_op(args, Expr::asinh, "asinh"),
                    "acosh" | "arccosh" => Self::unary_op(args, Expr::acosh, "acosh"),
                    "atanh" | "arctanh" => Self::unary_op(args, Expr::atanh, "atanh"),

                    // 聚合函数
                    "max" => Ok(Expr::max(args)),
                    "min" => Ok(Expr::min(args)),

                    // 关系运算
                    "eq" => Self::binary_op_boxed(args, Expr::Eq, "eq"),
                    "lt" => Self::binary_op_boxed(args, Expr::Lt, "lt"),
                    "gt" => Self::binary_op_boxed(args, Expr::Gt, "gt"),
                    "leq" | "le" => Self::binary_op_boxed(args, Expr::Leq, "leq"),
                    "geq" | "ge" => Self::binary_op_boxed(args, Expr::Geq, "geq"),
                    "neq" | "ne" => Self::binary_op_boxed(args, Expr::Neq, "neq"),

                    // 逻辑运算
                    "and" => Self::binary_op_boxed(args, Expr::And, "and"),
                    "or" => Self::binary_op_boxed(args, Expr::Or, "or"),
                    "not" => Self::unary_op_boxed(args, Expr::Not, "not"),

                    // 扩展分位数函数
                    "exp_ppf" => Self::binary_op(args, Expr::exp_ppf, "exp_ppf"),
                    "gamma_ppf" => Self::ternary_op(args, Expr::gamma_ppf, "gamma_ppf"),
                    "beta_ppf" => Self::ternary_op(args, Expr::beta_ppf, "beta_ppf"),
                    "weibull_ppf" => Self::ternary_op(args, Expr::weibull_ppf, "weibull_ppf"),
                    "lognorm_ppf" => Self::ternary_op(args, Expr::lognorm_ppf, "lognorm_ppf"),
                    "uniform_ppf" => Self::ternary_op(args, Expr::uniform_ppf, "uniform_ppf"),
                    "cauchy_ppf" => Self::ternary_op(args, Expr::cauchy_ppf, "cauchy_ppf"),

                    // 复数扩展
                    "complex_sinh" => Self::unary_op(args, Expr::complex_sinh, "complex_sinh"),
                    "complex_cosh" => Self::unary_op(args, Expr::complex_cosh, "complex_cosh"),
                    "complex_tanh" => Self::unary_op(args, Expr::complex_tanh, "complex_tanh"),
                    "complex_asinh" => Self::unary_op(args, Expr::complex_asinh, "complex_asinh"),
                    "complex_acosh" => Self::unary_op(args, Expr::complex_acosh, "complex_acosh"),
                    "complex_atanh" => Self::unary_op(args, Expr::complex_atanh, "complex_atanh"),
                    "complex_asin" => Self::unary_op(args, Expr::complex_asin, "complex_asin"),
                    "complex_acos" => Self::unary_op(args, Expr::complex_acos, "complex_acos"),
                    "complex_atan" => Self::unary_op(args, Expr::complex_atan, "complex_atan"),

                    // 数论函数
                    "gcd" => Self::binary_op(args, Expr::gcd, "gcd"),
                    "lcm" => Self::binary_op(args, Expr::lcm, "lcm"),
                    "permutation" | "perm" => Self::binary_op(args, Expr::permutation, "permutation"),

                    // 正交多项式
                    "legendre" => Self::binary_op(args, Expr::legendre, "legendre"),
                    "legendre_assoc" => Self::ternary_op(args, Expr::legendre_assoc, "legendre_assoc"),
                    "hermite" => Self::binary_op(args, Expr::hermite, "hermite"),
                    "laguerre" => Self::binary_op(args, Expr::laguerre, "laguerre"),
                    "laguerre_assoc" => Self::ternary_op(args, Expr::laguerre_assoc, "laguerre_assoc"),
                    "chebyshev_t" => Self::binary_op(args, Expr::chebyshev_t, "chebyshev_t"),
                    "chebyshev_u" => Self::binary_op(args, Expr::chebyshev_u, "chebyshev_u"),

                    // 椭圆积分
                    "ellipk" => Self::unary_op(args, Expr::ellip_k, "ellipk"),
                    "ellipe" => Self::unary_op(args, Expr::ellip_e, "ellipe"),

                    // 向量运算
                    "vector" => Ok(Expr::VectorLit(args)),
                    "dot" => Self::binary_op(args, Expr::dot, "dot"),
                    "cross" => Self::binary_op(args, Expr::cross, "cross"),
                    "vec_norm" => Self::unary_op(args, Expr::vec_norm, "vec_norm"),
                    "vec_normalize" | "normalize" => Self::unary_op(args, Expr::vec_normalize, "normalize"),
                    "vsum" => Self::unary_op(args, Expr::vsum, "vsum"),
                    "vprod" => Self::unary_op(args, Expr::vprod, "vprod"),
                    "vmean" => Self::unary_op(args, Expr::vmean, "vmean"),
                    "vmin" => Self::unary_op(args, Expr::vmin, "vmin"),
                    "vmax" => Self::unary_op(args, Expr::vmax, "vmax"),

                    // 矩阵运算
                    "matmul" => Self::binary_op(args, Expr::mat_mul, "matmul"),
                    "transpose" => Self::unary_op(args, Expr::transpose, "transpose"),
                    "det" => Self::unary_op(args, Expr::det, "det"),
                    "inv" => Self::unary_op(args, Expr::inv, "inv"),
                    "eigenvalues" => Self::unary_op(args, Expr::eigenvalues, "eigenvalues"),
                    "trace" => Self::unary_op(args, Expr::trace, "trace"),
                    "mat_norm" => Self::unary_op(args, Expr::mat_norm, "mat_norm"),

                    // 特殊函数
                    "gamma" => Self::unary_op(args, Expr::gamma, "gamma"),
                    "lgamma" | "gammaln" => Self::unary_op(args, Expr::lgamma, "lgamma"),
                    "digamma" | "psi" => Self::unary_op(args, Expr::digamma, "digamma"),
                    "beta" => Self::binary_op(args, Expr::beta_fn, "beta"),
                    "lbeta" | "logbeta" | "betaln" => Self::binary_op(args, Expr::lbeta, "lbeta"),
                    "erf" => Self::unary_op(args, Expr::erf, "erf"),
                    "erfc" => Self::unary_op(args, Expr::erfc, "erfc"),
                    "erfinv" => Self::unary_op(args, Expr::erfinv, "erfinv"),
                    "factorial" | "fact" => Self::unary_op(args, Expr::factorial, "factorial"),
                    "combination" | "comb" | "choose" => Self::binary_op(args, Expr::combination, "combination"),
                    "zeta" | "riemann_zeta" => Self::unary_op(args, Expr::zeta, "zeta"),

                    // 贝塞尔函数
                    "bessel_j0" | "j0" => Self::unary_op(args, Expr::bessel_j0, "bessel_j0"),
                    "bessel_j1" | "j1" => Self::unary_op(args, Expr::bessel_j1, "bessel_j1"),
                    "bessel_jn" | "jn" | "jv" => Self::binary_op(args, Expr::bessel_jn, "bessel_jn"),
                    "bessel_y0" | "y0" => Self::unary_op(args, Expr::bessel_y0, "bessel_y0"),
                    "bessel_y1" | "y1" => Self::unary_op(args, Expr::bessel_y1, "bessel_y1"),
                    "bessel_yn" | "yn" | "yv" => Self::binary_op(args, Expr::bessel_yn, "bessel_yn"),
                    "bessel_i0" | "i0" => Self::unary_op(args, Expr::bessel_i0, "bessel_i0"),
                    "bessel_i1" | "i1" => Self::unary_op(args, Expr::bessel_i1, "bessel_i1"),
                    "bessel_in" | "in" | "iv" => Self::binary_op(args, Expr::bessel_in, "bessel_in"),
                    "bessel_k0" | "k0" => Self::unary_op(args, Expr::bessel_k0, "bessel_k0"),
                    "bessel_k1" | "k1" => Self::unary_op(args, Expr::bessel_k1, "bessel_k1"),
                    "bessel_kn" | "kn" | "kv" => Self::binary_op(args, Expr::bessel_kn, "bessel_kn"),

                    // 概率分布
                    "norm_pdf" => Self::ternary_op(args, Expr::norm_pdf, "norm_pdf"),
                    "norm_cdf" | "ndtr" => Self::ternary_op(args, Expr::norm_cdf, "norm_cdf"),
                    "norm_ppf" | "ndtri" => Self::ternary_op(args, Expr::norm_ppf, "norm_ppf"),
                    "t_pdf" => Self::binary_op(args, Expr::t_pdf, "t_pdf"),
                    "t_cdf" => Self::binary_op(args, Expr::t_cdf, "t_cdf"),
                    "t_ppf" => Self::binary_op(args, Expr::t_ppf, "t_ppf"),
                    "chi2_pdf" => Self::binary_op(args, Expr::chi2_pdf, "chi2_pdf"),
                    "chi2_cdf" => Self::binary_op(args, Expr::chi2_cdf, "chi2_cdf"),
                    "chi2_ppf" => Self::binary_op(args, Expr::chi2_ppf, "chi2_ppf"),
                    "f_pdf" => Self::ternary_op(args, Expr::f_pdf, "f_pdf"),
                    "f_cdf" => Self::ternary_op(args, Expr::f_cdf, "f_cdf"),
                    "f_ppf" => Self::ternary_op(args, Expr::f_ppf, "f_ppf"),
                    "poisson_pmf" => Self::binary_op(args, Expr::poisson_pmf, "poisson_pmf"),
                    "poisson_cdf" => Self::binary_op(args, Expr::poisson_cdf, "poisson_cdf"),
                    "binomial_pmf" => Self::ternary_op(args, Expr::binomial_pmf, "binomial_pmf"),
                    "binomial_cdf" => Self::ternary_op(args, Expr::binomial_cdf, "binomial_cdf"),
                    "exponential_pdf" | "exp_pdf" => Self::binary_op(args, Expr::exponential_pdf, "exponential_pdf"),
                    "exponential_cdf" | "exp_cdf" => Self::binary_op(args, Expr::exponential_cdf, "exponential_cdf"),

                    // 复数运算
                    "complex" => Self::binary_op(args, Expr::complex, "complex"),
                    "real" | "re" => Self::unary_op(args, Expr::real, "real"),
                    "imag" | "im" => Self::unary_op(args, Expr::imag, "imag"),
                    "conj" | "conjugate" => Self::unary_op(args, Expr::conj, "conj"),
                    "carg" | "arg" => Self::unary_op(args, Expr::carg, "carg"),
                    "cabs" => Self::unary_op(args, Expr::cabs, "cabs"),
                    "polar" => Self::binary_op(args, Expr::polar, "polar"),

                    // 基础数学补充
                    "hypot" => Self::binary_op(args, Expr::hypot, "hypot"),
                    "hypot3" => Self::ternary_op(args, Expr::hypot3, "hypot3"),
                    "clamp" => Self::ternary_op(args, Expr::clamp, "clamp"),
                    "copysign" => Self::binary_op(args, Expr::copysign, "copysign"),
                    "fma" | "mul_add" => Self::ternary_op(args, Expr::fma, "fma"),
                    "logn" | "log_base" => Self::binary_op(args, Expr::logn, "logn"),
                    "sinc" => Self::unary_op(args, Expr::sinc, "sinc"),

                    // 高精度数值函数
                    "expm1" => Self::unary_op(args, Expr::expm1, "expm1"),
                    "log1p" => Self::unary_op(args, Expr::log1p, "log1p"),
                    "exp2" => Self::unary_op(args, Expr::exp2, "exp2"),

                    // 不完全伽马/贝塔函数
                    "gammainc" | "gammainc_lower" => Self::binary_op(args, Expr::gammainc, "gammainc"),
                    "gammaincc" | "gammainc_upper" => Self::binary_op(args, Expr::gammaincc, "gammaincc"),
                    "betainc" | "regularized_betainc" => Self::ternary_op(args, Expr::betainc, "betainc"),

                    // 扩展三角函数
                    "sec" | "secant" => Self::unary_op(args, Expr::sec, "sec"),
                    "csc" | "cosecant" => Self::unary_op(args, Expr::csc, "csc"),
                    "cot" | "cotangent" => Self::unary_op(args, Expr::cot, "cot"),
                    "asec" | "arcsec" => Self::unary_op(args, Expr::asec, "asec"),
                    "acsc" | "arccsc" => Self::unary_op(args, Expr::acsc, "acsc"),
                    "acot" | "arccot" => Self::unary_op(args, Expr::acot, "acot"),

                    // 扩展双曲函数
                    "sech" => Self::unary_op(args, Expr::sech, "sech"),
                    "csch" => Self::unary_op(args, Expr::csch, "csch"),
                    "coth" => Self::unary_op(args, Expr::coth, "coth"),
                    "asech" | "arsech" => Self::unary_op(args, Expr::asech, "asech"),
                    "acsch" | "arcsch" => Self::unary_op(args, Expr::acsch, "acsch"),
                    "acoth" | "arcoth" => Self::unary_op(args, Expr::acoth, "acoth"),

                    // Airy 函数
                    "airy_ai" | "airyai" => Self::unary_op(args, Expr::airy_ai, "airy_ai"),
                    "airy_bi" | "airybi" => Self::unary_op(args, Expr::airy_bi, "airy_bi"),

                    // 球谐函数
                    "spherical_harmonic" | "sph_harm" | "ylm" => {
                        if args.len() != 4 {
                            return Err(format!("spherical_harmonic 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::spherical_harmonic(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // Fresnel 积分
                    "fresnel_s" | "fresnels" | "fresnel" => Self::unary_op(args, Expr::fresnel_s, "fresnel_s"),
                    "fresnel_c" | "fresnelc" => Self::unary_op(args, Expr::fresnel_c, "fresnel_c"),

                    // 其他特殊函数
                    "dawson" | "dawsn" => Self::unary_op(args, Expr::dawson, "dawson"),
                    "exp_int" | "expint" | "ei" | "expi" => Self::unary_op(args, Expr::exp_int, "exp_int"),
                    "log_int" | "logint" | "li" => Self::unary_op(args, Expr::log_int, "log_int"),
                    "sin_int" | "sinint" | "si" => Self::unary_op(args, Expr::sin_int, "sin_int"),
                    "cos_int" | "cosint" | "ci" => Self::unary_op(args, Expr::cos_int, "cos_int"),

                    // Lambert W
                    "lambertw" | "lambert_w" | "w0" => Self::unary_op(args, Expr::lambertw, "lambertw"),
                    "lambertw_m1" | "lambert_wm1" | "wm1" => Self::unary_op(args, Expr::lambertw_m1, "lambertw_m1"),

                    // 球贝塞尔函数
                    "sph_bessel_j" | "spherical_jn" | "jl" => Self::binary_op(args, Expr::sph_bessel_j, "sph_bessel_j"),
                    "sph_bessel_y" | "spherical_yn" | "yl" => Self::binary_op(args, Expr::sph_bessel_y, "sph_bessel_y"),
                    "sph_bessel_i" | "spherical_in" | "il" => Self::binary_op(args, Expr::sph_bessel_i, "sph_bessel_i"),
                    "sph_bessel_k" | "spherical_kn" | "kl" => Self::binary_op(args, Expr::sph_bessel_k, "sph_bessel_k"),

                    // 超几何函数
                    "hyp0f1" | "0f1" => Self::binary_op(args, Expr::hyp0f1, "hyp0f1"),
                    "hyp1f1" | "1f1" | "kummer_m" => Self::ternary_op(args, Expr::hyp1f1, "hyp1f1"),
                    "hyp2f1" | "2f1" | "gauss_hypergeometric" => {
                        if args.len() != 4 {
                            return Err(format!("hyp2f1 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::hyp2f1(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // Kelvin 函数
                    "kelvin_ber" | "ber" | "kelvin" => Self::unary_op(args, Expr::kelvin_ber, "kelvin_ber"),
                    "kelvin_bei" | "bei" => Self::unary_op(args, Expr::kelvin_bei, "kelvin_bei"),
                    "kelvin_ker" | "ker" => Self::unary_op(args, Expr::kelvin_ker, "kelvin_ker"),
                    "kelvin_kei" | "kei" => Self::unary_op(args, Expr::kelvin_kei, "kelvin_kei"),

                    // 不完全椭圆积分
                    "ellipf" | "ellipkinc" | "elliptic_f" => Self::binary_op(args, Expr::ellipf, "ellipf"),
                    "ellipe_inc" | "ellipeinc" | "elliptic_e_inc" => Self::binary_op(args, Expr::ellipe_inc, "ellipe_inc"),
                    "ellippi" | "elliptic_pi" => Self::ternary_op(args, Expr::ellippi, "ellippi"),

                    // 其他特殊函数
                    "spence" | "dilog" | "li2" => Self::unary_op(args, Expr::spence, "spence"),
                    "polygamma" | "psi_n" => Self::binary_op(args, Expr::polygamma, "polygamma"),
                    "hankel1" | "hankel_1" => Self::binary_op(args, Expr::hankel1, "hankel1"),
                    "hankel2" | "hankel_2" => Self::binary_op(args, Expr::hankel2, "hankel2"),
                    "struve_h" | "struve" => Self::binary_op(args, Expr::struve_h, "struve_h"),
                    "struve_l" | "modstruve" => Self::binary_op(args, Expr::struve_l, "struve_l"),
                    "owens_t" | "owenst" => Self::binary_op(args, Expr::owens_t, "owens_t"),
                    "riemann_siegel_z" | "siegelz" => Self::unary_op(args, Expr::riemann_siegel_z, "riemann_siegel_z"),
                    "riemann_siegel_theta" | "siegeltheta" => Self::unary_op(args, Expr::riemann_siegel_theta, "riemann_siegel_theta"),

                    // Jacobi 椭圆函数
                    "jacobi_sn" | "sn" | "ellipj" => Self::binary_op(args, Expr::jacobi_sn, "jacobi_sn"),
                    "jacobi_cn" | "cn" => Self::binary_op(args, Expr::jacobi_cn, "jacobi_cn"),
                    "jacobi_dn" | "dn" => Self::binary_op(args, Expr::jacobi_dn, "jacobi_dn"),

                    // 广义正交多项式
                    "gegenbauer" | "ultraspherical" => Self::ternary_op(args, Expr::gegenbauer, "gegenbauer"),
                    "jacobi_p" | "jacobi_poly" => {
                        if args.len() != 4 {
                            return Err(format!("jacobi_p 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::jacobi_p(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // Mathieu 函数
                    "mathieu_a" => Self::binary_op(args, Expr::mathieu_a, "mathieu_a"),
                    "mathieu_b" => Self::binary_op(args, Expr::mathieu_b, "mathieu_b"),
                    "mathieu_ce" | "mathieu_cem" => Self::ternary_op(args, Expr::mathieu_ce, "mathieu_ce"),
                    "mathieu_se" | "mathieu_sem" => Self::ternary_op(args, Expr::mathieu_se, "mathieu_se"),

                    // Coulomb 波函数
                    "coulomb_f" | "coulombf" => Self::ternary_op(args, Expr::coulomb_f, "coulomb_f"),
                    "coulomb_g" | "coulombg" => Self::ternary_op(args, Expr::coulomb_g, "coulomb_g"),

                    // Wigner 符号
                    "wigner_3j" | "wigner3j" => {
                        if args.len() != 6 {
                            return Err(format!("wigner_3j 需要6个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::wigner_3j(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }
                    "wigner_6j" | "wigner6j" => {
                        if args.len() != 6 {
                            return Err(format!("wigner_6j 需要6个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::wigner_6j(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }
                    "wigner_9j" | "wigner9j" => {
                        if args.len() != 9 {
                            return Err(format!("wigner_9j 需要9个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::wigner_9j(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // Theta 函数
                    "theta1" | "jtheta1" => Self::binary_op(args, Expr::theta1, "theta1"),
                    "theta2" | "jtheta2" => Self::binary_op(args, Expr::theta2, "theta2"),
                    "theta3" | "jtheta3" => Self::binary_op(args, Expr::theta3, "theta3"),
                    "theta4" | "jtheta4" => Self::binary_op(args, Expr::theta4, "theta4"),

                    // 抛物柱面函数
                    "pbdv" | "parabolic_d" => Self::binary_op(args, Expr::pbdv, "pbdv"),
                    "pbvv" | "parabolic_v" => Self::binary_op(args, Expr::pbvv, "pbvv"),
                    "pbwa" | "parabolic_w" => Self::binary_op(args, Expr::pbwa, "pbwa"),

                    // 球扁旋转体波函数（长球）
                    "pro_ang1" | "prolate_ang1" => {
                        if args.len() != 4 {
                            return Err(format!("pro_ang1 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::pro_ang1(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }
                    "pro_rad1" | "prolate_rad1" => {
                        if args.len() != 4 {
                            return Err(format!("pro_rad1 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::pro_rad1(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }
                    "pro_rad2" | "prolate_rad2" => {
                        if args.len() != 4 {
                            return Err(format!("pro_rad2 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::pro_rad2(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // 球扁旋转体波函数（扁球）
                    "obl_ang1" | "oblate_ang1" => {
                        if args.len() != 4 {
                            return Err(format!("obl_ang1 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::obl_ang1(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }
                    "obl_rad1" | "oblate_rad1" => {
                        if args.len() != 4 {
                            return Err(format!("obl_rad1 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::obl_rad1(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }
                    "obl_rad2" | "oblate_rad2" => {
                        if args.len() != 4 {
                            return Err(format!("obl_rad2 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::obl_rad2(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // 修改 Fresnel 积分
                    "modfresnelp" | "mod_fresnel_plus" => Self::unary_op(args, Expr::mod_fresnel_p, "modfresnelp"),
                    "modfresnelm" | "mod_fresnel_minus" => Self::unary_op(args, Expr::mod_fresnel_m, "modfresnelm"),

                    // Wright 函数
                    "wright_bessel" => Self::ternary_op(args, Expr::wright_bessel, "wright_bessel"),
                    "wright_omega" | "wrightomega" => Self::unary_op(args, Expr::wright_omega, "wright_omega"),

                    // Voigt
                    "voigt" | "voigt_profile" => Self::ternary_op(args, Expr::voigt, "voigt"),

                    // Sigmoid/Logistic
                    "logit" => Self::unary_op(args, Expr::logit, "logit"),
                    "expit" | "sigmoid" | "logistic" => Self::unary_op(args, Expr::expit, "expit"),

                    // Box-Cox
                    "boxcox" => Self::binary_op(args, Expr::boxcox, "boxcox"),
                    "boxcox1p" => Self::binary_op(args, Expr::boxcox1p, "boxcox1p"),
                    "inv_boxcox" => Self::binary_op(args, Expr::inv_boxcox, "inv_boxcox"),
                    "inv_boxcox1p" => Self::binary_op(args, Expr::inv_boxcox1p, "inv_boxcox1p"),

                    // 信息论
                    "entr" | "entropy" => Self::unary_op(args, Expr::entr, "entr"),
                    "rel_entr" | "relative_entropy" => Self::binary_op(args, Expr::rel_entr, "rel_entr"),
                    "kl_div" | "kl_divergence" => Self::binary_op(args, Expr::kl_div, "kl_div"),

                    // 阶乘扩展
                    "factorial2" | "double_factorial" => Self::unary_op(args, Expr::factorial2, "factorial2"),
                    "factorialk" => Self::binary_op(args, Expr::factorialk, "factorialk"),
                    "stirling2" => Self::binary_op(args, Expr::stirling2, "stirling2"),
                    "poch" | "pochhammer" => Self::binary_op(args, Expr::poch, "poch"),

                    // Carlson 椭圆积分
                    "elliprc" => Self::binary_op(args, Expr::elliprc, "elliprc"),
                    "elliprd" => Self::ternary_op(args, Expr::elliprd, "elliprd"),
                    "elliprf" => Self::ternary_op(args, Expr::elliprf, "elliprf"),
                    "elliprg" => Self::ternary_op(args, Expr::elliprg, "elliprg"),
                    "elliprj" => {
                        if args.len() != 4 {
                            return Err(format!("elliprj 需要4个参数，实际 {} 个", args.len()));
                        }
                        let mut it = args.into_iter();
                        Ok(Expr::elliprj(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
                    }

                    // 扩展误差函数
                    "erfcx" => Self::unary_op(args, Expr::erfcx, "erfcx"),
                    "erfi" => Self::unary_op(args, Expr::erfi, "erfi"),
                    "erfcinv" => Self::unary_op(args, Expr::erfcinv, "erfcinv"),

                    // 扩展 Gamma
                    "hyperu" => Self::ternary_op(args, Expr::hyperu, "hyperu"),
                    "rgamma" => Self::unary_op(args, Expr::rgamma, "rgamma"),
                    "gammasgn" => Self::unary_op(args, Expr::gammasgn, "gammasgn"),

                    // 便利函数
                    "agm" => Self::binary_op(args, Expr::agm, "agm"),
                    "exprel" => Self::unary_op(args, Expr::exprel, "exprel"),
                    "xlogy" => Self::binary_op(args, Expr::xlogy, "xlogy"),
                    "xlog1py" => Self::binary_op(args, Expr::xlog1py, "xlog1py"),

                    // Zeta 扩展
                    "hurwitz_zeta" => Self::binary_op(args, Expr::hurwitz_zeta, "hurwitz_zeta"),
                    "zetac" => Self::unary_op(args, Expr::zetac, "zetac"),
                    "polylog" => Self::binary_op(args, Expr::polylog, "polylog"),

                    // === 缩放贝塞尔函数 ===
                    "i0e" | "bessel_i0e" => Self::unary_op(args, Expr::bessel_i0e, "i0e"),
                    "i1e" | "bessel_i1e" => Self::unary_op(args, Expr::bessel_i1e, "i1e"),
                    "ive" | "bessel_ine" => Self::binary_op(args, Expr::bessel_ine, "ive"),
                    "k0e" | "bessel_k0e" => Self::unary_op(args, Expr::bessel_k0e, "k0e"),
                    "k1e" | "bessel_k1e" => Self::unary_op(args, Expr::bessel_k1e, "k1e"),
                    "kve" | "bessel_kne" => Self::binary_op(args, Expr::bessel_kne, "kve"),
                    "jve" | "bessel_jne" => Self::binary_op(args, Expr::bessel_jne, "jve"),
                    "yve" | "bessel_yne" => Self::binary_op(args, Expr::bessel_yne, "yve"),
                    "hankel1e" => Self::binary_op(args, Expr::hankel1e, "hankel1e"),
                    "hankel2e" => Self::binary_op(args, Expr::hankel2e, "hankel2e"),

                    // === 贝塞尔函数导数 ===
                    "jvp" | "bessel_jnp" => Self::binary_op(args, Expr::bessel_jnp, "jvp"),
                    "yvp" | "bessel_ynp" => Self::binary_op(args, Expr::bessel_ynp, "yvp"),
                    "ivp" | "bessel_inp" => Self::binary_op(args, Expr::bessel_inp, "ivp"),
                    "kvp" | "bessel_knp" => Self::binary_op(args, Expr::bessel_knp, "kvp"),
                    "h1vp" | "hankel1p" => Self::binary_op(args, Expr::hankel1p, "h1vp"),
                    "h2vp" | "hankel2p" => Self::binary_op(args, Expr::hankel2p, "h2vp"),

                    // === Huber 损失 ===
                    "huber" => Self::binary_op(args, Expr::huber, "huber"),
                    "pseudo_huber" => Self::binary_op(args, Expr::pseudo_huber, "pseudo_huber"),

                    // === Kolmogorov-Smirnov ===
                    "kolmogorov" => Self::unary_op(args, Expr::kolmogorov, "kolmogorov"),
                    "kolmogi" => Self::unary_op(args, Expr::kolmogi, "kolmogi"),
                    "smirnov" => Self::binary_op(args, Expr::smirnov, "smirnov"),
                    "smirnovi" => Self::binary_op(args, Expr::smirnovi, "smirnovi"),

                    // === Faddeeva ===
                    "wofz" | "faddeeva" => Self::unary_op(args, Expr::wofz, "wofz"),

                    // === Dirichlet 核 ===
                    "diric" | "dirichlet" => Self::binary_op(args, Expr::diric, "diric"),

                    // === Tukey lambda ===
                    "tklmbda" | "tukey_lambda" => Self::binary_op(args, Expr::tklmbda, "tklmbda"),

                    // === Gamma/Beta 逆函数 ===
                    "gammaincinv" => Self::binary_op(args, Expr::gammaincinv, "gammaincinv"),
                    "gammainccinv" => Self::binary_op(args, Expr::gammainccinv, "gammainccinv"),
                    "betaincinv" => Self::ternary_op(args, Expr::betaincinv, "betaincinv"),

                    // === 高精度便利函数 ===
                    "cosm1" => Self::unary_op(args, Expr::cosm1, "cosm1"),
                    "powm1" => Self::binary_op(args, Expr::powm1, "powm1"),
                    "exp10" => Self::unary_op(args, Expr::exp10, "exp10"),
                    "log1pmx" => Self::unary_op(args, Expr::log1pmx, "log1pmx"),
                    "loggamma" => Self::unary_op(args, Expr::loggamma, "loggamma"),

                    // === 度数三角函数 ===
                    "cosdg" => Self::unary_op(args, Expr::cosdg, "cosdg"),
                    "sindg" => Self::unary_op(args, Expr::sindg, "sindg"),
                    "tandg" => Self::unary_op(args, Expr::tandg, "tandg"),
                    "cotdg" => Self::unary_op(args, Expr::cotdg, "cotdg"),
                    "radian" => Self::ternary_op(args, Expr::radian, "radian"),

                    // === Airy 扩展 ===
                    "airy" => Self::unary_op(args, Expr::airy_ai, "airy"),
                    "airye" => Self::unary_op(args, Expr::airy_aie, "airye"),
                    "aie" | "airy_aie" => Self::unary_op(args, Expr::airy_aie, "aie"),
                    "bie" | "airy_bie" => Self::unary_op(args, Expr::airy_bie, "bie"),
                    "aip" | "airy_aip" => Self::unary_op(args, Expr::airy_aip, "aip"),
                    "bip" | "airy_bip" => Self::unary_op(args, Expr::airy_bip, "bip"),
                    "itairy" => Self::unary_op(args, Expr::itairy, "itairy"),

                    // === 指数积分扩展 ===
                    "expn" => Self::binary_op(args, Expr::expn, "expn"),
                    "exp1" | "e1" => Self::unary_op(args, Expr::exp1, "exp1"),
                    "shi" => Self::unary_op(args, Expr::shi, "shi"),
                    "chi" => Self::unary_op(args, Expr::chi, "chi"),

                    // === Struve 积分 ===
                    "itstruve0" => Self::unary_op(args, Expr::itstruve0, "itstruve0"),
                    "it2struve0" => Self::unary_op(args, Expr::it2struve0, "it2struve0"),
                    "itmodstruve0" => Self::unary_op(args, Expr::itmodstruve0, "itmodstruve0"),

                    // === ML/统计扩展 ===
                    "log_expit" => Self::unary_op(args, Expr::log_expit, "log_expit"),
                    "softplus" => Self::unary_op(args, Expr::softplus, "softplus"),
                    "log_ndtr" => Self::unary_op(args, Expr::log_ndtr, "log_ndtr"),

                    // === Beta 补函数 ===
                    "betaincc" => Self::ternary_op(args, Expr::betaincc, "betaincc"),
                    "betainccinv" => Self::ternary_op(args, Expr::betainccinv, "betainccinv"),

                    // === 数论函数 ===
                    "bernoulli" => Self::unary_op(args, Expr::bernoulli, "bernoulli"),
                    "euler" => Self::unary_op(args, Expr::euler, "euler"),

                    // === 椭圆扩展 ===
                    "ellipkm1" => Self::unary_op(args, Expr::ellipkm1, "ellipkm1"),

                    // === Kelvin 导数 ===
                    "berp" | "kelvin_berp" => Self::unary_op(args, Expr::kelvin_berp, "berp"),
                    "beip" | "kelvin_beip" => Self::unary_op(args, Expr::kelvin_beip, "beip"),
                    "kerp" | "kelvin_kerp" => Self::unary_op(args, Expr::kelvin_kerp, "kerp"),
                    "keip" | "kelvin_keip" => Self::unary_op(args, Expr::kelvin_keip, "keip"),

                    // === 贝塞尔积分 ===
                    "besselpoly" => Self::ternary_op(args, Expr::besselpoly, "besselpoly"),

                    // === Wright Bessel 扩展 ===
                    "log_wright_bessel" => Self::ternary_op(args, Expr::log_wright_bessel, "log_wright_bessel"),

                    // === 二项系数扩展 ===
                    "binom" => Self::binary_op(args, Expr::binom, "binom"),

                    // === 分布函数 ===
                    "bdtr" => Self::ternary_op(args, Expr::bdtr, "bdtr"),
                    "bdtrc" => Self::ternary_op(args, Expr::bdtrc, "bdtrc"),
                    "bdtri" => Self::ternary_op(args, Expr::bdtri, "bdtri"),
                    "chdtr" => Self::binary_op(args, Expr::chdtr, "chdtr"),
                    "chdtrc" => Self::binary_op(args, Expr::chdtrc, "chdtrc"),
                    "chdtri" => Self::binary_op(args, Expr::chdtri, "chdtri"),
                    "fdtr" => Self::ternary_op(args, Expr::fdtr, "fdtr"),
                    "fdtrc" => Self::ternary_op(args, Expr::fdtrc, "fdtrc"),
                    "fdtri" => Self::ternary_op(args, Expr::fdtri, "fdtri"),
                    "stdtr" => Self::binary_op(args, Expr::stdtr, "stdtr"),
                    "stdtrc" => Self::binary_op(args, Expr::stdtrc, "stdtrc"),
                    "stdtrit" => Self::binary_op(args, Expr::stdtrit, "stdtrit"),
                    "pdtr" => Self::binary_op(args, Expr::pdtr, "pdtr"),
                    "pdtrc" => Self::binary_op(args, Expr::pdtrc, "pdtrc"),
                    "pdtri" => Self::binary_op(args, Expr::pdtri, "pdtri"),
                    "btdtr" => Self::ternary_op(args, Expr::btdtr, "btdtr"),
                    "btdtrc" => Self::ternary_op(args, Expr::btdtrc, "btdtrc"),
                    "gdtr" => Self::ternary_op(args, Expr::gdtr, "gdtr"),
                    "gdtrc" => Self::ternary_op(args, Expr::gdtrc, "gdtrc"),

                    // === 积分/ML 扩展 ===
                    "sici" => Self::unary_op(args, Expr::sici, "sici"),
                    "shichi" => Self::unary_op(args, Expr::shichi, "shichi"),
                    "softmax" => Self::unary_op(args, Expr::softmax, "softmax"),
                    "log_softmax" => Self::unary_op(args, Expr::log_softmax, "log_softmax"),
                    "logsumexp" => Self::unary_op(args, Expr::logsumexp, "logsumexp"),

                    // === GSL 扩展 ===
                    "ai_zero" | "airy_zero_ai" => Self::unary_op(args, Expr::airy_zero_ai, "ai_zero"),
                    "bi_zero" | "airy_zero_bi" => Self::unary_op(args, Expr::airy_zero_bi, "bi_zero"),
                    "bessel_zero_j0" | "j0_zero" => Self::unary_op(args, Expr::bessel_zero_j0, "bessel_zero_j0"),
                    "bessel_zero_j1" | "j1_zero" => Self::unary_op(args, Expr::bessel_zero_j1, "bessel_zero_j1"),
                    "bessel_zero_jnu" | "jnu_zero" => Self::binary_op(args, Expr::bessel_zero_jnu, "bessel_zero_jnu"),
                    "sph_legendre" | "lpmv" => Self::ternary_op(args, Expr::sph_legendre, "sph_legendre"),
                    "clausen" => Self::unary_op(args, Expr::clausen, "clausen"),
                    "debye" => Self::binary_op(args, Expr::debye, "debye"),
                    "synchrotron1" => Self::unary_op(args, Expr::synchrotron1, "synchrotron1"),
                    "synchrotron2" => Self::unary_op(args, Expr::synchrotron2, "synchrotron2"),
                    "transport" => Self::binary_op(args, Expr::transport, "transport"),
                    "fermi_dirac" => Self::binary_op(args, Expr::fermi_dirac, "fermi_dirac"),

                    _ => Err(format!("未知运算符: {}", op)),
                }
            }
        }
    }

    fn unary_op(args: Vec<Expr>, f: fn(Expr) -> Expr, name: &str) -> Result<Expr, String> {
        if args.len() != 1 {
            return Err(format!("{}运算符需要1个参数，实际提供{}个", name, args.len()));
        }
        Ok(f(args.into_iter().next().unwrap()))
    }

    fn binary_op(args: Vec<Expr>, f: fn(Expr, Expr) -> Expr, name: &str) -> Result<Expr, String> {
        if args.len() != 2 {
            return Err(format!("{}运算符需要2个参数，实际提供{}个", name, args.len()));
        }
        let mut iter = args.into_iter();
        Ok(f(iter.next().unwrap(), iter.next().unwrap()))
    }

    fn unary_op_boxed(args: Vec<Expr>, f: fn(Box<Expr>) -> Expr, name: &str) -> Result<Expr, String> {
        if args.len() != 1 {
            return Err(format!("{}运算符需要1个参数，实际提供{}个", name, args.len()));
        }
        Ok(f(Box::new(args.into_iter().next().unwrap())))
    }

    fn binary_op_boxed(args: Vec<Expr>, f: fn(Box<Expr>, Box<Expr>) -> Expr, name: &str) -> Result<Expr, String> {
        if args.len() != 2 {
            return Err(format!("{}运算符需要2个参数，实际提供{}个", name, args.len()));
        }
        let mut iter = args.into_iter();
        Ok(f(Box::new(iter.next().unwrap()), Box::new(iter.next().unwrap())))
    }

    fn ternary_op(args: Vec<Expr>, f: fn(Expr, Expr, Expr) -> Expr, name: &str) -> Result<Expr, String> {
        if args.len() != 3 {
            return Err(format!("{}运算符需要3个参数，实际提供{}个", name, args.len()));
        }
        let mut iter = args.into_iter();
        Ok(f(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
    }
}

impl Expr {
    /// 从 YAML 格式的 serde Value 反序列化为 Expr
    ///
    /// YAML 格式使用 `{op: "add", args: [...]}` 风格，
    /// 与 `#[derive(Serialize)]` 产生的外部标签格式不同。
    /// 此方法专门用于解析 YAML 配置文件。
    pub fn from_yaml_value<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let yaml_expr = YamlExpr::deserialize(deserializer)?;
        yaml_expr.into_expr().map_err(serde::de::Error::custom)
    }
}

/// 手写 `Deserialize`，使 `Expr` 直接支持文档/示例的 map 格式
/// （`{op: add, args: [...]}`、`{ref: x}`、`{const: 0}`、`if/then/else` 等）。
///
/// 这样无需在每个包含 `Expr` 的字段上标注 `#[serde(deserialize_with = ...)]`，
/// 嵌套表达式（如 `args` 中的子表达式）也会自动走同一解析逻辑。
impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Expr::from_yaml_value(deserializer)
    }
}

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_construction() {
        let expr = Expr::add(Expr::param("p1"), Expr::mul(Expr::param("p2"), Expr::var("x")));
        
        let params = expr.get_parameter_refs();
        assert!(params.contains(&"p1".to_string()));
        assert!(params.contains(&"p2".to_string()));
        
        let vars = expr.get_variable_refs();
        assert!(vars.contains(&"x".to_string()));
    }

    #[test]
    fn test_to_python() {
        let expr = Expr::add(Expr::param("p1"), Expr::var("x"));
        let python = expr.to_python("params");
        assert_eq!(python, "(params.p1 + x)");
    }

    #[test]
    fn test_to_rust() {
        let expr = Expr::mul(Expr::param("p1"), Expr::exp(Expr::var("x")));
        let rust = expr.to_rust();
        assert_eq!(rust, "(p1 * x.exp())");
    }

    #[test]
    fn test_to_latex() {
        let expr = Expr::div(Expr::var("a"), Expr::var("b"));
        let latex = expr.to_latex();
        assert_eq!(latex, "\\frac{a}{b}");
    }

    #[test]
    fn test_constants() {
        assert_eq!(Expr::Pi.to_python("p"), "np.pi");
        assert_eq!(Expr::E.to_python("p"), "np.e");
        assert_eq!(Expr::Pi.to_rust(), "std::f64::consts::PI");
        assert_eq!(Expr::E.to_rust(), "std::f64::consts::E");
        assert_eq!(Expr::Pi.to_latex(), "\\pi");
    }

    #[test]
    fn test_new_operators() {
        // 测试新增的运算符
        let ceil = Expr::ceil(Expr::var("x"));
        assert_eq!(ceil.to_python("p"), "np.ceil(x)");
        assert_eq!(ceil.to_rust(), "x.ceil()");

        let atan2 = Expr::atan2(Expr::var("y"), Expr::var("x"));
        assert_eq!(atan2.to_python("p"), "np.arctan2(y, x)");
        assert_eq!(atan2.to_rust(), "y.atan2(x)");

        let sinh = Expr::sinh(Expr::var("x"));
        assert_eq!(sinh.to_python("p"), "np.sinh(x)");
        assert_eq!(sinh.to_rust(), "x.sinh()");
    }

    #[test]
    fn test_if_then_else() {
        let expr = Expr::if_then_else(
            Expr::Gt(Box::new(Expr::var("x")), Box::new(Expr::Const(0.0))),
            Expr::var("x"),
            Expr::neg(Expr::var("x")),
        );
        
        let python = expr.to_python("p");
        assert!(python.contains("if"));
        assert!(python.contains("else"));
    }

    #[test]
    fn test_depth() {
        // 叶子节点深度为 1
        assert_eq!(Expr::Const(1.0).depth(), 1);
        assert_eq!(Expr::Pi.depth(), 1);
        
        // a + b 深度为 2
        let simple = Expr::add(Expr::var("a"), Expr::var("b"));
        assert_eq!(simple.depth(), 2);
        
        // a + (b * c) 深度为 3
        let nested = Expr::add(Expr::var("a"), Expr::mul(Expr::var("b"), Expr::var("c")));
        assert_eq!(nested.depth(), 3);
    }

    #[test]
    fn test_substitute() {
        let expr = Expr::add(Expr::var("x"), Expr::var("y"));
        let replaced = expr.substitute("x", &Expr::Const(5.0));
        
        match replaced {
            Expr::Add(left, _) => match *left {
                Expr::Const(v) => assert_eq!(v, 5.0),
                _ => panic!("替换失败"),
            },
            _ => panic!("结构错误"),
        }
    }

    #[test]
    fn test_yaml_deserialization() {
        let yaml = r#"
op: add
args:
  - { ref: p1 }
  - op: mul
    args:
      - { ref: p2 }
      - { ref: x }
"#;
        
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        
        match expr {
            Expr::Add(_, _) => (),
            _ => panic!("应该是 Add 节点"),
        }
        
        let params = expr.get_parameter_refs();
        assert!(params.contains(&"p1".to_string()));
        assert!(params.contains(&"p2".to_string()));
    }

    #[test]
    fn test_conditional_yaml() {
        let yaml = r#"
if: { op: gt, args: [{ ref: x }, { const: 0 }] }
then: { ref: x }
else: { op: neg, args: [{ ref: x }] }
"#;
        
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        
        match expr {
            Expr::IfThenElse { .. } => (),
            _ => panic!("应该是 IfThenElse 节点"),
        }
    }

    #[test]
    fn test_new_ops_yaml() {
        // 测试新增运算符的 YAML 解析
        let yaml = r#"
op: ceil
args: [{ ref: x }]
"#;
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(expr, Expr::Ceil(_)));

        let yaml = r#"
op: atan2
args:
  - { ref: y }
  - { ref: x }
"#;
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(expr, Expr::ATan2(_, _)));

        let yaml = r#"
op: sinh
args: [{ ref: x }]
"#;
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(expr, Expr::Sinh(_)));
    }

    #[test]
    fn test_pi_e_yaml() {
        // 测试 pi 和 e 常量
        let yaml = r#"{ ref: pi }"#;
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(expr, Expr::Pi));

        let yaml = r#"{ ref: e }"#;
        let expr: Expr = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(expr, Expr::E));
    }
}
