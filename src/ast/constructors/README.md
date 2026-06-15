# 表达式构造器模块 (constructors)

本模块按运算符类型组织 Expr 的构造方法。

## 模块结构

```
constructors/
├── mod.rs          # 模块导出
└── README.md       # 本文档
```

## 运算符分类

### 基础算术 (arithmetic)

- `add`, `sub`, `mul`, `div` - 四则运算
- `neg`, `abs` - 取负、绝对值
- `pow`, `sqrt`, `cbrt` - 幂运算和根号
- `mod`, `rem` - 取模和余数
- `floor`, `ceil`, `round`, `trunc` - 取整运算

### 三角函数 (trigonometric)

- `sin`, `cos`, `tan` - 基本三角函数
- `asin`, `acos`, `atan`, `atan2` - 反三角函数
- `sec`, `csc`, `cot` - 倒数三角函数
- `sinh`, `cosh`, `tanh` - 双曲函数
- `asinh`, `acosh`, `atanh` - 反双曲函数

### 指数对数 (exponential)

- `exp`, `exp2`, `expm1` - 指数函数
- `ln`, `log2`, `log10`, `log1p` - 对数函数
- `logn` - 任意底对数

### 特殊函数 (special)

- `gamma`, `lgamma`, `digamma`, `polygamma` - Gamma 函数
- `beta`, `lbeta`, `betainc` - Beta 函数
- `erf`, `erfc`, `erfinv`, `erfcinv` - 误差函数
- `zeta` - Riemann Zeta 函数
- `lambert_w`, `lambert_wm1` - Lambert W 函数

### Bessel 函数 (bessel)

- `bessel_j0`, `bessel_j1`, `bessel_jn` - 第一类 Bessel
- `bessel_y0`, `bessel_y1`, `bessel_yn` - 第二类 Bessel
- `bessel_i0`, `bessel_i1`, `bessel_in` - 修正第一类 Bessel
- `bessel_k0`, `bessel_k1`, `bessel_kn` - 修正第二类 Bessel
- `hankel1`, `hankel2` - Hankel 函数
- `struve_h`, `struve_l` - Struve 函数

### 正交多项式 (polynomial)

- `legendre`, `legendre_assoc` - Legendre 多项式
- `hermite` - Hermite 多项式
- `laguerre`, `laguerre_assoc` - Laguerre 多项式
- `chebyshev_t`, `chebyshev_u` - Chebyshev 多项式
- `gegenbauer` - Gegenbauer 多项式

### 椭圆函数 (elliptic)

- `ellip_k`, `ellip_e` - 完全椭圆积分
- `ellip_f`, `ellip_e_inc`, `ellip_pi` - 不完全椭圆积分
- `jacobi_sn`, `jacobi_cn`, `jacobi_dn` - Jacobi 椭圆函数

### 分布函数 (distribution)

- `norm_cdf`, `norm_ppf`, `norm_pdf` - 正态分布
- `chi2_cdf`, `chi2_ppf` - 卡方分布
- `t_cdf`, `t_ppf` - t 分布
- `f_cdf`, `f_ppf` - F 分布
- `bdtr`, `chdtr`, `fdtr`, `stdtr`, `pdtr` 等 - SciPy 风格分布

### GSL 扩展 (gsl)

- `clausen` - Clausen 函数
- `debye` - Debye 函数
- `synchrotron1`, `synchrotron2` - Synchrotron 函数
- `transport` - Transport 函数
- `fermi_dirac` - Fermi-Dirac 函数
- `airy_zero_ai`, `airy_zero_bi` - Airy 零点
- `bessel_zero_j0`, `bessel_zero_j1`, `bessel_zero_jnu` - Bessel 零点

## 使用示例

```rust
use equation_compiler::ast::Expr;

// 基础算术
let sum = Expr::add(Expr::Var("x".into()), Expr::Const(1.0));

// 三角函数
let sine = Expr::sin(Expr::Var("theta".into()));

// 特殊函数
let gamma_val = Expr::gamma(Expr::Var("n".into()));

// Bessel 函数
let j0 = Expr::bessel_j0(Expr::Var("r".into()));
```
