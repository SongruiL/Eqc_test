# EQC Studio · GP 面板规格（前端展示与人机交互）

> 状态：设计稿（2026-06-24，与首席科学家讨论后定方向）。待评审实现。
> 关联：`docs/spec-genetic-programming.md`（GP 引擎 G0-G5）、Studio 既有架构（`eqc serve` + `studio.html`，server-first、EQC 持有事实、契约只增不改）。

## 1. 立场：GP 是 human-in-the-loop，前端的活是「让裁决变容易」

GP 与 optimize/calibrate 的根本不同：后两者给**一个答案**（最优参数/标定值）；**GP 提议、科学家裁决**。所以 GP 面板的核心**不是显示一个结果**，而是把首席科学家的**判断**变容易：

> 「这是精度↔复杂度前沿上的 N 个候选形式；每个怎么拟合、哪个是你**现有形式**（rediscovery）、哪个是**新假设**——你来挑、你来采纳。」

**归属**：GP 面板在**专家 Studio**（科学家的建模工具），**不在园区视图**。分工：园区员工**录入田间数据**（喂 GP 适应度，与标定同一份输入）；首席科学家**跑 GP + 裁决候选**。

## 2. 贯穿原则（承袭既有前端）

- **EQC 持有事实**：GP 结果（前沿/公式/形式识别/rediscovery/草稿）经一个**版本化 JSON 端点** `/api/evolve` 出；前端只组合面板、不重实现逻辑。契约只增不改。
- **大量复用既有积木**：`pareto_chart_svg`（前沿散点，多目标 DE 已建）、`convergence_chart_svg`（收敛史）、`expr_mathml`（公式 2D 渲染）、园区录入网格（观测输入）、轨迹叠加（候选 vs 观测，复用 `/api/simulate` 折线图机制）。**GP 面板 ≈ 把已有积木重新拼 + 一个端点 + 人在环的"裁决"交互。**
- **human-in-the-loop**：GP 提议前沿，科学家点/比/采纳，**不自动应用**。写回模型须显式、谨慎。
- **计算重 → 异步**：GP（尤 memetic）耗时，前端需 spinner / 后台任务 + 轮询（见 §6）。

## 3. 工作流 → 面板（5 步）

### 3.1 发起（Setup）
- **靶点选择**：模型契约 `/api/model` 已暴露每方程的 `gp_target`（G0）。面板列出所有 🟠 进化靶点（方程友好名 + 语法 + 现有形式），用户点一个。
- **观测数据**：复用**园区录入网格**——拟合变量 = spec 的 `output`；观测 = 该区已录入的稀疏 CSV（与 `eqc calibrate` 同源）。显示"实测 N 点"。
- **配置**：pop / gens / seed；开关 **Pareto**（精度 vs 复杂度，默认开）、**memetic**（内层 DE 标定常数，默认关，标注"更准但更慢"）。`baseline_form` 自动取该方程现有形式（用于 rediscovery 判定）。

### 3.2 进度（Progress）
- 点「开始进化」→ spinner + **收敛曲线**（`convergence_chart_svg`，复用 optimize/calibrate 已有件）。异步见 §6。

### 3.3 ★ Pareto 前沿（GP 招牌输出）
- **散点图**：x=复杂度（节点数）、y=拟合误差（rmse），每点一个候选——**直接复用 `pareto_chart_svg`**（多目标 DE 已建，点可点击）。
- 鼠标在点之间移动 = 在「更简单但略差 ↔ 更复杂但更准」之间权衡；**拐点**最有价值。

### 3.4 候选详情（点一个前沿点）
- **公式**：该候选常数代回后的 2D 公式（`expr_mathml`）。
- **拟合**：候选轨迹**叠在观测点上**（复用 `/api/simulate` 折线图 + 观测散点叠加）。
- **机理形式 + rediscovery 徽章**（G5）：
  - 「**= 你现有形式（rediscovery，机理验证）**」绿徽章；或
  - 「**新假设 🟠**：语法内另一种机理形式，待田间证伪」黄徽章；或
  - 「自定义结构，需人工审」。

### 3.5 与现有形式对比 + 采纳（Adopt）
- **并排对比**：现模型方程 vs 选中 GP 候选（两条公式 + 两条轨迹 vs 观测 + 两个 rmse/复杂度）。
- **采纳**（人在环裁决点）：
  - 生成**溯源条目草稿**（G5 `provenance_stub`）供复核；可在面板内编辑分类（rediscovery→🟢/🔵，新形式→🟠）后**下载/复制**。
  - **写回模型**：**显式、谨慎**——v1 **只产出新方程文本**（patched `.eq.yaml` 片段）让科学家复制粘贴，**不自动改盘上模型**（尊重「EQC 持有模型、人决定」）。自动写回作为后续选项（带确认 + 备份 + diff）。

## 4. JSON 契约：`/api/evolve`

新增端点（仿 `/api/optimize`），请求带 GP spec（target/output/observed/drivers/steps/evolve 配置/baseline_form），同步或异步（§6）返回：

```jsonc
{
  "target": "BB5-DORM", "output": "dormancy_released", "grammar": "monotone_gate",
  "mode": "Pareto+memetic",
  "pareto_front": [
    { "complexity": 6, "error": 0.031, "consts": [...], "formula": "...",
      "mechanistic_form": "linear_ramp", "rediscovery": true,
      "provenance_suggestion": "🟢/🔵 rediscovery ...",
      "trajectory": { "DAT": [...], "value": [...] }   // 该候选拟合轨迹(供叠加)
    },
    { "complexity": 9, "error": 0.018, "mechanistic_form": "sigmoid", "rediscovery": false, ... }
  ],
  "baseline": { "formula": "...", "form": "linear_ramp", "trajectory": {...} }, // 现有形式(对比用)
  "observed": { "DAT": [...], "value": [...] },         // 观测散点
  "convergence_svg": "<svg.../>", "pareto_svg": "<svg.../>",
  "provenance_stub": "### [BB5-DORM·GP] ..."            // 选中点的草稿(或前端按选中点取)
}
```

前端只渲染：散点（pareto_svg 或自画 from front）、点选→公式(formula→MathML via expr_mathml 已有)+轨迹叠加+徽章、对比 baseline、采纳出 stub。**所有计算在 Rust 侧，前端零逻辑重实现。**

## 5. 复用清单（强调"几乎全是已有积木"）

| 需要 | 复用已有 |
|------|----------|
| 前沿散点 | `chart::pareto_chart_svg`（多目标 DE 已建，点可点击 `data-i`） |
| 收敛曲线 | `chart::convergence_chart_svg`（optimize/calibrate 已用） |
| 公式 2D | `report::expr_mathml`（契约 `mathml` 字段已有） |
| 观测录入 | 园区录入网格 + `/api/observations`（标定已建） |
| 轨迹叠加 | `/api/simulate` 折线图机制 + 观测散点 |
| 靶点列表 | `/api/model` 的 `gp_target` 字段（G0 已暴露） |
| 引擎 | `gp::evolve_pareto` / `form_report` / `provenance_stub`（G3-G5 已建） |
| 异步 spinner | optimize 面板已有模式 |

**新增的真正只有**：`/api/evolve` 端点（薄编排）+ studio.html 一个 GP 面板（拼装上述积木 + 采纳交互）+（可选）异步任务机制。

## 6. 计算重 → 异步（关键工程点）

GP 比 optimize/calibrate 更慢（memetic = DE 套 DE）。三档方案，按需升级：
- **v1 同步 + spinner**：小规模（pop/gens 适中、默认关 memetic）下几秒~十几秒，同步请求 + spinner 即可（同 optimize 面板）。**先做这个。**
- **v2 后台任务 + 轮询**：大规模/memetic 下，`/api/evolve` 起后台任务返回 task_id，前端轮询 `/api/evolve/status?id=` 拿进度（当前代/最佳）。
- 进度回调：evolve 循环可每代回调（已有 `history`），后台任务把 history 实时吐给前端画收敛曲线。

## 7. 与多槽位联合进化的衔接（前瞻）

引擎侧若上**多槽位联合进化**（一次进化模型全部 🟠 靶），前端 GP 面板自然扩展：靶点选择从"单选"变"多选/全选"；Pareto 前沿的复杂度轴 = 各槽位复杂度之和；候选详情按槽位分区展示各自的公式 + 形式 + rediscovery。契约 `pareto_front[i]` 增 `slots: [{target, formula, form, rediscovery}]`。**面板结构不变、只是每个前沿点内含多槽位**。

## 8. 分阶段（建议）

| 阶段 | 内容 | 依赖 |
|------|------|------|
| **S1** `/api/evolve` 端点（同步）+ 契约 | 薄编排 evolve_pareto + form_report，返回前沿+SVG+草稿 JSON | 引擎已就绪 |
| **S2** Studio GP 面板（发起+进度+前沿散点+候选详情+徽章） | 拼装已有积木 + 点选交互 | S1 |
| **S3** 对比 + 采纳（草稿下载/复制；新方程文本产出） | 人在环裁决 | S2 |
| **S4** 异步任务（大规模/memetic） | 后台任务 + 轮询 | S2 |
| **S5** 多槽位前端扩展 | 靶点多选 + 槽位分区详情 | 引擎多槽位 + S3 |

**真用价值**：田间数据到（云南 2026-07）→ 园区录入 → 科学家在此面板跑 GP、看前沿、判 rediscovery、采纳——GP 从"能跑"变"科学家能用来做模型决策"。
</content>
