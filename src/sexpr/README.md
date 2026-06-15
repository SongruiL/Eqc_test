# S表达式解析器模块

本模块提供将S表达式（S-Expression）格式的数学公式解析为AST的功能。

## 概述

S表达式是一种类Lisp的前缀表示法，具有以下优势：

- **无歧义**: 完全括号化，无需运算符优先级规则
- **易解析**: 语法简单，解析器实现直接
- **AI友好**: LLM可以可靠地生成正确的S表达式
- **人类可读**: 比YAML更紧凑，比LaTeX更规范

## 语法示例

```lisp
;; 基础运算
(add x y)
(mul (pow x 2) (sin (div pi 2)))

;; 条件表达式
(if (gt x 0) (sqrt x) 0)

;; 求和/连乘
(sum i 1 n (pow i 2))
(product k 1 10 (add k 1))

;; 分段函数
(piecewise
  ((lt x 0) (neg x))
  ((eq x 0) 0)
  :otherwise x)

;; 常量和变量
pi e 3.14159 x param_1
```

## 模块结构

```
src/sexpr/
├── mod.rs           # 模块入口，导出公共API
├── error.rs         # 错误类型定义
├── lexer.rs         # 词法分析器（字符串 -> Token流）
├── ast.rs           # S表达式AST定义
├── parser.rs        # 语法分析器（Token流 -> SExpr AST）
├── converter.rs     # AST转换器（SExpr -> Expr）
├── to_yaml.rs       # YAML序列化（Expr -> YAML）
└── README.md        # 本文档
```

## 使用方法

### 基础用法

```rust
use equation_compiler::sexpr::{parse, parse_to_expr, parse_to_yaml};

// 解析S表达式为SExpr AST
let sexpr = parse("(add x 1)")?;

// 解析并转换为Expr AST
let expr = parse_to_expr("(add x (mul y 2))")?;

// 解析并转换为YAML
let yaml = parse_to_yaml("(sin (div pi 2))")?;
```

### CLI用法

```bash
# 从S表达式转换为YAML
eqc convert "(add x (mul y 2))" -o output.eq.yaml

# 从文件转换
eqc convert input.sexpr -o output.eq.yaml

# 输出为JSON格式
eqc convert "(sin x)" --format json
```

## 支持的运算符

### 算术运算

| 运算符 | 用法 | 描述 |
|--------|------|------|
| `add` | `(add x y)` | 加法 |
| `sub` | `(sub x y)` | 减法 |
| `mul` | `(mul x y)` | 乘法 |
| `div` | `(div x y)` | 除法 |
| `neg` | `(neg x)` | 取负 |
| `pow` | `(pow x n)` | 幂运算 |
| `abs` | `(abs x)` | 绝对值 |
| `mod` | `(mod x y)` | 取模 |
| `ceil` | `(ceil x)` | 向上取整 |
| `floor` | `(floor x)` | 向下取整 |
| `round` | `(round x)` | 四舍五入 |
| `trunc` | `(trunc x)` | 截断 |
| `sign` | `(sign x)` | 符号函数 |

### 三角函数

| 运算符 | 用法 | 描述 |
|--------|------|------|
| `sin` | `(sin x)` | 正弦 |
| `cos` | `(cos x)` | 余弦 |
| `tan` | `(tan x)` | 正切 |
| `asin` | `(asin x)` | 反正弦 |
| `acos` | `(acos x)` | 反余弦 |
| `atan` | `(atan x)` | 反正切 |
| `atan2` | `(atan2 y x)` | 双参数反正切 |

### 双曲函数

| 运算符 | 用法 | 描述 |
|--------|------|------|
| `sinh` | `(sinh x)` | 双曲正弦 |
| `cosh` | `(cosh x)` | 双曲余弦 |
| `tanh` | `(tanh x)` | 双曲正切 |
| `asinh` | `(asinh x)` | 反双曲正弦 |
| `acosh` | `(acosh x)` | 反双曲余弦 |
| `atanh` | `(atanh x)` | 反双曲正切 |

### 超越函数

| 运算符 | 用法 | 描述 |
|--------|------|------|
| `exp` | `(exp x)` | 指数函数 |
| `ln` | `(ln x)` | 自然对数 |
| `log10` | `(log10 x)` | 常用对数 |
| `log2` | `(log2 x)` | 二进制对数 |
| `sqrt` | `(sqrt x)` | 平方根 |
| `cbrt` | `(cbrt x)` | 立方根 |

### 特殊函数

| 运算符 | 用法 | 描述 |
|--------|------|------|
| `gamma` | `(gamma x)` | Gamma函数 |
| `lgamma` | `(lgamma x)` | 对数Gamma函数 |
| `beta` | `(beta a b)` | Beta函数 |
| `erf` | `(erf x)` | 误差函数 |
| `erfc` | `(erfc x)` | 补误差函数 |
| `factorial` | `(factorial n)` | 阶乘 |
| `combination` | `(combination n k)` | 组合数 |
| `zeta` | `(zeta s)` | Riemann Zeta函数 |

### 关系和逻辑运算

| 运算符 | 用法 | 描述 |
|--------|------|------|
| `eq` | `(eq x y)` | 相等 |
| `lt` | `(lt x y)` | 小于 |
| `gt` | `(gt x y)` | 大于 |
| `leq` | `(leq x y)` | 小于等于 |
| `geq` | `(geq x y)` | 大于等于 |
| `neq` | `(neq x y)` | 不等于 |
| `and` | `(and a b)` | 逻辑与 |
| `or` | `(or a b)` | 逻辑或 |
| `not` | `(not x)` | 逻辑非 |

### 特殊形式

| 形式 | 语法 | 描述 |
|------|------|------|
| 条件 | `(if cond then else)` | 条件表达式 |
| 求和 | `(sum i lo hi body)` | 求和 |
| 连乘 | `(product k lo hi body)` | 连乘 |
| 分段 | `(piecewise (c1 v1) ... :otherwise v)` | 分段函数 |
| Lambda | `(lambda x body)` | Lambda表达式 |

## 错误处理

解析器提供详细的错误信息：

```
error[E001]: 未知运算符 'foobar'
  --> input.sexpr:3:5
   |
 3 |     (foobar x y)
   |      ^^^^^^ 未识别的运算符
   |
help: 相似的运算符: foo, bar, floor
```

## 工作流生成（静态注册版）

本模块支持从带注解的 S表达式文件自动生成低代码平台的工作流定义和 Rust 算子代码。

### 注解格式

在 S表达式前使用 `;;` 注释添加元数据：

```lisp
;; @module: phenoflex.core
;; @name: PhenoFlex核心模块
;; @description: 物候预测模型的核心算子

;; @operator: phenoflex.temp_kelvin
;; @type: formula
;; @name: 温度转开尔文
;; @category: 物理转换
;; @description: 将摄氏度转换为开尔文温度
;; @latex: T_K = T + 273
;; @input: T, Number, required, 温度(摄氏度)
;; @input_latex: T
;; @input_paper_ref: 原始温度输入，单位摄氏度
;; @output: TK, Number, 开尔文温度
;; @output_latex: T_K
(add T 273)
```

### 注解类型

| 注解 | 级别 | 必需 | 说明 |
|------|------|------|------|
| `@module` | 模块 | 是 | 模块ID（如 `phenoflex.core`） |
| `@name` | 模块/算子 | 是 | 显示名称 |
| `@description` | 模块/算子 | 否 | 详细描述 |
| `@operator` | 算子 | 是 | 算子ID（如 `phenoflex.temp_kelvin`） |
| `@type` | 算子 | 否 | 算子类型：`operator`/`formula`/`equation_network` |
| `@category` | 算子 | 是 | 分类（如 `物理转换`、`冷量模型`） |
| `@latex` | 算子 | 否 | 公式的 LaTeX 表示 |
| `@input` | 算子 | 是 | 输入参数定义 |
| `@input_latex` | 输入 | 否 | 输入参数的 LaTeX 名称（如 `T_K`、`\xi`） |
| `@input_paper_ref` | 输入 | 否 | 输入参数的论文引用说明 |
| `@output` | 算子 | 是 | 输出参数定义 |
| `@output_latex` | 输出 | 否 | 输出参数的 LaTeX 名称 |

### 算子类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `operator` | 运算符：单个符号代表的基础运算 | sigmoid, exp |
| `formula` | 公式：单一数学公式（默认） | Arrhenius 方程 |
| `equation_network` | 方程网络架构：多个公式组合 | GDH 分段函数 |

### 输入/输出参数格式

```
;; @input: 参数名, 类型, 必需性, 描述, 默认值(可选)
;; @input: T, Number, required, 温度(摄氏度)
;; @input: offset, Number, optional, 偏移量, 273

;; @output: 参数名, 类型, 描述
;; @output: TK, Number, 开尔文温度
```

支持的类型：`Number`、`String`、`Boolean`、`Array`

### 生成产物

```bash
eqc workflow phenoflex.sexpr -o ./output --operators
```

生成：
- `phenoflex_core_workflow.json` - 工作流 JSON 定义
- `phenoflex_core_operators.rs` - Rust 算子实现
- `register.rs` - 静态注册函数
- `mod.rs` - 模块导出

### 完整示例

参见 `tests/sexpr_samples/phenoflex_full.sexpr`（PhenoFlex 物候预测模型）。

## 测试

```bash
# 运行所有sexpr相关测试
cargo test --all-features sexpr

# 运行集成测试
cargo test --all-features --test sexpr_test
```

## 运算符覆盖

- **支持367个运算符入口点**（与YAML解析器100%对齐）
- **覆盖所有运算符类别**：
  - 基础算术（add, sub, mul, div, pow, mod 等）
  - 三角/双曲函数（sin, cos, tan, sinh, cosh, tanh 等）
  - 特殊函数（gamma, beta, erf, bessel 等）
  - 概率分布（norm, t, chi2, f, poisson, binomial 等）
  - 正交多项式（legendre, hermite, laguerre, chebyshev 等）
  - 椭圆函数（jacobi, ellipk, ellipe, carlson 等）
  - 机器学习（sigmoid, softmax, huber, boxcox 等）
  - Mathieu, Coulomb, Wigner, Airy, Struve 等高级函数
  - 所有scipy分布别名（bdtr, chdtr, fdtr 等）
  - GSL扩展函数
- **支持运算符别名**（如 `arcsin` = `asin`，`sigmoid` = `expit`）
- **测试覆盖**：100个集成测试，9个测试样例文件
