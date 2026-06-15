# 数学运算演示 (MATH_DEMO)

**模型**: MathDemo
**描述**: 演示 equation-compiler 支持的各种数学运算符

## 参数

| 名称 | 中文名 | 默认值 | 单位 | 可优化 |
|------|--------|--------|------|--------|
| p1 | 振幅参数 | 2.5 | 无量纲 | 是 |
| p2 | 频率参数 | 1 | Hz | 是 |
| p3 | 阻尼系数 | 0.1 | 1/s | 是 |

## 变量

| 名称 | 类型 | 单位 | 描述 |
|------|------|------|------|
| t | Input | s | 时间变量 |
| x | Input | m | 空间位置 |

## 方程

### damped_oscillation - 阻尼振荡

**输出**: `y_damped`

**公式**:

$$p1 \times e^{-p3 \times t} \times \sin(2 \times \pi \times p2 \times t)$$

可读形式: `y = A \cdot e^{-\lambda t} \cdot \sin(2\pi \omega t)`

**依赖**: t, p1, p3, p2

### tanh_activation - 双曲正切激活

**输出**: `y_tanh`

**公式**:

$$\tanh(p1 \times x)$$

可读形式: `y = \tanh(p_1 \cdot x)`

**依赖**: x, p1

### angle_calc - 角度计算

**输出**: `theta`

**公式**:

$$\text{atan2}(x, t)$$

可读形式: `\theta = \arctan2(x, t)`

**依赖**: x, t

### rounding_demo - 取整运算

**输出**: `y_round`

**公式**:

$$\lfloor x \rfloor + \lceil \frac{t}{2} \rceil$$

可读形式: `y = \lfloor x \rfloor + \lceil t/2 \rceil`

**依赖**: x, t

### sign_abs_demo - 符号与绝对值

**输出**: `y_sign`

**公式**:

$$\text{sgn}(x) \times \sqrt{|x|}$$

可读形式: `y = \text{sgn}(x) \cdot \sqrt{|x|}`

**依赖**: x

### root_log_demo - 立方根与对数

**输出**: `y_log`

**公式**:

$$\sqrt[3]{x} + \log_{2}(t + 1)$$

可读形式: `y = \sqrt[3]{x} + \log_2(t + 1)`

**依赖**: x, t

### relu_activation - ReLU激活

**输出**: `y_relu`

**公式**:

$$\begin{cases} x & \text{if } x > 0 \\ 0 & \text{otherwise} \end{cases}$$

可读形式: `y = \max(0, x)`

**依赖**: x

### modulo_demo - 取余运算

**输出**: `y_mod`

**公式**:

$$\lfloor t \times 10 \rfloor \mod 3$$

可读形式: `y = \lfloor 10t \rfloor \mod 3`

**依赖**: t

### inverse_trig - 反三角函数

**输出**: `y_asin`

**公式**:

$$2 \times \arcsin(\frac{x}{|x| + 1})$$

可读形式: `y = 2 \arcsin\left(\frac{x}{|x| + 1}\right)`

**依赖**: x

### exp_growth - 指数增长

**输出**: `y_exp`

**公式**:

$$e^{p3 \times t}$$

可读形式: `y = e^{\lambda t}`

**依赖**: t, p3
