# GP 候选采纳纪律（形态 A · Claude Code 开发会话 SOP）

> 进化图论 arc · Tier3 Phase 3。这是 **GP 候选筛选 + 采纳的标准流程 / 可复现 prompt**——
> 每次 `eqc evolve` 出候选后，由 **Claude Code（开发态 agent）** 照此走：读报告 → 多准则判断 →
> 自主采纳到 `gp/` 分支 → 报告；转正合 main 由首席拍板。承 spec `docs/spec-gp-candidate-branching.md`。
> **两个 AI 分工铁律**：GP 筛选/采纳 = 模型开发决策 = Claude Code；**不**是前端指月、**不**在 serve 塞运行时 LLM。
>
> 首个实跑实例（本文档的活模板）= gpdemo3 互作门控采纳（Phase 2·commit `d508fe5`→转正 `7180b3b`）。

---

## 0. 触发与定位
- **触发**：首席（或我）跑 `eqc evolve <模型> --spec <spec> -o candidates.json`（或 serve `/api/evolve`）出候选。
- **我做什么**：对**硬过滤幸存者**做多准则判断 + 自主采纳最优到 `gp/<模型>/<靶点>` 分支 + 报告。
- **我不做什么**：不重复机械红线判断（引擎已做，见 §2）；不自动转正 main（§5）；不无人值守（每次采纳都出报告）。

---

## 1. 读 candidates.json（每候选字段）
```
pareto_front[].{
  error(rmse), complexity(节点数), formula, expr_yaml(可粘贴·常数已代回),
  mechanistic_form(形式识别), rediscovery(是否复原基线形式),
  graph_evidence.{
    passes_hard_filters,                    # §2 三红线机械裁决
    adds_algebraic_loop, breaks_conservation,
    disqualifying_confounded,               # 红线③命中（拖既有参数下水）
    coefficient_cluster,                    # 候选自身系数簇（非淘汰·标定信号·§3）
    added_edges,                            # 「长出什么」（如 [d2,y]）
    removed_edges, changed_equations, distance, complexity_delta,
    conservation[],                         # 逐守恒律 baseline vs patched 残差
    reject_reasons }
}
```

## 2. 硬过滤已做的（机械 · 不重复判断）
引擎（`gp::hard_filter::graph_evidence`）已机械筛掉三条**结构红线**——任一命中 = `passes_hard_filters:false`：
1. **加代数环**（破求解结构）。
2. **破守恒律**（模型声明 `meta.balance` 且 patched 短仿真残差超容差；无声明诚实跳过）。
3. **新令既有可辨识参数不可辨识/混淆**（收窄红线③：新混淆对里至少一端是候选 `__c` 之外的既有参数）。

→ **我只对 `passes_hard_filters:true` 的候选做判断**；被淘汰者在报告里记「为什么被砍」（供首席看，§6），不复活。

## 3. 我的多准则判断（对幸存者 · 不是选 RMSE 最低）
| 准则 | 看什么 | 倾向 |
|---|---|---|
| **拟合** | rmse、per-treatment、外推 | 好，但**不唯一** |
| **简约** | complexity、常数个数、complexity_delta | 同等拟合选更简约 |
| **机理** | mechanistic_form、rediscovery、added_edges 是否有物理意义 | 长出的新边要讲得通（如 d2 抬阈值） |
| **provenance** | 靶点原 provenance | `猜测`/`推导` 该进化；`文献` 基座别动 |
| **图论证据** | coefficient_cluster 大小、conservation 余量、distance | 系数簇小=标定负担轻；守恒余量大=稳 |

**典型取舍**（pilot 例）：候选 3 rmse 0.013 远优、long 出 `[d2,y]` 与真值同构、零新混淆对拖既有参数 → 荐；
单输入候选 rmse 0.27 结构上拟合不了非单调 → 否。**若拟合略逊但机理更正 / 系数簇更小 → 可荐**，理由写清。

## 4. 采纳落盘（8 步 · grounded in pilot）
> **机制铁律**：采纳 = 我 Read→Edit→git，**不写 Rust 序列化器**。整份 `serde_yaml::to_string(&EquationFile)` re-emit
> **已证坏**（Expr 只 derive Serialize 出 `!Add` 外标签、手写 Deserialize 只认 `{op,args}` map）。
> candidates.json 的 `expr_yaml` 已是 `to_yaml_value` 正确 map 格式，我照它 Edit。

1. **选定候选** + 写清为什么（对比落选者）。
2. **建分支**：`git checkout -b gp/<模型>/<靶点>`——**在模型所在仓**（真作物 = crop-models；gpdemo3 fixture 在 Eqc_test 是特例）。
3. **Edit 靶方程 expression** 为候选形式；**★把 GP 常数命名为 `optimizable` 参数**（`default` = GP 拟合值），
   **不塞魔法字面量**——这些正是 `coefficient_cluster`、要联合标定；**删掉被弃的旧参数**（如旧 ramp 的 gate_a/gate_b）。
4. **写留痕**（承 [[model-evolution-traceability]]）：`meta.version` bump + `meta.lineage:{parent:"MODEL@<commit>", step:"..."}`
   + 靶方程 `reference` 记 GP 采纳来由（rmse 前后、grammar、seed）。**靶方程 + 命名参数 `provenance: GP`**（§7①）。
5. **验证四连**：
   - `eqc validate <模型>` → 合法。
   - `eqc simulate ... ` → rmse 应 ≈ 候选的 error（复现）。
   - `eqc structure <模型> --identifiability` → **确认系数簇**（命名参数应现为混淆候选 = 标定坑）。
   - `eqc evolution <模型>` → **自动**（走 meta.lineage 派生）把采纳当进化边 + 生成 `calibration_pitlist`。
6. **写 `adopted.md`**（采纳报告·见 §6 结构）。
7. **commit 到 gp/ 分支** + `git push -u origin gp/<模型>/<靶点>`（署名 lzyayy·gh https）。
8. **报告首席**：采纳了什么 / 为什么（多准则）/ 图论证据 / 落选者及原因 / 诚实边界。

## 5. 自主边界 & 转正
- **采纳到 `gp/` 分支 = 我自主**（安全：过硬过滤 + 分支隔离不碰 main + git 可回退 + 必报告）。
- **转正 `gp/`→main = 首席拍板**。机制 = **`git merge --no-ff gp/<分支> -m "转正: ..."`**（显式合并留「从 gp 分支提升」痕迹·非 fast-forward/cherry-pick）。后续 Studio 进化史视图可加「转正按钮」自动化此步（作 Tier2 回退的镜像）。
- 转正后基线仍可经 git（parent commit）/gp 分支取回。

## 6. 报告纪律（不静默丢）
- **`adopted.md`**（每 gp/ 分支一份）结构：采纳了什么 / 为什么（多准则表）/ 标定坑（硬过滤→采纳模型独立确认）/ 留痕 / 诚实边界。
- **落选者**：硬过滤淘汰的 + 多准则落选的，都在报告里一句话记因（"候选1 RMSE 最低但 added d2→y 让 c 不可辨识 → 否"），**不静默截断**。

## 7. 诚实边界 & 开放约定
1. **`Provenance` 枚举已加 `GP` 值**（2026-07-08·首席拍板）：GP 采纳的经验形式（靶方程 + 命名的系数参数）
   标 **`provenance: GP`**（正交于 文献/平移/推导/猜测 阶梯·属"数据发现"源）；GP 来由（grammar/rmse/seed）
   记 `reference`、血缘记 `meta.lineage`。**`GP` 不算 `is_uncertain`**（深思熟虑采纳·非"请进化我"占位；要继续
   进化须显式带 `gp_target`）。系数簇仍待真数据联合标定（`calibration_pitlist` 自动列）。
2. **gp/ 分支落模型仓**（真作物 = crop-models per-repo；引擎仓只在 fixture 特例）。
3. **命名参数的 default = GP 拟合值、非真标定值**：系数簇需真数据联合标定 / 加正交多工况实验拆共线（`calibration_pitlist` 自动列）。
4. **受约束 GP 不长新节点**（只引 gp_target.inputs + `__c`）→ 硬过滤主要在边/参数级；未来放开语法则硬过滤要扩。
5. **形态 B（多 agent 评审 workflow）**：候选多 / 要多视角交叉验证时启用（每候选派守恒/可辨识/简约/机理评委独立打分 + 对抗质证再综合）。默认形态 A。

---

*相关：spec `docs/spec-gp-candidate-branching.md` · `docs/spec-model-evolution-arc.md` §3.4 · 记忆 [[eqc-model-evolution-arc]]（Tier3 全进度）· [[model-evolution-traceability]]（留痕纪律）· [[analytical-agent-design]]（GP 筛选交 Claude Code 的原始决策）。*
