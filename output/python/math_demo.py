# 数学运算演示
# 模块: MATH_DEMO (MathDemo)
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass
import numpy as np

from .params import MATH_DEMOParams

def damped_oscillation(t: float, params: MATH_DEMOParams) -> float:
    """
    damped_oscillation: 阻尼振荡
    
    公式: y = A \cdot e^{-\lambda t} \cdot \sin(2\pi \omega t)
    """
    return (params.p1 * (np.exp((-(params.p3 * t))) * np.sin(((2 * np.pi) * (params.p2 * t)))))

def tanh_activation(x: float, params: MATH_DEMOParams) -> float:
    """
    tanh_activation: 双曲正切激活
    
    公式: y = \tanh(p_1 \cdot x)
    """
    return np.tanh((params.p1 * x))

def angle_calc(x: float, t: float, params: MATH_DEMOParams) -> float:
    """
    angle_calc: 角度计算
    
    公式: \theta = \arctan2(x, t)
    """
    return np.arctan2(x, t)

def rounding_demo(x: float, t: float, params: MATH_DEMOParams) -> float:
    """
    rounding_demo: 取整运算
    
    公式: y = \lfloor x \rfloor + \lceil t/2 \rceil
    """
    return (np.floor(x) + np.ceil((t / 2)))

def sign_abs_demo(x: float, params: MATH_DEMOParams) -> float:
    """
    sign_abs_demo: 符号与绝对值
    
    公式: y = \text{sgn}(x) \cdot \sqrt{|x|}
    """
    return (np.sign(x) * np.sqrt(np.abs(x)))

def root_log_demo(x: float, t: float, params: MATH_DEMOParams) -> float:
    """
    root_log_demo: 立方根与对数
    
    公式: y = \sqrt[3]{x} + \log_2(t + 1)
    """
    return (np.cbrt(x) + np.log2((t + 1)))

def relu_activation(x: float, params: MATH_DEMOParams) -> float:
    """
    relu_activation: ReLU激活
    
    公式: y = \max(0, x)
    """
    return (x if (x > 0) else 0)

def modulo_demo(t: float, params: MATH_DEMOParams) -> float:
    """
    modulo_demo: 取余运算
    
    公式: y = \lfloor 10t \rfloor \mod 3
    """
    return np.mod(np.floor((t * 10)), 3)

def inverse_trig(x: float, params: MATH_DEMOParams) -> float:
    """
    inverse_trig: 反三角函数
    
    公式: y = 2 \arcsin\left(\frac{x}{|x| + 1}\right)
    """
    return (2 * np.arcsin((x / (np.abs(x) + 1))))

def exp_growth(t: float, params: MATH_DEMOParams) -> float:
    """
    exp_growth: 指数增长
    
    公式: y = e^{\lambda t}
    """
    return (np.e ** (params.p3 * t))
