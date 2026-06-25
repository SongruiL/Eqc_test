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

### 3.4 决策优化（仿真优化）—— 在前向模型上搜最优决策

前向模型回答「当前条件下系统会怎样」；优化层回答「想要某种结果（产量最大/利润最高/按时上市…）该怎么做」。做法是**在前向模型上搜索**：试一组决策 → 跑仿真 → 用目标方程打分 → 调整再试（不解析反推方程）。设计见 `docs/spec-optimization.md`。

**决策 spec** 是与模型**分离**的独立产物（「可控」是问题/场景属性，不写进模型）：

```yaml
optimize:
  objective:
    expr: "(sub (mul (final Y) price) (mul CO2 co2_cost))"  # 目标 = 一条 S 表达式
    sense: max                                              # max / min
  constants: { price: 30.0, co2_cost: 0.12 }                # 目标里的非模型量
  knobs:
    - { var: CO2, kind: driver_const, bounds: [400, 1200], unit: ppm }   # 恒定驱动
    - { var: Pd,  kind: param,        bounds: [4, 12] }                  # 标量参数
  environment: scenario/weather_s4_400.csv   # 不可控环境（相对 spec 目录解析）
  optimizer: { method: de, pop: 25, iters: 80, seed: 42 }   # 定种子 → 可复现
```

- **旋钮（决策变量）** = 模型的**外部输入**（EQC 能自动列：参数/状态初值/驱动）。三种 `kind`（阶段 1 仅标量）：`param`（覆盖参数）/ `init`（覆盖状态量初值）/ `driver_const`（把某驱动整列设成常数）。可行域 `bounds`、单位、代价由人在 spec 里声明。
- **目标/约束 = S 表达式**，复用解释器，领域无关。其「变量」是对整段轨迹的**时间归约**：`final/at/max/min/mean/total`（区别于逐日算子与 `vsum`）；还能引用旋钮值与常量（成本项要用）。约束 `expr ≤ max` 用**惩罚法**（线性外罚，权重可经 spec 的 `penalty_weight:` 覆盖）；`eqc optimize` 会**逐约束报告**满足/违反与违反量、并标记整体可行性。
- **优化器**：差分进化 **DE**（免导数、对非光滑/阈值/多峰鲁棒、定种子可复现；垃圾候选给最差值不崩）。
- **敏感性预筛**：`eqc optimize --prescreen`（单目标）在搜索前对每个旋钮 ±10% 扰动看**目标**变化，把近零影响的旋钮**固定在基线**、只搜敏感旋钮（缩小维度）。例：S4 上 Pd 对产量 |Δ|≈0.0003（相对 CO₂ 的 0.0005）→ 自动固定，最优产量几乎不变。（`eqc sweep --sensitivity` 也可做手动预筛，但作用于参数对单变量。）
- **多目标（雏形）**：spec 再写一条 `objective2`，即进多目标模式——**单次 MO-DE**（带 Pareto 支配选择 + 拥挤度截断，~40 点）一次跑出**权衡前沿**（如「产量最大 vs CO₂ 用量最小」）。CLI 打印前沿表；Studio 画散点曲线、点选某点即叠加该点整季轨迹。
- 草莓 S4 实测：最大化产量 → CO₂/Pd 顶界 Y=10.95 kg/m²；利润变体（CO₂ 有成本）→ 最优 CO₂≈757 ppm（内点）；带约束 `max(LAI) ≤ 10`（涌现量）→ 最优 CO₂≈681/Pd=4、Y=9.36、峰值 LAI 恰好顶到 10（约束起作用、可行）。Pd 最优与 `eqc sweep` 网格逐位一致；各最优点用独立 `eqc simulate` 复现逐位一致。

### 3.5 参数标定（用实测数据反推参数）

机理模型的结构（方程）来自文献，但参数常是估计值（如 S4 的 `Kc` 注明「待重标定」）。**标定 = 用田间实测反推出最吻合现实的参数**，让模型量级可信——这是「在未标定模型上优化 = 拿错模型推错决策」的解药，也是通往 GP 的桥。设计见 `docs/spec-calibration.md`。

标定与决策优化是**同一外循环、共用同一评估核**，只换「旋钮=参数、目标=误差」：

```yaml
optimize:
  objective: { expr: "(rmse Y obs_Y)", sense: min }     # 误差（可多变量加权：(add (rmse Y oY) (mul w (rmse LAI oL)))）
  knobs:
    - { var: LUE, kind: param, bounds: [1, 6] }
    - { var: Kc,  kind: param, bounds: [100, 600] }
  environment: scenario/weather_s4_400.csv               # 同期天气
  observed: field_obs.csv                                # 实测数据 CSV（首列 DAT + 各观测列，空格=未测）
  optimizer: { method: de, pop: 30, iters: 100, seed: 42 }
```

- **误差算子**（作用于「仿真序列 vs 实测序列」）：`rmse / mae / nse（纳什效率，max）/ bias`。实测**稀疏**（周期性取样即可）。
- **可观测 vs 不可观测**：只能标定被观测约束住的参数。库强/LUE/分配比等模型内部量测不到、靠可观测输出（产量/生物量/LAI 的**时间序列**）反推。要时间序列（非单期末值）+ 处理梯度，并警惕「异参同效」。
- **recover-the-params 自验**（无需真数据即可建+验）：用已知参数造一条轨迹当伪实测，标定能把参数找回。实测：S4 用 LUE=4.0 造伪实测 → `eqc calibrate` 找回 LUE=4.000000、误差 0。
- **可辨识性 / 「该测什么」助手** `eqc identify`：标定前对每个候选参数 ±10% 扰动，量其对每个候选可观测变量整条轨迹的**相对** RMS 影响 → 告诉你「要定准某参数最该测哪个变量、哪些参数无观测能约束（不可辨识）、哪些参数对可能异参同效」，直接产出给园区的测量清单。S4 实测洞见：**LUE** 测谁都行（全局乘子）；**Pd** 必须测 **F**（果鲜重）——因 `Y=F·Pd/1000` 使产量 Y 几乎不随 Pd 变，只测 Y 标不出 Pd；**Kc** 在 CO₂≡400(=Cref) 下 `f_CO2≡1`、**不可辨识**——要标 Kc 必须有 CO₂ 处理梯度。

## 4. CLI 命令速查

```bash
eqc build --input <目录> --output <目录> --format all   # 生成 Python/Rust/JSON/Markdown/LaTeX；动态模型额外生成 python/<id>_sim.py（可独立运行的逐日仿真器，与 eqc simulate 同语义）
eqc validate <目录>                                      # 校验（解析/引用/类型/环检测 + 跨模块结构过定）
eqc structure <模型.eq.yaml> [--json] [--identifiability] [--metrics] [--layout3d]  # 结构分析：二部图+匹配+DM 分解（自由变量/块三角求解顺序/代数环/过欠定）；--identifiability 加结构可辨识性（参数→可测可达性=不可辨识 + 混淆候选，互补数值 eqc identify）；--metrics 加网络指标（度/介数/PageRank 中心性找枢纽 + 社区/模块度对照 meta.modules + 深度）；--layout3d 加 3D 力导向坐标（Rust 算确定性；深度→z、社区→簇位、介数→大小；坐标走 --json，前端只渲染）；--json 出 StructureJson 契约
eqc diff <旧模型> <新模型> [--json]                     # 版本结构 diff：两版本增删点/边 + 形式改变的方程（同 output 换式子=GP 进化信号）+ 结构距离/边相似度；按本地名对齐（跨版本 meta.id 不同也对得齐）；--json 出 GraphDiffJson
eqc graph <目录> --format mermaid                        # 输出依赖图（mermaid/dot）
eqc list <目录>                                          # 列出方程
eqc convert "(add x (mul y 2))" -o out.eq.yaml           # 单个 S 表达式 -> YAML
eqc workflow <注解sexpr> -o <目录> --operators           # 注解 sexpr -> workflow/算子
eqc check-dims <目录> [--strict]                         # 量纲一致性 + 跨模块耦合单位检查
eqc report <小目录> -o model.html [--layout layered|force|forrester]  # 自包含 HTML 报告（Forrester 库存-流量图 + DAG + 二维公式）；--layout 选结构图布局
eqc simulate <模型.eq.yaml> --drivers w.csv [--params s.json] -o out.csv  # 逐日仿真动态模型，输出轨迹 CSV
eqc sweep <模型.eq.yaml> --drivers w.csv --param LUE --range 1:5:9 --var Y [--reduce final] -o sweep.csv  # 扫一个参数看输出响应
eqc sweep <模型.eq.yaml> --drivers w.csv --sensitivity --var Y [--percent 10]  # 全局敏感性：各标量参数对 Y 的影响排序
eqc serve <模型.eq.yaml> [--drivers w.csv] [--params s.json] [--port 7878]  # EQC Studio：浏览器里看模型 + 跑仿真画轨迹（单模型）
eqc serve <eqc-workspace.yaml | 含该清单的目录>                              # 多模型工作区：Studio 顶部下拉切草莓/番茄/蓝莓/温室，免重启（每模型在清单里配自己的 path/drivers）
eqc export <模型.eq.yaml> [-o model.json]                # 导出模型 JSON 契约（前端/工具消费用，可检视；每个变量/参数带 display_name 友好名）
eqc optimize <模型.eq.yaml> --spec problem.yaml [--drivers w.csv] [--prescreen] [-o result.json]  # 仿真优化：DE 搜旋钮空间求目标最优（spec 含 objective2 则多目标 Pareto；--prescreen 先剔低敏感旋钮）
eqc calibrate <模型.eq.yaml> --spec calib.yaml [--drivers w.csv] [--observed obs.csv] [-o result.json]  # 参数标定：用实测数据反推参数（旋钮=参数、目标=预测vs实测误差）
eqc identify <模型.eq.yaml> --spec calib.yaml [--drivers w.csv] [--observables Y,LAI] [-o report.json]  # 可辨识性：标定前看「要定准哪个参数、最该测哪个变量」（服务实验设计）
eqc evolve <模型.eq.yaml> --spec gp.yaml --drivers w.csv --observed obs.csv [-o result.json]  # 受约束 GP：在某 gp_target 靶点进化方程结构（spec 含 pareto/memetic/joint/baseline_form）；印最佳形式+rediscovery+溯源草稿
```

> **EQC Studio（交互式前端）**：`eqc serve <模型> --drivers w.csv` 起一个本地服务（`http://localhost:7878/`）。浏览器里左边是 Forrester 图 + 二维公式，右边是**整季仿真折线图**（勾选变量即画其轨迹，如产量 Y）。编辑模型保存即自动刷新。
> - 端点：`/api/model`（JSON 契约）、`/api/report[?layout=force]`（HTML 报告）、`/api/simulate`（轨迹 JSON）、`/api/chart.svg?vars=Y,TDM`（折线图 SVG）、`/api/optimize?spec=problem.yaml`（跑优化、返回最优旋钮+收敛轨迹+收敛曲线 SVG）、`/api/models`（多模型工作区花名册，前端建选择器用）、`/api/llm`（前端 AI 助手代理 Claude，见下）。
> - **GP 端点**：`/api/evolve?target=<靶id>&zone=&pop=&gens=`（同步单靶 Pareto，返回前沿+每点公式/拟合/rediscovery/采纳产物）；`/api/evolve/start?...&memetic=&targets=A,B`（异步起后台任务→`{task_id}`，放开 memetic/大规模、`targets=` 多靶=联合进化）；`/api/evolve/status?id=`（轮询进度：当前代+实时收敛曲线 SVG，完成内嵌完整前沿 JSON）。
> - **多模型工作区（免重启切模型）**：`eqc serve` 指向一个 `eqc-workspace.yaml`（或含它的目录）→ Studio 顶部出现模型下拉，切草莓/番茄/蓝莓/温室不重启，与粒度/布局/处理区自由组合。清单逐条声明 `{id, name, path, drivers?, params?, data_dir?}`（路径相对清单目录；每模型实测数据默认隔离在 `observations/<id>/`）。作物目录里是版本史（s1..s8）且每模型驱动不同，故用**显式清单**而非目录扫描。所有 model-bound 端点接 `?model=<id>`（缺省=花名册第一个）。**单模型模式（指向单个 `.eq.yaml`）行为逐位不变**、选择器隐藏。
> - **耦合视图（温室↔作物连成一张大图）**：清单加 `couplings: [{id, name, models:[id..], links:[{to: CROP.invar, from: GH.outvar}]}]` → 选择器多出「耦合视图」分组项；选中即把多个模型的结构图连成一张图（作物驱动 ← 温室输出，跨模型边由 `source:` 机制画出）。**不改 canonical 模型**——serve 加载耦合条目时在内存里给作物 Input 注入 `source`（不落盘）。耦合条目**只看结构图**（仿真/录入/标定需选单作物模型，会友好拦截）；模块级要清爽需被耦合的模型都有 `meta.modules`。
> - **结构图布局可切换**（面板右上「Forrester / 力导向 / 分层」切换条，选择记在浏览器里）：`forrester`=学术风（存量横向主干 + 速率阀门 + 辅助/参数/驱动作卫星就近摆放，最像作物模型论文图）；`force`=力导向有机网络（确定性、可复现）；`layered`=自上而下分层（基线，已修复"环把层号顶飞"的高度爆炸）。布局全由 EQC-Rust 算坐标、出 SVG，前端只切换。
> - **缩放 + 专注**：工具栏 `−/适应/+` 缩放结构图（拖动滚动条平移，比例记在浏览器里）；`⛶ 专注` 一键全屏只看结构图、再点恢复双栏。缩放/专注是 Studio 行为，离线报告仍零 JS。
> - **节点交互**：鼠标**悬停**节点 → 浮出注释（变量名·分类·单位 + 备注 + 二维公式 + 来源，全取自 `/api/model` 契约）；**点击**节点 = 切换选中（高亮节点+公式 + 画其轨迹），再点取消，依次点多个曲线累加，与右栏复选框双向同步。联动逻辑在 Studio（同源 iframe），报告本身只带 `data-var`/`data-output` 数据属性，仍零 JS。
> - **情景探索器**：曲线下方「情景」面板自动列出**标量参数 + 状态量初值**（各一行滑块+数值框，向量参数跳过）；拖动/输入即**实时重算曲线**（防抖），改过标蓝，「重置默认」复位。机制：覆盖经 `/api/chart.svg?p=name:val,…&init=name:val,…&d=name:val,…`（`/api/simulate` 同）交给 EQC 重算——`--drivers`/`--params` 不再启动时冻结（`d=` 把某驱动整列设成常数，对应 `driver_const` 旋钮）。
> - **决策优化面板**：页面底部「决策优化」面板——填决策 spec 路径（相对模型目录）点「运行优化」，跑 DE（数十秒，release 更快、编辑模型时暂不可用），显示**最优旋钮 + 目标值 + 可行性/逐约束 + 收敛曲线**（EQC 自生成 SVG）；「叠加最优旋钮到曲线」把最优旋钮喂回情景、在「整季仿真轨迹」里画出最优整季轨迹。**多目标**（spec 含 `objective2`）则画 **Pareto 前沿散点曲线**，点选某点即叠加该点的整季轨迹。后端 `/api/optimize` 与 CLI `eqc optimize` 共用同一计算与 JSON。
> - **受约束 GP 面板（专家视图，human-in-the-loop）**：列出模型的 🟠 进化靶点（`gp_target`）→ **多选**（选 1=单靶、≥2=联合进化，捕捉槽位间耦合）→ 配置 pop/gens/seed + memetic 勾选 → 「开始进化」。走异步后台任务：spinner 显示**第 g/G 代 + 实时收敛曲线**；完成出 **Pareto 前沿散点**（精度 vs 复杂度，点拐点）。点某前沿点 → 候选详情：rediscovery 徽章（🟢 复原现有形式=机理验证 / 🟠 新假设 / 自定义）、二维公式（MathML）、与现有形式**并排对比**（rmse/复杂度）+ 拟合轨迹叠观测、**采纳**（生成可编辑的溯源条目草稿 + 可粘贴的 `.eq.yaml` 方程片段，复制/下载，**只产文本不写盘**）。联合模式下候选详情按**槽位分区**，每槽各自上述全套。观测取当前处理区录入数据（拟合各靶点的输出变量）。GP 提议、科学家裁决——不自动改模型。
> - **结构图拖拽**：拖**空白**=平移画布；拖**节点方框**=移动它、连线跟随（手动错开遮挡，会话内有效、刷新复位）；**轻点**节点=选中。三者按落点/位移自动区分。
> - **友好显示名（非数学用户看懂）**：图表勾选框、耦合勾选框、图例、结构图节点统一显**中文名**（代号进 hover）。显示名由 EQC 单一权威算（契约 `display_name`：变量 `label` → 方程中文名 → 参数 `name_cn` → 延迟寄存器派生「源（上一步）」→ 代号兜底）；cohort 分量显 `果碳[1]`…`果碳[10]`、与向量分量风格一致。要给某变量定中文名 = 在模型里加 `label:`（缺省自动取方程中文名）。
> - **AI 助手「问AI」（v2 前端，`/v2`）**：右上「🤖 问AI」抽屉——用自然语言指挥整个前端（导航/查模型/调情景参数/跑仿真/切模型/写处理区设置）。架构 = **命令注册表（`frontend/src/lib/commands.svelte.ts`）= 前端能力唯一真相源**：⌘K 面板与 AI 工具都从它派生，**加一条命令 = 面板按钮 + AI 能力同时获得**。前端跑 agent loop（`lib/agent.svelte.ts`：注册表→自动生成 Anthropic tools→`tool_use`→执行 handler→`tool_result`→循环至 `end_turn`），落盘类命令（写盘）执行前**弹确认框**。后端 `/api/llm` 只**注入 key + 转发** Claude（key 绝不下发浏览器）。模型默认 **Sonnet 4.6**，env `EQC_LLM_MODEL` 一行可换（无需重建前端）。**配置**：在启动 serve 的目录放一个 gitignored 的 `.eqc-secret`（`KEY=VALUE` 行，模板见 `eqc-secret.example`），填 `ANTHROPIC_API_KEY`（必填）+ `EQC_LLM_PROXY`（直连被墙的机器填本地代理）+ 可选 `EQC_LLM_MODEL`；serve 启动自动加载、打印「🤖 AI 助手已配置」。先非流式，SSE 留后续。
> - 原则：**EQC 始终是唯一权威**，前端只显示 EQC 生成的 SVG/MathML/JSON——前端与 EQC 之间只有一条「可检视、只增不改」的契约（`eqc export` 可随时打印它），所以随 EQC 升级而升级时低风险、易排查。后续增量：LLM 流式 SSE、更多 Agent 命令、GP 结构 diff。

> `report`/`check-dims` 的"目录"是装 `.eq.yaml` 的文件夹（与 build/validate 同）。`report` 会把目录内所有文件合成一张 DAG，**指向一两个相关模块的小目录**，别指整个 `examples/`（52 模块图会过大）。

## 5. 模块地图（`src/` 各模块功能）

| 模块 | 作用 | 关键内容 |
|------|------|----------|
| `ast/` | **强类型 AST**：`Expr` 枚举（360+ 算子变体）+ 代码生成 | `Expr`；`to_python/to_rust/to_latex`（各一个穷尽 match）；`from_yaml_value`（map 格式反序列化，已手写 `Deserialize`）；`substitute`/`visitor` |
| `ops/` | **算子注册表（单一真相源）** | `OperatorSpec{name,arity,eval,rust,python,latex}`；`as_operator(&Expr)->(名,参数)`；52 个标量算子在此定义一次，求值器与 codegen 共用 |
| `eval/` | **树遍历求值器** | `Expr::eval(&Env)`；`Env`/`EvalMode`（默认严格，非有限即报错）；`eval_special`（gamma/erf/正态等，部分需 `advanced_math`）|
| `sim/` | **逐日仿真引擎** | `simulate(file,&SimInput)->SimOutput`；显式 Euler 日步进，积分状态量(`rate`)+延迟寄存器(`prev`)，步内拓扑序，内置 `DAT`；环/缺驱动校验；`build_plan->SimPlan`（与 codegen 共用的单一真相源）|
| `optimize/` | **优化层（仿真优化）** | `objective`（时间归约 `final/at/max/min/mean/total` + 目标 S 表达式求值）；`problem`（决策 spec：目标/旋钮/约束/优化器）；`core`（目标评估核 `evaluate`：旋钮赋值→跑 sim→归约成代价+约束惩罚，垃圾候选给最差值）；`de`（差分进化，确定性 PRNG）|
| `gp/` | **受约束遗传编程** | `grammar`（5 套受约束语法 + `Candidate{骨架,可调常数}`）；`constraints`（量纲/单调/有界先验过滤）；`operators`（变异/交叉/常数扰动 + 形式骨架匹配）；`fitness`（patch 候选→仿真→rmse vs 观测）；`pareto`（精度 vs 复杂度 NSGA-II + memetic + 进度回调）；`provenance`（机理形式识别 + rediscovery 判定 + 溯源草稿）；`joint`（多槽位联合进化 + Pareto-joint）。`eqc evolve` CLI + Studio GP 面板（`/api/evolve[/start|/status]`）的引擎 |
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
**Studio 可视化/交互 arc**：三档可切换布局（分层/力导向/Forrester 学术风）、缩放+专注、悬停注释、点击多选联动、情景探索器（浏览器调参实时重算）、结构图拖拽。
**工具层 arc**：`eqc sweep`（参数扫描 + 全局敏感性）、`eqc build` 生成可独立运行的 Python 仿真器（标量+向量，与 `eqc simulate` 对拍一致）。
**优化层 arc（阶段 1）**：`src/optimize` + `eqc optimize --spec`——时间归约词汇、决策 spec（标量旋钮 param/init/driver_const）、目标评估核、差分进化 DE。在草莓 S4 上「扫 CO₂/Pd 求产量或利润最优」，与 `eqc sweep` 网格 / 独立 `eqc simulate` 交叉核对逐位一致。设计见 `docs/spec-optimization.md`。
**标定/耦合 arc**：`eqc calibrate`/`eqc identify`（参数标定 + 可辨识性，设计 `docs/spec-calibration.md`）；多速率耦合仿真 `eqc couple` + Studio 耦合视图/仿真/优化（温室↔作物，设计 `docs/spec-coupled-simulation.md`）。
**受约束 GP arc（核心愿景，已落地）**：`src/gp`（5 套受约束语法 + Candidate 基因组 + 树算子 + 模型级 fitness + Pareto/memetic + 溯源回流 + 多槽位联合）+ `eqc evolve` CLI + **Studio GP 面板（S1–S5+B2：`/api/evolve[/start|/status]` 端点、靶点多选、异步进度、Pareto 前沿、rediscovery 徽章、对比+采纳、多槽位分区）**。全程合成数据端到端验证（从观测复原已知形式）。设计见 `docs/spec-genetic-programming.md`、`docs/spec-gp-studio.md`。**唯一未做 = 真进化（等云南 2026-07 田间数据）。**

**前端 LLM Agent「问AI」arc（v2，已落地）**：命令注册表（`frontend/src/lib/commands.svelte.ts`）→ 自动派生 Anthropic tools → 前端 agent loop（`lib/agent.svelte.ts`，流式 SSE）→ 后端 `/api/llm[/stream]` 代理 Claude（key 走 gitignored `.eqc-secret`）→ confirm 闸护栏落盘类命令。prompt 缓存（system+tools+对话三断点）。**★标准约定：以后新增任何前端可操作功能，必须同时在命令注册表登记一条命令** → ⌘K 面板 + AI 助手自动同获。
- **配置 key**：启动 serve 的目录放 `.eqc-secret`（模板 `eqc-secret.example`）：`ANTHROPIC_API_KEY=` + 直连被墙的机器加 `EQC_LLM_PROXY=http://127.0.0.1:10808` + 可选 `EQC_LLM_MODEL=`。
- **e2e（Playwright，系统 Edge，方案 C）**：确定性 mock 套（主力，零成本）+ 真 LLM 冒烟（抽检）。跑 mock：`cmd /c "set EQC_LLM_MOCK=1&& eqc.exe serve eqc-workspace.yaml --port 7885"` 起 serve →（`cd frontend`）`$env:EQC_E2E_BASE='http://localhost:7885'; npm run test:e2e`；跑真冒烟：真 key serve + `$env:EQC_E2E_REAL='1'; npm run test:e2e`。改前端流程：`cd frontend; npm run check + npm run build`（产 `studio_v2.html`）→ `cargo build --release --features cli --offline` → 重启 serve。
下一步备选：优化层阶段 3+（时变控制曲线参数化、离散旋钮、CMA-ES/贝叶斯）；矩阵 eval（V4）、cohort 在图上分组显示、报告增强、耦合的时间尺度聚合；Studio 前端大修（见下次讨论）。
