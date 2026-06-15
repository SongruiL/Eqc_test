# Schema 模块

本模块定义方程文件的数据模式（Schema）。

## 职责

- 定义方程文件的结构化类型
- 提供 Serde 序列化/反序列化支持
- 验证数据完整性

## 模块结构

```
schema/
├── mod.rs            # 模块入口
├── equation_file.rs  # 方程文件结构
├── equation.rs       # 方程定义结构
├── parameter.rs      # 参数定义结构
├── variable.rs       # 变量定义结构
└── README.md         # 本文档
```

## 核心类型

### `EquationFile`

方程文件的顶层结构：

```rust
pub struct EquationFile {
    pub meta: Meta,                        // 元数据
    pub parameters: HashMap<String, Parameter>,  // 参数定义
    pub variables: HashMap<String, Variable>,    // 变量定义
    pub equations: Vec<Equation>,          // 方程列表
}
```

### `Meta`

文件元数据：

```rust
pub struct Meta {
    pub id: String,           // 唯一标识符
    pub model: String,        // 模型名称
    pub name_cn: String,      // 中文名称
    pub version: String,      // 版本号
    pub description: String,  // 描述
}
```

### `Parameter`

输入参数定义：

```rust
pub struct Parameter {
    pub name_cn: String,      // 中文名称
    pub dtype: DataType,      // 数据类型
    pub default: Option<f64>, // 默认值
    pub unit: Option<String>, // 单位
    pub min: Option<f64>,     // 最小值
    pub max: Option<f64>,     // 最大值
}
```

### `Variable`

变量定义：

```rust
pub struct Variable {
    pub name_cn: String,      // 中文名称
    pub dtype: DataType,      // 数据类型
    pub var_type: VarType,    // 变量类型（output/internal）
    pub unit: Option<String>, // 单位
}
```

### `Equation`

方程定义：

```rust
pub struct Equation {
    pub id: String,           // 方程 ID
    pub name: String,         // 方程名称
    pub output: String,       // 输出变量名
    pub expression: Expr,     // 表达式 AST
}
```

## 数据类型

```rust
pub enum DataType {
    Float,    // 浮点数
    Int,      // 整数
    Bool,     // 布尔值
    Vector,   // 向量
    Matrix,   // 矩阵
    Complex,  // 复数
}
```

## 变量类型

```rust
pub enum VarType {
    Output,    // 输出变量
    Internal,  // 内部变量
    State,     // 状态变量
}
```

## 使用示例

```rust
use equation_compiler::schema::{EquationFile, Parameter, Variable};

// 创建参数
let param = Parameter {
    name_cn: "输入 x".to_string(),
    dtype: DataType::Float,
    default: Some(1.0),
    unit: None,
    min: None,
    max: None,
};

// 创建变量
let var = Variable {
    name_cn: "输出 y".to_string(),
    dtype: DataType::Float,
    var_type: VarType::Output,
    unit: None,
};
```
