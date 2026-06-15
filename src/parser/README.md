# 解析器模块 (parser)

本模块负责解析 YAML 格式的方程定义文件。

## 职责

- 解析 YAML 文件到结构化数据
- 验证文件格式
- 将 YamlExpr 转换为 Expr AST

## 模块结构

```
parser/
├── mod.rs          # 模块入口
├── yaml_parser.rs  # YAML 解析实现
└── README.md       # 本文档
```

## 文件格式

方程定义文件使用 YAML 格式，扩展名为 `.eq.yaml`：

```yaml
meta:
  id: EXAMPLE_MODEL
  model: ExampleModel
  name_cn: 示例模型
  version: "1.0"
  description: 示例方程模型

parameters:
  x:
    name_cn: 输入 x
    dtype: float
    default: 1.0
  y:
    name_cn: 输入 y
    dtype: float
    default: 2.0

variables:
  result:
    name_cn: 计算结果
    dtype: float
    var_type: output

equations:
  - id: compute_result
    name: 计算结果
    output: result
    expression:
      op: add
      args:
        - { ref: x }
        - { ref: y }
```

## 核心组件

### `YamlParser`

YAML 解析器，提供：

```rust
// 解析单个文件
let file = YamlParser::parse_file("model.eq.yaml")?;

// 解析目录中的所有文件
let files = YamlParser::parse_directory("examples/")?;
```

### `YamlExpr`

YAML 表达式的中间表示，支持：

- 常量引用 `{ const: 3.14 }`
- 参数引用 `{ ref: param_name }`
- 变量引用 `{ var: var_name }`
- 运算符调用 `{ op: sin, args: [...] }`

## 错误处理

解析器会报告详细的错误信息：

- 文件不存在
- YAML 语法错误
- 缺少必需字段
- 未知运算符
- 参数数量不匹配

## 使用示例

```rust
use equation_compiler::parser::YamlParser;

// 解析方程文件
let equation_file = YamlParser::parse_file("model.eq.yaml")?;

// 访问元数据
println!("模型 ID: {}", equation_file.meta.id);

// 遍历方程
for eq in &equation_file.equations {
    println!("方程: {} -> {}", eq.id, eq.output);
}
```
