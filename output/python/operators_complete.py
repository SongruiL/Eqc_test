# 完整运算符测试
# 模块: OPERATORS_COMPLETE (OperatorsComplete)
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass
import numpy as np

from .params import OPERATORS_COMPLETEParams

def test_add(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_add: 加法测试
    """
    return (params.p1 + params.p2)

def test_sub(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_sub: 减法测试
    """
    return (params.p1 - params.p2)

def test_mul(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_mul: 乘法测试
    """
    return (params.p1 * params.p2)

def test_div(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_div: 除法测试
    """
    return (params.p1 / params.p2)

def test_neg(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_neg: 取负测试
    """
    return (-params.p1)

def test_pow(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_pow: 幂运算测试
    """
    return (params.p1 ** params.p2)

def test_abs(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_abs: 绝对值测试
    """
    return np.abs((-params.p1))

def test_mod(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_mod: 取余测试
    """
    return np.mod(params.p1, params.p2)

def test_ceil(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_ceil: 向上取整测试
    """
    return np.ceil(params.p1)

def test_floor(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_floor: 向下取整测试
    """
    return np.floor(params.p1)

def test_round(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_round: 四舍五入测试
    """
    return np.round(params.p1)

def test_trunc(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_trunc: 截断取整测试
    """
    return np.trunc(params.p1)

def test_sign(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_sign: 符号函数测试
    """
    return np.sign(params.p1)

def test_fract(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_fract: 小数部分测试
    """
    return np.modf(params.p1)[0]

def test_recip(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_recip: 倒数测试
    """
    return (1.0 / params.p1)

def test_clamp(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_clamp: 截断范围测试
    """
    return np.clip(params.p1, 1, 3)

def test_exp2(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_exp2: 2的幂测试
    """
    return np.exp2(params.p2)

def test_expm1(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_expm1: 高精度指数测试
    """
    return np.expm1(params.p3)

def test_ln1p(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_ln1p: 高精度对数测试
    """
    return np.log1p(params.p3)

def test_logbase(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_logbase: 任意底对数测试
    """
    return (np.log(8) / np.log(2))

def test_hypot(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_hypot: 斜边长测试
    """
    return np.hypot(3, 4)

def test_degrees(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_degrees: 弧度转角度测试
    """
    return np.degrees(params.p5)

def test_radians(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_radians: 角度转弧度测试
    """
    return np.radians(params.p4)

def test_copysign(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_copysign: 复制符号测试
    """
    return np.copysign(params.p1, (-1))

def test_mul_add(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_mul_add: 融合乘加测试
    """
    return (params.p1 * params.p2 + params.p3)

def test_div_euclid(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_div_euclid: 欧几里得除法测试
    """
    return np.floor(7 / 3)

def test_rem_euclid(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_rem_euclid: 欧几里得取余测试
    """
    return (7 % 3)

def test_round_ties_even(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_round_ties_even: 银行家舍入测试
    """
    return np.rint(params.p1)

def test_midpoint(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_midpoint: 中点测试
    """
    return ((params.p1 + params.p2) / 2.0)

def test_sec(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_sec: 正割测试
    """
    return (1.0 / np.cos(params.p5))

def test_csc(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_csc: 余割测试
    """
    return (1.0 / np.sin(params.p5))

def test_cot(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_cot: 余切测试
    """
    return (1.0 / np.tan(params.p5))

def test_exp(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_exp: 指数函数测试
    """
    return np.exp(1)

def test_ln(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_ln: 自然对数测试
    """
    return np.log(params.p1)

def test_log10(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_log10: 常用对数测试
    """
    return np.log10(100)

def test_log2(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_log2: 二进制对数测试
    """
    return np.log2(8)

def test_sqrt(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_sqrt: 平方根测试
    """
    return np.sqrt(params.p1)

def test_cbrt(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_cbrt: 立方根测试
    """
    return np.cbrt(27)

def test_sin(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_sin: 正弦测试
    """
    return np.sin(params.p5)

def test_cos(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_cos: 余弦测试
    """
    return np.cos(params.p5)

def test_tan(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_tan: 正切测试
    """
    return np.tan(params.p5)

def test_asin(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_asin: 反正弦测试
    """
    return np.arcsin(params.p3)

def test_acos(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_acos: 反余弦测试
    """
    return np.arccos(params.p3)

def test_atan(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_atan: 反正切测试
    """
    return np.arctan(params.p1)

def test_atan2(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_atan2: 二参数反正切测试
    """
    return np.arctan2(1, 1)

def test_sinh(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_sinh: 双曲正弦测试
    """
    return np.sinh(params.p3)

def test_cosh(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_cosh: 双曲余弦测试
    """
    return np.cosh(params.p3)

def test_tanh(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_tanh: 双曲正切测试
    """
    return np.tanh(params.p3)

def test_asinh(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_asinh: 反双曲正弦测试
    """
    return np.arcsinh(params.p1)

def test_acosh(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_acosh: 反双曲余弦测试
    """
    return np.arccosh(params.p1)

def test_atanh(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_atanh: 反双曲正切测试
    """
    return np.arctanh(params.p3)

def test_max(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_max: 最大值测试
    """
    return np.max([params.p1, params.p2, params.p3])

def test_min(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_min: 最小值测试
    """
    return np.min([params.p1, params.p2, params.p3])

def test_pi(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_pi: 圆周率测试
    """
    return (np.pi * 2)

def test_e(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_e: 自然常数测试
    """
    return (np.e ** 2)

def test_comparison(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_comparison: 比较运算测试
    """
    return (1 if (params.p1 > params.p2) else 0)

def test_piecewise(params: OPERATORS_COMPLETEParams) -> float:
    """
    test_piecewise: 分段函数测试
    """
    return (0 if (params.p1 < 1) else (1 if (params.p1 < 2) else 2))
