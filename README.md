# Equation Compiler 方程编译器

将 YAML 方程定义编译为多种输出格式的 Rust 库和 CLI 工具。

## 特性

- **单一真相源**：方程只在 YAML 文件中定义一次
- **多格式输出**：Python、Rust 算子、JSON、Markdown、LaTeX
- **S表达式解析**：支持类Lisp语法的数学公式输入
- **DAG 分析**：自动构建依赖图，检测循环依赖
- **GP 友好**：AST 结构支持遗传编程操作
- **低代码集成**：可生成 lowcode 平台兼容的算子和流程定义

## 安装

### 作为库依赖

```toml
[dependencies]
equation-compiler = { git = "https://github.com/Boshenaware/equation-compiler" }
```

### 安装 CLI 工具

```bash
cargo install --git https://github.com/Boshenaware/equation-compiler --features cli
```

## 快速开始

### 作为库使用

```rust
use equation_compiler::{Compiler, GeneratorKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Compiler::new()
        .load_directory("./equations")?
        .validate()?
        .build_dag()?
        .generate(GeneratorKind::Python, "./output")?;
    
    println!("生成完成");
    Ok(())
}
```

### 使用 CLI

```bash
# 编译方程文件
eqc build --input ./equations --output ./generated --format all

# 仅验证
eqc validate ./equations

# 检查量纲一致性与跨模块耦合单位（科学正确性护栏）
eqc check-dims ./equations          # --strict 时有错误返回非零退出码

# 生成自包含 HTML 模型报告（DAG 图 + 二维公式，完全离线、浏览器直接打开）
eqc report ./equations -o model.html

# 输出 DAG 图
eqc graph ./equations --format mermaid

# 将 S表达式转换为 YAML
eqc convert "(add x (mul y 2))" -o output.eq.yaml

# 从文件转换
eqc convert input.sexpr -o output.eq.yaml

# 从带注解的 S表达式生成工作流和算子（静态注册版）
eqc workflow phenoflex.sexpr -o ./output --operators
```

### 从论文生成工作流（推荐流程）

1. **编写带注解的 S表达式文件**

```lisp
;; @module: phenoflex.core
;; @name: PhenoFlex核心模块
;; @description: 物候预测模型的核心算子

;; @operator: phenoflex.temp_kelvin
;; @name: 温度转开尔文
;; @category: 物理转换
;; @description: 将摄氏度转换为开尔文温度 TK = T + 273
;; @input: T, Number, required, 温度(摄氏度)
;; @output: TK, Number, 开尔文温度
(add T 273)

;; @operator: phenoflex.gdh
;; @name: GDH响应函数
;; @category: 热量模型
;; @description: 生长度时响应函数
;; @input: T, Number, required, 温度(摄氏度)
;; @input: Tb, Number, optional, 基础温度, 4
;; @input: Tu, Number, optional, 最适温度, 25
;; @input: Tc, Number, optional, 上限温度, 36
;; @output: gdh, Number, GDH响应值
(piecewise
  ((lt T Tb) 0)
  ((leq T Tu) (mul (div (sub Tu Tb) 2) (sub 1 (cos (mul pi (div (sub T Tb) (sub Tu Tb)))))))
  :otherwise 0)
```

2. **生成工作流和算子代码**

```bash
eqc workflow tests/sexpr_samples/phenoflex_full.sexpr \
  -o ../src/lowcode/operators/generated --operators
```

3. **将生成的代码集成到后端**

```bash
# 生成的文件:
# - phenoflex_core_workflow.json  (工作流定义)
# - phenoflex_core_operators.rs   (Rust算子实现)
# - register.rs                   (注册函数)
# - mod.rs                        (模块导出)

# 在 src/lowcode/registry/builder.rs 中添加:
use crate::lowcode::operators::generated::register_generated_operators;
register_generated_operators(&mut registry);
```

4. **将工作流导入为模板**

运行 `PostgreSQL/init/11_phenoflex_template.sql` 将工作流导入为公共模板，供所有用户使用。

## 方程定义格式

方程使用 YAML 格式定义（`.eq.yaml` 文件）：

```yaml
meta:
  id: "PHOTO"
  model: "QualiTree"
  name_cn: "光合作用"
  name_en: "Photosynthesis"

parameters:
  p1:
    name_cn: "基础光饱和光合"
    type: float
    default: 20.14
    unit: "μmol CO₂/m²/s"

variables:
  Pmax_l:
    type: intermediate
    dtype: float
    description: "动态光饱和光合速率"

equations:
  - id: "PHOTO-01"
    name: "动态Pmax"
    output: Pmax_l
    expression:
      op: add
      args:
        - { ref: p1 }
        - op: mul
          args:
            - { ref: p2 }
            - { ref: reserve_ratio }
```

## 输出格式

| 格式 | 说明 | 用途 |
|------|------|------|
| Python | 可执行 Python 代码 | 科学计算 |
| RustOperator | Rust 算子代码 | lowcode 平台 |
| WorkflowJson | 流程定义 JSON | lowcode 平台导入 |
| Markdown | DAG 文档 | 项目文档 |
| LaTeX | 数学公式 | 论文写作 |

## S表达式输入

除了 YAML 格式，编译器还支持 S表达式（S-Expression）语法，这是一种类Lisp的前缀表示法：

```lisp
;; 基础运算
(add x y)
(mul (pow x 2) (sin (div pi 2)))

;; 条件表达式
(if (gt x 0) (sqrt x) 0)

;; 求和
(sum i 1 n (pow i 2))

;; 分段函数
(piecewise
  ((lt x 0) (neg x))
  :otherwise x)
```

**优势**：
- **无歧义**：完全括号化，无需运算符优先级
- **AI友好**：LLM可以可靠生成正确的S表达式
- **易验证**：解析结果直接对应AST结构
- **完整覆盖**：支持367个运算符入口点，与YAML解析器100%对齐

详见 [src/sexpr/README.md](src/sexpr/README.md)

## 项目结构

```
equation-compiler/
├── src/
│   ├── lib.rs          # 库入口
│   ├── main.rs         # CLI 入口
│   ├── error.rs        # 错误类型
│   ├── schema/         # 数据结构定义
│   ├── ast/            # AST 节点
│   ├── parser/         # YAML 解析器
│   ├── sexpr/          # S表达式解析器（新）
│   ├── validator/      # 验证器
│   ├── dag/            # DAG 构建器
│   └── generators/     # 代码生成器
├── tests/
│   ├── sexpr_test.rs   # S表达式测试
│   └── sexpr_samples/  # 测试样例
└── examples/           # YAML 示例
```

## 支持的运算符

方程编译器支持超过 **359 个核心运算符**（542 个 YAML 入口点），**100% 覆盖 SciPy/GSL 数学库**：

### 测试覆盖
- **测试模块数**: 52 个
- **测试方程数**: 609 个
- **已测试运算符**: 540 个
- **覆盖率**: 99.6%（仅 e/pi 常量未覆盖）

### 基础运算（~40个）
- 算术运算：add, sub, mul, div, pow, mod, neg, abs, ceil, floor, round, trunc, sign
- 超越函数：exp, ln, log10, log2, sqrt, cbrt, hypot, hypot3, logn, expm1, log1p, exp2
- 三角函数：sin, cos, tan, asin, acos, atan, atan2, sec, csc, cot, asec, acsc, acot
- 双曲函数：sinh, cosh, tanh, asinh, acosh, atanh, sech, csch, coth, asech, acsch, acoth
- 其他：clamp, copysign, fma, sinc

### 特殊函数（~115个）
- 伽马函数：gamma, lgamma, digamma, polygamma, gammainc, gammaincc, rgamma, gammasgn, hyperu, gammaincinv, gammainccinv, loggamma
- 贝塔函数：beta, lbeta, betainc, betaincinv, betaincc, betainccinv
- 误差函数：erf, erfc, erfinv, erfcx, erfi, erfcinv, wofz (Faddeeva)
- Airy 扩展：airy_ai, airy_bi, airy_aie, airy_bie, airy_aip, airy_bip, itairy
- 指数积分扩展：expn, exp1, shi, chi
- Struve 积分：itstruve0, it2struve0, itmodstruve0
- Kelvin 导数：berp, beip, kerp, keip
- 数论函数：bernoulli, euler
- 贝塞尔积分：besselpoly
- Wright 扩展：log_wright_bessel
- 组合数学：factorial, factorial2, factorialk, combination, permutation, stirling2, poch
- Airy 函数：airy_ai, airy_bi
- 积分函数：fresnel_s, fresnel_c, dawson, exp_int, log_int, sin_int, cos_int
- Lambert W：lambertw, lambertw_m1
- 超几何函数：hyp0f1, hyp1f1, hyp2f1
- Kelvin 函数：kelvin_ber, kelvin_bei, kelvin_ker, kelvin_kei
- Struve 函数：struve_h, struve_l
- Hankel 函数：hankel1, hankel2, hankel1e, hankel2e
- Jacobi 椭圆：jacobi_sn, jacobi_cn, jacobi_dn
- Mathieu 函数：mathieu_a, mathieu_b, mathieu_ce, mathieu_se
- Coulomb 波函数：coulomb_f, coulomb_g
- Wigner 符号：wigner_3j, wigner_6j, wigner_9j
- Theta 函数：theta1, theta2, theta3, theta4
- 抛物柱面函数：pbdv, pbvv, pbwa
- 球扁旋转体波：pro_ang1, pro_rad1, pro_rad2, obl_ang1, obl_rad1, obl_rad2
- 修改 Fresnel：modfresnelp, modfresnelm
- Wright 函数：wright_bessel, wright_omega
- Voigt 函数：voigt
- Carlson 椭圆积分：elliprc, elliprd, elliprf, elliprg, elliprj
- Zeta 扩展：hurwitz_zeta, zetac, polylog
- Kolmogorov-Smirnov：kolmogorov, kolmogi, smirnov, smirnovi
- Dirichlet 核：diric
- Tukey lambda：tklmbda
- 其他：zeta, spherical_harmonic, spence, owens_t, riemann_siegel_z

### 机器学习/统计函数（~30个）
- Sigmoid：logit, expit (sigmoid), log_expit, softplus
- Softmax：softmax, log_softmax, logsumexp
- Box-Cox：boxcox, boxcox1p, inv_boxcox, inv_boxcox1p
- 信息论：entr, rel_entr, kl_div
- Huber 损失：huber, pseudo_huber
- 正态分布扩展：log_ndtr
- 便利函数：agm, exprel, xlogy, xlog1py, binom
- 高精度函数：cosm1, powm1, exp10, log1pmx
- 椭圆扩展：ellipkm1
- 积分组合：sici, shichi

### 度数三角函数（5个）
- 度数版本：cosdg, sindg, tandg, cotdg
- 转换函数：radian (度分秒转弧度)

### 贝塞尔函数（32个，需要 `gsl_math` feature）
- 第一类：bessel_j0, bessel_j1, bessel_jn
- 第二类：bessel_y0, bessel_y1, bessel_yn
- 修正第一类：bessel_i0, bessel_i1, bessel_in
- 修正第二类：bessel_k0, bessel_k1, bessel_kn
- 球贝塞尔：sph_bessel_j, sph_bessel_y, sph_bessel_i, sph_bessel_k
- 缩放版本：i0e, i1e, ive, k0e, k1e, kve, jve, yve
- 导数版本：jvp, yvp, ivp, kvp, h1vp, h2vp

### 概率分布（~45个，需要 `advanced_math` feature）
- 正态分布：norm_pdf, norm_cdf, norm_ppf, ndtr, ndtri
- t 分布：t_pdf, t_cdf, t_ppf, stdtr, stdtrc, stdtrit
- 卡方分布：chi2_pdf, chi2_cdf, chi2_ppf, chdtr, chdtrc, chdtri
- F 分布：f_pdf, f_cdf, f_ppf, fdtr, fdtrc, fdtri
- 泊松分布：poisson_pmf, poisson_cdf, pdtr, pdtrc, pdtri
- 二项分布：binomial_pmf, binomial_cdf, bdtr, bdtrc, bdtri
- Beta 分布：btdtr, btdtrc
- Gamma 分布：gdtr, gdtrc
- 指数分布：exponential_pdf, exponential_cdf, exp_ppf
- 其他：gamma_ppf, beta_ppf, weibull_ppf, lognorm_ppf, uniform_ppf, cauchy_ppf

### 复数运算（~16个，需要 `advanced_math` feature）
- 基础：complex, real, imag, conj, carg, cabs, polar
- 复数三角/双曲函数：complex_sinh, complex_cosh, complex_tanh, complex_asinh, complex_acosh, complex_atanh, complex_asin, complex_acos, complex_atan

### 正交多项式（14个，需要 `gsl_math` feature）
- 勒让德：legendre, legendre_assoc
- 厄米：hermite
- 拉盖尔：laguerre, laguerre_assoc
- 切比雪夫：chebyshev_t, chebyshev_u
- Gegenbauer：gegenbauer（超球多项式）
- Jacobi：jacobi_p
- 完全椭圆积分：ellipk, ellipe
- 不完全椭圆积分：ellipf, ellipe_inc, ellippi

### 数论函数（3个）
- gcd（最大公约数）
- lcm（最小公倍数）
- permutation（排列数）

### 微积分运算（4个，需要 `calculus` feature）
- Lambda 表达式
- integrate（定积分）
- derivative（导数）
- limit（极限）

### 向量/矩阵运算（13个，需要 `matrix` feature）
- 向量：vector, dot, cross, vec_norm, vec_normalize
- 矩阵：matrix, matmul, transpose, det, inv, eigenvalues, trace, mat_norm

### 逻辑/关系运算（~15个）
- 关系：eq, lt, gt, leq, geq, neq
- 逻辑：and, or, not
- 条件：if_then_else, piecewise
- 聚合：max, min, sum, product

## Features

| Feature | 说明 | 依赖 |
|---------|------|------|
| `cli` | 启用命令行工具 | clap |
| `advanced_math` | 高级数学函数 | statrs, puruspe, num-complex |
| `gsl_math` | GSL 数学函数 | GSL (需要 libgsl-dev) |
| `calculus` | 微积分运算 | peroxide |
| `matrix` | 矩阵运算 | nalgebra |
| `full` | 启用所有功能 | 上述所有 |

## 设计参考

- [Content MathML](https://www.w3.org/TR/MathML3/chapter4.html) - AST 节点结构
- [OpenMath](https://openmath.org/cd/) - 运算符命名规范
- [SBML](https://sbml.org/) - 方程文件组织结构

## License

MIT
