# 代码生成器模块 (generators)

本模块负责从方程文件生成各种目标输出。

## 职责

- 生成 Python 代码（使用 NumPy/SciPy）
- 生成 Rust 代码（使用 puruspe/GSL）
- 生成 LaTeX 文档
- 生成 Markdown 文档
- 生成工作流 JSON

## 模块结构

```
generators/
├── mod.rs            # 模块入口
├── python.rs         # Python 代码生成器
├── rust_operator.rs  # Rust 运算符代码生成器
├── latex.rs          # LaTeX 文档生成器
├── markdown.rs       # Markdown 文档生成器
├── workflow_json.rs  # 工作流 JSON 生成器
└── README.md         # 本文档
```

## Python 生成器

生成使用 NumPy 和 SciPy 的 Python 代码：

```python
import numpy as np
from scipy import special

def compute(params):
    y = np.sin(params.x) + np.cos(params.y)
    return y
```

特点：
- 自动导入所需库
- 参数通过命名空间访问
- 支持所有 359 个运算符

## Rust 生成器

生成 Rust 运算符实现代码：

```rust
fn compute(params: &Params) -> f64 {
    params.x.sin() + params.y.cos()
}
```

特点：
- 类型安全
- 支持条件编译（feature flags）
- 支持 GSL 高级函数

## LaTeX 生成器

生成数学公式文档：

```latex
\documentclass{article}
\begin{document}
\begin{equation}
y = \sin(x) + \cos(y)
\end{equation}
\end{document}
```

特点：
- 完整的 LaTeX 文档结构
- 支持希腊字母和特殊符号
- 自动处理分数和上下标

## Markdown 生成器

生成人类可读的方程文档：

```markdown
## 方程: compute_y

**描述**: 计算 y 值

**公式**:
$$y = \sin(x) + \cos(y)$$

**参数**:
- `x`: 输入 x 值
- `y`: 输入 y 值
```

## 工作流 JSON 生成器

生成用于工作流系统的 JSON 配置：

```json
{
  "equations": [
    {
      "id": "compute_y",
      "inputs": ["x", "y"],
      "output": "y",
      "dependencies": []
    }
  ]
}
```

## 使用示例

```rust
use equation_compiler::generators::{PythonGenerator, LatexGenerator};

// 生成 Python 代码
let python_code = PythonGenerator::generate(&equation_file)?;

// 生成 LaTeX 文档
let latex_doc = LatexGenerator::generate(&equation_file)?;
```
