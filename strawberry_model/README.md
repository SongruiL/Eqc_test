# 温室草莓机理模型（基于 EQC）

用 [EQC](https://github.com/SongruiL/Eqc_test)（方程编译器）搭建的温室草莓（'Benihoppe'，促成栽培）源-库干物质分配机理模型。以 **Sugiyama et al. 2025** 的纯方程骨架为起点，逐步补全机制。

> **状态**：结构逐步完整、每条公式标注来源、**但数值仍为示意**——`anthesisGDD / SLA / ksen / Kc …` 等参数均标「待重标定」，需真实 'Benihoppe' 田间数据标定+验证（R²）后才可信。合成天气仅用于演示模型行为。

## 版本进程

| 文件 | 在前一版上加了什么 | 把哪个「输入」变「计算」 | 主要来源 |
|------|------|------|------|
| `strawberry_v1.eq.yaml` | Sugiyama 18 方程骨架（cohort 宏展开，92 变量）| — | Sugiyama 2025 |
| `strawberry_v1_vector.eq.yaml` | 把 cohort 写成**向量**（28 变量，产量 Y 与宏展开版逐位一致）| — | Sugiyama 2025 |
| `strawberry_s1.eq.yaml` | **物候**：开花日由累积温度阈值计算；**基点温度** fT；**第二花序批次**（6 花序）| 开花日 | + Labadie 2019、Hopf 2022 |
| `strawberry_s2.eq.yaml` | **叶面积**：LAI=SLA×叶干重（追踪叶干重 + 一阶叶衰老）→ 只靠天气预测 | LAI | + Hopf 2022(SLA)、de Koning/Heuvelink |
| `strawberry_s3.eq.yaml` | **采收速率**（ΔY，月/日采收时序，不动已验证核心）| —（采收已隐含于 RFG）| Sugiyama 2025 |
| `strawberry_s4.eq.yaml` | **动态 LUE**：LUE 随 CO₂ 变（管理决策旋钮）| LUE 常数→动态 | + Higashide & Heuvelink 2009 |

S4 是当前最全版本：模型只靠 **天气(T、Sr) + CO₂ + 初始状态** 预测。

## 怎么运行

先在 EQC 仓库里构建 `eqc`（见该仓库 README），然后：

```bash
# 看模型（浏览器交互：Forrester 图 + 公式 + 跑仿真画轨迹 + 每条公式「📖 来源」）
eqc serve strawberry_s4.eq.yaml --drivers scenario/weather_s4_enriched.csv
#   浏览器开 http://localhost:7878/

# 跑仿真出轨迹 CSV
eqc simulate strawberry_s4.eq.yaml --drivers scenario/weather_s4_400.csv -o out.csv

# 导出模型 JSON 契约 / 校验
eqc export strawberry_s4.eq.yaml -o model.json
eqc validate <含 .eq.yaml 的目录>
```

驱动数据（`scenario/`）：`weather_s2.csv`（T,Sr，240 天，S1/S2/S3 用）、`weather_s4_400.csv`（+CO₂=400）、`weather_s4_enriched.csv`（冬季 CO₂=800）。`*_out.csv` 是仿真产物，可重新生成。

**关键演示**：S4 在 CO₂=400 时产量与 S3 逐位一致（向后兼容）；冬季充 CO₂ 到 800 ppm → 产量 +14.6%——这就是「该不该充 CO₂」的决策能力。

## 来源与标注原则

- **多来源、每条公式标引用、不编造。** 模型里每条方程都有 `reference` 字段，前端报告/Studio 以「📖 来源」显示；未标注的会高亮「⚠ 未标注来源」。
- 文献缺方程而需补的（如叶衰老、动态 LUE 的函数形式），用**作物建模通行的标准式跨作物平移**，并在 `reference` 里注明「标准式/跨作物平移」与参数「待重标定」。
- 文献依据：Sugiyama 2025（骨架）、Labadie & Guédon 2019（物候/花序批次）、Hopf 2022 CROPGRO-Strawberry（基点温度、SLA）、Higashide & Heuvelink 2009（CO₂-动态 LUE）、Chen 2009（叶相对生长）、de Koning 1994 / Heuvelink 1996（源-库分配）。完整综述见 `草莓模型文献综述.md`。

## 局限（诚实）

- **数值待重标定**：开花积温阈值（借自法国 June-bearing 基因型）、SLA、叶衰老率、CO₂ 半饱和常数等均为估计/借用值；需真实 'Benihoppe' 数据。
- **合成天气**：演示模型行为，非验证。
- **叶面积量级偏高**（峰值 ~8 vs 真实 ~3–4）、未做品质/糖度（需田间反馈，暂缓）。
- 矩阵运算、连续开花的更细批次动态等未做。

## 与 EQC 的关系

整个 S1–S4 **没有改动 EQC 引擎一行代码**——全部建立在 EQC 的向量/状态量/cohort 能力之上（值类型 `Value{标量|向量|矩阵}`、逐元素广播、向量归约 `vsum`、积分状态量、延迟寄存器）。这也反向验证了 EQC 这套建模地基的通用性。

> 文献 PDF 在本地 `literature/`（受版权所限**不随仓库提交**）。
