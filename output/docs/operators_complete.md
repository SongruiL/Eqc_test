# 完整运算符测试 (OPERATORS_COMPLETE)

**模型**: OperatorsComplete
**描述**: 测试所有支持的数学运算符

## 参数

| 名称 | 中文名 | 默认值 | 单位 | 可优化 |
|------|--------|--------|------|--------|
| p5 | 弧度参数 | 0.7854 | - | 是 |
| p1 | 测试参数1 | 2.5 | - | 是 |
| p3 | 测试参数3 | 0.5 | - | 是 |
| p2 | 测试参数2 | 1.5 | - | 是 |
| p4 | 角度参数 | 45 | - | 是 |

## 变量

| 名称 | 类型 | 单位 | 描述 |
|------|------|------|------|
| x | Intermediate | - | - |
| y | Intermediate | - | - |

## 方程

### test_add - 加法测试

**输出**: `y_add`

**公式**: $p1 + p2$

**依赖**: p1, p2

### test_sub - 减法测试

**输出**: `y_sub`

**公式**: $p1 - p2$

**依赖**: p1, p2

### test_mul - 乘法测试

**输出**: `y_mul`

**公式**: $p1 \times p2$

**依赖**: p1, p2

### test_div - 除法测试

**输出**: `y_div`

**公式**: $\frac{p1}{p2}$

**依赖**: p1, p2

### test_neg - 取负测试

**输出**: `y_neg`

**公式**: $-p1$

**依赖**: p1

### test_pow - 幂运算测试

**输出**: `y_pow`

**公式**: $p1^{p2}$

**依赖**: p1, p2

### test_abs - 绝对值测试

**输出**: `y_abs`

**公式**: $|-p1|$

**依赖**: p1

### test_mod - 取余测试

**输出**: `y_mod`

**公式**: $p1 \mod p2$

**依赖**: p1, p2

### test_ceil - 向上取整测试

**输出**: `y_ceil`

**公式**: $\lceil p1 \rceil$

**依赖**: p1

### test_floor - 向下取整测试

**输出**: `y_floor`

**公式**: $\lfloor p1 \rfloor$

**依赖**: p1

### test_round - 四舍五入测试

**输出**: `y_round`

**公式**: $\text{round}(p1)$

**依赖**: p1

### test_trunc - 截断取整测试

**输出**: `y_trunc`

**公式**: $\text{trunc}(p1)$

**依赖**: p1

### test_sign - 符号函数测试

**输出**: `y_sign`

**公式**: $\text{sgn}(p1)$

**依赖**: p1

### test_fract - 小数部分测试

**输出**: `y_fract`

**公式**: $\text{frac}(p1)$

**依赖**: p1

### test_recip - 倒数测试

**输出**: `y_recip`

**公式**: $\frac{1}{p1}$

**依赖**: p1

### test_clamp - 截断范围测试

**输出**: `y_clamp`

**公式**: $\text{clamp}(p1, 1, 3)$

**依赖**: p1

### test_exp2 - 2的幂测试

**输出**: `y_exp2`

**公式**: $2^{p2}$

**依赖**: p2

### test_expm1 - 高精度指数测试

**输出**: `y_expm1`

**公式**: $(e^{p3} - 1)$

**依赖**: p3

### test_ln1p - 高精度对数测试

**输出**: `y_ln1p`

**公式**: $\ln(1 + p3)$

**依赖**: p3

### test_logbase - 任意底对数测试

**输出**: `y_logbase`

**公式**: $\log_{2}{8}$

### test_hypot - 斜边长测试

**输出**: `y_hypot`

**公式**: $\sqrt{3^2 + 4^2}$

### test_degrees - 弧度转角度测试

**输出**: `y_degrees`

**公式**: $p5^\circ$

**依赖**: p5

### test_radians - 角度转弧度测试

**输出**: `y_radians`

**公式**: $\text{rad}(p4)$

**依赖**: p4

### test_copysign - 复制符号测试

**输出**: `y_copysign`

**公式**: $\text{copysign}(p1, -1)$

**依赖**: p1

### test_mul_add - 融合乘加测试

**输出**: `y_mul_add`

**公式**: $(p1 \times p2 + p3)$

**依赖**: p1, p2, p3

### test_div_euclid - 欧几里得除法测试

**输出**: `y_div_euclid`

**公式**: $\lfloor 7 / 3 \rfloor$

### test_rem_euclid - 欧几里得取余测试

**输出**: `y_rem_euclid`

**公式**: $7 \mod 3$

### test_round_ties_even - 银行家舍入测试

**输出**: `y_round_even`

**公式**: $\text{round}_{even}(p1)$

**依赖**: p1

### test_midpoint - 中点测试

**输出**: `y_midpoint`

**公式**: $\frac{p1 + p2}{2}$

**依赖**: p1, p2

### test_sec - 正割测试

**输出**: `y_sec`

**公式**: $\sec(p5)$

**依赖**: p5

### test_csc - 余割测试

**输出**: `y_csc`

**公式**: $\csc(p5)$

**依赖**: p5

### test_cot - 余切测试

**输出**: `y_cot`

**公式**: $\cot(p5)$

**依赖**: p5

### test_exp - 指数函数测试

**输出**: `y_exp`

**公式**: $e^{1}$

### test_ln - 自然对数测试

**输出**: `y_ln`

**公式**: $\ln(p1)$

**依赖**: p1

### test_log10 - 常用对数测试

**输出**: `y_log10`

**公式**: $\log_{10}(100)$

### test_log2 - 二进制对数测试

**输出**: `y_log2`

**公式**: $\log_{2}(8)$

### test_sqrt - 平方根测试

**输出**: `y_sqrt`

**公式**: $\sqrt{p1}$

**依赖**: p1

### test_cbrt - 立方根测试

**输出**: `y_cbrt`

**公式**: $\sqrt[3]{27}$

### test_sin - 正弦测试

**输出**: `y_sin`

**公式**: $\sin(p5)$

**依赖**: p5

### test_cos - 余弦测试

**输出**: `y_cos`

**公式**: $\cos(p5)$

**依赖**: p5

### test_tan - 正切测试

**输出**: `y_tan`

**公式**: $\tan(p5)$

**依赖**: p5

### test_asin - 反正弦测试

**输出**: `y_asin`

**公式**: $\arcsin(p3)$

**依赖**: p3

### test_acos - 反余弦测试

**输出**: `y_acos`

**公式**: $\arccos(p3)$

**依赖**: p3

### test_atan - 反正切测试

**输出**: `y_atan`

**公式**: $\arctan(p1)$

**依赖**: p1

### test_atan2 - 二参数反正切测试

**输出**: `y_atan2`

**公式**: $\text{atan2}(1, 1)$

### test_sinh - 双曲正弦测试

**输出**: `y_sinh`

**公式**: $\sinh(p3)$

**依赖**: p3

### test_cosh - 双曲余弦测试

**输出**: `y_cosh`

**公式**: $\cosh(p3)$

**依赖**: p3

### test_tanh - 双曲正切测试

**输出**: `y_tanh`

**公式**: $\tanh(p3)$

**依赖**: p3

### test_asinh - 反双曲正弦测试

**输出**: `y_asinh`

**公式**: $\text{asinh}(p1)$

**依赖**: p1

### test_acosh - 反双曲余弦测试

**输出**: `y_acosh`

**公式**: $\text{acosh}(p1)$

**依赖**: p1

### test_atanh - 反双曲正切测试

**输出**: `y_atanh`

**公式**: $\text{atanh}(p3)$

**依赖**: p3

### test_max - 最大值测试

**输出**: `y_max`

**公式**: $\max(p1, p2, p3)$

**依赖**: p1, p2, p3

### test_min - 最小值测试

**输出**: `y_min`

**公式**: $\min(p1, p2, p3)$

**依赖**: p1, p2, p3

### test_pi - 圆周率测试

**输出**: `y_pi`

**公式**: $\pi \times 2$

### test_e - 自然常数测试

**输出**: `y_e`

**公式**: $e^{2}$

### test_comparison - 比较运算测试

**输出**: `y_comp`

**公式**: $\begin{cases} 1 & \text{if } p1 > p2 \\ 0 & \text{otherwise} \end{cases}$

**依赖**: p1, p2

### test_piecewise - 分段函数测试

**输出**: `y_piecewise`

**公式**: $\begin{cases} 0 & \text{if } p1 < 1 \\ 1 & \text{if } p1 < 2 \\ 2 & \text{otherwise} \end{cases}$

**依赖**: p1
