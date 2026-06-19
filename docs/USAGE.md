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

### 3.1 动态过程模型（状态量 + 逐日仿真）

WOFOST/Sugiyama 这类**机理模型**是日步长动态系统：有随时间积分的状态量、累积量。EQC 用变量上的元数据表达它们，由 `eqc simulate` 做**显式 Euler 逐日积分**：

- `class: <Forrester 分类>`——`state`(状态量) / `rate`(速率) / `driving`(驱动) / `auxiliary`(辅助) / `parameter` / `semi_state`(半状态) / `boundary`(边界)。可省略，缺省按结构推断。
- **积分状态量**：`{ class: state, init: <初值>, rate: <速率变量名> }`——仿真器每步 `X[n]=X[n-1]+rate[n]`，状态量**本身不在 `equations:` 里写表达式**。
- **延迟寄存器（半状态量）**：`{ class: semi_state, init: <初值>, prev: <来源变量名> }`——`X[n]=src[n-1]`，用于差分（如 `ΔX=X−X_prev`）。
- **内置变量 `DAT`**：第几天（1 起），无需声明即可在方程里引用（物候/开花门控）。
- 求值为严格模式：除零/NaN/Inf 报错（早失败）。

### 3.2 同期群 cohort（一组同类个体）

果序、叶片这种「很多个同类个体各自成长、再汇总」的结构，用 **cohort** 模板写一次、自动展开成标量（加载期 YAML 宏，引擎不感知）：

```yaml
cohorts:
  fruit: { size: 3, index: q }          # q = 1..3
parameters:
  anthesis: { cohort: fruit, name_cn: 开花日, values: [40, 80, 120] }   # 每个个体一个值
variables:
  TF: { cohort: fruit, class: state, init: 0.0, rate: rateTF }          # 每个个体一个状态量
equations:
  - { output: GS, expression: { op: mul, args: [ {const: 0.24},
      { op: sum_over, over: fruit, body: { ref: DRFG, at: q } } ] } }     # Σ over fruit
```

语法：`cohort: <家族>`（变量/参数/方程模板）、`{ref: X, at: q}`（取第 q 个）、`{idx: q}`（下标当数字）、`{op: sum_over, over: <家族>, body: …}`（求和，`prod_over` 求积）。展开后是 `TF__1/2/3` 等纯标量。完整示例：草莓 v1（Sugiyama 2025 骨架）在**独立工作目录** `strawberry_model/strawberry_v1.eq.yaml`（与本仓库平级、不随仓库提交，含文献综述与 OA PDF）。

> **模型结构 vs 情景数据分离**：模型文件只写结构与方程；逐日天气走 `--drivers` CSV、按个体常数（如实测开花日）走 `--params` JSON。换一季只换情景数据，不动模型。

### 3.3 向量化 cohort（推荐）—— 一个变量装一组

cohort 还可以直接写成**向量变量**（不用宏展开），更贴近数学、图上一个节点。做法：用**向量参数**当「种子」（`values: [...]`），其余靠广播自动传开，聚合用 `vsum`：

```yaml
parameters:
  anthesis: { name_cn: 各果序开花日, values: [40, 80, 120] }   # 向量参数（长度=果序数）
variables:
  active: { class: auxiliary }                          # 自动成向量
  TF:     { class: state, init: 0.0, rate: rateTF }     # 向量状态量（init 标量广播）
  GS:     { class: auxiliary }                          # 标量（vsum 归约）
equations:
  - { output: active, expression: { op: geq,  args: [ {ref: DAT}, {ref: anthesis} ] } }   # 逐元素 → 向量
  - { output: GS,     expression: { op: vsum, args: [ {ref: gs} ] } }                      # Σ over 向量
```

- 求值/仿真按 `Value{标量|向量|矩阵}` 运行；**52 个标量算子自动逐元素**（广播：标量↔任意形状、同形状逐元素）。
- 向量算子：`vsum/vprod/vmean/vmin/vmax`（归约）、`dot/cross/vec_norm/vec_normalize`。
- 仿真输出把向量变量**展平**成 `DF[1]/DF[2]/…`（CSV/图表各画一条分量线；Studio 里勾选 `DF` 即画全部分量）。
- 完整对照：`../strawberry_model/strawberry_v1_vector.eq.yaml`（向量版 **28 变量**）与标量宏展开版 `strawberry_v1.eq.yaml`（**92 变量**）**产量 Y 逐位一致**。设计见 `docs/spec-vector-matrix.md`。矩阵 eval（matmul/det…）尚未实现（后置）。

## 4. CLI 命令速查

```bash
eqc build --input <目录> --output <目录> --format all   # 生成 Python/Rust/JSON/Markdown/LaTeX；动态模型额外生成 python/<id>_sim.py（可独立运行的逐日仿真器，与 eqc simulate 同语义）
eqc validate <目录>                                      # 校验（解析/引用/类型/环检测）
eqc graph <目录> --format mermaid                        # 输出依赖图（mermaid/dot）
eqc list <目录>                                          # 列出方程
eqc convert "(add x (mul y 2))" -o out.eq.yaml           # 单个 S 表达式 -> YAML
eqc workflow <注解sexpr> -o <目录> --operators           # 注解 sexpr -> workflow/算子
eqc check-dims <目录> [--strict]                         # 量纲一致性 + 跨模块耦合单位检查
eqc report <小目录> -o model.html [--layout layered|force|forrester]  # 自包含 HTML 报告（Forrester 库存-流量图 + DAG + 二维公式）；--layout 选结构图布局
eqc simulate <模型.eq.yaml> --drivers w.csv [--params s.json] -o out.csv  # 逐日仿真动态模型，输出轨迹 CSV
eqc sweep <模型.eq.yaml> --drivers w.csv --param LUE --range 1:5:9 --var Y [--reduce final] -o sweep.csv  # 扫一个参数看输出响应
eqc sweep <模型.eq.yaml> --drivers w.csv --sensitivity --var Y [--percent 10]  # 全局敏感性：各标量参数对 Y 的影响排序
eqc serve <模型.eq.yaml> [--drivers w.csv] [--params s.json] [--port 7878]  # EQC Studio：浏览器里看模型 + 跑仿真画轨迹
eqc export <模型.eq.yaml> [-o model.json]                # 导出模型 JSON 契约（前端/工具消费用，可检视）
```

> **EQC Studio（交互式前端）**：`eqc serve <模型> --drivers w.csv` 起一个本地服务（`http://localhost:7878/`）。浏览器里左边是 Forrester 图 + 二维公式，右边是**整季仿真折线图**（勾选变量即画其轨迹，如产量 Y）。编辑模型保存即自动刷新。
> - 端点：`/api/model`（JSON 契约）、`/api/report[?layout=force]`（HTML 报告）、`/api/simulate`（轨迹 JSON）、`/api/chart.svg?vars=Y,TDM`（折线图 SVG）。
> - **结构图布局可切换**（面板右上「Forrester / 力导向 / 分层」切换条，选择记在浏览器里）：`forrester`=学术风（存量横向主干 + 速率阀门 + 辅助/参数/驱动作卫星就近摆放，最像作物模型论文图）；`force`=力导向有机网络（确定性、可复现）；`layered`=自上而下分层（基线，已修复"环把层号顶飞"的高度爆炸）。布局全由 EQC-Rust 算坐标、出 SVG，前端只切换。
> - **缩放 + 专注**：工具栏 `−/适应/+` 缩放结构图（拖动滚动条平移，比例记在浏览器里）；`⛶ 专注` 一键全屏只看结构图、再点恢复双栏。缩放/专注是 Studio 行为，离线报告仍零 JS。
> - **节点交互**：鼠标**悬停**节点 → 浮出注释（变量名·分类·单位 + 备注 + 二维公式 + 来源，全取自 `/api/model` 契约）；**点击**节点 = 切换选中（高亮节点+公式 + 画其轨迹），再点取消，依次点多个曲线累加，与右栏复选框双向同步。联动逻辑在 Studio（同源 iframe），报告本身只带 `data-var`/`data-output` 数据属性，仍零 JS。
> - **情景探索器**：曲线下方「情景」面板自动列出**标量参数 + 状态量初值**（各一行滑块+数值框，向量参数跳过）；拖动/输入即**实时重算曲线**（防抖），改过标蓝，「重置默认」复位。机制：覆盖经 `/api/chart.svg?p=name:val,…&init=name:val,…`（`/api/simulate` 同）交给 EQC 重算——`--drivers`/`--params` 不再启动时冻结。
> - **结构图拖拽**：拖**空白**=平移画布；拖**节点方框**=移动它、连线跟随（手动错开遮挡，会话内有效、刷新复位）；**轻点**节点=选中。三者按落点/位移自动区分。
> - 原则：**EQC 始终是唯一权威**，前端只显示 EQC 生成的 SVG/MathML/JSON——前端与 EQC 之间只有一条「可检视、只增不改」的契约（`eqc export` 可随时打印它），所以随 EQC 升级而升级时低风险、易排查。后续增量：点节点高亮、浏览器内编辑、LLM 问答、GP 结构 diff。

> `report`/`check-dims` 的"目录"是装 `.eq.yaml` 的文件夹（与 build/validate 同）。`report` 会把目录内所有文件合成一张 DAG，**指向一两个相关模块的小目录**，别指整个 `examples/`（52 模块图会过大）。

## 5. 模块地图（`src/` 各模块功能）

| 模块 | 作用 | 关键内容 |
|------|------|----------|
| `ast/` | **强类型 AST**：`Expr` 枚举（360+ 算子变体）+ 代码生成 | `Expr`；`to_python/to_rust/to_latex`（各一个穷尽 match）；`from_yaml_value`（map 格式反序列化，已手写 `Deserialize`）；`substitute`/`visitor` |
| `ops/` | **算子注册表（单一真相源）** | `OperatorSpec{name,arity,eval,rust,python,latex}`；`as_operator(&Expr)->(名,参数)`；52 个标量算子在此定义一次，求值器与 codegen 共用 |
| `eval/` | **树遍历求值器** | `Expr::eval(&Env)`；`Env`/`EvalMode`（默认严格，非有限即报错）；`eval_special`（gamma/erf/正态等，部分需 `advanced_math`）|
| `sim/` | **逐日仿真引擎** | `simulate(file,&SimInput)->SimOutput`；显式 Euler 日步进，积分状态量(`rate`)+延迟寄存器(`prev`)，步内拓扑序，内置 `DAT`；环/缺驱动校验 |
| `units/` | **量纲系统（科学护栏）** | `Dimension`（7 SI 指数）；`Unit{dim,scale,offset}`；`parse_unit`/`convert`；`check_expr`/`check_equation_file`/`check_coupling` |
| `report/` | **HTML 报告** | `generate_report`：MathML 二维公式 + **Forrester 库存-流量图**（存量矩形/速率阀门/驱动椭圆/物质流粗线 vs 信息流虚线）+ 角色分色 DAG，零第三方、离线 |
| `sexpr/` | **S 表达式流水线 B** | lexer/parser/converter（sexpr→Expr）；`workflow`（注解 sexpr→ModuleDef）；`operator_gen`（→ AST JSON / SQL）；`to_yaml` |
| `parser/` | **YAML 方程文件解析** | `parse_file`/`parse_directory`；加载后调用 `expand_cohorts`（cohort 模板宏展开）+ `reclassify_parameters`（把引用到参数名的 Var 改成 Param）|
| `schema/` | **数据结构** | `EquationFile`/`Metadata`/`Parameter`/`Variable`/`Equation`/`DataType`/`VarClass`(8 类 Forrester)；`Variable` 含 `class`/`init`/`rate`/`prev`；map 用 `IndexMap`（输出可复现）|
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
**动态建模 arc（route B，2026-06）**：状态量元数据（`class`/`init`/`rate`/`prev`）、逐日仿真引擎 `src/sim` + `eqc simulate`、cohort 同期群宏展开、Forrester 库存-流量图渲染。首个动态示例 `../strawberry_model/strawberry_v1.eq.yaml`（草莓 Sugiyama 骨架，可跑）。
下一步备选：**GP 约束进化层**（核心愿景，fitness=跑仿真 vs 实测，需先讨论可进化节点/约束）、codegen 积分循环的**向量(numpy)/Rust 目标**（标量 Python 已完成，见 CHANGELOG）、cohort 在图上分组显示、报告增强、耦合的时间尺度聚合。
