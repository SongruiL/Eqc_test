# 代码生成模块 (codegen)

本模块负责将 Expr AST 转换为不同目标语言的代码。

## 模块结构

```
codegen/
├── mod.rs          # 模块导出
├── python.rs       # Python 代码生成 (NumPy/SciPy)
├── rust.rs         # Rust 代码生成
├── latex.rs        # LaTeX 数学公式生成
└── README.md       # 本文档
```

## 支持的目标语言

### Python

- 使用 NumPy (`np.*`) 进行数值计算
- 使用 SciPy (`scipy.special.*`) 进行特殊函数计算
- 参数通过 `params_prefix` 访问（如 `params.x`）

### Rust

- 使用 `std::f64` 标准库函数
- 使用 `puruspe` 和 `scirs2_special` 进行特殊函数计算
- 使用 `GSL` 进行高级数学运算（需要启用 `gsl_math` feature）

### LaTeX

- 生成标准数学公式排版
- 支持希腊字母、分数、上下标等
- 可直接嵌入 LaTeX 文档

## Trait 接口

```rust
pub trait ToPython {
    fn to_python(&self, params_prefix: &str) -> String;
}

pub trait ToRust {
    fn to_rust(&self) -> String;
}

pub trait ToLatex {
    fn to_latex(&self) -> String;
}
```

## 使用示例

```rust
use equation_compiler::ast::{Expr, ToPython, ToRust, ToLatex};

let expr = Expr::add(Expr::Var("x".into()), Expr::Const(1.0));

println!("Python: {}", expr.to_python("params"));  // (x + 1)
println!("Rust: {}", expr.to_rust());              // (x + 1.0)
println!("LaTeX: {}", expr.to_latex());            // x + 1
```

## 扩展指南

添加新运算符的代码生成支持：

1. 在 `expr.rs` 的 `to_python()` 方法中添加对应的 match 分支
2. 在 `expr.rs` 的 `to_rust()` 方法中添加对应的 match 分支
3. 在 `expr.rs` 的 `to_latex()` 方法中添加对应的 match 分支
4. 确保使用正确的库函数和格式
