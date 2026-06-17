# 设计规格：量纲系统（科学正确性护栏）

> 状态：Phase 4a + 4b 已实现并验证。
> 模块：`src/units/`。

## 目标
给 EQC 加一道**科学正确性护栏**：在把论文里的数学模型耦合成方程网络时，自动发现
量纲错误（加减不同量纲、超越函数参数带量纲、接口处单位不匹配等）。这也是将来约束
GP「不产生物理上胡说的公式」的基础。

## Phase 4a：量纲检查（已完成）

### 表示
`Dimension`：7 个 SI 基本量的整数指数向量（质量 M、长度 L、时间 T、温度 Θ、
物质量 N、电流 I、光强 J）。全 0 = 无量纲。代数：乘→指数相加，除→相减，
整数幂→指数乘 n，平方根/立方根→指数能整除时整除、否则无法表示（返回 None）。

### 单位解析
`parse_dimension(&str) -> Option<Dimension>`：把单位字符串解析为量纲。
- 基本/导出单位注册表：m, g, s, K/degC, mol, N, Pa, J, W, Hz, L, ha, min/h/d/yr…
- SI 词头（k, m, u/µ, M, G…）：不影响量纲，仅影响比例（Phase 4b 才用），解析时忽略。
- 复合单位：`/`（除）、`*` 或 `·`（乘）、尾部指数（`m2`、`m^2`、`s-1`）。
  例：`umol/m2/s` → N·L^-2·T^-1，`mol/mol` → 无量纲，`kPa` → M·L^-1·T^-2。
- **无法识别的单位返回 `None`**：检查时跳过、不误报（务实容错）。

### 检查器
`check_expr(expr, &env) -> (Option<Dimension>, Vec<DimError>)` 在 AST 上传播量纲，
规则按算子分类（复用 `ops::as_operator`）：
- 加减/比较/聚合/分支：两侧（多侧）同量纲，否则 `Mismatch`。
- 乘除：量纲相乘/相除。
- 超越函数（exp/ln/三角/双曲…）、逻辑：参数须无量纲，否则 `NonDimensionless`；结果无量纲。
- pow：底无量纲→无量纲；指数为整数常量→量纲缩放；否则未知。
- sqrt/cbrt：指数能整除则开方，否则未知。
- 量纲未知（缺单位声明 / 暂不支持的算子）→ 返回 `None`，跳过不报。

`check_equation_file(&EquationFile)`（cli）：由 parameters/variables 的 `unit`
建立量纲环境，对每条方程推断右侧量纲并收集错误，且检查右侧量纲是否与**声明的输出
变量量纲**一致。

### 刻意未做（避免误报破坏现有流程）
- **未接入默认 `validate`**：现有示例单位不全，作为独立 API 提供，避免大量误报。

## Phase 4b：单位换算与耦合（已完成）
- `Unit` = 量纲 + 比例因子 `scale` + 偏移量 `offset`（`value_SI = scale*value + offset`）。
  解析器升级为 `parse_unit`（带词头/复合的比例累乘；偏移仅对单一记号有效，如 degC）。
- `Unit::affine_to` / `convert`：同量纲单位间的仿射换算 `target = factor*x + shift`。
  - ✅ 比例：km→m ×1000、h→s ×3600、kPa→Pa ×1000。
  - ✅ 仿射：20°C → 293.15 K（含 +273.15 偏移）。
  - 量纲不同 -> `None`。
- `check_coupling(&[EquationFile])`：扫描带 `source` 的输入变量，比对源输出变量的单位：
  量纲不同报 `DimensionMismatch`；量纲同、单位不同给出 `ConversionNeeded{factor, shift}`；
  源变量缺失报 `SourceNotFound`。

### Phase 4b 已知简化（留待后续）
- ⚠️ **时间尺度聚合**（逐时 vs 日均）不是单位换算，本期不处理——`h→s` 这类是
  「同一物理量的不同时间单位」可换算，但「逐时气温 → 日均气温」是**聚合**，需在
  耦合层显式建模，不能用一个系数。
- 复合单位中的偏移按 0 处理（如 degC/day 只取比例）。

## 后续可选
- 把量纲规则收进 `OperatorSpec` 的预留槽位，使量纲检查也完全注册表驱动。
- 把单位升级为变量/参数的结构化字段（而非自由字符串 + 解析）。
- 把量纲/耦合检查接入 CLI 子命令（如 `eqc check-dims`）。
