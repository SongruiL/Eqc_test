# 高级运算符测试
# 模块: OPERATORS_ADVANCED (OperatorsAdvanced)
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass
import numpy as np

from .params import OPERATORS_ADVANCEDParams

def test_gamma(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_gamma: Gamma函数测试
    """
    return scipy.special.gamma(params.p1)

def test_loggamma(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_loggamma: LogGamma函数测试
    """
    return scipy.special.loggamma(params.p1)

def test_beta(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_beta: Beta函数测试
    """
    return scipy.special.beta(params.p1, params.p2)

def test_betainc(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_betainc: 不完全Beta函数测试
    """
    return scipy.special.betainc(params.p1, params.p2, params.p3)

def test_erf(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_erf: 误差函数测试
    """
    return scipy.special.erf(params.p3)

def test_erfc(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_erfc: 补误差函数测试
    """
    return scipy.special.erfc(params.p3)

def test_besselj(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_besselj: 第一类贝塞尔函数测试
    """
    return scipy.special.jv(params.p5, params.p1)

def test_bessely(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_bessely: 第二类贝塞尔函数测试
    """
    return scipy.special.yv(params.p5, params.p1)

def test_besseli(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_besseli: 修正贝塞尔第一类测试
    """
    return scipy.special.iv(params.p5, params.p1)

def test_besselk(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_besselk: 修正贝塞尔第二类测试
    """
    return scipy.special.kv(params.p5, params.p1)

def test_digamma(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_digamma: Digamma函数测试
    """
    return scipy.special.digamma(params.p1)

def test_factorial(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_factorial: 阶乘测试
    """
    return scipy.special.factorial(params.p4)

def test_norm_cdf(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_norm_cdf: 正态分布CDF测试
    """
    return scipy.stats.norm.cdf(0, loc=0, scale=1)

def test_norm_pdf(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_norm_pdf: 正态分布PDF测试
    """
    return scipy.stats.norm.pdf(0, loc=0, scale=1)

def test_chi2_cdf(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_chi2_cdf: 卡方分布CDF测试
    """
    return scipy.stats.chi2.cdf(params.p1, params.p4)

def test_t_cdf(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_t_cdf: t分布CDF测试
    """
    return scipy.stats.t.cdf(1, params.p4)

def test_poisson_pmf(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_poisson_pmf: 泊松分布PMF测试
    """
    return scipy.stats.poisson.pmf(3, params.p1)

def test_binomial(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_binomial: 组合数测试
    """
    return scipy.special.comb(10, 3)

def test_complex(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_complex: 复数构造测试
    """
    return complex(params.p1, params.p2)

def test_complex_real(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_complex_real: 复数实部测试
    """
    return np.real(complex(params.p1, params.p2))

def test_complex_imag(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_complex_imag: 复数虚部测试
    """
    return np.imag(complex(params.p1, params.p2))

def test_combined_special(params: OPERATORS_ADVANCEDParams) -> float:
    """
    test_combined_special: 组合特殊函数测试
    """
    return (scipy.special.gamma(params.p1) + scipy.special.erf(params.p3))
