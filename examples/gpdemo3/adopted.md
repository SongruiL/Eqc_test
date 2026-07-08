# GP 候选采纳记录 —— gpdemo3 互作门控（Tier3 Phase 2 首例）

> 分支 `gp/gpdemo3/y`（承 main `d07af6c`）· 采纳人 = Claude Code（开发态 agent·自主到 gp 分支）·
> 转正合 main 待首席拍板。这是进化图论 arc · Tier3「GP 候选分支化」的**第一例端到端采纳**。

## 采纳了什么
把靶方程 `GPDEMO3-GATE`（output `y`）从**单输入 d1 linear ramp**采纳为 **expit 双输入互作门**：

```
前身  y = clamp((d1 − gate_a)/gate_b, 0, 1)          # 单调 in d1，拟合不了非单调真值
采纳  y = expit(kg_slope·((d1 − kg_thr) − kg_d2·d2)) # 引入 d2、长出 d2→y 新边
```

- 来源：`eqc evolve`（受约束 GP·monotone_gate·seed=1·Pareto 前沿互作候选）。
- 拟合：**rmse 0.278 → 0.013**（复现真值 `expit(0.8·(d1−15−d2))`）。仿真核对：rmse 0.01288、y 峰 0.902@DAT18。
- 3 系数命名为参数 `kg_slope=0.7872 / kg_thr=15.185 / kg_d2=0.9723`（GP 最佳拟合值·`optimizable`·待联合标定）。

## 为什么采纳（多准则）
| 准则 | 评价 |
|---|---|
| 拟合 | rmse 0.278→0.013，21× 改善；单输入形式**结构上不可能**拟合非单调 → 必须引 d2。 |
| 机理 | 互作门 = 阈值随 d2 漂移，与真值同构；`sigmoid_chill_heat_interaction` 形式识别命中。 |
| 结构硬过滤 | ✓过：**无代数环、无破守恒（无 meta.balance·诚实跳过）、无令既有参数不可辨识**。 |
| provenance | 靶点原 `猜测`（占位）→ 正是该进化的靶（受约束 GP 自动选 `猜测`/`推导`）。 |
| 图论证据 | `added_edges=[[d2,y]]`（长出新边）· `distance` 增 · 系数簇 3（见下）。 |

## 标定坑（硬过滤 → 采纳模型独立确认）
硬过滤报告 `coefficient_cluster=[__c0,__c1,__c2]`（候选自身 3 常数结构共线·**非淘汰**，按收窄红线③作报告）。
采纳后对模型跑 `eqc structure --identifiability` **独立确认**：
```
混淆候选：{kg_d2, kg_slope} · {kg_d2, kg_thr} · {kg_slope, kg_thr}
```
→ 3 系数同进本方程、结构无法区分 → **须联合标定 / 加多工况正交实验**（单工况定不出三者）。
这是「混淆团 = 经验式系数簇」规律在 GP 采纳侧的活例：进化出经验响应式 = 引入其系数簇 = 标定规划抓手。

## 留痕（承 model-evolution-traceability 纪律）
- `meta.version` 1.0 → **1.1**；`meta.lineage.parent = GPDEMO3@d07af6c`。
- 靶方程 `reference` 记 GP 采纳来由；3 参数 `provenance: 猜测`（GP 拟合值待标定）。
- 进化分析器/动画自动纳入本分支（`meta.lineage` 自动派生血缘 + `diff_models` 分支 vs main）。

## 诚实边界
- **provenance 标 `GP`**（2026-07-08 首席拍板加 `Provenance::GP` 枚举·靶方程 + 3 命名系数参数均标 GP·正交于
  文献/平移/推导/猜测 阶梯 = 「数据发现」源）；GP 来由记 `reference`、血缘记 `meta.lineage`。`GP` 不算 `is_uncertain`
  （深思熟虑采纳·再进化靠显式 gp_target）。
- 分支落在 **Eqc_test 引擎仓**（gpdemo3 fixture 在此）；真作物采纳的 gp/ 分支应落 **crop-models**（per-repo 约定）。
- 合成 demo·常数为 GP 拟合值非真标定值。转正 = `--no-ff` merge gp/→main（首席拍板·2026-07-08 已转正 `7180b3b`）。
