# 光合作用
# 模块: PHOTO (QualiTree)
# 自动生成的代码，请勿手动编辑

from dataclasses import dataclass
import numpy as np

from .params import PHOTOParams

def photo_01(reserve_ratio: float, params: PHOTOParams) -> float:
    """
    PHOTO-01: 动态Pmax
    
    公式: Pmax_l = p1 + p2 × reserve_ratio
    
    参考: Lescourret 1998
    """
    return (params.p1 + (params.p2 * reserve_ratio))

def photo_02(Pmax_l: float, ppfd: float, params: PHOTOParams) -> float:
    """
    PHOTO-02: Higgins光响应
    
    公式: A = (Pmax_l + p3) × (1 - exp(-p4 × PPFD / (Pmax_l + p3))) - p3
    
    参考: Higgins 1992, Eq. 3
    """
    return (((Pmax_l + params.p3) * (1 - np.exp((((-params.p4) * ppfd) / (Pmax_l + params.p3))))) - params.p3)
