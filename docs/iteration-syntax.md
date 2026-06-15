# S表达式工作流配置语法

通过 S 表达式文件（`.sexpr`）定义工作流时，可在模块注解后添加：
- **工作流输入输出定义** - 显式声明工作流的外部接口
- **控制流配置** - 迭代执行模式、累加器、终止条件等

## 完整语法示例

```sexpr
;; @module: phenoflex.core
;; @name: PhenoFlex核心模块
;; @description: 物候预测模型

;; ============================================================================
;; 工作流控制流配置 - 迭代执行模式
;; ============================================================================
;; @execution_mode: iterative
;; @time_series: T
;; @accumulator: y from chill_portion.delta_cp op sum init 0
;; @accumulator: z from effective_heat.delta_z op sum init 0
;; @state_var: S from state_adjustment.S init 1.0 lag 1
;; @termination: accumulator z >= 6172

;; ============================================================================
;; 工作流级别输入输出定义（论文规范）
;; ============================================================================
;; @workflow_input: T, Array<Number>, required, 小时温度序列(摄氏度)
;; @workflow_input: yc, Number, optional, 冷量需求阈值(CP), 40
;; @workflow_input: zc, Number, optional, 热量需求阈值(GDH), 6172
;;
;; @workflow_output: y_final, Number, 最终冷量累积(CP), accumulator y
;; @workflow_output: z_final, Number, 最终热量累积(GDH), accumulator z
;; @workflow_output: bloom_reached, Boolean, 是否达到初花期, node stage_check.reached

;; @operator: phenoflex.temp_kelvin
;; ...后续算子定义
```

---

## 工作流输入定义

### 格式

```
@workflow_input: <名称>, <类型>, required|optional, <描述>[, <默认值>][, bind <节点>.<端口>]
```

### 参数说明

| 参数 | 说明 | 示例 |
|------|------|------|
| 名称 | 输入变量名 | `T`, `yc` |
| 类型 | 数据类型 | `Number`, `Array<Number>`, `String` |
| required/optional | 是否必填 | `required` |
| 描述 | 输入说明 | `小时温度序列` |
| 默认值 | 可选，默认值 | `40` |
| bind | 可选，绑定到节点端口 | `bind temp_kelvin.T` |

### 示例

```sexpr
;; 必填数组输入
;; @workflow_input: T, Array<Number>, required, 小时温度序列(摄氏度)

;; 可选参数带默认值
;; @workflow_input: yc, Number, optional, 冷量需求阈值, 40

;; 显式绑定到节点
;; @workflow_input: Tb, Number, optional, 基础温度, 4, bind gdh.Tb
```

### 元数据自动继承

工作流输入会**自动继承**绑定算子输入的 `latex_name` 和 `paper_ref` 字段。

例如，当 `@workflow_input: T, Array<Number>, required, 温度` 绑定到 `temp_kelvin.T` 时：

```sexpr
;; 算子定义中的元数据
;; @operator: phenoflex.temp_kelvin
;; @input: T, Number, required, 温度(摄氏度)
;; @input_latex: T
;; @input_paper_ref: 原始温度输入，单位摄氏度
```

生成的工作流输入 JSON 将自动包含：

```json
{
  "name": "T",
  "type": "Array<Number>",
  "required": true,
  "description": "温度",
  "latex_name": "T",           // 自动继承
  "paper_ref": "原始温度输入，单位摄氏度",  // 自动继承
  "bind_to": {"node": "temp_kelvin", "port": "T"}
}
```

**继承规则**：
- 工作流输入显式指定的值优先
- 未指定时从绑定的算子输入继承
- 支持 `latex_name` 和 `paper_ref` 两个字段

---

## 工作流输出定义

### 格式

```
@workflow_output: <名称>, <类型>, <描述>, <来源类型> <来源名称>[.<端口>]
```

### 来源类型

| 来源类型 | 格式 | 说明 |
|---------|------|------|
| `accumulator` | `accumulator <累加器名>` | 从累加器获取最终值 |
| `node` | `node <节点ID>.<端口>` | 从节点输出获取 |

### 示例

```sexpr
;; 从累加器获取
;; @workflow_output: y_final, Number, 最终冷量累积, accumulator y
;; @workflow_output: z_final, Number, 最终热量累积, accumulator z

;; 从节点输出获取
;; @workflow_output: bloom_reached, Boolean, 是否达到初花期, node stage_check.reached
```

---

## 控制流注解说明

| 注解 | 格式 | 说明 |
|------|------|------|
| `@execution_mode` | `single` \| `iterative` | 执行模式 |
| `@time_series` | `变量名` | 时间序列输入（每次迭代取一个元素） |
| `@accumulator` | `name from node.port op operation init value` | 累加器定义 |
| `@state_var` | `name from node.port init value lag steps` | 状态变量（跨迭代传递） |
| `@termination` | `accumulator name op threshold` \| `exhaust` \| `iterations count` | 终止条件 |

## 累加器格式

```
@accumulator: <名称> from <节点ID>.<端口> op <操作> init <初始值>
```

| 参数 | 说明 | 示例 |
|------|------|------|
| 名称 | 累加器变量名 | `y`, `z` |
| 节点ID.端口 | 数据来源 | `chill_portion.delta_cp` |
| 操作 | sum/max/min/count/last/average | `sum` |
| 初始值 | 累加器初始值 | `0` |

## 状态变量格式

```
@state_var: <名称> from <节点ID>.<端口> init <初始值> lag <滞后步数>
```

| 参数 | 说明 | 示例 |
|------|------|------|
| 名称 | 状态变量名 | `S`, `x_prev` |
| 节点ID.端口 | 数据来源 | `state_adjustment.S` |
| 初始值 | 初始值（第一次迭代使用） | `1.0` |
| 滞后步数 | 1=上一步x[i-1], 2=上上步x[i-2] | `1` |

## 终止条件格式

```sexpr
;; 累加器阈值
@termination: accumulator <名称> <操作符> <阈值>

;; 遍历完输入
@termination: exhaust

;; 固定迭代次数
@termination: iterations <次数>
```

| 操作符 | 说明 |
|--------|------|
| `>=` | 大于等于 |
| `>` | 大于 |
| `<=` | 小于等于 |
| `<` | 小于 |
| `==` | 等于 |

## 生成的 JSON 示例

使用上述 S 表达式生成的工作流 JSON：

```json
{
  "nodes": [...],
  "edges": [...],
  "inputs": [
    {
      "name": "T",
      "type": "Array<Number>",
      "required": true,
      "description": "小时温度序列(摄氏度)",
      "latex_name": "T",
      "paper_ref": "原始温度输入，单位摄氏度",
      "bind_to": {"node": "temp_kelvin", "port": "T"}
    },
    {
      "name": "yc",
      "type": "Number",
      "required": false,
      "description": "冷量需求阈值(CP)",
      "default": "40",
      "latex_name": "y_c",
      "paper_ref": "论文Table 1，冷量需求阈值，品种特异参数"
    },
    {
      "name": "zc",
      "type": "Number",
      "required": false,
      "description": "热量需求阈值(GDH)",
      "default": "6172",
      "latex_name": "z_c",
      "paper_ref": "论文Table 1，热量需求阈值，品种特异参数"
    }
  ],
  "outputs": [
    {
      "name": "y_final",
      "type": "Number",
      "description": "最终冷量累积(CP)",
      "source_type": "accumulator",
      "bind_from": {"accumulator": "y"}
    },
    {
      "name": "z_final",
      "type": "Number",
      "description": "最终热量累积(GDH)",
      "source_type": "accumulator",
      "bind_from": {"accumulator": "z"}
    },
    {
      "name": "bloom_reached",
      "type": "Boolean",
      "description": "是否达到初花期",
      "source_type": "node",
      "bind_from": {"node": "stage_check", "port": "reached"}
    }
  ],
  "control_flow": {
    "execution_mode": "iterative",
    "iteration": {
      "time_series_inputs": ["T"],
      "accumulators": [
        {
          "name": "y",
          "source_node": "chill_portion",
          "source_port": "delta_cp",
          "operation": "sum",
          "initial_value": 0
        },
        {
          "name": "z",
          "source_node": "effective_heat",
          "source_port": "delta_z",
          "operation": "sum",
          "initial_value": 0
        }
      ],
      "state_vars": [
        {
          "name": "S",
          "source_node": "state_adjustment",
          "source_port": "S",
          "initial_value": 1.0,
          "lag": 1
        }
      ],
      "termination": {
        "condition": {
          "type": "accumulator_threshold",
          "name": "z",
          "op": "gte",
          "threshold": 6172
        }
      }
    }
  }
}
```

## 相关文档

- [低代码平台 API 文档](../../backend-docs/lowcode/低代码平台.md) - 前端如何调用迭代工作流 API
- [PhenoFlex 完整示例](../tests/sexpr_samples/phenoflex_full.sexpr) - 实际的迭代工作流定义
