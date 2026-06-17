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

## 工程基线
- 测试：100+ 个，`cargo test --features cli`（含特殊函数时加 `advanced_math`）全绿。
- 远程：github.com/SongruiL/Eqc_test，SSH 推送。
- 文档：见 `docs/USAGE.md`（架构与模块地图）、`docs/spec-*.md`（设计规格）。

## 下一步（未做）
- **GP 约束进化层**：核心愿景，动手前需讨论 fitness 数据来源、可进化 vs 冻结节点、约束方式。
- 报告增强（按模块分图、点节点高亮、显示单位/出处）。
- codegen 三个穷尽 match 里已迁移算子的不可达死分支——用宏重构彻底删除（同时保留穷尽性检查）。
- 耦合的"时间尺度聚合"（逐时 vs 日均，非单纯单位换算）。
