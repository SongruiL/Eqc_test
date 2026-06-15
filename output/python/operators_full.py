# 完整运算符测试
# 模块: OPERATORS_FULL (OperatorsFull)
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass
import numpy as np

from .params import OPERATORS_FULLParams

def test_erfinv(params: OPERATORS_FULLParams) -> float:
    """
    test_erfinv: 逆误差函数
    """
    return scipy.special.erfinv(params.p1)

def test_sinc(params: OPERATORS_FULLParams) -> float:
    """
    test_sinc: Sinc函数
    """
    return np.sinc(params.p1 / np.pi)

def test_trigamma(params: OPERATORS_FULLParams) -> float:
    """
    test_trigamma: Trigamma函数
    """
    return scipy.special.polygamma(1, params.p3)

def test_exp_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_exp_cdf: 指数分布CDF
    """
    return scipy.stats.expon.cdf(params.p1, scale=1.0/params.p5)

def test_exp_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_exp_pdf: 指数分布PDF
    """
    return scipy.stats.expon.pdf(params.p1, scale=1.0/params.p5)

def test_uniform_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_uniform_cdf: 均匀分布CDF
    """
    return scipy.stats.uniform.cdf(params.p1, loc=0, scale=1-0)

def test_uniform_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_uniform_pdf: 均匀分布PDF
    """
    return scipy.stats.uniform.pdf(params.p1, loc=0, scale=1-0)

def test_gamma_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_gamma_cdf: 伽马分布CDF
    """
    return scipy.stats.gamma.cdf(params.p1, a=params.p3, scale=1.0/params.p4)

def test_gamma_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_gamma_pdf: 伽马分布PDF
    """
    return scipy.stats.gamma.pdf(params.p1, a=params.p3, scale=1.0/params.p4)

def test_beta_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_beta_cdf: 贝塔分布CDF
    """
    return scipy.stats.beta.cdf(params.p1, a=params.p3, b=params.p4)

def test_beta_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_beta_pdf: 贝塔分布PDF
    """
    return scipy.stats.beta.pdf(params.p1, a=params.p3, b=params.p4)

def test_f_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_f_cdf: F分布CDF
    """
    return scipy.stats.f.cdf(params.p1, dfn=params.p11, dfd=params.p12)

def test_f_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_f_pdf: F分布PDF
    """
    return scipy.stats.f.pdf(params.p1, dfn=params.p11, dfd=params.p12)

def test_weibull_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_weibull_cdf: 威布尔分布CDF
    """
    return scipy.stats.weibull_min.cdf(params.p1, c=params.p3, scale=params.p5)

def test_weibull_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_weibull_pdf: 威布尔分布PDF
    """
    return scipy.stats.weibull_min.pdf(params.p1, c=params.p3, scale=params.p5)

def test_lognorm_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_lognorm_cdf: 对数正态分布CDF
    """
    return scipy.stats.lognorm.cdf(1, s=params.p7, scale=np.exp(params.p6))

def test_lognorm_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_lognorm_pdf: 对数正态分布PDF
    """
    return scipy.stats.lognorm.pdf(1, s=params.p7, scale=np.exp(params.p6))

def test_cauchy_cdf(params: OPERATORS_FULLParams) -> float:
    """
    test_cauchy_cdf: 柯西分布CDF
    """
    return scipy.stats.cauchy.cdf(params.p1, loc=params.p6, scale=params.p7)

def test_cauchy_pdf(params: OPERATORS_FULLParams) -> float:
    """
    test_cauchy_pdf: 柯西分布PDF
    """
    return scipy.stats.cauchy.pdf(params.p1, loc=params.p6, scale=params.p7)

def test_binom_pmf(params: OPERATORS_FULLParams) -> float:
    """
    test_binom_pmf: 二项分布PMF
    """
    return scipy.stats.binom.pmf(params.p9, n=params.p8, p=params.p2)

def test_geom_pmf(params: OPERATORS_FULLParams) -> float:
    """
    test_geom_pmf: 几何分布PMF
    """
    return scipy.stats.geom.pmf(params.p9, p=params.p2)

def test_hypergeom_pmf(params: OPERATORS_FULLParams) -> float:
    """
    test_hypergeom_pmf: 超几何分布PMF
    """
    return scipy.stats.hypergeom.pmf(2, M=20, n=7, N=12)

def test_neg_binom_pmf(params: OPERATORS_FULLParams) -> float:
    """
    test_neg_binom_pmf: 负二项分布PMF
    """
    return scipy.stats.nbinom.pmf(params.p9, n=5, p=params.p2)

def test_norm_ppf(params: OPERATORS_FULLParams) -> float:
    """
    test_norm_ppf: 正态分布分位数
    """
    return scipy.stats.norm.ppf(params.p2, loc=params.p6, scale=params.p7)

def test_t_ppf(params: OPERATORS_FULLParams) -> float:
    """
    test_t_ppf: t分布分位数
    """
    return scipy.stats.t.ppf(params.p2, df=params.p10)

def test_chi2_ppf(params: OPERATORS_FULLParams) -> float:
    """
    test_chi2_ppf: 卡方分布分位数
    """
    return scipy.stats.chi2.ppf(params.p2, df=params.p10)

def test_f_ppf(params: OPERATORS_FULLParams) -> float:
    """
    test_f_ppf: F分布分位数
    """
    return scipy.stats.f.ppf(params.p2, dfn=params.p11, dfd=params.p12)

def test_complex_exp(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_exp: 复数指数
    """
    return np.exp(complex(0, 3.14159))

def test_complex_ln(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_ln: 复数对数
    """
    return np.log(complex(1, 1))

def test_complex_sin(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_sin: 复数正弦
    """
    return np.sin(complex(params.p1, 0))

def test_complex_cos(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_cos: 复数余弦
    """
    return np.cos(complex(params.p1, 0))

def test_complex_tan(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_tan: 复数正切
    """
    return np.tan(complex(params.p1, 0))

def test_complex_sqrt(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_sqrt: 复数平方根
    """
    return np.sqrt(complex(-1, 0))

def test_complex_pow(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_pow: 复数幂
    """
    return np.power(complex(2, 1), complex(0.5, 0))

def test_complex_norm_sqr(params: OPERATORS_FULLParams) -> float:
    """
    test_complex_norm_sqr: 复数范数平方
    """
    return np.abs(complex(3, 4))**2
