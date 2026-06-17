# EQC 使用指南与架构总览

> 本文给「使用者」和「下一轮接手的 AI」看：EQC 是做什么的、怎么用、各模块负责什么。

## 1. 这个项目是做什么的

EQC（equation-compiler）是一个 **Rust 库 + CLI 工具（二进制名 `eqc`）**，用于**农业复杂生态系统的数学建模**。核心理念是**单一真相源**：数学关系只写一次（S 表达式 / YAML），由 EQC 映射成多种产物——可执行代码、二维公式、依赖图、量纲检查等。

最终愿景（尚未实现）：在 S 表达式基因型上做**约束遗传编程（GP）**，让模型在已有力学骨架内进化、降低误差。当前已把"数学模型开发工具"这块地基打扎实。

## 2. 两条流水线

EQC 内部有**两条相对独立**的流水线：

| 流水线 | 输入 | 经过 | 产物 |
|--------|------|------|------|
| **A. 方程文件** | `.eq.yaml`（含 meta/parameters/variables/equations）| parser → schema → validator → dag → generators / eval / units / report | Python/Rust/LaTeX/Markdown 代码、DAG、数值求值、量纲检查、HTML 报告 |
| **B. 注解 S 表达式** | 带 `;; @module/@operator` 注解的 `.sexpr` | sexpr（lexer/parser/converter/workflow）| Workflow JSON、SQL 模板（对接 lowcode 平台）|

> `eqc report` / `check-dims` / `build` / `validate` / `graph` 走 **A**；`eqc workflow` / `validate-sexpr` 走 **B**；`eqc convert` 把单个 S 表达式转成 YAML 表达式。

## 3. 模型文件格式（`.eq.yaml`，流水线 A）

```yaml
meta:
  id: WOFOST
  model: WOFOST
  name_cn: WOFOST作物生长模型
parameters:                      # 常量；现在可用任意有意义的名字（见 §6）
  Tbase: { name_cn: 基点温度, type: float, default: 3.0, unit: degC }
variables:
  Tmax: { type: input, dtype: float, unit: degC, description: 日最高气温 }
  Tavg: { type: intermediate, dtype: float, unit: degC, description: 日均温 }
equations:
  - id: WOF-01
    name: 日均温
    output: Tavg
    expression: { op: div, args: [ { op: add, args: [ {ref: Tmax}, {ref: Tmin} ] }, {const: 2} ] }
```

表达式用 **`{op, args}` / `{ref: 名}` / `{const: 数}`** 的 map 形式（等价于 S 表达式 `(div (add Tmax Tmin) 2)` 的树）。完整可用示例见 `examples/wofost.eq.yaml`、`examples/photo.eq.yaml`。

## 4. CLI 命令速查

```bash
eqc build --input <目录> --output <目录> --format all   # 生成 Python/Rust/JSON/Markdown/LaTeX
eqc validate <目录>                                      # 校验（解析/引用/类型/环检测）
eqc graph <目录> --format mermaid                        # 输出依赖图（mermaid/dot）
eqc list <目录>                                          # 列出方程
eqc convert "(add x (mul y 2))" -o out.eq.yaml           # 单个 S 表达式 -> YAML
eqc workflow <注解sexpr> -o <目录> --operators           # 注解 sexpr -> workflow/算子
eqc check-dims <目录> [--strict]                         # 量纲一致性 + 跨模块耦合单位检查
eqc report <小目录> -o model.html                        # 自包含 HTML 报告（DAG + 二维公式）
```

> `report`/`check-dims` 的"目录"是装 `.eq.yaml` 的文件夹（与 build/validate 同）。`report` 会把目录内所有文件合成一张 DAG，**指向一两个相关模块的小目录**，别指整个 `examples/`（52 模块图会过大）。

## 5. 模块地图（`src/` 各模块功能）

| 模块 | 作用 | 关键内容 |
|------|------|----------|
| `ast/` | **强类型 AST**：`Expr` 枚举（360+ 算子变体）+ 代码生成 | `Expr`；`to_python/to_rust/to_latex`（各一个穷尽 match）；`from_yaml_value`（map 格式反序列化，已手写 `Deserialize`）；`substitute`/`visitor` |
| `ops/` | **算子注册表（单一真相源）** | `OperatorSpec{name,arity,eval,rust,python,latex}`；`as_operator(&Expr)->(名,参数)`；52 个标量算子在此定义一次，求值器与 codegen 共用 |
| `eval/` | **树遍历求值器** | `Expr::eval(&Env)`；`Env`/`EvalMode`（默认严格，非有限即报错）；`eval_special`（gamma/erf/正态等，部分需 `advanced_math`）|
| `units/` | **量纲系统（科学护栏）** | `Dimension`（7 SI 指数）；`Unit{dim,scale,offset}`；`parse_unit`/`convert`；`check_expr`/`check_equation_file`/`check_coupling` |
| `report/` | **HTML 报告** | `generate_report`：MathML 二维公式（浏览器原生）+ EQC 自生成 SVG DAG，零第三方、离线 |
| `sexpr/` | **S 表达式流水线 B** | lexer/parser/converter（sexpr→Expr）；`workflow`（注解 sexpr→ModuleDef）；`operator_gen`（→ AST JSON / SQL）；`to_yaml` |
| `parser/` | **YAML 方程文件解析** | `parse_file`/`parse_directory`；加载后调用 `reclassify_parameters`（把引用到参数名的 Var 改成 Param）|
| `schema/` | **数据结构** | `EquationFile`/`Metadata`/`Parameter`/`Variable`/`Equation`/`DataType`；map 用 `IndexMap`（输出可复现）|
| `validator/` | **验证器** | `type_checker`（Numeric/Boolean）、`reference_checker`（引用是否定义）、`cycle_detector`（环）|
| `dag/` | **DAG 构建** | 由 parameters/variables/equations 建节点、由引用建边，petgraph 拓扑排序；`DagNode.metadata` 用 IndexMap |
| `generators/` | 各格式生成器 | `python`/`rust_operator`/`latex`/`markdown`/`workflow_json` |
| `main.rs` | CLI 入口 | clap 子命令；`run_build/validate/graph/list/convert/workflow/check_dims/report/...` |

## 6. 关键约定与注意事项

- **表达式 map 格式**：`{op,args}`/`{ref}`/`{const}`。`Expr` 的 `Deserialize` 是手写的（不是默认 derive），专门解析这种格式。
- **参数命名**：早期要求参数必须叫 `p1/p2`；**现已修复**——`parameters:` 里声明的任意名字，加载后会自动被识别为参数（`reclassify_parameters`）。
- **输出可复现**：参数/变量/DAG 元数据用 `IndexMap`，同输入永远生成逐字节相同的输出。
- **量纲检查不接默认 validate**：因现有示例单位不全，量纲检查是独立的 `eqc check-dims`，不会让 `validate` 误报。
- **求值器严格模式**：默认除零/NaN/Inf 报 `NonFinite`；将来 GP 可关掉让 NaN 当惩罚。

## 7. 本机构建 / 测试（Windows，无管理员权限）

- 工具链 PATH：`C:\Users\lzyay\winlibs\mingw64\bin;C:\Users\lzyay\Rust196\Rust\bin;C:\Program Files\Git\cmd`
- 网络：git/cargo 前清代理变量 `$env:HTTP_PROXY=''; $env:HTTPS_PROXY=''`（cargo 走 `.cargo/config.toml` 的 rsproxy 镜像）。
- 构建：`cargo build --features cli --offline`（产物 `target\debug\eqc.exe`）。
- 测试：`cargo test --features cli --offline`；含特殊函数时 `cargo test --features "cli advanced_math"`。
- **跳过 `gsl_math`**（需 GSL C 库，本机无）；`full` 也别用（含 gsl_math）。

## 8. 路线图（已完成 / 下一步）

已完成：求值器、算子注册表（52 算子）、特殊函数（advanced_math）、量纲检查+单位换算+耦合、`eqc check-dims`、`eqc report` 可视化、生成器确定性、参数命名修复。
下一步备选：**GP 约束进化层**（核心愿景，需先讨论 fitness 数据/可进化节点/约束）、报告增强、codegen 死分支宏重构、耦合的时间尺度聚合。
