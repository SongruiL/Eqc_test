# 高级运算符测试 (OPERATORS_ADVANCED)

**模型**: OperatorsAdvanced
**描述**: 测试特殊函数、概率分布、复数运算等高级数学运算符

## 参数

| 名称 | 中文名 | 默认值 | 单位 | 可优化 |
|------|--------|--------|------|--------|
| p4 | 整数参数 | 5 | - | 是 |
| p5 | 阶数参数 | 2 | - | 是 |
| p2 | 测试参数2 | 1.5 | - | 是 |
| p3 | 测试参数3 | 0.5 | - | 是 |
| p1 | 测试参数1 | 2.5 | - | 是 |

## 变量

| 名称 | 类型 | 单位 | 描述 |
|------|------|------|------|
| y | Intermediate | - | - |
| x | Intermediate | - | - |

## 方程

### test_gamma - Gamma函数测试

**输出**: `y_gamma`

**公式**: $\Gamma(p1)$

**依赖**: p1

### test_loggamma - LogGamma函数测试

**输出**: `y_loggamma`

**公式**: $\ln\Gamma(p1)$

**依赖**: p1

### test_beta - Beta函数测试

**输出**: `y_beta`

**公式**: $\mathrm{B}(p1, p2)$

**依赖**: p1, p2

### test_betainc - 不完全Beta函数测试

**输出**: `y_betainc`

**公式**: $I_{p3}(p1, p2)$

**依赖**: p1, p2, p3

### test_erf - 误差函数测试

**输出**: `y_erf`

**公式**: $\mathrm{erf}(p3)$

**依赖**: p3

### test_erfc - 补误差函数测试

**输出**: `y_erfc`

**公式**: $\mathrm{erfc}(p3)$

**依赖**: p3

### test_besselj - 第一类贝塞尔函数测试

**输出**: `y_besselj`

**公式**: $J_{p5}(p1)$

**依赖**: p5, p1

### test_bessely - 第二类贝塞尔函数测试

**输出**: `y_bessely`

**公式**: $Y_{p5}(p1)$

**依赖**: p5, p1

### test_besseli - 修正贝塞尔第一类测试

**输出**: `y_besseli`

**公式**: $I_{p5}(p1)$

**依赖**: p5, p1

### test_besselk - 修正贝塞尔第二类测试

**输出**: `y_besselk`

**公式**: $K_{p5}(p1)$

**依赖**: p5, p1

### test_digamma - Digamma函数测试

**输出**: `y_digamma`

**公式**: $\psi(p1)$

**依赖**: p1

### test_factorial - 阶乘测试

**输出**: `y_factorial`

**公式**: $p4!$

**依赖**: p4

### test_norm_cdf - 正态分布CDF测试

**输出**: `y_norm_cdf`

**公式**: $\Phi\left(\frac{0 - 0}{1}\right)$

### test_norm_pdf - 正态分布PDF测试

**输出**: `y_norm_pdf`

**公式**: $\frac{1}{1\sqrt{2\pi}} e^{-\frac{(0 - 0)^2}{21^2}}$

### test_chi2_cdf - 卡方分布CDF测试

**输出**: `y_chi2_cdf`

**公式**: $F_{\chi^2_{p4}}(p1)$

**依赖**: p1, p4

### test_t_cdf - t分布CDF测试

**输出**: `y_t_cdf`

**公式**: $F_{t_{p4}}(1)$

**依赖**: p4

### test_poisson_pmf - 泊松分布PMF测试

**输出**: `y_poisson`

**公式**: $\frac{p1^{3} e^{-p1}}{3!}$

**依赖**: p1

### test_binomial - 组合数测试

**输出**: `y_binomial`

**公式**: $\binom{10}{3}$

### test_complex - 复数构造测试

**输出**: `y_complex`

**公式**: $p1 + p2i$

**依赖**: p1, p2

### test_complex_real - 复数实部测试

**输出**: `y_real`

**公式**: $\Re(p1 + p2i)$

**依赖**: p1, p2

### test_complex_imag - 复数虚部测试

**输出**: `y_imag`

**公式**: $\Im(p1 + p2i)$

**依赖**: p1, p2

### test_combined_special - 组合特殊函数测试

**输出**: `y_combined`

**公式**: $\Gamma(p1) + \mathrm{erf}(p3)$

**依赖**: p1, p3
