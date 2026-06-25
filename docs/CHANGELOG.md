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

### EQC Studio 节点交互（卫星智能分边 + 悬停注释 + 点击多选联动）
在布局基础上把结构图变"可问可玩"。**联动逻辑全在 Studio（同源 iframe），报告本身仍零 JS**（节点只带 `data-var`、公式块只带 `data-output` 数据属性 + 高亮 CSS）。

- **卫星智能分边**：Forrester 布局里卫星（辅助/参数/驱动）的上/下分配从"机械轮流"改为**局部搜索最小化"交叉+密度"代价**——直连卫星趋同侧（连线不穿主干）、两侧按 x 均衡。确定性（无随机数）。
- **悬停注释**：鼠标移到节点 → 浮出注释卡片（变量名·分类·单位 + 备注 description + 二维公式 MathML + 来源），内容全部取自 `/api/model` 契约（`EqJson.mathml` 直接复用），`pointer-events:none` 跟随定位、避开屏幕边缘。
- **点击多选联动**：点节点 = 切换选中（高亮节点+公式 + 画其轨迹）；再点取消；依次点多个曲线累加；与右栏复选框双向同步。**不再自动滚动**（注释已可见）。
- Rust 侧：节点 `<g>` 加 `data-var`、公式块加 `data-output`、高亮样式 `.hl`；契约/报告测试加断言（报告仍断言不含 `<script>`）。

### EQC Studio 情景探索器（浏览器调参数/初值 → 实时重算曲线）
让 `--drivers`/`--params` 不再启动时冻结：页面上拖滑块/填数值改**参数和状态量初值**，曲线**实时重算**。EQC 仍是唯一权威——"情景"只作查询参数传给服务端重算。
- **后端**：`SimInput` 加 `init_overrides`（状态量/延迟寄存器初值覆盖，优先于变量上的 `init:`）；`serve.rs` 给 `/api/chart.svg` 与 `/api/simulate` 解析 `p=name:val,…`（参数）与 `init=name:val,…`（初值），叠加在启动级 `--params` 之上传入仿真。
- **契约**：`ParamJson` 加 `values` 字段（向量参数的各分量），前端据此**跳过向量参数**（cohort 种子不可被标量覆盖）。
- **前端**：从 `/api/model` 自动生成情景面板（标量参数 + 状态量初值，各一行滑块+数值框），改动防抖 150ms 重设 `chart.svg` 的 src 实时重绘；改过标蓝；「重置默认」。复用现有 `<img>` 图表机制，无新增渲染。
- 后续可做（暂缓）：滑块合理范围/步长、叠加"基线曲线"对比、天气（drivers）整体缩放旋钮。
- 测试 154 lib + 4 + 100 全绿。

### EQC Studio 结构图拖拽（画布平移 + 节点移动）
结构图上三种鼠标操作，按落点与位移阈值自动区分（拖动超阈值抑制点击、避免误选）：
- **拖空白 = 平移画布**：横向滚 `.dag`、纵向滚 iframe 窗口（像看地图）。
- **拖节点方框 = 移动它，连线实时跟随**：用来手动错开自动布局难免的遮挡。**会话内有效**，刷新/换布局复位，不写回模型。
- **轻点节点 = 选中**（高亮 + 画轨迹），不动就不算拖。
- 实现：Rust 给节点 `<g>` 加 `data-id`/`data-cx,cy,hw,hh`、边 `<path>` 加 `data-from`/`data-to`（报告仍零 JS）；Studio 用 SVG `getScreenCTM` 把屏幕位移换算成用户坐标（兼容缩放），拖动时重算该节点的连线（中心到框边 + 微弯，与 Rust 自由布局一致）。

### CLI `eqc sweep`：参数扫描 + 全局敏感性
让科学家直接问"参数怎么影响输出"，也是 GP/优化的铺路石（fitness = 跑仿真看输出）。纯复用 `sim::simulate`。
- **单参数扫描**：`eqc sweep <模型> --drivers w.csv --param LUE --range 1.0:5.0:9 --var Y [--reduce final|max|mean|min] [--params base.json] [--steps N] -o sweep.csv` —— 把参数在区间取 N 点各跑一次，输出对某变量的响应 CSV；结尾打印响应范围 + 最大值位置。
- **全局敏感性（OAT）**：`--sensitivity --var Y [--percent 10]` —— 每个标量参数各 ±percent% 各跑一遍，按对 `var` 的影响（Δvar + 归一化弹性）从大到小排序输出 CSV，一眼看出"哪个参数最关键"。向量参数（cohort 种子）与默认值为 0 的参数自动跳过。
- 校验：扫描的参数须是标量参数（向量参数报错并提示）；输出变量须在轨迹里（向量用 `名[1]`）。
- 草莓 S4 实测：LUE（弹性 +1.03）、DMC（−1.01）主导产量；Kc 在当前工况下对 Y 无影响。
- 新增 `parse_range`/`reduce_series` 单测（main bin）。

### `eqc build` 生成积分循环：动态模型 → 可独立运行的 Python 仿真器
补上动态模型「单一真相源 → 可部署代码」这条断掉的承诺（此前 `eqc build` 按静态网络生成、状态量无更新代码）。
- **步进计划抽象（单一真相源）**：`sim` 新增 pub `build_plan(file) -> SimPlan`——拓扑序的步内计算（`PlanStep::{Equation,Integrator}`）+ 延迟寄存器 + 驱动量清单。`simulate`（树遍历引擎）与代码生成器**共用同一份计划** → 生成代码与引擎逐步一致（correct-by-construction）。`simulate` 据此重构（初值覆盖改到「用时点」应用），草莓 Y 逐位不变（零回归）。
- **Python 仿真器生成**（`generators/python_sim.rs`）：动态模型经 `eqc build --format python` 额外输出 `{id}_sim.py`——可独立运行的 `simulate(drivers, steps[, params][, init])`：逐日显式 Euler、`X[n]=X[n-1]+rate[n]`、延迟寄存器、内置 `DAT`，方程体复用 `Expr::to_python`；带 `__main__`（读驱动量 CSV、打印各变量末值，便于对拍）。
- **向量模型支持（numpy）**：向量参数生成为 `np.array`、记录用 `_rec` 展平为 `name[i]`（与引擎 flatten 一致）、首步延迟寄存器按来源形状广播（复刻引擎 step-0 reshape）。同一份生成器标量/向量通吃。
- **验证**（本机 Python 3.10，与 `eqc simulate` 对拍）：标量版 `strawberry_v1`——**Y 逐位完全一致（6.7058324979969655）**，92 变量末值 73 个逐位相同、其余 <1e-9；向量版 `strawberry_s4`——156 变量末值 112 个逐位相同、其余 <1e-9，**0 不一致**（差异皆 numpy vs Rust 超越函数末位）。
- **Rust 目标后置**。生成器单测覆盖动态/静态判定与关键结构。

### 优化层 arc（阶段 1：决策优化 = 仿真优化）
从「当前条件下系统会怎样」迈向「想要某种结果该怎么做」。不解析反推方程（机理模型含阈值/分段无法求逆），而是**在前向模型上搜索**：试一组决策 → 跑仿真 → 打分 → 调整再试。设计见 `docs/spec-optimization.md`。三层架构：搜索算法 / **目标评估核** / 前向模型（解释器）。新增 `src/optimize`。

- **时间归约词汇**（`objective.rs`）：`final/at/max/min/mean/total` 作用于 `SimOutput` 一条整季轨迹（区别于逐日算子与 `vsum`）。`eval_objective(expr, &SimOutput, &bindings)` 复用现有解析器/AST/求值器——先 `sexpr::parse`，在 **SExpr 层**把每个归约子式就地替换成数，再 `convert`+`eval` 求剩下的纯算术。**不新增 AST 变体**（不污染 360 变体枚举、不动三个 codegen），也避开与逐元素 `max/min` 的命名冲突（消歧：`final/at/total/mean` 是归约专用词；`max/min` 仅当 `(max 单轨迹变量)` 时作归约）。
- **决策 spec**（`problem.rs`）：与模型**分离**的独立产物（「可控」是场景属性而非变量固有属性）。YAML 顶层 `optimize:`——`objective{expr,sense}` + `knobs[{var,kind,bounds,unit}]`（`kind∈{param,init,driver_const}`，阶段 1 仅标量）+ `constants` + `constraints[{expr,max}]` + `environment` + `optimizer{method,pop,iters,seed}`。
- **目标评估核**（`core.rs`）：`evaluate(file, problem, &knob_values, &drivers, steps) -> EvalOutcome`——装配 `SimInput`（param→覆盖 / init→初值覆盖 / driver_const→整列常数）→ `simulate` → 目标归约 + sense + 约束惩罚（`expr≤max` 线性外罚）→ 代价。**绑定**：模型标量参数默认值 ← 常量 ← 旋钮当前值（旋钮优先），故目标里 `Pd`（也是旋钮）取试验值、单价/成本取常量。**鲁棒**：发散/缺驱动/非有限/求值失败一律给 `WORST_COST=1e18` 不崩。`validate_problem` 跑前校验旋钮种类/边界。决策优化 / 参数标定 / 未来 GP-fitness **共用这一层**。
- **差分进化 DE**（`de.rs`）：免导数、对非光滑/阈值/多峰鲁棒。手搓确定性 PRNG（SplitMix64，不引入 `rand` 依赖）→ 同 `seed` + 确定性代价 = **逐位可复现**。DE/rand/1/bin + 边界钳制；`DeResult{best_x,best_cost,history}`。
- **CLI**：`eqc optimize <模型> --spec problem.yaml [--drivers w.csv] [--steps N] [-o result.json]`——读模型+决策 spec → DE 搜旋钮 → 打印最优旋钮/目标值/可行性/收敛、写结果 JSON。
- **草莓 S4 验证**（`../strawberry_model/optimize_s4*.yaml`）：
  - 最大化产量（旋钮 CO₂ driver_const + Pd param）→ CO₂=1200、Pd=12（均顶界）、Y=10.9503 kg/m²。
  - **交叉核对 (a)**：Pd-only 优化最优 Pd=12、Y=**7.561201**，与 `eqc sweep --param Pd --range 4:12:17` 网格 argmax **逐位一致**。
  - **交叉核对 (b)**：用独立 `eqc simulate`（CO₂≡1200 常列 + Pd=12 覆盖）复现最优点 Y=**10.950268524327067**，与优化器目标值**逐位一致**（验证 driver_const + param + final 归约一致）。
  - **利润变体** `(sub (mul (final Y) price) (mul CO2 co2_cost))`：最优 CO₂=**757 ppm（内点**，成本项把它从 1200 拉回）、利润 199.87，比两边界各高 8–11%——证明优化器响应目标**结构**而非顶界。端到端**可复现**（重跑同结果）。
- 单测 26 个（objective 6 + problem/core 8 + de 6 + 已计入）覆盖归约/算术/边界、评估核（最大化/driver_const/min+常量/约束惩罚/垃圾候选不崩/validate）、DE（Sphere/Rosenbrock 收敛/同种子可复现/边界/单调/零维）。
- **阶段 2-A 约束一等公民**（`feb22ad` + 本次）：核里把约束从「只有总 penalty」做细——`ConstraintStatus{expr,value,max,violation}`、`EvalOutcome.constraints` 逐约束明细；惩罚权重可经决策 spec `penalty_weight:` 覆盖（默认 `DEFAULT_PENALTY_WEIGHT=1e9`）。`eqc optimize` 控制台 + result JSON **逐约束报告**满足/违反与违反量、标记整体可行性。S4 算例：约束 `max(LAI) ≤ 10`（涌现量，LAI=SLA·WLV·Pd·1e-4）→ 最优从无约束的 CO₂1200/Pd12（Y10.95、峰值LAI12.66）推到 **CO₂≈681/Pd=4（Y9.36、峰值LAI 恰好=10、可行）**——优化器自行权衡「Pd 直接抬 LAI、CO₂ 增产对 LAI 代价更小」。约束值 `max(LAI)=10` 与独立 `eqc simulate` 逐位一致。
- **阶段 2-B Studio 可视化优化**（B1 `0603623` + 本次）：抽 `optimize::run` + `result_json` 库函数（CLI 与 serve **共用同一计算与 JSON**）；serve 新增 `/api/optimize?spec=` 端点。`chart.rs` 加 `convergence_chart_svg`（代价 vs 代数，EQC 自生成 SVG），端点响应注入 `convergence_svg`（CLI 写文件的 result_json 保持纯数据）。`/api/chart.svg`/`/api/simulate` 加 `d=name:val` **驱动常量覆盖**（driver_const 旋钮的最优轨迹靠它叠加）。Studio 底部「决策优化」面板：填 spec → 转圈跑 DE → 显示最优旋钮/目标/可行性/逐约束 + 收敛曲线 → 「叠加最优旋钮到曲线」把最优喂回情景画最优整季轨迹。性能记录：S4 spec debug 107s / release 32.5s（解释器为瓶颈，提速作为独立后续）；UX 按「触发→转圈→出结果」。
- **解释器提速**（`2fabf2c` P1 + `735fc00` P2）：P1 去掉热路径每次求值的 `env.clone()`（新增 `Expr::eval_in` 就地求值，sim 逐方程改用它）→ **optimize 32.5s→15.8s（~2×）**，惠及 sim/sweep/optimize/未来 GP-fitness。P2 sim 跨步复用 env（`Env::put` get_mut 避免键重分配）→ ~4%（噪声内，String 分配并非次瓶颈，剩余开销在树遍历本身）。草莓 Y 逐位不变。
- **阶段 2-D 多目标雏形**（D1 `e8557b3` + 本次 D2/D3）：spec 加可选 `objective2` → 多目标模式。`evaluate_mo`（双目标代价向量，惩罚加到每目标）；`differential_evolution_mo`——**单次 MO-DE** = DE/rand/1/bin + Pareto 支配选择 + 非支配存档 + 拥挤度截断到 40 点（解决单调权衡下存档无界膨胀），一次运行近似整条前沿、确定性。`run_mo`/`mo_result_json`；CLI 检测 `objective2` 打印前沿表。`chart.rs` 加 `pareto_chart_svg`（散点+连线，每点 `data-i` 可点选）；serve `/api/optimize` 多目标分支注入 `pareto_svg`；Studio 优化面板画前沿、点选某点即叠加该点整季轨迹（复用 `applyBestKnobs`）。S4 实测（产量最大 vs CO₂ 用量最小）：40 点光滑前沿，(Y10.95,CO₂用量288000)@1200 → (Y7.56,96000)@400，Pd 全程 12。
- **阶段 2-C 敏感性自动预筛**（本次）：`optimize::prescreen`——优化前对每个旋钮在基线（边界中点）±10% 各扰动一次、看**目标** `|Δobj|`，< `rel`×最大变化者判低敏感。`eqc optimize --prescreen`（单目标）把低敏感旋钮**固定在基线**（边界收拢）、只搜敏感旋钮。与 `eqc sweep --sensitivity` 同思路但作用于旋钮（含 init/driver_const）+ 目标。S4 实测：Pd 对产量 |Δ|=0.000271（相对 CO₂ 的 0.0005）→ 自动固定于基线 8，最优 Y=10.94998 vs 不预筛 10.95027（差 0.003%）。
- **阶段 2 完成（A 约束 + B 可视化 + 解释器提速 + D 多目标 + C 预筛）。**
- **后置**（spec §8 阶段 3+）：曲线参数化时变控制、离散旋钮、**参数标定（接田间数据）**、其它优化器（CMA-ES/贝叶斯）；解释器进一步提速（换更快哈希/编译成扁平形式）；更远是 GP 约束进化层（复用本评估核当 fitness 引擎）。

### 参数标定层（Cal arc，由云南 2026-07 栽培实验驱动）
让模型可信的关键一步、也是通往 GP 的桥：用实测数据反推参数。与决策优化**同一外循环、共用评估核**，只换「旋钮=参数、目标=误差」。设计见 `docs/spec-calibration.md`。
- **Cal-1 误差算子**（`0465cca`）：`objective.rs` 加 `rmse/mae/nse/bias`——把仿真序列与实测序列逐(观测)日比对归约成标量；写法 `(rmse Y obs_Y)`，可多变量加权组合。沿用时间归约套路在 SExpr 层替换、复用 convert+eval，不新增 AST 变体。实测稀疏（`名→[(天,值)]`）。`eval_objective_obs`（带实测）/`eval_objective`（不带，决策优化照旧）。
- **Cal-2 贯通 + CLI**（本次）：`core.rs` 用 wrapper 模式把实测贯通评估核（`evaluate_obs`/`evaluate_mo_obs`/`prepare(observed)`，决策优化的 `evaluate` 等仍是零实测 wrapper、零改动）；`run_obs` 同理。`scenario.rs::load_observed_csv`（稀疏，首列 DAT）；`Problem.observed` 字段。新 CLI `eqc calibrate <模型> --spec calib.yaml [--drivers w.csv] [--observed obs.csv]`（底层复用 `run_obs`）。**recover-the-params 验证**：单测（合成 gain=3 找回）+ 端到端（S4 用 LUE=4.0 造伪实测 → `eqc calibrate` 找回 LUE=4.000000、误差 0）。
- **Cal-4 可辨识性/「该测什么」助手**（本次）：`optimize::identifiability`——对每个候选参数 ±10% 扰动，量其对每个候选可观测变量整条轨迹的**相对** RMS 影响（÷ 基线 RMS，跨量级可比）→ 敏感矩阵。`eqc identify <模型> --spec calib.yaml [--observables Y,LAI]`：报告「参数→最该测的观测」、不可辨识参数（无观测能约束）、可能异参同效的参数对（敏感模式皮尔逊相关 >0.99）+ 测量清单。`core.rs::simulate_candidate`（pub）；`Problem.observables` 字段。S4 实测洞见：LUE 测谁都行（全局乘子）；Pd 必须测 F（Y=F·Pd/1000 使 Y 几乎不随 Pd → 只测 Y 标不出 Pd）；Kc 在 CO₂≡400(=Cref) 下 f_CO2≡1 不可辨识（需 CO₂ 处理梯度）。2 个单测。
- 后续：数据到位后真标 S4 / 带根系-水肥的新模型。

### DAG 多层级 + 多模型工作区 arc（结构图三粒度 + 免重启切模型）
首席科学家驱动：参数级 DAG 对复杂模型太眼花，想按方程/模块级看；并最终把温室↔作物耦合成一张大图。
- **step 1 三粒度**（前序）：`Metadata.modules`（模型声明子模块）+ `DagLevel{Variable,Equation,Module}`/`collapse_dag` + `report::generate_report_leveled` + serve `/api/report?level=` + Studio 粒度选择器 + 友好中文节点名 + 按模块上色图例 + 角色图 Forrester 主干布局。
- **step 2 多模型选择器（免重启切模型）**（本次）：`eqc serve` 指向 **工作区清单 `eqc-workspace.yaml`**（或含它的目录）→ Studio 顶部模型下拉切草莓/番茄/蓝莓/温室，免重启，与粒度/布局/处理区组合。`serve.rs`：`Ctx`→模型花名册 `Vec<ModelEntry>{id,name,path,drivers,params,data_dir}`（单/多模型统一，单模型=1 条、行为逐位不变），`resolve_model` 按 `?model=<id>`，新端点 `/api/models`，全部 model-bound 端点接 `model=`，每模型实测数据隔离 `observations/<id>/`。用**显式清单**而非目录扫描——作物目录是版本史（s1..s8）且每模型驱动不同。`studio.html`：顶部 `<select>` 选择器（多模型才显示）、`loadModels`/`applyModel`/`modelParam`、`model=` 串入 10 个 fetch + boot 先取花名册。活体：4 模型仿真全跑通、`?model=` 正确切换、单模型零回归。
- **step 3 耦合视图机制（3a，本次）**：清单 `couplings:` → serve 加载耦合条目时**同时**加载参与文件、按 `links` **在内存里**给作物 Input 注入 `source`（不落盘）→ `build_dag` 产出跨模型边 → 因两模块都在场、validator 通过。**关键避坑**：直接把 `source:` 写进 canonical 作物模型会让其单独 serve 时 validate 失败（`UndefinedReference{模块}`，因温室没一起加载）→ 砸园区视图；故走**清单叠加层 + 内存注入**，canonical 零改动。`ModelEntry.coupling`、`load_model_files`、`coupled_guard`（仿真/录入/标定对耦合条目友好拦截）、`/api/models` 带 `coupled` 标记、`render_report` 改吃 `&[EquationFile]`。前端：选择器「耦合视图」optgroup + 选中时标题/提示。温室×蓝莓验证：合成图画出 `T_air→T`/`Q_sun→Sr` 两条跨模型边、canonical 蓝莓磁盘 `source:`=0 仍仿真。
- **step 3b 模块标注 + auto-模块按模型分前缀**：模型侧给 greenhouse_v1（5 模块）/blueberry_bb5（6 模块，50 eq 全覆盖、0「其他」）补 `meta.modules`（在 greenhouse-model/crop-models 库）。引擎侧 `compute_submodules`：**多模型（耦合视图）时给通用 auto-模块（驱动量/参数/其他）加模型前缀**（`GREENHOUSE_V1·驱动量` ≠ `BLUEBERRY_BB5·驱动量`），否则两模型 driver 层会按字符串折叠成一个 hub；命名模块本就不撞、不加前缀；单模型（1 文件）不加前缀=零回归。结果：耦合模块级跨模型桥清爽——`温度/辐射 → BLUEBERRY_BB5·驱动量 → 物候/冠层光合`（温室气候→蓝莓），温室自身室外输入在独立的 `GREENHOUSE_V1·驱动量`。
- **耦合视图= 仅结构图**；耦合仿真（温室输出实时喂作物 + 跨时间尺度）是另一大引擎特性，deferred。

### 杂项修复
- **`eqc validate` / `eqc report` 接受单文件**：原先无条件 `load_directory`，对单个 `.eq.yaml` 撞 `read_dir` 的「目录名无效」os-267；改为 `input.is_file()` → `load_file`、否则 `load_directory`（与 `eqc serve` 一致）。
- **温室 `greenhouse_v1_ctrl` 季节门控 `Numeric×Boolean` 修复**（在 greenhouse-model 库）：GH-HEATSP/VENTSP/PHIINJ 的 `force+(sp−force)·lt(…)` / `phi·geq(…)` 改写为 `if(cond, then, else)` → 类型干净（之前 `validate` 报 3 错、只能 `simulate`）；**逐位一致**（lt/geq∈{0,1}，if-select ≡ 0/1 乘），优化脚本不受影响。耦合视图现可换上带控温的温室。

### 耦合仿真 C1（多速率、单向）—— 见 docs/spec-coupled-simulation.md
温室（快、秒级）↔ 作物（慢、日级）一次集成运行的地基。规范 D1–D6 已定。
- **C1a `Stepper` 重构**：把单模型每步逻辑（DAT→驱动→延迟→拓扑序方程/积分→快照 prev）抽成可复用 `Stepper`，`simulate` 变薄封装。耦合与单模型逐步共用同一步进=单一真相源。**零回归**：草莓 v1 Y=6.7058324979969655 逐位一致、S4 向量路径 SHA1 相同。
- **C1b 多速率耦合**：`Metadata.dt_seconds`（各模型自描述步长折秒：温室 10、日级作物 86400；耦合统一到秒，D1）。`sim::simulate_coupled`：双 `Stepper`，每慢步跑 R=dt_slow秒/dt_fast秒 个快步，快→慢 `mean`/`integral`（带 scale 单位换算）聚合喂作物，无反馈（C1 单向）。`Agg`/`CoupledLink`/`CoupledInput`/`CoupledOutput` + `SimError::Coupling`。CLI `eqc couple --fast --slow --weather --link to=from[:agg[:scale]]`。
- **验证**：机制单测（R/mean/integral·dt_fast 解析命中）；金标准——温室→蓝莓 `eqc couple` 内联聚合的 T/Sr 与全精度 `aggregate_to_daily` 逻辑**逐位一致（最大绝对差 0）**，即一次集成运行复现离线两趟管道。214 lib + 3 + 4 + 100 全绿。
### 耦合仿真 C2（双向滞后反馈，先只 CO₂）
作物回拉温室 CO₂ → 闭环。单向离线管道做不到。
- **C2a 引擎**：`simulate_coupled` 加 `feedback`（慢→快 hold，**滞后一慢步**：本慢步用作物上一步值、首步 init、日末更新——`_prev` 破环抬到耦合界面，无步内代数环）+ `FeedbackLink` + CLI `--feedback to=from[:scale[:init]]`。合成测试坐实滞后。
- **C2b 引擎+建模**：`CoupledOutput.fast`（温室变量日均聚合）+ CLI `--fast-out`/`--fast-params`/`--slow-params`（温室控制旋钮，C3 优化要用）。温室变体 `greenhouse_v1_crop.eq.yaml`：`phi_ass` 改成由作物喂——`phi_ass=co2_uptake_in/(n_air·h_gh)`（复用 h_gh、n_air≈41.6）。番茄 T3：`dt_seconds=3600` + 接口 `co2_uptake_inst=(MC_AirBuf−全部回气呼吸)×(1000/30)`（净同化−生长/维持呼吸，mg CH2O/m²/s→µmol CO₂/m²/s）。
- **验证（闭环 A/B，温室×番茄）**：白天反馈开→温室 CO₂ −5 ppm（作物吃 CO₂）、夜间 +0.9（呼吸释放）；CO₂ 降→番茄光合降（P_gross ON 261.5<OFF 263.2）=**自限双向效应**（单向管道做不到）。稳态 phi_inj−phi_ass=vent·(CO₂−CO₂_out) 验证 ON≈400/OFF=405。
### 耦合仿真 C3（耦合优化 = "一个循环里"）—— 最小可用
把 `simulate_coupled`（双向）包成 `eqc optimize` 的前向模型：DE 搜温室/作物参数旋钮、目标归约**作物轨迹**。= `optimize_force_de.py` 在 Python 外面用离线管道做的事，搬进 EQC 一个进程、一份声明式 spec、用测过的 DE。
- **复用**：`de::differential_evolution`（通用）+ `objective::eval_objective`（归约对象换成作物 slow 轨迹、裸符号绑旋钮）。**新建** `optimize/coupled.rs`：`run_coupled`/`CoupledModel`/`CoupledOptimizeResult`（室外天气只克隆一次，每评估仅改 params）。
- **spec**：决策 spec 加 `coupling:` 块（`fast/slow/weather/links/feedback/steps/fast_params/slow_params`，路径相对 spec 目录）；旋钮 `kind: fast_param/slow_param`；`eqc optimize <任意> --spec coupled.yaml` 自动走耦合路径（`run_optimize_coupled`）。`KnobKind::FastParam/SlowParam`（单模型路径拒绝）。
- **范围 v1**：单目标、无约束（约束/Pareto/Studio 面板复用 = 后续，需把 `core` 抽象成前向模型无关）。
- **验证（演示）**：温室×番茄，旋钮=CO₂ 注入 phi_inj，目标 `(sub (final TDM)(mul phi_inj co2cost))`。先 `eqc couple` 扫 TDM-vs-phi_inj 定 co2cost=2.0 → DE 找到**内部最优 phi_inj=0.102 ppm/s**（= 边际 TDM 增益 = co2cost 的交点，在双向前向模型上搜）。release 23s vs debug 2m44s（优化层用 release）。+2 单测（最优/拒绝坏旋钮）。217 lib 全绿。
- **耦合仿真/优化接进 Studio（C1–C3 上界面）**：清单耦合条目升级为**可仿真**（`fast`/`slow`/`weather`/`links`(带 agg)/`feedback`/`fast_params`）；`fast` 存在即启用。serve：`Coupling.sim`、`/api/couple`（跑 `simulate_coupled` → 作物+温室合成轨迹）、`/api/couple.svg`（轨迹图）、`/api/couple-optimize`（spec 的 knobs/objective → `run_coupled` DE）、`/api/models` 加 `sim_capable` 标记。**视图升级**：可仿真耦合的结构图用 fast/slow + **links 和 feedback** 注入 source → 画出**双向边**（含作物→温室反馈边），`greenhouse_v1_crop` 因 co2_uptake_in 被反馈 source 化而校验通过。studio.html：选中可仿真耦合 → 「耦合仿真/优化」面板（跑仿真画温室 CO₂+作物轨迹、跑优化显示最优旋钮+收敛），隐藏单模型轨迹/决策面板。视图专用耦合（蓝莓/草莓，无反馈）保持现状。活体：温室×番茄 `/api/couple` 48 步、`/api/couple-optimize` 找到 phi_inj=0.102（=CLI 一致）。217 lib 全绿、node --check 过。
- **下一步 C4（后续）**：紧耦合（步内迭代 / sub-day 作物快通量）去滞后、提精度——仅当滞后误差被证明要紧时。

### 友好显示名（图表勾选框/图例/DAG 统一显中文，代号进 hover）
首席科学家反馈：图表变量勾选框、参数级 DAG 节点显的是**代号**（`rate_CBuf`/`t_FGP_1`/`温室:phi_ass`），非数学用户看不懂。原 `build_dag` 后置已为 DAG 节点算友好标签，但 JSON 契约没暴露这个"显示名"，故图表/勾选框只能用代号。原则：**EQC 单一权威拥有显示名**（一处优先级逻辑），前端只显示。
- **`EquationFile::display_name(name)` 单一权威**（`schema/equation_file.rs`）：优先级 `变量 label → 方程中文名 → 参数 name_cn → 延迟寄存器 prev 派生「源（上一步）」→ 代号`。`build_dag` 后置改调它（删去重复的 `compute_eqnames`，DAG 标签行为逐位不变）。
- **契约暴露**：`VarJson`/`ParamJson` 各加 `display_name` 字段（恒有值，未标注时 == 代号）；`eqc export` JSON 即带出。
- **cohort 友好名**：`cohort_expand` 复制成员时给 `label` 追加 `[i]`（与向量分量 `name[i]` 风格一致），故 cohort 基变量标一次 → 各分量 `果碳[1]`…`果碳[10]` 各不同名；与 prev 派生可组合（`果碳[3]（上一步）`）。
- **图表 SVG 图例友好**（静态 SVG 无 hover）：`line_chart_svg` 加 `label_of` 解析器；serve 新增 `trajectory_label`（Rust 版，保留 `温室:` 前缀 + `[i]` 后缀、中间按契约 `display_name` 翻译），`/api/chart.svg`/`/api/couple.svg` 传入。
- **前端薄展示**（`studio.html`）：勾选框/耦合勾选框显 `display_name`、代号进 `title` hover；`displayNameOf`/`coupleLabel` 翻译耦合轨迹键。
- **逐作物补标注**（在 greenhouse-model / crop-models 库）：草莓 S8 / 番茄 T3 / 蓝莓 BB5 + 温室 v1/crop 给所有兜底基变量补 `label`（驱动量 + 状态量 + cohort 基量）。五模型 `eqc export` 兜底全部清零。
- 测试：新增 `display_name` 四级优先级 + prev 派生 + cohort label `[i]` + 图例友好名共 5 个单测。**220 lib + 3 bin + 4 + 100 sexpr 两配置全绿**；加 label 不影响仿真数值（草莓/番茄冒烟一致）。

## 工程基线
- 测试：220 lib + 3 bin + 4 + 100 sexpr，`cargo test --features cli`（含特殊函数时加 `advanced_math`）全绿。
- 远程：github.com/SongruiL/Eqc_test，SSH 推送。
- 文档：见 `docs/USAGE.md`（架构与模块地图）、`docs/spec-*.md`（设计规格）。

### 受约束遗传编程（GP）arc（v0.8 / v0.9 + 联合进化）
核心愿景落地：在机理骨架的「假设留白（🟠）」处进化方程结构。设计 `docs/spec-genetic-programming.md`；进化-冻结边界来自理论溯源逐方程分类（🟢/🔵 冻结、🟠 进化）。新增 `src/gp/`：
- **G0** `gp_target` 进化靶点元数据（`Equation.gp_target`，additive 契约字段；溯源标签 → 机读边界；蓝莓 3 靶回填）。
- **G1** 5 套受约束语法（monotone_gate / saturating_sink / allocation_fraction / temperature_response / growth_curve，按构造保证单调/有界）+ 量纲软过滤 + 数值先验检查。
- **G2** 树遗传算子（变异/交叉/常数扰动）+ **`Candidate{骨架, 可调常数向量}`** 基因组（结构常数固定、可调常数 `__c{i}` 占位分离）；算子契约：合法父代 → 必返合法后代（重过滤 + 回退）。
- **G3** GP 主循环（co-evolve，泛型于误差闭包）+ 模型级适应度（patch 候选 → 仿真 → rmse vs 观测）+ **`eqc evolve` CLI** + 合成复原验收。
- **G4** 多目标 Pareto（精度 vs 复杂度，NSGA-II 式非支配 + 拥挤截断）+ memetic（内层 DE 标定候选常数向量）。
- **G5** 进化式回流溯源：识别 GP 选了哪种机理形式、判 rediscovery（撞现有形式 = 机理验证 → 升 🟠→🟢/🔵）、自动生成溯源条目草稿。
- **多槽位联合进化**（`src/gp/joint.rs`）：基因组 = 一组候选（每槽一棵树）；`patch_multi`（各槽 patch 进对应方程，常数命名空间化 `__s{k}_c{i}` 防撞）→ `evaluate_multi`（**一次仿真**对所有观测变量算误差）→ 捕捉槽位间耦合（单槽独立做不到）。`evolve_joint`（单目标）。
- **Pareto-joint**（`evolve_joint_pareto`）：联合进化 + (总精度, 总复杂度) 前沿——整模型尺度的"精度 vs 简洁"权衡，每点 = 一套形式。`pareto.rs` 抽出按目标值的 NSGA-II 助手（单槽 + 联合共用）；前沿去重。
- 全程合成数据验证（端到端：从观测复原已知形式，结构 + 常数近乎精确；联合：两槽同时复原）；复用 `optimize` 评估核/DE、`sim`、`units`、S-expr = 基因组。**真进化解锁 = 云南 2026-07 田间数据**。
### EQC Studio · GP 面板（前端 S1–S5 + B2）
human-in-the-loop——GP 提议、科学家裁决。设计 `docs/spec-gp-studio.md`；EQC 持有事实（`/api/evolve*` 契约），前端只拼装、大量复用已有积木（pareto/convergence 图、MathML、园区录入网格、轨迹叠加）。契约只增不改。
- **S1** `/api/evolve` 同步端点：薄编排 `evolve_pareto` + `form_report` + `patch_model`/`simulate` → Pareto 前沿 JSON（每点公式/轨迹/机理形式/rediscovery + baseline 对比 + 观测散点 + 前沿散点 SVG）。靶点元数据从模型 `gp_target` 自动取，前端只递交「选哪个靶 + 几个旋钮」。
- **S2** studio.html GP 面板（专家视图）：靶点列表 → 选 → 配置 → Pareto 散点（点选）→ 候选详情（公式 MathML + 自画拟合叠观测图 + rediscovery 徽章）。契约加 `formula_mathml`。
- **B2** 自动 rediscovery：`identify_form_of_expr` + `concrete_matches_skeleton`（`__c` 占位匹配字面常数/参数）→ 引擎自动识别现有方程的机理形式作 baseline_form，徽章自动点亮，无需手填。
- **S3** 对比 + 采纳：候选 vs 现有形式并排（rmse/复杂度）；「采纳此候选」→ 生成可编辑的**溯源条目草稿**（markdown）+ 可粘贴的 **`.eq.yaml` 方程片段**（常数代回字面值，`eqc validate` 回环通过），复制/下载，**只产文本不写盘**。顺手修 `to_yaml` 漏 `Clamp` 的真 bug。
- **S4** 异步任务：`/api/evolve/start` 起后台线程→`{task_id}`、`/api/evolve/status` 轮询（当前代 + 实时收敛曲线，完成内嵌结果）；放开 memetic + 大规模（同步会超时）。引擎加 `evolve_pareto_cb`/`evolve_joint_pareto_cb`（每代进度回调，原函数 no-op 委托、零回归）。
- **S5** 多槽位联合进化前端：靶点**多选**（≥2 = 联合，`targets=`）；候选详情按**槽位分区**，每槽各自公式/形式/rediscovery/拟合/采纳；Pareto 复杂度轴 = 各槽之和。
- 验证：合成 demo（gpdemo 单靶 / gpdemo2 双门控联合）端到端 LIVE——start→轮询→done，逐槽 rediscovery、采纳片段回环。254 lib + 3 bin + 4 + 100 sexpr 两配置全绿。**剩：真进化（等云南 2026-07 田间数据）。**

### EQC Studio · 前端 LLM Agent「问AI」（v2，`/v2`）
用自然语言指挥整个前端——不止对话，能**执行命令**（导航/查模型/调情景参数/跑仿真/切模型/写处理区设置）。架构 = **命令注册表 = 前端能力唯一真相源**：⌘K 面板与 AI 工具都从 `frontend/src/lib/commands.svelte.ts` 派生，**加一条命令 = 面板按钮 + AI 能力同获**，零额外胶水。同构 EQC「契约只增不改、UI 是契约派生」。
- **后端薄代理**：`src/serve.rs` `POST /api/llm` —— 前端组完整 Anthropic 请求体（model/system+cache_control/tools/messages），后端只**注入 key + 转发** Claude（key 绝不下发浏览器）；失败统一返回 Anthropic 风格 `{type:error,...}` 信封。新增出站 HTTPS 客户端 `ureq`（进 `cli` feature，rustls 纯 Rust TLS、自带根证书、契合手写阻塞 server）。
- **凭证 = 本地密钥文件**：`load_secret_file()` 启动读 gitignored `.eqc-secret`（`KEY=VALUE`：`ANTHROPIC_API_KEY` + 可选 `EQC_LLM_PROXY`/`EQC_LLM_MODEL`），只设尚未存在的 env、真 env 优先；模板 `eqc-secret.example`。`EQC_LLM_PROXY` 让直连被墙的机器走本地代理；`EQC_LLM_MODEL` 后端覆盖请求体 model = 一行换模型不重建前端。
- **前端 agent loop**：`lib/agent.svelte.ts` —— 注册表→自动生成 tools（id 含 `.` 清洗成合法 tool name）；`tool_use`→执行 handler→`tool_result`→循环至 `end_turn`（≤12 轮）；**confirm 闸**——`access:'danger'` 落盘类命令执行前弹确认框（被取消→优雅继续）。上下文 = 静态系统提示 + 模型摘要（各打 prompt 缓存断点）+ 当前界面状态（非缓存后缀）。`Command` 增可选元数据 `description/params/required/access/confirm/aiHidden`，`run` 改带参返结果。
- **命令集**（首批）：导航 8 项 + describe_model/describe_variable/run_simulation/select_vars/set_scenario_param/reset_scenario/switch_model/switch_zone/**save_zone_management**（danger 落盘）。`components/AgentChat.svelte` 右侧抽屉（气泡+工具卡+结果+确认卡），TopBar「🤖 问AI」开关。
- **模型 = Sonnet 4.6**（dev 默认；多轮工具往返的甜点；`EQC_LLM_MODEL` 一行可换 Opus/商用）。**验证**：真 key 端到端跑通——Sonnet 在我们的 tools 下并行/多轮选对工具、loop 收敛、danger 命令正确触发确认框（浏览器实测）。svelte-check 0 错 0 警。

**前端 Agent 后续四块（#1–#4，均已推送）**
- **#1 流式 SSE**：`POST /api/llm/stream` 强制 `stream:true`、把上游 Anthropic SSE 原样透传（`Connection: close` 逐块 flush，手写 server 每连接一线程不阻塞）；前端 `streamLlm` 用 fetch 流式 reader 解析 `content_block_delta` 等、边收边填活消息 → 文字逐字蹦。★Svelte5 坑：push 进 `$state` 后须经 `agent.convo[idx]`（代理）mutate。
- **#2 扩命令**：`get_value_at`（查某天某变量值）、`set_scenario_init`/`set_scenario_driver`（补全情景三件套）、`run_optimize`/`run_calibrate`（danger，跑 DE/标定）。复用 api.ts。
- **#3 上下文/缓存**：`messagesWithCache` 给对话末块挂第 3 个 `cache_control` 断点（多轮循环/长对话增量命中）；前缀干净（静态提示+模型摘要在断点内、当前界面状态在断点后不失效）。实测两次相同请求经 /api/llm：1st `cache_creation=6362`、2nd `cache_read=6362`（命中~0.1×）；前缀 ≥~2048 token（sonnet 下限）才激活。
- **#4 e2e 方案 C（Playwright，系统 Edge）**：① 后端 **Mock 模式**（`EQC_LLM_MOCK=1`）——从最后一条 user 消息读 `[[MOCK 工具 {json}]]` 指令确定性返回 tool_use，驱动**真**前端 loop/handler/confirm/store；`build_mock` 兼容 #3 缓存的数组包装形态。② `e2e/mock.spec.cjs`（默认 5 用例：select_vars 真执行 / confirm 允许真落盘 / confirm 取消 / 并行多工具 / 后端错误）+ `e2e/real.spec.cjs`（`EQC_E2E_REAL=1` 才跑 2 真冒烟），`test.skip` 互斥门控。实测 mock 5 passed（零成本）、real 2 passed。跑法见 USAGE。
- **★标准约定**：以后**新增任何用户可在前端操作的功能，必须同时在 `frontend/src/lib/commands.svelte.ts` 注册一条命令**（带 description/params/access），这样 ⌘K 面板与 AI 助手自动同获该能力——加功能=加命令，零额外胶水。

### 模型图论分析 arc · GA-1（结构分析地基：二部图 + 匹配 + DM 分解）
把模型当**图**来严谨分析的第一阶段（理论见 `docs/theory-model-graph-analysis.md`，spec §4）。纯 Rust、数据无关、可单测，不碰数值求解（只**定位**代数环）。新增 `src/graph/`：
- **`bipartite.rs`**：`EquationFile[] → BipartiteGraph`。复用 `get_variable_refs/get_parameter_refs` 抽边；一个方程的边集 = `refs ∪ {output}`（EQC 里 `output` 是 LHS、不在 refs 里）。参数/驱动/状态全算变量节点（让 DM 自动分出自由变量）。节点用 `MODULE.name`（同 DAG 约定），跨模块 `source:` 输入折叠进上游 output 节点 → 多模块作为一个结构系统。
- **`matching.rs`**：Hopcroft–Karp 最大匹配（O(E·√V)）+ 与作者 `output:` 对照 → `MatchingReport{最大匹配大小, 作者是否完美匹配, 结构是否奇异, 唯一性(best-effort 交替环检测), 与作者指派差异}`。适定性的结构必要条件（必要非充分，不替代数值）。
- **`dm.rs`**：Dulmage–Mendelsohn 分解 → `StructureReport{自由变量(欠定块), 求解块(方定块,块下三角顺序), 超定方程, 结构奇异, 匹配报告}`。方定块用作者 output 定向 + petgraph `tarjan_scc` 求块三角；SCC>1 或 RHS 自引用 = 代数环。= EQC 现有「拓扑排序 + 环检测」的严谨完整版。
- **CLI `eqc structure <模型> [--json]`**：人读报告（适定性/自由变量/求解顺序/代数环）或 `StructureJson` 契约（`export.rs`，additive，`schema_version` 不变）。
- **接入 `validate`**：诚实边界——单文件重复 output（→`DuplicateDefinition`）、代数环（→`CyclicDependency`）现有校验已覆盖；本轮只补**跨模块系统级过定**（耦合折叠后不同模块两方程撞同一节点）这个现有校验看不到的缺口（`structural_checker.rs` + `StructurallyOverDetermined`）。
- **验证**：合成玩具图逐一对拍（链式=全单点块三角；造环=一个 SCC 块；自引用=单点环；写重 output=超定；漏方程=落自由变量非错误；方程多于变量=结构奇异）。真模型：草莓 S4（26 eqs，完美唯一匹配、无环）、v1 cohort（66 eqs）跑通，validate 无回归。**关键洞察**：动态模型里状态量（如 DF）本步是**携带的自由变量**，故 FF/产量路在本步独立于光合路——结构分析正确分离了「本步代数依赖」与「跨步状态耦合」。12 个新测试，两 feature 配置（cli / cli+advanced_math）共 267 lib 绿。

### 模型图论分析 arc · GA-2（结构可辨识性，图论必要条件版）
互补数值 `eqc identify` 的**便宜、数据无关、更早**的图论先验筛（理论笔记 §2.4，spec §4 GA-2）。新增 `src/graph/identifiability.rs`：
- **有向影响图**（专建，不复用 GA-1 无向二部图、也不用 `build_dag`——后者缺积分边）：节点=全部符号，边 = `ref→output` ∪ `source→input` ∪ **`rate源→state`（积分）** ∪ **`prev源→semistate`（延迟）**。含积分/延迟边是关键：动态模型里 `param→rate→state→可测` 才连得通，否则误报不可辨识。
- **不可辨识（可达性）**：参数在影响图上到任何可测变量都无路径 ⇒ 结构不可辨识（数据再多也定不出）。necessary。
- **混淆候选**：进入**完全相同方程集合**的参数对 ⇒ 下游影响路径集必然相同 ⇒ 无观测能区分。necessary-not-sufficient，喂数值版确认。
- **可测集** = `measurable:true` 变量；若一个都没标，回退所有 `type:output` 变量（同数值版默认）。
- **节点命名单一真相源**：把 GA-1 内联的规范化/source 折叠抽成 `bipartite.rs::NodeResolver`，两张图共用。
- **CLI** `eqc structure <模型> --identifiability`（人读 + `StructureJson.identifiability` 可选字段，additive，`schema_version` 不变）。
- **诚实边界**：图法只给必要条件，不替代微分代数充分判定（SIAN 类）。
- **验证**：合成对拍（只测 y → 只到 z 的参数判不可辨识；动态链 `k→rate→(积分)→S→y` 判可辨识，验积分边接对；同方程参数 → 混淆候选；不同方程 → 不配对）。**真模型草莓 S4**：10 参数全可达可测（无不可辨识），并正确标出混淆候选 **{Cref, Kc}**——二者只同进 CO₂ 响应式 SB-CO2（`f_CO2=(CO2/Cref)·(Cref+Kc)/(CO2+Kc)`），恰是理论预言的「Kc 在 CO₂≡参考点不可辨识」现象；且 necessary-not-sufficient（CO₂ 变化时可分离）正好交数值版确认。4 新测试，两 feature 配置共 272 lib 绿。

## 下一步（未做）/ 当前不足

**EQC 工具层**
- **交互式前端 EQC Studio（进行中，走「本地服务」路线）**：
  - Phase 1 `eqc serve`：监听模型、存盘即重生成、`localhost` 自动刷新（手写极小 HTTP，零新依赖）。
  - Phase 2（已完成）：**JSON 契约 + 整季仿真折线图**。新增 `eqc export`（导出模型 JSON 契约 `src/export.rs`，schema_version 版本化、只增不改、可检视）；`src/chart.rs`（EQC 自生成轨迹折线图 SVG，零图表库）；`eqc serve` 扩成 Studio：`--drivers/--params` + 端点 `/api/model`(JSON)、`/api/report`(HTML)、`/api/simulate`(轨迹 JSON)、`/api/chart.svg`(折线图)；前端页面 `src/serve_assets/studio.html`（打包进二进制，零构建步骤）——浏览器左看 Forrester 图+公式、右看整季产量曲线。`src/scenario.rs` 抽出驱动量/参数加载，simulate 与 serve 共用。
  - 原则：EQC 始终是唯一权威、前端只显示其 SVG/MathML/JSON 产物，契约只增不改 → 随 EQC 升级低风险、易排查。下一步：点节点高亮、浏览器内编辑、LLM 问答、GP 结构 diff。可后续包成 Tauri 桌面应用或 VS Code 扩展。
- **codegen 不生成积分循环**：`eqc build` 仍按静态网络生成代码，状态量（`state`）没有逐步更新代码——动态模型目前只能用 `eqc simulate`（树遍历）跑，不能导出独立可运行的 Python/Rust 仿真器。
- cohort 在图上显示为展开的标量（`DF__1/2/3`），未按家族分组显示。
- **GP 约束进化层 — 引擎（G0-G5 + 联合 + Pareto-joint）+ Studio GP 面板（S1-S5+B2）全部完成**（见上「受约束遗传编程 arc」「EQC Studio · GP 面板」）。剩：**真进化（等云南 2026-07 田间数据）**——贯穿全项目的等待点（标定/优化/GP 都已备好、合成验证过、等数据）。
- 报告增强（按模块分图、点节点高亮、显示单位/出处）；codegen 死分支宏重构；耦合的时间尺度聚合（逐时 vs 日均）。

**草莓模型层**（详见 `../strawberry_model/strawberry_v1.eq.yaml` 顶部「模型短板」注释）
- 物候/发育周期、LAI 是**外部输入而非计算**（下一步接 BBCH 物候 + 叶面积子模型）。
- **无采摘/移除模拟**（果实留株、累积果重≡产量；缺采后库重释放与再分配反馈）。
- LUE 常数（不响应 CO₂/光强/发育）；缺基点温度；单品种；果实品质/糖度无机理；连续开花未按离散花序批次建模。
- 当前合成天气演示，量级未对照论文验证（需真实 'Benihoppe' 数据）。
