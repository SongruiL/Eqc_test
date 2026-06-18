# 更新日志

本仓库 `SongruiL/Eqc_test` 由 `Boshenaware/equation-compiler` 接手，在其半成品基础上持续完善"数学模型开发工具"。按时间顺序记录主要工作。

## 已完成

### 修复：YAML 表达式反序列化
- 原 `Expr` 用默认 derive 反序列化（期望 `!标签`），与文档/示例的 `{op,args}` map 格式不符，导致整个 YAML 路径（build/validate）失效、5 个测试失败。
- 改为手写 `impl Deserialize for Expr`，委托已有的 `from_yaml_value`。`build`/`validate` 恢复正常。

### 规划与求值器（Phase 1）
- 定稿 `docs/spec-operator-registry-and-evaluator.md`。
- 新增 `src/eval`：树遍历求值器 `Expr::eval`、`Env`、`EvalMode`（默认严格）、`EvalError`。
- 钉死语义：`sign(0)=0`、`mod` 为数学 floored 取模。

### 算子注册表（Phase 2a/2b/2c）
- 新增 `src/ops`：`OperatorSpec`（每算子一处定义求值 + 三语言代码模板）+ `as_operator`。
- 分批把 **52 个标量算子**迁入注册表；求值器与 `to_rust/to_python/to_latex` 共用，消除多处重复。
- 求值器完全注册表驱动，删除 `apply_scalar` 等中间机制。
- 修正跨目标不一致：Rust 的 `sign`（signum→sgn0=0）、`mod`（rem_euclid→floored）。

### 生成器确定性
- `EquationFile` 的 parameters/variables 与 `DagNode.metadata` 由 `HashMap` 改 `IndexMap`（新增 indexmap 依赖）。
- 同一二进制两次构建的生成产物由 54 个文件不同降为**逐字节一致**，输出可复现。

### 量纲系统（Phase 4a/4b）
- 新增 `src/units`：`Dimension`（7 SI 指数）、`parse_dimension`、量纲一致性检查（加减/比较/超越函数参数）。
- 升级为 `Unit{dim,scale,offset}` + `convert`/`affine_to`（含 °C↔K 仿射换算）。
- `check_coupling`：跨模块耦合接口单位检查（量纲不兼容 / 需换算 / 源缺失）。

### 特殊函数求值（Phase 3）
- `eval_special`：纯函数（factorial/logit/expit）+ `advanced_math` 下的 gamma/lgamma/digamma/beta/lbeta/erf/erfc/erfinv/正态分布。
- 顺手修 codegen bug：`to_rust` 的 digamma/lbeta 原引用 puruspe 不存在的函数，改用 statrs。

### CLI 与可视化
- `eqc check-dims <目录> [--strict]`：命令行跑量纲 + 耦合检查。
- `eqc report <目录> -o model.html`：自包含 HTML 报告——MathML 二维公式（浏览器原生）+ EQC 自生成 SVG DAG，零第三方、完全离线、约 10+ KB。

### 参数命名修复
- 原约定参数必须叫 `p1/p2`（`is_param_name` 只认 `p`+数字），有意义的名字（如 `Tbase`）会被当变量、引用检查报错。
- 新增 `EquationFile::reclassify_parameters`，在 `parse_file` 加载后把引用到参数名的 `Var` 重分类为 `Param`。现在参数可用**任意有意义的名字**。
- 新增真实示例 `examples/wofost.eq.yaml`（简化 WOFOST 作物生长模型，11 方程，含命名参数与单位）。

### 动态过程建模 arc（route B）
由「建草莓机理模型」驱动，让 EQC 能**运行**时间步进模型并画成 Forrester 图。

- **状态量元数据（B1）**：`Variable` 新增 `class`（8 类 Forrester `VarClass`，提到 `schema` 作单一真相源，`sexpr::workflow` 重导出）、`init`、`rate`（积分状态量 `X[n]=X[n-1]+rate[n]`）、`prev`（延迟寄存器 `X[n]=src[n-1]`）。`effective_class()` 自动推断。
- **逐日仿真引擎（B2）**：`src/sim`——`simulate(file,&SimInput)->SimOutput`，显式 Euler（dt=1 天），步内拓扑序求值，积分/延迟跨步，内置 `DAT`（天数），环/缺驱动校验，严格求值。CLI `eqc simulate --drivers w.csv [--params s.json] -o out.csv`。
- **同期群 cohort 宏展开**：`src/parser/cohort_expand.rs`——`cohorts:`/`cohort:`/`{ref:X,at:q}`/`{idx:q}`/`sum_over`，加载期对 `serde_yaml::Value` 展开成纯标量，引擎/AST/标量管线零改动。
- **Forrester 库存-流量图（B3）**：`report` 新增视图——存量(矩形)/速率(六边阀门)/驱动(椭圆)/参数(胶囊)/半状态(虚框)/边界(梯形)，物质流(速率→存量,橙粗线) vs 信息流(灰虚线)；复用 DAG 节点并补积分边。
- **验证器适配**：`DAT` 列为保留内置变量；跨步状态量（无方程）豁免「输出须有方程」检查。
- **首个动态模型**：`../strawberry_model/strawberry_v1.eq.yaml`——Sugiyama 2025 温室草莓源-库骨架（18 方程，cohort 果序×3/叶×12），合成天气下 `eqc simulate` 跑通一季，产量曲线单调、各果序按开花日激活。结构忠实、量级为合成演示（未对照论文验证）。

### 向量/矩阵 arc（可求值向量）
让求值器/仿真器能真正计算向量（cohort=向量），不只是生成 numpy 代码。设计见 `docs/spec-vector-matrix.md`。本期先把**向量**做透，矩阵 eval 后置。

- **V0 值类型 + 广播**：`eval` 从返回 `f64` 升级为 `Value{Scalar|Vector|Matrix}`；`Env` 改存 Value；加 `eval_scalar` 垫片（标量调用点用，语义不变）。注册表快路径 `broadcast_apply`：52 个标量算子**自动逐元素**（标量广播、同形状逐元素、不匹配报 `ShapeMismatch`）。`VectorLit/MatrixLit` 求值。
- **V1 向量算子**：新增 1 个 AST 节点 `Reduce{kind,arg}`（vsum/vprod/vmean/vmin/vmax）+ 补全 6 处穷尽 match；实现 `Reduce/Dot/Cross/VecNorm/VecNormalize` 的 eval（后四个 AST 早有、之前 `Unsupported`）；YAML/sexpr 解析接通。
- **V2 仿真向量化**：`Parameter.values`（向量参数）；`value_binop`（广播二元运算）；仿真器 Value 级——向量参数、向量状态量逐元素积分、跨步 `prev` 映射、输出展平成 `name[i]`。
- **V3 草莓向量版**：`strawberry_v1_vector.eq.yaml`——cohort 直接写成向量（果序 3、叶 12），**28 变量 / 19 方程**（标量宏展开版 92 / 66），**产量 Y=6.7058… 与标量版逐位一致**。图表/契约处理向量变量（Studio 勾选 `DF` 画其 3 条分量线）。修了一个真 bug：向量延迟寄存器首步标量 init 广播到来源形状（保证输出形状跨步一致）。
- 每期 `cargo test` 全绿、草莓标量 Y 不变（零回归）。矩阵 eval（matmul/det/inv/trace/eigen）仍 `Unsupported`（后置 V4，届时对齐 codegen）。

### EQC Studio 结构图布局 arc（可切换布局 + Forrester 学术风 + 缩放/专注）★ 重大更新
让结构图从"自上而下死板分层"升级为**可切换、像论文图一样可读**。用户（首席科学家）反馈原分层"又高又瘦、要狂滚、中间全是长线"。核心原则不变：布局算法全在 **EQC-Rust（算坐标 + 出 SVG）**，Studio 只负责切换/缩放，**离线保存的报告仍零 JS**。

- **布局接缝**：新增 `src/report/layout.rs`——`LayoutKind{Layered,Forrester,Force}` + `compute()` 把"算坐标"从渲染中抽出，Forrester 库存-流量图与角色 DAG 共用同一套布局。新增 `report::generate_report_with(files,dag,layout)`（`generate_report` 默认分层、向后兼容）。三处接入：HTTP `/api/report?layout=`、CLI `eqc report --layout`、Studio 顶部三档切换条（选择存 `localStorage`，自动刷新后保留）。自由布局（force/forrester）的边改用"框边到框边"的微弯曲线（`edge_path`/`box_exit`，端点裁到节点框边）。
- **力导向布局（force）**：Fruchterman-Reingold（斥力 `k²/d` + 沿边引力 `d²/k` + 经典边框约束）。**确定性**：初始位置按黄金角螺旋铺开、不用随机数 → 同输入永远同坐标、报告可复现。理想边长 ≈ 一个节点宽 → 连线短、紧凑、不稀疏。
- **Forrester 学术风（forrester）**：**存量/速率/边界排成横向主干**（按依赖层序，材料流左→右）；**辅助/参数/驱动作"卫星"**用力导向摆在主干上下两侧、就近其相连节点（主干钉死、只松弛卫星、各保持一侧不压主干线）；半状态（延迟寄存器 `X_prev`）作卫星浮放。纯静态模型（无主干）自动回退力导向。最贴近作物模型论文里的系统动力学结构图。
- **缩放 + 专注**：Studio 工具栏加 `−/适应/+` 缩放（伸进**同源 iframe** 设各结构图 SVG 显示宽度、容器滚动平移；`适应`=填满面板宽；比例存 `localStorage`）与 `⛶ 专注`（全屏只看结构图、隐藏右侧曲线、再点恢复双栏）。报告里结构图移出窄栏（`.wrap`，原限宽 1100px）→ 专注全屏时能占满整屏；公式仍留窄栏好读。
- **分层布局破环修复**：分层用最长路径算层号，但 Forrester 图的积分边（速率→存量）制造**环**，松弛沿环每轮 +1，把层号顶到数百层（S4 画布高 **37424px**，表现为"顶上一小团 + 拖一屏长线、看不到底"）。改为**先拓扑排序、只让"前向边"参与算层**（制造环的回边照常绘制、但不算层），层号被真实依赖深度限住 —— S4 高度 **37424 → 2004**。
- 测试：新增 layout 单测（解析往返 / 分层链 / 力导向确定性且有界 / Forrester 主干共线 / 含环不爆 / 无主干回退）+ serve `parse_layout` + studio 资产打包断言。**153 lib + 4 + 100 sexpr 全绿**。

## 工程基线
- 测试：153 lib + 4 + 100 sexpr，`cargo test --features cli`（含特殊函数时加 `advanced_math`）全绿。
- 远程：github.com/SongruiL/Eqc_test，SSH 推送。
- 文档：见 `docs/USAGE.md`（架构与模块地图）、`docs/spec-*.md`（设计规格）。

## 下一步（未做）/ 当前不足

**EQC 工具层**
- **交互式前端 EQC Studio（进行中，走「本地服务」路线）**：
  - Phase 1 `eqc serve`：监听模型、存盘即重生成、`localhost` 自动刷新（手写极小 HTTP，零新依赖）。
  - Phase 2（已完成）：**JSON 契约 + 整季仿真折线图**。新增 `eqc export`（导出模型 JSON 契约 `src/export.rs`，schema_version 版本化、只增不改、可检视）；`src/chart.rs`（EQC 自生成轨迹折线图 SVG，零图表库）；`eqc serve` 扩成 Studio：`--drivers/--params` + 端点 `/api/model`(JSON)、`/api/report`(HTML)、`/api/simulate`(轨迹 JSON)、`/api/chart.svg`(折线图)；前端页面 `src/serve_assets/studio.html`（打包进二进制，零构建步骤）——浏览器左看 Forrester 图+公式、右看整季产量曲线。`src/scenario.rs` 抽出驱动量/参数加载，simulate 与 serve 共用。
  - 原则：EQC 始终是唯一权威、前端只显示其 SVG/MathML/JSON 产物，契约只增不改 → 随 EQC 升级低风险、易排查。下一步：点节点高亮、浏览器内编辑、LLM 问答、GP 结构 diff。可后续包成 Tauri 桌面应用或 VS Code 扩展。
- **codegen 不生成积分循环**：`eqc build` 仍按静态网络生成代码，状态量（`state`）没有逐步更新代码——动态模型目前只能用 `eqc simulate`（树遍历）跑，不能导出独立可运行的 Python/Rust 仿真器。
- cohort 在图上显示为展开的标量（`DF__1/2/3`），未按家族分组显示。
- **GP 约束进化层**：核心愿景，但需要**完善的仿真模型 + 田间反馈数据**才有意义（fitness=跑仿真 vs 实测），属较远目标；动手前讨论可进化 vs 冻结节点、约束方式。
- 报告增强（按模块分图、点节点高亮、显示单位/出处）；codegen 死分支宏重构；耦合的时间尺度聚合（逐时 vs 日均）。

**草莓模型层**（详见 `../strawberry_model/strawberry_v1.eq.yaml` 顶部「模型短板」注释）
- 物候/发育周期、LAI 是**外部输入而非计算**（下一步接 BBCH 物候 + 叶面积子模型）。
- **无采摘/移除模拟**（果实留株、累积果重≡产量；缺采后库重释放与再分配反馈）。
- LUE 常数（不响应 CO₂/光强/发育）；缺基点温度；单品种；果实品质/糖度无机理；连续开花未按离散花序批次建模。
- 当前合成天气演示，量级未对照论文验证（需真实 'Benihoppe' 数据）。
