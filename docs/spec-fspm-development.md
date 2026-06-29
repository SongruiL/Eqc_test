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
| F3 | 逐叶器官化 + 简版光竞争（几何→光→碳反馈） | 后续 spec |
| F4 | 花穗起始（合轴）+ 每穗花数 + 坐果率（果数 emergent） | 后续 spec |
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
