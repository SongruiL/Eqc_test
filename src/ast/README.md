# AST 模块

抽象语法树（Abstract Syntax Tree）模块定义了方程表达式的内存表示。

## 职责

- 定义表达式的强类型 AST 节点（`Expr` 枚举）
- 提供 AST 遍历的访问者模式（`ExprVisitor` trait）
- 支持代码生成（`to_python()`, `to_rust()`, `to_latex()`）
- 支持表达式分析（引用收集、深度计算、变量替换）
- 作为 YAML 解析和 S表达式解析的统一目标格式

## 核心类型

### `Expr` 枚举

强类型的表达式节点，每个运算符都是独立的变体。支持超过 345 个运算符：

```rust
pub enum Expr {
    // 叶子节点
    Const(f64),      // 常量
    Var(String),     // 变量引用
    Param(String),   // 参数引用
    Pi,              // 圆周率 π
    E,               // 自然常数 e
    
    // 算术运算
    Add(Box<Expr>, Box<Expr>),  // 加法
    Sub(Box<Expr>, Box<Expr>),  // 减法
    // ... 更多运算符
    
    // 扩展运算符
    ExpPpf(Box<Expr>, Box<Expr>),     // 指数分布分位数
    Gcd(Box<Expr>, Box<Expr>),         // 最大公约数
    Legendre(Box<Expr>, Box<Expr>),    // 勒让德多项式
    Integrate { ... },                  // 定积分
    VectorLit(Vec<Expr>),              // 向量字面量
    // ... 更多扩展运算符
}
```

### `ExprVisitor` trait

访问者模式接口，用于 AST 遍历。

## 支持的运算符类别

### 基础运算符（~45个）
- 算术运算：add, sub, mul, div, pow, mod, neg, abs, ceil, floor, round, trunc, sign
- 超越函数：exp, ln, log10, log2, sqrt, cbrt, hypot, hypot3, logn, expm1, log1p, exp2
- 三角函数：sin, cos, tan, asin, acos, atan, atan2, sec, csc, cot, asec, acsc, acot
- 双曲函数：sinh, cosh, tanh, asinh, acosh, atanh, sech, csch, coth, asech, acsch, acoth
- 其他：clamp, copysign, fma, sinc

### 特殊函数（115+个，需要 advanced_math/gsl_math）
- 伽马：gamma, lgamma, digamma, polygamma, gammainc, gammaincc, rgamma, gammasgn, hyperu, gammaincinv, gammainccinv, loggamma
- 贝塔：beta, lbeta, betainc, betaincinv, betaincc, betainccinv
- 误差函数：erf, erfc, erfinv, erfcx, erfi, erfcinv, wofz (Faddeeva)
- Airy 扩展：airy_ai, airy_bi, airy_aie, airy_bie, airy_aip, airy_bip, itairy
- 指数积分扩展：expn, exp1, shi, chi
- Struve 积分：itstruve0, it2struve0, itmodstruve0
- Kelvin 导数：berp, beip, kerp, keip
- 数论函数：bernoulli, euler
- 贝塞尔积分：besselpoly
- Wright 扩展：log_wright_bessel
- 组合数学：factorial, factorial2, factorialk, combination, stirling2, poch
- Airy 函数：airy_ai, airy_bi
- 积分函数：fresnel_s, fresnel_c, dawson, exp_int, log_int, sin_int, cos_int
- Lambert W：lambertw, lambertw_m1
- 超几何：hyp0f1, hyp1f1, hyp2f1
- Kelvin：kelvin_ber, kelvin_bei, kelvin_ker, kelvin_kei
- Struve：struve_h, struve_l
- Hankel：hankel1, hankel2, hankel1e, hankel2e
- Jacobi 椭圆：jacobi_sn, jacobi_cn, jacobi_dn
- Mathieu：mathieu_a, mathieu_b, mathieu_ce, mathieu_se
- Coulomb 波函数：coulomb_f, coulomb_g
- Wigner 符号：wigner_3j, wigner_6j, wigner_9j
- Theta 函数：theta1, theta2, theta3, theta4
- 抛物柱面：pbdv, pbvv, pbwa
- 球扁旋转体波：pro_ang1, pro_rad1, pro_rad2, obl_ang1, obl_rad1, obl_rad2
- 修改 Fresnel：modfresnelp, modfresnelm
- Wright：wright_bessel, wright_omega
- Voigt：voigt
- Carlson 椭圆积分：elliprc, elliprd, elliprf, elliprg, elliprj
- Zeta 扩展：hurwitz_zeta, zetac, polylog
- Kolmogorov-Smirnov：kolmogorov, kolmogi, smirnov, smirnovi
- Dirichlet 核：diric
- Tukey lambda：tklmbda
- 球谐函数：spherical_harmonic
- 其他：zeta, spence, owens_t, riemann_siegel_z

### 机器学习/统计函数（25个）
- Sigmoid：logit, expit, log_expit, softplus
- Box-Cox：boxcox, boxcox1p, inv_boxcox, inv_boxcox1p
- 信息论：entr, rel_entr, kl_div
- Huber 损失：huber, pseudo_huber
- 正态分布扩展：log_ndtr
- 便利函数：agm, exprel, xlogy, xlog1py, binom
- 高精度函数：cosm1, powm1, exp10, log1pmx
- 椭圆扩展：ellipkm1

### 度数三角函数（5个）
- 度数版本：cosdg, sindg, tandg, cotdg
- 转换函数：radian (度分秒转弧度)

### 贝塞尔函数（32个，需要 gsl_math）
- 第一类：bessel_j0, bessel_j1, bessel_jn
- 第二类：bessel_y0, bessel_y1, bessel_yn
- 修正第一类：bessel_i0, bessel_i1, bessel_in
- 修正第二类：bessel_k0, bessel_k1, bessel_kn
- 球贝塞尔：sph_bessel_j, sph_bessel_y, sph_bessel_i, sph_bessel_k
- 缩放版本：i0e, i1e, ive, k0e, k1e, kve, jve, yve
- 导数版本：jvp, yvp, ivp, kvp, h1vp, h2vp

### 概率分布（25个，需要 advanced_math）
- 正态：norm_pdf, norm_cdf, norm_ppf
- t 分布：t_pdf, t_cdf, t_ppf
- 卡方：chi2_pdf, chi2_cdf, chi2_ppf
- F 分布：f_pdf, f_cdf, f_ppf
- 泊松：poisson_pmf, poisson_cdf
- 二项：binomial_pmf, binomial_cdf
- 指数：exponential_pdf, exponential_cdf, exp_ppf
- 其他 PPF：gamma_ppf, beta_ppf, weibull_ppf, lognorm_ppf, uniform_ppf, cauchy_ppf

### 复数运算（16个，需要 advanced_math）
- 基础：complex, real, imag, conj, carg, cabs, polar
- 三角/双曲：complex_sinh, complex_cosh, complex_tanh, complex_asinh, complex_acosh, complex_atanh, complex_asin, complex_acos, complex_atan

### 数论函数（3个）
- gcd, lcm, permutation

### 正交多项式与椭圆积分（14个，需要 gsl_math）
- legendre, legendre_assoc, hermite
- laguerre, laguerre_assoc
- chebyshev_t, chebyshev_u
- gegenbauer, jacobi_p
- 完全椭圆积分：ellipk, ellipe
- 不完全椭圆积分：ellipf, ellipe_inc, ellippi

### 微积分运算（4个，需要 calculus）
- Lambda 表达式
- integrate（定积分）
- derivative（导数）
- limit（极限）

### 向量运算（5个，需要 matrix）
- vector, dot, cross, vec_norm, vec_normalize

### 矩阵运算（8个，需要 matrix）
- matrix, matmul, transpose, det, inv
- eigenvalues, trace, mat_norm

### 逻辑/关系运算（15个）
- 关系：eq, lt, gt, leq, geq, neq
- 逻辑：and, or, not
- 条件：if_then_else, piecewise
- 聚合：max, min, sum, product

## 使用示例

### 构造表达式

```rust
use equation_compiler::ast::Expr;

// 构造 sin(x) + cos(y)
let expr = Expr::add(
    Expr::sin(Expr::var("x")),
    Expr::cos(Expr::var("y")),
);

// 构造新运算符
let gcd_expr = Expr::gcd(Expr::constant(12.0), Expr::constant(8.0));
let legendre = Expr::legendre(Expr::constant(3.0), Expr::var("x"));
```

### 从 S表达式构造

```rust
use equation_compiler::sexpr::parse_to_expr;

// 从 S表达式解析
let expr = parse_to_expr("(add (sin x) (cos y))").unwrap();

// 复杂表达式
let complex = parse_to_expr("(if (gt x 0) (sqrt x) 0)").unwrap();
```

### 生成代码

```rust
// 生成 Python 代码
let python_code = expr.to_python("params");
// => "np.sin(x) + np.cos(y)"

// 生成 Rust 代码
let rust_code = expr.to_rust();
// => "x.sin() + y.cos()"

// 生成 LaTeX
let latex = expr.to_latex();
// => "\\sin(x) + \\cos(y)"
```

## 文件结构

```
ast/
├── mod.rs           # 模块入口，导出所有公共接口
├── expr.rs          # Expr 枚举定义和核心方法实现
├── visitor.rs       # 访问者模式实现
├── codegen/         # 代码生成模块
│   ├── mod.rs       # 模块导出
│   ├── python.rs    # Python 代码生成 trait
│   ├── rust.rs      # Rust 代码生成 trait
│   ├── latex.rs     # LaTeX 代码生成 trait
│   └── README.md    # 代码生成模块文档
├── constructors/    # 构造器模块（按运算符类型分类）
│   ├── mod.rs       # 模块导出
│   └── README.md    # 构造器模块文档
├── parse/           # YAML 解析模块
│   ├── mod.rs       # 模块导出
│   └── README.md    # 解析模块文档
└── README.md        # 本文档
```

## 运算符统计

- **核心运算符**: 359 个
- **YAML 入口点**: 542 个（包含别名）
- **已测试运算符**: 540 个（通过 YAML 测试覆盖）
- **覆盖率**: 99.6%（仅 e/pi 常量未覆盖）
- **测试模块数**: 52 个
- **测试方程数**: 609 个
