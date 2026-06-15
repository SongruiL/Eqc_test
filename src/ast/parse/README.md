# YAML 解析模块 (parse)

本模块负责将 YAML 格式的方程定义解析为 Expr AST。

## 模块结构

```
parse/
├── mod.rs          # 模块导出
└── README.md       # 本文档
```

## YAML 表达式格式

### 基本结构

```yaml
expression:
  op: 运算符名称
  args:
    - { ref: 参数名 }      # 引用参数
    - { var: 变量名 }      # 引用变量
    - 嵌套表达式           # 递归表达式
```

### 常量和引用

```yaml
# 引用参数
{ ref: p1 }

# 引用变量
{ var: x }

# 常量值
{ const: 3.14 }
```

### 运算符调用

```yaml
# 一元运算符
expression:
  op: sin
  args:
    - { ref: theta }

# 二元运算符
expression:
  op: add
  args:
    - { ref: a }
    - { ref: b }

# 三元运算符
expression:
  op: clamp
  args:
    - { ref: x }
    - { ref: min }
    - { ref: max }
```

## 支持的运算符

当前支持 **542 个运算符入口点**（包含别名），对应 **359 个核心运算符**。

### 别名示例

| 主名称 | 别名 |
|--------|------|
| `asin` | `arcsin` |
| `lgamma` | `gammaln` |
| `lbeta` | `betaln` |
| `norm_cdf` | `ndtr` |
| `norm_ppf` | `ndtri` |
| `bessel_jn` | `jv` |

## 错误处理

解析器会报告以下错误：

- 未知运算符
- 参数数量不匹配
- 无效的表达式格式

## 扩展指南

添加新运算符的 YAML 支持：

1. 在 `expr.rs` 的 `YamlExpr::into_expr()` 方法中添加运算符映射
2. 使用 `Self::unary_op`、`Self::binary_op` 或 `Self::ternary_op` 辅助函数
3. 可以添加多个别名映射到同一运算符

```rust
"new_op" | "new_op_alias" => Self::unary_op(args, Expr::new_op, "new_op"),
```
