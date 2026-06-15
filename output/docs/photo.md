# 光合作用 (PHOTO)

**模型**: QualiTree
**描述**: 计算叶片光合速率
**参考文献**: Lescourret 1998; Higgins 1992

## 参数

| 名称 | 中文名 | 默认值 | 单位 | 可优化 |
|------|--------|--------|------|--------|
| p4 | 量子效率 | 0.044 | μmol CO₂/μmol photon | 是 |
| p2 | 储备反馈系数 | -66.95 | μmol CO₂/m²/s | 是 |
| p1 | 基础光饱和光合 | 20.14 | μmol CO₂/m²/s | 是 |
| p3 | 暗呼吸速率 | 0.72 | μmol CO₂/m²/s | 否 |

## 变量

| 名称 | 类型 | 单位 | 描述 |
|------|------|------|------|
| reserve_ratio | Input | dimensionless | 储备/生物量比 |
| A_leaf | Output | μmol CO₂/m²/s | 叶片光合速率 |
| Pmax_l | Intermediate | μmol CO₂/m²/s | 动态光饱和光合速率 |
| ppfd | Input | μmol/m²/s | 光合有效辐射 |

## 方程

### PHOTO-01 - 动态Pmax

**输出**: `Pmax_l`

**公式**:

$$p1 + p2 \times reserve_{ratio}$$

可读形式: `Pmax_l = p1 + p2 × reserve_ratio`

**参考**: Lescourret 1998

**依赖**: reserve_ratio, p1, p2

### PHOTO-02 - Higgins光响应

**输出**: `A_leaf`

**公式**:

$$Pmax_{l} + p3 \times 1 - e^{\frac{-p4 \times ppfd}{Pmax_{l} + p3}} - p3$$

可读形式: `A = (Pmax_l + p3) × (1 - exp(-p4 × PPFD / (Pmax_l + p3))) - p3`

**参考**: Higgins 1992, Eq. 3

**依赖**: Pmax_l, ppfd, p3, p4
