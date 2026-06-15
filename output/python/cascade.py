# 级联计算演示
# 模块: CASCADE (CascadeDemo)
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass
import numpy as np

from .params import CASCADEParams

def stage1(x: float, params: CASCADEParams) -> float:
    """
    stage1: 一阶变换
    """
    return (params.p1 * x)

def stage2(y1: float, x: float, params: CASCADEParams) -> float:
    """
    stage2: 二阶变换
    """
    return (y1 + (params.p2 * x))

def stage3(y2: float, params: CASCADEParams) -> float:
    """
    stage3: 最终输出
    """
    return np.sqrt(np.abs(y2))
