# spec：FSPM 全发育 arc + 功能层④「节间伸长 + 节点出现」（F2，v1 待施工批准）

> 建在已完成的 FSPM 地基（风险1-2）+ 功能层①②③（聚合/器官流/真实几何 v1）之上。
> 目标：把番茄从「成熟阶段」往前推到**营养体发育**——节点按 plastochron 逐个长出、节间随热龄伸长，
> 株高从此 emergent（非 GIS 常数）。这是「出芽→复叶→节间→花芽→坐果→成熟」**全发育大 arc 的第一步**。
> 科学基座见 [`crop-models/tomato/番茄FSPM模型文献综述.md`](../../crop-models/tomato/番茄FSPM模型文献综述.md) §10（2026-06-30 deep research，18 确证 7 驳）。
> 红线沿用：器官身份结构化一等；**L1**（预分配+门控出现，引擎不大改）；架构不依赖具体数值（待云南标定）。

## 0. 框架：全发育 arc（F 版本谱系）

`T1-3` = Vanthoor **functional 谱系**（箱车，冻结作回归锚）。FSPM **另起 F 谱系**，每步 bump 版本 + 记模块日志：

| 版本 | 模块 | 状态 |
|---|---|---|
| **F1** | 器官碳经济（每果热龄 beta·共同池·结构固定 6 节·叶集总） | ✅ 现 `tomato_fspm_organ`（风险4） |
| **F2** | **节间伸长 + 节点 plastochron 出现**（株高 emergent） | **本 spec** |
| F2.5 | 节/果出现时序统一合轴尺度（消 desync） | ✅ 已实现（commit 51297b5） |
| F3 | 逐叶器官化 + 简版光竞争（几何→光→碳反馈） | ✅ 已实现（本节 §F3） |
| F4 | 花穗起始（合轴）+ 每穗花数 + 坐果率（果数 emergent） | ✅ 已实现（本节 §F4） |
| F5+ | L2 动态长器官 + 全版 3D 光追 | 远期 |

## 1. 已对齐决策（首席科学家 2026-06-30 拍板）

| # | 抉择 | 决定 | 依据 |
|---|---|---|---|
| 1 | 节间长公式 | **解耦·热龄 logistic**（[S25] diaf022，indeterminate）；非碳耦合异速（[B25] dwarf） | 文献综述 §10.1；indeterminate 实拟·解耦·最简，合"形态独立于碳"设计 |
| 2 | 热钟基温 | **形态子模型独立热钟 `Tbase_morph=4°C`**（直接用已发表 °Cd 参数）；碳模型 Tbase=10/8 不动 | §10.4；°Cd 参数与拟合基温绑死，研究明确警告勿混 |
| 3 | DIF / R:FR | **F2 不做**（确认缺口，番茄无可实现方程，chrysanthemum 模型被驳）；留 `推导/猜测` 后续 | §10.5；核心（温度/热龄）全 `文献·确证`，先扎实 |
| 4 | phyllochron | **沿用模型已有 `phyllo=35 °Cd`**（在 32–48 范围内、已在用） | §10.2；与现果实错峰共用同一参数 |
| 5 | provenance | **加结构化 `provenance` 字段**（文献/平移/推导/猜测），机器可读 | 首席科学家定；喂 GP 选靶点 + 动画上色 |

## 2. 科学基座（方程，接文献综述 §10）

| 量 | 方程 | 参数（占位/文献） | 出处档 |
|---|---|---|---|
| 节间长 | `internode_len = maxlen / (1 + e^(−klen·(node_age − tmlen)))` | maxlen 0.053–0.071 m · klen ≈0.020 · tmlen 110–117 °Cd | **文献**（[S25] Eq.4，确证 3-0） |
| 节点出现 | 节点 k 出现当 `Tsum_morph ≥ (k−1)·phyllo` | phyllo=35 °Cd（沿用） | **文献**（[S25]+[SL]，确证 2-1） |
| 节点热龄 | `node_age = ∫ max(0,T−Tbase_morph)·node_gate` | Tbase_morph=4°C | **文献**（§10.4） |
| DIF/R:FR | —（缺口，F2 不做） | — | **缺口**（§10.5） |

`internode_len` 是 node_age 的**纯函数**（不耗碳）→ EQC 作 auxiliary 直接算，无需积分（类比从 age 算 cw）。

## 3. `provenance` 字段（EQC schema additive，决策5）

每条方程可标来源档，与现有 `reference:`（引用）配合 = 完整出处。
```rust
// schema：Equation 新增
#[serde(skip_serializing_if = "Option::is_none")]
pub provenance: Option<Provenance>,
pub enum Provenance { Literature, Transferred, Derived, Guess }  // 文献/平移/推导/猜测
```
- YAML：`provenance: 文献`（或 literature/transferred/derived/guess）。additive、None 省略、纯 Functional 契约逐字节不变。
- **下游用途**：①GP 自动选靶点（`猜测`/`推导` → 可进化，`文献` → 冻结基座）——与现有 `gp_target` 协同；②契约 `EqJson.provenance` 带出 → 两个生长动画**按出处上色**（GA-6b 模型逐章长出 + GIS 植株 3D）；③`eqc validate` 可报"本模型猜测方程占比"。
- 实现点：`schema/equation.rs` 加字段 + 手写/serde 反序列化（中文枚举值映射）+ `export.rs::EqJson` 镜像 + `contract.ts`。**随 F2 落地、用真方程测**。

## 4. F2 模型设计（节间伸长 + 节点出现）

**新增形态热钟**（独立于碳模型的 Tsum/dT_f）：
```yaml
Tbase_morph: { default: 4.0, unit: degC }            # 形态发育基温（[S25] 拟合基温）
rate_Tsum_morph = max(0, T − Tbase_morph)            # 形态积温速率
Tsum_morph: { class: state, init: 0, rate: rate_Tsum_morph }   # 形态发育积温
```

**新增 per-metamer（节点级，of: metamer，6 实例）**：
```yaml
theta_node:    of: metamer   # 出现阈值 =(rank−1)·phyllo          {rank:self} 折常量
node_gate:     of: metamer   # ramp((Tsum_morph−theta_node)/w_app,0,1)  0未现→1已现
rate_nodeage:  of: metamer, class: rate   # max(0,T−Tbase_morph)·node_gate
node_age:      of: metamer, class: state, rate: rate_nodeage     # 出现后累积热龄
internode_len: of: metamer   # maxlen/(1+e^(−klen·(node_age−tmlen)))   ← 新科学·provenance:文献
node_height:   of: metamer   # Σ_{j≤k} internode_len_j（累积；用 prev 链或 over 前驱）
```
- `theta_node` 用 §风险4 的 `{rank:self}` 折常量（已有特性）。`node_gate` 复刻 fruit gate 机制（L1 门控出现）。
- `node_height` = 该节位离地高 = 下方各节间长之和；用 chain `prev` 链累加（`node_height_k = node_height_{k-1} + internode_len_k`，根节点 = internode_len_1）。**这是几何摆放器要读的关键量**（§5）。

**★时钟一致性（请首席科学家确认）**：节点出现用 `Tsum_morph`（Tbase=4），而 F1 现有**果实出现**用 `Tsum`（Tbase=10）→ 两者基温不同、整季会**轻微 desync**（果与其节位错位）。两个选择：
- **(A·推荐) 纯加法**：F2 只加节点/节间，**果实区室一字不动**（F1 碳结果**逐位不变**=回归锚保住）；node/fruit desync 作**已知近似**，留 F2.5/F3 把果实出现绑到其节位出现来消除。
- (B) 立即统一：把果实 `theta` 也移到 `Tsum_morph`（Tbase 10→4）→ 节果同钟、无 desync，但**改了 F1 行为**（fruit 出现时序变）、回归锚需重设。
推荐 **A**（锚保住、风险低、desync 是可见但无害的近似，诚实标注）。

## 5. 几何绑定（GIS 侧 `fspmPlant.js`）

- v1 摆放器用**常数 `L_int`** 算节点高度 `h_k = baseH + (k−1)·L_int`。
- **F2 改**：`eqcFspmPlant` 多拉 `internode_len__k` / `node_height__k` 序列；摆放器 `h_k = node_height_k`（从模型读）。节点未出现（`node_gate≈0`）→ 不画该节及其上器官。
- **3D 回报**：拖生长滑块 → 株从矮苗**逐节长出 + 节间随热龄伸长**（株高 emergent）；果穗挂在真实高度的节位上。
- 资产插槽不变（茎/叶/果照旧）；只是节点高度数据源从常数换成模型变量。

## 6. 版本与模块日志

- `tomato_fspm.eq.yaml` 的 `meta` bump：`version: F2`，加 `changelog:` 段（F1→F2 加了哪些模块 + 出处）。
- `meta.modules` 加两个子系统：「**节间伸长**」(F-INTLEN…)、「**节点出现**」(F-NODEGATE…) → 自动进 GA-6b 模型生长动画 + GIS 植株发育阶段。
- **F1 作回归锚**：若采决策4-A（纯加法），F1 碳轨迹逐位不变可校验（守恒 4e-11 不变）。

## 7. 施工分步（每步 `cargo build/test --features cli --offline` 绿×2 配置 / GIS `npm build`+preview · 用户点头再提交）

1. **`provenance` 字段（EQC，§3）**：schema + 反序列化（中文枚举）+ EqJson/contract.ts 镜像 + 合成单测。**最小、隔离、先行**（独立于 F2 科学，可单独验证+提交）。
2. **F2 模型模块（EQC，§4）**：`tomato_fspm.eq.yaml` 加形态热钟 + per-metamer 节点门控/热龄/节间长/节点高度，每条带 `provenance`；bump meta F2 + changelog + modules。**验证**：`validate`（方程数+）/`structure`（🌿 metamer×6 含 internode_len）/`simulate`（节间长随 node_age S 形增、node_height 累加、**决策4-A 下 F1 碳逐位不变**）+ 量纲（internode_len[m]、Tsum_morph[°Cd]）。
3. **几何绑定（GIS，§5）**：`eqcFspmPlant` 拉 internode_len/node_height；摆放器节点高度从模型读 + node_gate 控显隐。**验证**：全新 server，进番茄房拖滑块见**株逐节长高**、零 console 报错。
4. **（顺带）契约/动画**：provenance 经 EqJson 带出，3D 拓扑/生长动画按出处上色（可并入或留打磨）。

## 8. 边界与留后

- **确认缺口（诚实，留后续 `推导/猜测`+标定）**：DIF→节间长、R:FR→避荫伸长（§10.5）；要时从他作物平移或推导、交 GP/标定。
- **节位剖面**：indeterminate 长主茎（20–40+ 节）的 rank 依赖 maxlen 剖面未参数化（[S25] 三品种单一 maxlen）→ F2 先用单一 maxlen，节位剖面留标定。
- **node/fruit 时钟统一**（若采 4-A）：把果实出现绑到其节位出现 → F2.5/F3。
- **后续版本**：F3 逐叶+光竞争（几何升为模型输入）、F4 花穗+坐果（合轴 §10.2 + truss 率 §10.3）、F5 L2 动态创建。
- **每步一验、绿灯 + 用户点头再提交**。

> **★F2 = 让番茄植株「长起来」**：节点按 plastochron 逐个长出、节间随热龄伸长，株高从模型 emergent（非常数）；核心方程全 `文献·确证`（[S25] indeterminate），DIF/R:FR 诚实留缺口；新增 `provenance` 字段把"出处诚实"做成机器可读、喂 GP + 动画。这是全发育大 arc 的第一块，3D 上拖滑块即见「矮苗逐节长高」。

---

## F3：逐叶器官化 + 简版光竞争（已实现）

> FSPM 的核心理由（光竞争）。科学基座见文献综述 §11（deep research 已 fact-check）。**几何→光→碳反馈环闭合在 EQC 内**：用 F2 的 `node_height` 按高度算逐叶光（垂直 Beer 剖面），无新 GIS→EQC 通道。

**已对齐决策（按推荐全要）**：①逐叶光合用 **Thornley 非直角双曲线**（比纯 LUE 好·会饱和，[B25] 参数）；②`k_ext` **固定 0.75**（番茄实测·动态 0.5/sinβ 留后）；③叶面积参数占位推导（形式文献·值待标定）；④`Psat=27` 矮生偏低·calibration false 待标定；⑤Beer **只垂直、不分辨株型架构**（全 3D 光追留远期）。

**模型设计（`tomato_fspm.eq.yaml`）**：
- per-metamer：`leaf_area`(=node_gate·max_leaf·logistic(node_age)·解耦 [S25]·provenance 推导) → `lai_above`(=leaf_area(next)+lai_above(next)·chain next 累加·顶=0) → `light_leaf`(=PAR·exp(−k·lai_above)·Beer·文献) → `photo_rate`(Thornley NRH·文献) → `photo_leaf`(=leaf_area·photo_rate·c_assim·推导)。
- 冠层：`LAI`=Σleaf_area、`A_gross`=Σphoto_leaf（替集总 LUE·PAR·f_int）。**删集总 LUE/SLA/f_int**。
- **几何绑定（GIS）**：`eqcFspmPlant` 拉 `leaf_area` 序列；摆放器叶大小 ∝ leaf_area（幼叶小·展开后满）。

**验证**：validate 282 方程；simulate **守恒 3.91e-11**（碳源变·结构不变·守恒取代 bit-identical）、**光竞争**=light_leaf 底 61→顶 400 µmol/m²/s（底叶被 5 片遮到 15%）、LAI=3、A_gross=819 mg/m²/步、新锚 C_fruit_tot=19397、Y=19.4 g/m²。

**留后**：叶面积参数云南标定、FvCB 升级（Sarlikioti Jmax 265→180 梯度）、全 3D 光追、逐叶碳（现叶碳仍集总）。

> **★F3 = 光竞争落地**：每叶按高度接光、上方叶遮下方叶（Beer）、Thornley 逐叶光合 Σ 成冠层源；几何→光→碳反馈环闭合在 EQC 内（用 F2 node_height）。这是 FSPM 的核心理由；3D 上叶随热龄展开、下层叶处更暗。

---

## F4：花穗起始 + 坐果率（果数 emergent，已实现）

> 让果数从碳经济长出来。科学基座见文献综述 §12。**闭合"碳→果数"环**（如 F3 的几何→光→碳）。

**已对齐决策（按推荐全要）**：①坐果绑源-库比 **GreenLab P_set(Q/D)**（非 Vanthoor 绝对碳流）+ Vanthoor 温度门；②确定式**连续**（非随机·EQC 友好）；③每穗花数**固定上限 8**（中果型分化定·research 确证花数近固定）；④Psat... 等占位待标；⑤**坐果锁存**（花开期定、冻结）。**★方法论（首席科学家定·标准约定）**：不反感开源代码/软件，但 SSOT 不直接调用外部模块（避屎山）→提取数学方法融入·开源软件运算逻辑披露比论文更详细=大参考·用数学不用源码无版权风险·引用即可（GreenLab 等好用就用）。

**模型设计（`tomato_fspm.eq.yaml`）**：
- `count:8`（每穗 8 花·固定）；`QD=clamp(Ap/ssp,0,5)`（clamp 防定结构后期 Q/D 假暴增）。
- `set_prop=gate·clamp(1−e^(−2.39(QD−0.12)),0,1)·g_Tset`（P_set·GreenLab Kang2011 文献 + 温度梯形·Vanthoor 文献；×出现门；弱光经 Q/D=碳饥饿落花 Li2022）。
- **★坐果锁存**：`set_gate`（状态）花开窗 `age<set_dur≈40°Cd` 内锁该花【开花时】Q/D×温度定值、过窗冻结（`rate=set_window·(set_prop−set_gate_prev)`）——坐果是一次性承诺·不被后期翻案（provenance 推导）。
- 入分配 `A_fr=ss·share·set_gate`（未坐→≈0 等效败育·余碳留缓冲库→守恒）；`set_count=Σset_gate`（有效果数 emergent）。
- **几何（GIS）**：未坐果（碳≈0）不画 → 3D 每穗果数 emergent；簇内位扩到 8。

**★引擎修**：`serve.rs` 请求线程改 `thread::Builder::stack_size(64MB)`——48 果 621 方程的 pass/eval 递归在默认 spawn 栈（Win ~1MB）溢出（CLI 主线程 8MB 不溢）→ serve 现像 CLI 一样扛大模型。

**验证**：validate 621 方程；**守恒 3.1e-11/1.5e-11**；**★果数 emergent 实测**：理想光坐 33/48·Y=21.7 vs 弱光 PAR120 坐 21/48·Y=9.0（碳缺→花开期低 Q/D→多败育·锁存冻结不翻案；★订正：旧稿"43/9"把产量 Y 误记为坐果数，实测 set_count=33.2/21.2、Y=21.7/9.0）；serve simulate 1.4MB/1.17s 不再崩。

**留后**：P_set 品种标定、每穗花数=f(条件)、三因子交互、带移除采收流（消 Q/D 后期暴增根因）、人工疏花操作、苹果随机分化范式。

> **★F4 = 果数 emergent**：每穗 8 花、坐果率绑源-库比（碳足多坐·碳缺落果）+ 温度门、花开期锁存；有效果数从碳经济长出来（理想 33 vs 弱光 21）。闭合"碳→果数"环。番茄确定式·苹果随机（半马尔可夫）留后。

---

## F5：采收流 + indeterminate 全季稳态（夯实基础阶段）

> 把番茄从「determinate 一茬 48 果」推到「indeterminate 全季稳态 + 分批采收」——这是承接云南随作物周期（2026-07→2027-03）分批到来的标定数据的**基础设施**（数据要跑满整季才有阶段对应物）。分三步：**F5a 采收（已实现）→ F5b 扩规模+引擎硬化 → F5c 标定接口**。
> ★科学澄清：Q/D 后期伪暴增 / buffer 虚胀的真因是 **determinate（无新库进场）**，不是"没采收"——成熟果汇强已≈0，移除它不改 ssp。采收流真实价值=**产量分批曲线 + 守恒账闭合**；消虚胀靠 F5b indeterminate。两块拼图各司其职。

### F5a 采收流（物理移除，已实现）

**核心**：成熟果（果热龄 > `te_harvest`≈550°Cd = 果停长 te 481 + 转色）结构碳按 `k_harvest` 指数移除、离开在株生物量、进累计采收库 `Y_harvest_cum`。

**★守恒升级**：`C_fr` 负采收项与 `Y_harvest_cum` 正项为【同一 `Σrate_harvest` 聚合】（单一真相源·守恒漏不掉，同"缓冲库排出=Σ分配"）→ 守恒从 C_total 升级到 **`C_system = C_total + Y_harvest_cum`**（C_total 自身因采收移出不再守恒）。产量主指标改 `Y_harvest`（分批累计采收·田间称重对标），`Y_fruit` 降级为在株果（随采收回落）。

**模型设计（`tomato_fspm.eq.yaml`）**：
- 参数：`te_harvest=550°Cd` / `w_harvest=15` / `k_harvest=0.3`（provenance 推导·待标定）。
- per-fruit：`harvest_gate`(=clamp((age−te_harvest)/w_harvest,0,1)) → `rate_harvest`(=harvest_gate·C_fr_prev·k_harvest，C_fr_prev 避代数环) → `rate_Cfr` 加 −rate_harvest。
- 累计采收：`Y_harvest_cum`(state, rate=Σrate_harvest)；诊断 `C_system` / `Y_harvest` / `Y_total`。

**验证**：validate 721 方程；**守恒机器精度（max残差 2e-11，与 F4 baseline 逐位相同·采收账精确抵消·零回归）**；采收动态=在株果 step40 见顶 14.16→采收启动→step80 采空、累计采收阶梯升到 21.74；总产出 `Y_total=21.74=F4 的 Y`（采收不改总产出）。

**★诚实标注**：①F4 实测坐果数订正（旧"43/9"是把产量 Y 误记为坐果数）——理想光 set_count=33.2·Y=21.7、弱光 set_count=21.2·Y=9.0。②**★守恒机器精度（2e-11）·无早期瞬态**：此前 F5a/F5b 反复报的"开季 71 残差/Cbuf 耗尽瞬态"是**守恒检查的 off-by-one 假象**（错把 Δstock[t→t+1] 配 net[t]，应配 net[t+1]——轨迹里 `state[n]=state[n-1]+dt·rate[n]`、rate 与源汇同步用 state[n-1]）；F5c 守恒诊断 CLI 修正对齐后确认模型**自始至终机器精度守恒**。③6 节 determinate 下采完即空、Cbuf 后期虚胀（step149=79356）——F5b indeterminate 才消。

### F5b indeterminate 扩规模（已实现·18 节云南模式）

metamer 6→**18**（云南长季留 18 穗打顶；荷兰 30-40 穗性能已验证扛得住）。**★引擎 pass 迭代化实测【无需】**：topo_order 早是 Kahn 迭代（深度无关）、FixD 把聚合 lower 成 VectorLit+Reduce 迭代 → eval 对器官数深度无关；40 穗 4631 方程 CLI 8MB 不溢栈（18穗3.7s/40穗8.6s·debug·250步）→ 引擎零改动（推翻"200 果会溢栈"预期）。**稳态验证**：18 节长节期 buffer~296/QD~0.74 稳·在株果 18.8 稳态挂果（vs 6 节 F5a 末 Cbuf=79356/QD撞5）→ 消 determinate 虚胀/暴增。株高 emergent 2.88m；整季产 69.5g/m²·坐 105/144。**★L1 边界**：18 穗打顶长满后季末采空+虚胀（拔园后非物理），真无限需 L2（毕业步）。**B2 可视化**：番茄温室进室内纯渲 18 节真植株（去占位锥·近相机 40 株），3D 见细主茎18节+沿茎绿叶+顶部红熟果穗。

### F5c 标定接口夯实

承接云南分批标定数据的工具链。三块：**① 守恒诊断 CLI（已实现）→ ② measurable 量审计 → ③ 参数选靶**。

**① 守恒诊断 CLI（已实现）**：`meta.balance` 声明守恒律（`BalanceLaw{name,stock,sources,sinks,tol}`·additive·**单一真相源·守恒结构进契约**）+ `eqc simulate --check-balance` 逐步核 `|Δstock−dt·(Σ源−Σ汇)|≤tol`（超容差非零退出·标定脚本可捕捉）。tomato 声明碳守恒律 `{stock:C_system, sources:[A_gross], sinks:[resp_total], tol:1e-6}`，实测 **max残差 2e-11 @step97 ✅**。**★步对齐**：`state[n]=state[n-1]+dt·rate[n]`、rate 与源汇 auxiliary 同步用 `state[n-1]` → 「进入第 n 步的存量变化 Δstock[n]」配 `net[n]`（差一步把"相邻步通量之差"误报成不守恒——**这正是此前"71"的真因，模型实则机器精度守恒**）。`balance_residual` 纯函数 + 3 单测（守恒/通量爬升对齐/漏项）。落地：schema `BalanceLaw` + lib/main `--check-balance` + tomato 声明；两配置 cargo test 全绿。**契约镜像（export/contract.ts）暂后置**（无前端消费者，见正文讨论）。

**② measurable 量审计（待做）**：田间能测的量都标 measurable + 口径对齐云南（株高/LAI/坐果数/单果重/累计采收）；★单果重加 `fruit_fresh_weight` 派生量（=C_fruit_tot/set_count/株密度/干物质率→g/果·田间称重对标）。

**③ 参数选靶（待做）**：参数级标定元数据（文献=冻结、推导/猜测=可标定），喂受约束 GP。

> **★F5a = 采收落地**：成熟果物理移除→分批采收产量曲线 emergent（在株见顶回落+累计阶梯上升）；守恒升级到 C_system 且与 F4 逐位一致=零回归。下一步 F5b 让植株 indeterminate 跑满整季。
