# spec：FSPM 功能层② —— 器官流 / 器官级源-库碳经济（风险4，v1，待施工批准）

> 建在已完成的 **FSPM 地基**（[`spec-fspm-foundation.md`](spec-fspm-foundation.md)，风险1 实例身份 + 风险2 NodeResolver）
> 与 **功能层① 拓扑聚合算子**（[`spec-fspm-aggregation.md`](spec-fspm-aggregation.md)，风险3 `{agg: sum/mean, over: children/all}`）之上。
> 目标：把番茄整株源-库（Vanthoor/De Koning 箱车）**下沉到器官级**——每个单果各记自己的热龄、各有汇强、
> 共享缓冲库**按相对汇强**把碳分到每个器官，状态 = 生长 + Σ流入 − Σ流出，且**质量守恒由构造保证**。
> 红线沿用：器官实例身份永远**结构化一等**；**L1** = 加载期按静态拓扑 lower 成标量，**引擎不改**。

## 0. 缘起与现状核对（已查代码 + 文献，决定设计）

**当前番茄 T3**（[`crop-models/tomato/tomato_t3.eq.yaml`](../../crop-models/tomato/tomato_t3.eq.yaml)）是忠实的 **Vanthoor (2011) 整株源-库 + De Koning (1994) 固定箱车**：
- 整株池：`C_Buf`（缓冲库）、`C_Leaf`、`C_Stem`（各一个标量）。
- 果实：`cohort fruit, nDev=10` 按**发育阶段** q 分箱（**非**单果），`N_Fruit[q]`/`C_Fruit[q]`，碳/果数沿阶段 `q→q+1` 流（`offset:-1` 箱车）。
- **碳分配已是「相对汇强 + 共同池」**，只是索引在发育阶段：`MC_BufFruit_st[q] = η·N[q]·GR[q]·(总果流)`，`η=1/Σ(N·GR)`。

**引擎现实**（[`src/sim/mod.rs:338`](../src/sim/mod.rs)）：显式 Euler `X[n]=X[n-1]+rate·dt`，每 state 配**一个** `rate` 变量；但 rate 变量的**表达式可以是任意组合**——箱车的 `rate_C[q]=MC_BufFruit_st[q]+MC_out[q-1]−MC_out[q]−MC_FruitAir[q]` 就是手写的「Σ流入−Σ流出」。**故"器官多源流"用现有「风险1 实例化 + 风险3 聚合 + rate 组合表达式」即可表达，引擎一行不改。**

**风险4 的真正内容** = 不是加引擎特性，而是：①把碳经济索引从"发育阶段"换到"单个器官"（每果各记热龄）；②补一个让"每果错峰出现"可写的小特性（`{rank}` 序号访问器）；③把质量守恒做成可验证。

## 1. 已对齐决策（首席科学家 2026-06-29 拍板）

| # | 抉择 | 决定 | 理由（一句话） |
|---|---|---|---|
| 1 | 箱车去留 | **器官级：每果各记热龄；丢箱车。T3 箱车原封保留**（回归锚 + 整株级参考） | 文献定论（§2）；单果 = 自己的 size-1 cohort，箱车自动退化；B2 真植株 3D 需逐果质量 |
| 2 | 生长曲线 | **beta 生长函数（Yin 2003）** | 确定性生长（有限终点 te）、参数可解释（w_max/t_m/t_e）、现代 FSPM 标准；架构对曲线无关、可换 |
| 3 | 坐果 | **v1 确定式**（每花→果、封顶、错峰 ψ）；GreenLab 源-库比概率式作后续层 | 先把碳经济+守恒做扎实；落果文献"预测不准"；确定式=可逐位回归 |
| 4 | 架构 | **不加 `flow:` 一等化、引擎不动**；只补「每器官序号 `{rank}`」+ 守恒诊断 | 风险3 聚合已保守恒；共同池本无成对流；最小引擎面（EQC 哲学） |
| 5 | 范围 | **v1 只「果实到单果」**；叶/茎保持整株集总（照 T3） | 逐叶碳只有配逐叶截光（风险5 几何）才有意义；果=产量+B2+逐果BER 回报集中 |

## 2. 科学基座（器官级源-库，文献引用；见记忆 `fspm-tomato-literature`）

deep research（已 fact-check，22 确认）结论：器官级源-库的**压倒性共识** = **共同同化物池（Heuvelink 1995）**，每器官汇强 = 其**潜在生长率**，共享池按相对汇强**全局**分配（非 Münch/传输阻力）。最佳可适配模板 = **Butturini et al. 2025/2026, in silico Plants 8(1):diaf024**（GroIMP/MTG 矮番茄，给完整方程组）+ Smolenova et al. 2025（WUR）+ GreenLab-tomato（Kang/de Reffye 2011, AoB 107(5):805）。

**核心方程（Butturini 2025，日/热时间步长，直接适配）**：

| 量 | 形式 | 出处 |
|---|---|---|
| 器官热龄生长（质量） | `w_str(t) = w_str,max · [ (1 + (te−t)/(te−tm)) · (t/te)^(te/(te−tm)) ]`，`0≤t<te`，t=器官**自己**的热龄[°Cd] | Eq.12（Yin 2003 beta） |
| 器官潜在生长率（=汇强基） | `g_pot = d w_str / dt`（beta 的时间导数，Yin 2003 闭式） | Eq.13 内 |
| 器官汇强 | `ss = g_pot · ASR`（ASR=同化物需求 gCH₂O/g结构DM） | Eq.13 |
| 整株汇强 | `ssp = Σ ss`（全器官） | — |
| 可用池 | `Ap = Anet + Cbuf_prev`，`Anet = A_gross − Σ维持呼吸` | Eq.11 |
| 分配 | `A = ss`（若 Ap>ssp，余量→缓冲）；否则 `A = (ss/ssp)·Ap` | Eq.14 |

**汇强=潜在生长率**的经典验证：Heuvelink & Marcelis 1989（一穗留 1 vs 2 果，单果生长率相同 → 即潜在率，与供给无关）。**坐果**（v1 用 Butturini 确定式）：每花→果、封顶（实测 ~8/穗）、穗内错峰热延迟 ψ；GreenLab 概率式 `P_set=1−e^(−2.39(Q/D−0.12))`, R²=0.77 留后续层。**勿用**：PMC3189842 的幂函数逐果汇强（对抗验证已驳）。

> 占位说明：beta 导数的精确闭式、ASR/w_max/t_m/t_e/ψ/封顶数等**具体参数由首席科学家 + Butturini/Yin 原文 + 云南田间数据填**；架构不依赖具体数值。本 spec 锁定的是**结构**（共同池 + 相对汇强 + 每器官热龄 + 守恒），非某一组系数。

## 3. 架构判断：多源器官流**不需要新引擎特性**，守恒由聚合天然保证

把上面的共同池碳经济翻成 EQC（每果一份 = 风险1 `of:`；整株汇总 = 风险3 `over:all`；穗级 = 风险3 `over:children`）：

```yaml
# —— 每器官汇强（of: fruit；门控 gate 见 §5）——
ss:   { of: fruit }     # ss_i = g_pot(age_i) · gate_i · ASR_fruit
# —— 整株汇强 = 叶 + 茎 + Σ各果（风险3 over:all）——
ssp = ss_leaf + ss_stem + {agg: sum, over: all, of: fruit, body: ss}
share = min(Ap, ssp) / max(ssp, TINY)      # ∈[0,1]；Ap>ssp→1(满潜在)，否则 Ap/ssp
# —— 每器官分配（of: fruit）= 单一真相源的那一处 ——
A:    { of: fruit }     # A_i = ss_i · share
# —— 每果碳状态：rate = 流入 − 生长呼吸（组合表达式、一个 rate 变量）——
rate_Ci = A − c_g_fruit·A                   # of: fruit
C:    { of: fruit, class: state, rate: rate_Ci }
# —— 缓冲库排出 = 各器官收到的总和（同一个 A 经 over:all）← 守恒在此 ——
total_alloc = A_leaf + A_stem + {agg: sum, over: all, of: fruit, body: A}
rate_Cbuf = Anet − total_alloc              # 余量(Anet−Σ分配)留存缓冲
Cbuf: { class: state, rate: rate_Cbuf, prev: Cbuf_prev }
```

**守恒为什么天然成立**：缓冲库排出 `Σ A_i`、每个果收到 `A_i`——两边引用**同一个 `A` 变量**（缓冲库经 `over:all` 聚合、果经 self），单一真相源、漏不掉。合并各状态速率：`rate_Cbuf + Σ rate_Ci + rate_Cleaf + rate_Cstem = Anet − Σ生长呼吸`，即 **净碳流入 = ΔΣ碳存量 + 呼吸**（呼吸=唯一物理"出口"，CO₂）。逐位验证见 §7。

**故确认决策4**：不做 `flow:` 一等化、不把 `rate` 扩成多贡献。共同池**本无成对的"A→B 流"**（它是一个池按份额扇出到 N 个器官，`flow from A to B` 抽象不匹配），且聚合已兜住守恒。`flow:` 留作"以后真有痛点再说"。

## 4. 唯一新特性：`{rank}` 每器官序号访问器（决策4 的"每器官参数/索引"）

**为什么需要**：器官级的价值在于**错峰**——早果晚果竞争同一个池（经典源-库）。若 24 果同时出现、同步变老 → 全同 → 退化回集总。错峰需要每果的"出现热时间阈值" `θ_i = (节位−1)·phyllochron + (果位−1)·ψ`，即需要器官的**序号**。地基只有 `of: self/parent/prev/next`（取邻居实例），**没有序号访问**（"per 参数/实例索引"是地基已记的 deferred 项）。

**设计**（最小、通用，类比 cohort 的 `{idx: q}`）：

```yaml
{ rank: self }     # 本实例在同胞组里的 1-based 序号 = 实例 id 的末段路径分量
{ rank: parent }   # 父实例的序号（上溯一层）
```

- 实例 id 是路径式（地基：`fruit#3.2` = 节3 的第2果）。v1 结构（§6）= metamer 链 + fruit per metamer → 果 id `"3.2"`：`{rank:self}=2`（果位）、`{rank:parent}=3`（节位）。两者都单层、够用。
- **加载期 lower**：`structure_expand` 在实例化 `for:E` / 聚合 body 时，把 `{rank:self}`→`{const: <self.id 末段>}`、`{rank:parent}`→`{const: <parent.id 末段>}`。**纯常量折叠，引擎/codegen 永不见 `rank`**（同聚合、同 cohort `{idx}`）。
- 实现点：[`src/parser/structure_expand.rs`](../src/parser/structure_expand.rs) 的 `rewrite_refs` 加一个 `{rank: …}` 分支（在 `{ref}`/`{agg}` 之前判），用 `RefCtx.inst` / `inst.parent` 取 id 末段转整数。越界/无父 → 报错（`BadRefOf` 类）。
- 边界：本轮只 `self`/`parent`（够 v1 两层）；`grandparent`/任意祖先留后（真 truss 三层结构时再加）。

## 5. 门控激活（错峰出现，L1「预分配 + 门控」的兑现）

L1 = 24 果全预分配，各按热时间阈值"长出"。每果一个**激活门** + 自己的**热龄**：

```yaml
theta_appear = ({rank:parent} − 1)·phyllochron + ({rank:self} − 1)·psi   # of: fruit；出现阈值[°Cd]
gate = ramp( (T_Can_Sum − theta_appear) / w_appear, 0, 1 )                # 0未出现→1已出现（平滑、可微）
rate_age = max(0, T − Tbase_fruit) · gate                                  # of: fruit
age = { of: fruit, class: state, init: 0, rate: rate_age }                 # 出现后才累积热龄
ss  = g_pot(age) · gate · ASR_fruit                                        # 未出现 ss=0 → 不进 ssp、不抽碳
```

- **未出现的果**：`gate=0` → `ss=0` → 对 `ssp = Σ ss` 贡献 0（`sum` 对 0 项无碍）→ 不抽碳、`age` 不累积。**自然处理"还没坐的果"，无需条件聚合。**
- **生长终点**：beta 导数在 `age≥te` 处自然→0 → `ss→0` → 不再抽碳。**无需箱车的末阶段流出。**
- **采收/移除**：v1 简化——果长到 te 即停长、留在株上，`Y = Σ 已熟果(age≥te)质量`（不回移）。**带移除的采收流**（果熟→出株→腾库）作后续（与 GreenLab 概率坐果同层）。已在 §9 标注。
- ⚠️ **`mean` over 已激活器官**（如"已坐果的平均单果重"）需**条件聚合** `over: children where active` —— 风险3 spec 已 defer 到此；本轮**sum 路径用 ss=0 门控即可、不内建条件聚合**，`mean-over-active` 仍留后（v1 的均值类输出可先不做或用全集）。

## 6. `tomato_fspm` 模型设计（v1，单果分辨率）

**结构**（复用地基 count/chain/per；本地 demo，不入 eqc repo）：

```yaml
structure:
  entities:
    metamer: { count: 6, topology: chain }   # 6 个结果节位（主茎果位的抽象；非全部节间）
    fruit:   { per: metamer, count: 4 }       # 每节位 4 果 → 24 果
# 真 truss-on-every-3rd-node（borne_on/at）+ 营养节间 = 后续；v1 "结果节位"=穗的抽象
```

- 复用 T3：整株光合链（FvCB → `MC_AirBuf`）、温度状态、维持/生长呼吸系数、叶/茎集总池、水–N–EC–Ca 外壳**原样平移**。
- **改的只有果实区室**：箱车 → 器官级（§3 碳经济 + §5 门控）。
- **穗级库强**（用户要的"果穗库强=Σ各果汇强"）= metamer 视角的 `{agg: sum, over: children, body: ss}`（节位的 children=其果）。**演示 `over:children`、给逐节位诊断**（1 节位≈1 穗）。
- **整株光合驱动**仍整株（叶集总；逐叶截光=风险5）。"冠层Σ各节叶面积"在 v1 数值等价集总 LAI，**留风险5**（届时逐叶配逐叶光才有新信息）。
- 产量/品质输出：`Y = Σ 熟果质量`（`over:all of fruit`）、逐果/逐穗鲜重与 BER 可派生（BER 外壳平移）。

**这套 demo 方程仍是占位骨架**——结构（共同池+相对汇强+每果热龄+守恒）锁定，beta 导数/ASR/各参数由首席科学家+文献+田间数据填。

## 7. 守恒诊断（决策4 的安全网，可选/末步）

运行时质量平衡审计，抓"忘了用聚合量排空池 / 写错符号"这唯一真失败模式：

- 每步检查 `| Anet − ΔΣC_states − Σ呼吸 |  ≤  tol`（`C_states` = Cbuf + ΣC_fruit + C_leaf + C_stem）。
- 实现选型（施工时定）：① `eqc simulate` 加 `--check-balance <pool规约>` 标志，按声明的"池 + 其流"在每步比对；② 或一个独立 `eqc check-conservation` 子命令跑一遍仿真核对。倾向 ①（轻、就地）。
- 模型侧加一处 additive 声明（如 `meta.conservation: {in: Anet, stores: [...], out: [...]}`）让诊断知道核对谁；**不改引擎积分逻辑**。
- 这是诊断/校验层，**可与 §6 解耦、最后做**；若时间紧可先靠 §3 的手工逐位核对兜住。

## 8. 施工分步（每步 `cargo build --features cli --offline` + `cargo test --features cli --offline` 绿×2 配置 + 用户点头再提交）

1. **`{rank}` 访问器（§4）**：`structure_expand.rs::rewrite_refs` 加 `{rank: self|parent}` → 常量折叠；越界报错。合成单测（番茄 fixture：果 `3.2` → self=2/parent=3）+ 端到端 validate。**最小、隔离、先行。**
2. **门控激活 + 每果热龄（§5）**：番茄 fixture 加 `theta_appear`/`gate`/`age`/`rate_age`，验证未出现果 `ss=0`/`age` 不动、出现后累积；simulate 无错。
3. **`tomato_fspm` 器官级碳经济（§6，主交付）**：建模型——共同池 `Ap/Anet`、`ssp=Σss`、`share`、每果 `A`/`C`、缓冲库 `rate_Cbuf=Anet−total_alloc`、穗级 `over:children`。validate + simulate；**手工逐位核对守恒**（§3 合并式）。本地 demo（与 `tomato_fspm_demo`/gpdemo 同，不入 eqc repo）。
4. **守恒诊断（§7，可选）**：`--check-balance` 或子命令 + 模型 `conservation` 声明；番茄 demo 上验证平衡 ≈0。
5. **契约/前端顺带**：器官级 `ss`/`A`/`C` 经现有 `VarJson.instance` + `structure.aggregations`（风险3 已铺）自动带出；3D 拓扑图例显穗级 Σ 聚合。**逐果质量→逐果 3D 大小 = 风险5**，本轮只确保 export 干净。

## 9. 铁约束 与 边界

- **铁回归锚**：T3（cohort 箱车）**一字不改、仿真逐位不变**（草莓 S8 `Y=7.558441109220954` 同理不受影响）。`tomato_fspm` 是**新模型**、不碰 T3。
- **L1 不改引擎**：`{rank}`/门控/聚合全加载期 lower 成标量；`sim`/`eval`/`Stepper` 照跑、不读结构身份。
- **架构不依赖具体方程**：beta 导数/ASR/参数/封顶/ψ 由首席科学家+文献+数据+量纲/边界检查+受约束 GP/标定填。
- **留后（记着）**：① 真 truss（`borne_on`/`at: every:3` + 营养节间，三层 → `{rank:grandparent}`）；② 带移除的采收流（果熟出株腾库）；③ GreenLab 源-库比概率坐果/落果（连续比例、可微）；④ 条件聚合 `over: children where active`（mean-over-active）；⑤ 逐叶/逐节间器官化 + 逐叶截光（绑风险5 几何）；⑥ `flow:` 一等化（仅当建模实测易错才回头加）；⑦ Münch/传输阻力（文献：仅极端局部源-库失衡才需，番茄全局池够）。
- **每步一验、绿灯 + 用户点头再提交**（项目一贯节奏）。

> **★风险4 = 把器官级源-库碳经济做成现实**：每果各记热龄（beta）、共享池按相对汇强分到每果、守恒由聚合保证；新特性仅 `{rank}` 一个 + 可选守恒诊断；引擎不改、T3 不动。这是通往 GIS 数字孪生 B2（真植株 3D）的真科学下一块——器官量做出来，风险5 几何/glTF 才有东西可画。
