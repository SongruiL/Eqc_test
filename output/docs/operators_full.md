# 完整运算符测试 (OPERATORS_FULL)

**模型**: OperatorsFull
**描述**: 测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展

## 参数

| 名称 | 中文名 | 默认值 | 单位 | 可优化 |
|------|--------|--------|------|--------|
| p2 | 概率p | 0.5 | - | 是 |
| p7 | 标准差σ | 1 | - | 是 |
| p4 | 参数β | 3 | - | 是 |
| p9 | 成功次数 | 3 | - | 是 |
| p3 | 参数α | 2 | - | 是 |
| p12 | 自由度2 | 10 | - | 是 |
| p6 | 均值μ | 0 | - | 是 |
| p8 | 试验次数 | 10 | - | 是 |
| p1 | 输入x | 0.5 | - | 是 |
| p11 | 自由度1 | 5 | - | 是 |
| p5 | 参数λ | 1 | - | 是 |
| p10 | 自由度 | 5 | - | 是 |

## 变量

| 名称 | 类型 | 单位 | 描述 |
|------|------|------|------|
| y | Intermediate | - | - |

## 方程

### test_erfinv - 逆误差函数

**输出**: `y_erfinv`

**公式**: $\mathrm{erf}^{-1}(p1)$

**依赖**: p1

### test_sinc - Sinc函数

**输出**: `y_sinc`

**公式**: $\mathrm{sinc}(p1)$

**依赖**: p1

### test_trigamma - Trigamma函数

**输出**: `y_trigamma`

**公式**: $\psi'(p3)$

**依赖**: p3

### test_exp_cdf - 指数分布CDF

**输出**: `y_exp_cdf`

**公式**: $F_{\mathrm{Exp}(p5)}(p1)$

**依赖**: p1, p5

### test_exp_pdf - 指数分布PDF

**输出**: `y_exp_pdf`

**公式**: $f_{\mathrm{Exp}(p5)}(p1)$

**依赖**: p1, p5

### test_uniform_cdf - 均匀分布CDF

**输出**: `y_uniform_cdf`

**公式**: $F_{U(0,1)}(p1)$

**依赖**: p1

### test_uniform_pdf - 均匀分布PDF

**输出**: `y_uniform_pdf`

**公式**: $f_{U(0,1)}(p1)$

**依赖**: p1

### test_gamma_cdf - 伽马分布CDF

**输出**: `y_gamma_cdf`

**公式**: $F_{\Gamma(p3,p4)}(p1)$

**依赖**: p1, p3, p4

### test_gamma_pdf - 伽马分布PDF

**输出**: `y_gamma_pdf`

**公式**: $f_{\Gamma(p3,p4)}(p1)$

**依赖**: p1, p3, p4

### test_beta_cdf - 贝塔分布CDF

**输出**: `y_beta_cdf`

**公式**: $F_{\mathrm{Beta}(p3,p4)}(p1)$

**依赖**: p1, p3, p4

### test_beta_pdf - 贝塔分布PDF

**输出**: `y_beta_pdf`

**公式**: $f_{\mathrm{Beta}(p3,p4)}(p1)$

**依赖**: p1, p3, p4

### test_f_cdf - F分布CDF

**输出**: `y_f_cdf`

**公式**: $F_{F(p11,p12)}(p1)$

**依赖**: p1, p11, p12

### test_f_pdf - F分布PDF

**输出**: `y_f_pdf`

**公式**: $f_{F(p11,p12)}(p1)$

**依赖**: p1, p11, p12

### test_weibull_cdf - 威布尔分布CDF

**输出**: `y_weibull_cdf`

**公式**: $F_{W(p3,p5)}(p1)$

**依赖**: p1, p3, p5

### test_weibull_pdf - 威布尔分布PDF

**输出**: `y_weibull_pdf`

**公式**: $f_{W(p3,p5)}(p1)$

**依赖**: p1, p3, p5

### test_lognorm_cdf - 对数正态分布CDF

**输出**: `y_lognorm_cdf`

**公式**: $F_{\mathrm{LN}(p6,p7)}(1)$

**依赖**: p6, p7

### test_lognorm_pdf - 对数正态分布PDF

**输出**: `y_lognorm_pdf`

**公式**: $f_{\mathrm{LN}(p6,p7)}(1)$

**依赖**: p6, p7

### test_cauchy_cdf - 柯西分布CDF

**输出**: `y_cauchy_cdf`

**公式**: $F_{C(p6,p7)}(p1)$

**依赖**: p1, p6, p7

### test_cauchy_pdf - 柯西分布PDF

**输出**: `y_cauchy_pdf`

**公式**: $f_{C(p6,p7)}(p1)$

**依赖**: p1, p6, p7

### test_binom_pmf - 二项分布PMF

**输出**: `y_binom_pmf`

**公式**: $P(X=p9 | n=p8, p=p2)$

**依赖**: p9, p8, p2

### test_geom_pmf - 几何分布PMF

**输出**: `y_geom_pmf`

**公式**: $P(X=p9 | p=p2)$

**依赖**: p9, p2

### test_hypergeom_pmf - 超几何分布PMF

**输出**: `y_hypergeom_pmf`

**公式**: $P(X=2 | N=20, K=7, n=12)$

### test_neg_binom_pmf - 负二项分布PMF

**输出**: `y_neg_binom_pmf`

**公式**: $P(X=p9 | r=5, p=p2)$

**依赖**: p9, p2

### test_norm_ppf - 正态分布分位数

**输出**: `y_norm_ppf`

**公式**: $\Phi^{-1}_{(p6,p7)}(p2)$

**依赖**: p2, p6, p7

### test_t_ppf - t分布分位数

**输出**: `y_t_ppf`

**公式**: $t^{-1}_{p10}(p2)$

**依赖**: p2, p10

### test_chi2_ppf - 卡方分布分位数

**输出**: `y_chi2_ppf`

**公式**: $\chi^{2,-1}_{p10}(p2)$

**依赖**: p2, p10

### test_f_ppf - F分布分位数

**输出**: `y_f_ppf`

**公式**: $F^{-1}_{(p11,p12)}(p2)$

**依赖**: p2, p11, p12

### test_complex_exp - 复数指数

**输出**: `y_complex_exp`

**公式**: $e^{0 + 3.14159i}$

### test_complex_ln - 复数对数

**输出**: `y_complex_ln`

**公式**: $\ln(1 + 1i)$

### test_complex_sin - 复数正弦

**输出**: `y_complex_sin`

**公式**: $\sin(p1 + 0i)$

**依赖**: p1

### test_complex_cos - 复数余弦

**输出**: `y_complex_cos`

**公式**: $\cos(p1 + 0i)$

**依赖**: p1

### test_complex_tan - 复数正切

**输出**: `y_complex_tan`

**公式**: $\tan(p1 + 0i)$

**依赖**: p1

### test_complex_sqrt - 复数平方根

**输出**: `y_complex_sqrt`

**公式**: $\sqrt{-1 + 0i}$

### test_complex_pow - 复数幂

**输出**: `y_complex_pow`

**公式**: $(2 + 1i)^{0.5 + 0i}$

### test_complex_norm_sqr - 复数范数平方

**输出**: `y_complex_norm_sqr`

**公式**: $|3 + 4i|^2$
