# DAG 模块

有向无环图（Directed Acyclic Graph）模块用于方程依赖关系的分析和排序。

## 职责

- 构建方程之间的依赖关系图
- 检测循环依赖
- 提供拓扑排序

## 模块结构

```
dag/
├── mod.rs       # 模块入口
├── builder.rs   # DAG 构建器
└── README.md    # 本文档
```

## 核心类型

### `DagBuilder`

DAG 构建器，负责：

1. 收集方程定义
2. 分析表达式中的变量引用
3. 建立节点间的边（依赖关系）
4. 提供拓扑排序后的执行顺序

## 使用示例

```rust
use equation_compiler::dag::DagBuilder;

// 从方程文件构建 DAG
let dag = DagBuilder::from_equations(&equations);

// 获取拓扑排序后的执行顺序
let sorted = dag.topological_sort()?;

// 检查是否有循环依赖
if dag.has_cycle() {
    panic!("存在循环依赖！");
}
```

## 依赖检测

DAG 模块会分析每个方程表达式中的变量引用：

- `Expr::Var(name)` - 引用其他方程的输出变量
- `Expr::Param(name)` - 引用输入参数（不产生依赖）

只有变量引用会产生边（依赖关系），参数引用不会。

## 错误处理

- `CycleError` - 检测到循环依赖时抛出，包含循环路径信息
