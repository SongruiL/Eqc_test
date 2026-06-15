# 验证器模块 (validator)

本模块负责验证方程文件的语义正确性。

## 职责

- 检查引用完整性
- 检测循环依赖
- 验证类型一致性

## 模块结构

```
validator/
├── mod.rs               # 模块入口
├── reference_checker.rs # 引用检查器
├── cycle_detector.rs    # 循环依赖检测器
├── type_checker.rs      # 类型检查器
└── README.md            # 本文档
```

## 验证器组件

### `ReferenceChecker`

检查所有引用的有效性：

- 参数引用必须在 `parameters` 中定义
- 变量引用必须在 `variables` 中定义
- 方程输出变量必须存在

错误示例：
```
未定义的引用: 变量 'undefined_var' 在 方程 eq1
```

### `CycleDetector`

检测方程之间的循环依赖：

- 使用深度优先搜索（DFS）
- 报告循环路径

错误示例：
```
循环依赖: eq1 -> eq2 -> eq3 -> eq1
```

### `TypeChecker`

验证类型一致性：

- 检查运算符参数类型
- 验证表达式结果类型
- 检查类型转换

错误示例：
```
类型错误: 方程 eq1 的表达式返回 Vector，但输出变量 y 期望 Float
```

## 使用示例

```rust
use equation_compiler::validator::{Validator, ValidationResult};

// 创建验证器
let validator = Validator::new();

// 验证方程文件
match validator.validate(&equation_file) {
    Ok(()) => println!("验证通过"),
    Err(errors) => {
        for error in errors {
            eprintln!("错误: {}", error);
        }
    }
}
```

## 验证流程

1. **引用检查** - 确保所有 `ref` 和 `var` 引用有效
2. **循环检测** - 确保方程间无循环依赖
3. **类型检查** - 确保类型一致性

## 命令行使用

```bash
# 验证单个目录
eqc validate examples/

# 验证结果
✅ 验证通过
   - 模块数: 18
   - 方程数: 159
```

## 错误级别

- **Error** - 严重错误，必须修复
- **Warning** - 警告，建议修复
- **Info** - 提示信息

## 配置选项

```rust
pub struct ValidatorConfig {
    pub strict_mode: bool,       // 严格模式
    pub allow_unused: bool,      // 允许未使用的定义
    pub check_numeric_ranges: bool, // 检查数值范围
}
```
