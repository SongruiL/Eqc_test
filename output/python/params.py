# 参数定义
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass

@dataclass
class CASCADEParams:
    """级联计算演示模块参数"""
    p1: float = 0.5  # 一阶系数
    p2: float = 0.3  # 二阶系数

@dataclass
class MATH_DEMOParams:
    """数学运算演示模块参数"""
    p1: float = 2.5  # 振幅参数 [无量纲]
    p2: float = 1  # 频率参数 [Hz]
    p3: float = 0.1  # 阻尼系数 [1/s]

@dataclass
class PHOTOParams:
    """光合作用模块参数"""
    p4: float = 0.044  # 量子效率 [μmol CO₂/μmol photon]
    p2: float = -66.95  # 储备反馈系数 [μmol CO₂/m²/s]
    p1: float = 20.14  # 基础光饱和光合 [μmol CO₂/m²/s]
    p3: float = 0.72  # 暗呼吸速率 [μmol CO₂/m²/s]
