# spec：FSPM 功能层③ —— 真实植株几何 / 3D（风险5 v1，✅ 已实现并验证）

> **状态（2026-06-30）**：v1 施工分步 §7 ①–④ 全部完成并验证（GIS 仓 `smart-agriculture-gis`：
> `src/lib/fspmPlant.js` 新增 + `eqcApi.js`/`CesiumMap.vue` 接线）。番茄房进室内渲 12 株器官级 FSPM
> （果径∝`C_fr`、色∝`age/te`、`gate` 控显隐），室内生长滑块拖动看错峰生长（6%幼苗→25%转色→100%熟果），
> 全新 server 零 console 报错。EQC 内核零改动（几何是 viewer 侧映射）。**下一程 = 全发育（见
> `spec-fspm-development.md`，待写）：节间伸长/节点出现 → 逐叶+光竞争 → 花穗+坐果 → L2 动态创建。**

> 建在已完成的 **FSPM 地基**（[`spec-fspm-foundation.md`](spec-fspm-foundation.md)，风险1 实例身份 + 风险2 NodeResolver）、
> **拓扑聚合算子**（[`spec-fspm-aggregation.md`](spec-fspm-aggregation.md)，风险3）、**器官流**（[`spec-fspm-organ-flow.md`](spec-fspm-organ-flow.md)，风险4）之上。
> 目标：把器官级的量（逐果碳/热龄、节位/穗拓扑）做成温室内**可见的真实植株几何**，喂 GIS 数字孪生 B2
> （温室内真植株 3D，见 GIS 仓 `CLAUDE.md` 路线 B2）。
> **红线沿用**：器官身份结构化一等；EQC 计算内核 v1 **零改动**（几何是 viewer 侧的可视化映射，非模型输出）。

## 0. 已对齐决策（首席科学家拍板）

| # | 抉择 | 决定 | 理由（一句话） |
|---|---|---|---|
| 1 | 几何引擎 / 落点 | **GIS 侧程序化吃 EQC 契约**（非 EQC 出 glTF；非 L-System），且**器官做成可替换资产插槽** | 沿用 B1 已验证的程序化→Cesium 管线；生长动画白送；EQC 零改动；插槽让 3D 建模师的未来资产 drop-in |
| 1b | 资产插槽范围 | **果 + 叶都是插槽**（程序化现在 / 各物种 glTF 以后） | 各物种叶/果形状不同，未来放仿真模型 → 几何与美术解耦、按物种替换 |
| 2 | v1 范围 | 单株番茄 / 到单果（24 果）/ 静态但 live（滑块回放）/ 相对大小 / 代表株（不去集总） | 如实显示模型真算的东西，不伪造未算的；回报集中在果实 |
| 3 | 叶 | v1 通用叶（数据集总）；逐叶碳/逐叶截光留风险5+ | 模型叶集总、无逐叶数据；逐叶截光是几何反哺的**下一步科学**（v2） |
| 4 | 果几何精度 | 经纬球（示意）；以后请 3D 建模师建仿真果实 glTF | FSPM 模型示意即可；精细网格是美术资产、走插槽替换 |

**核心立场（SSOT 三分）**：**EQC 拥生理数据**（碳/龄/拓扑）· **GIS 拥摆位**（叶序+拓扑→坐标）· **3D 建模师拥美术资产**（果/叶网格）。谁都不越界——EQC 不持网格，GIS 不重算生理，美术不碰数据管线。

---

## 1. 核心架构：器官资产插槽（organ asset slot）

每个器官类型 = **三件套**，统一抽象：

```
器官实例 = 摆位变换(从拓扑+叶序算)  +  数据绑定(契约变量→大小/色/显隐)  +  可替换资产(程序化 | glTF)
```

| 器官 | 摆位 | 数据绑定（契约量） | 资产 v1 → 以后 |
|---|---|---|---|
| **果 fruit** | 节位高度 + 穗向 + 簇内位 | 半径∝`C_fr` · 色∝`age/te` · 显隐∝`gate` | 经纬球 → 仿真果 glTF |
| **叶 leaf** | 节位高度 + 叶序方位 | 大小∝(v1: 常数/LAI 均摊) · 角∝叶序 | 通用叶片 → 各物种叶 glTF |
| 茎 stem | 节链轴 | 高度∝节数·节间长 | 锥形线/柱 |
| （花/茎段…） | 同上 | 后续 | 后续 |

**叶的"形状"vs"数据"必须分清**（否则与决策3 矛盾）：
- **形状（资产/网格）= 插槽，第一天就预留**：v1 填通用叶片，建模师来换各物种 glTF。
- **数据（大小/角度/逐叶碳）= 仍集总**：v1 用常数/LAI 均摊；风险5+ 逐叶截光科学来了才驱动插槽的大小/角度参数。

**按物种 keying**：资产按 `(crop, organ)` 索引，延续 `interiorPlants.js` 现有 `CROP_FORM` 表。**v1 只番茄有器官级 FSPM 模型**（`tomato_fspm_organ`）→ 番茄长真 FSPM 植株、草莓/蓝莓维持 `interiorPlants` 占位，插槽替它们留位，等其 FSPM 模型再填。

**插槽接口（概念，施工时定签名）**：
```
renderOrgan(asset, modelMatrix, scalars) → Cesium 几何
  // v1 程序化实现：返回一个 GeometryInstance（经纬球 / 叶片面片），合并进单 Primitive
  // 以后 glTF 实现：返回一个 Cesium.Model 实例，modelMatrix = ENU·平移(节)·旋转(叶序)·缩放(半径)
  // 上层（摆位+绑定）对两种实现无感 → 美术资产一次替换、数据管线不动
```

---

## 2. 现状核对（数据接口，已实测 `/api/model` + `/api/simulate`）

**契约已提供（生理，够用）**：
- `structure.entities`：`metamer×6 (chain)`、`fruit×24`。
- `structure.instances`（30）：节 `"1".."6"`；果 `"1.1".."6.4"`，带 `parent:"metamer#k"`。
- `structure.topology`（29 边）：5 succession（节链 1→…→6）+ 24 contains（节→果）。**= 植株骨架的直接来源**。
- `/api/simulate` **逐果序列**：`C_fr__k_f`（终值≈924mg、错峰 S 曲线）、`age__k_f`、`gate__k_f`，各 150 步；`node_sink__k`、`C_fruit_tot`。
- **`VarJson.instance = {entity:"fruit", id:"1.1"}`** → 序列键 `C_fr__1_1` 与结构身份**免字符串反解**互映。

**契约不提供（几何/形态，住 GIS 侧参数，非模型输出）**：
- 节间长、叶面积分布、果径绝对 cm、真实 3D 坐标。这些是**几何参数 + 摆放器**，像温室的 `ghParams` 一样住 GIS（可做滑块）。**v1 需 EQC 零新增数据。**

**为何不复用 `layout3d`**（[`src/graph/layout3d.rs`](../src/graph/layout3d.rs)）：它是**方程依赖图**的力导向嵌入（坐标=f(计算深度/社区/介数)），果会被其数学依赖打散，植物学上无意义。植株坐标另从**拓扑+叶序**算（§3），是更简单的确定性计算。

---

## 3. 摆放器算法（botanical placer，GIS 侧确定性，无 RNG）

从 `structure.topology` 算每器官的局部 ENU 坐标（米），全程确定性：

```
茎轴：竖直（可选微 lean）。节 k（succession 链序）在高度 h_k = h0 + k · L_int
叶序：节 k 方位角 θ_k = θ0 + k · 137.5°（黄金角螺旋）
果穗：节 k 的穗在 θ_k 方向、自节微垂；4 果（k.1..k.4）沿穗轴按果序簇状散开
单果：经纬球，半径 = 数据绑定(§4)，放穗内簇位
叶  ：节 k 一片（或数片）叶片资产，方位 θ_k（与穗错开，如对生/互生），叶柄长 L_pet
```

- **节序**：从 succession 边定 1→6 顺序（拓扑权威，不靠 id 数值假设）。
- **果挂哪个节**：从 contains 边（节#k → 果#k.f）或 `instance.parent`。
- **参数**（GIS 侧、按物种、可滑块）：`L_int`（节间长，番茄≈0.07–0.10m）、`L_pet`、`θ0`、叶 phyllotaxy 模式。
- 局部坐标 → 经 `eastNorthUpToFixedFrame(株位)` 摆到温室内（与 `interiorPlants.js` 同一 ENU 套路）。

---

## 4. 数据绑定（契约 → 几何映射）

**逐果序列取数**：果 `instance.id = "k.f"`（来自 `structure.instances`）→ 键 `C_fr__{k}_{f}`（`.`→`_`，约定构造；`VarJson.instance` 可权威交叉核对）。

| 量 | 绑定 | 说明 |
|---|---|---|
| 果半径 | `r = r_min + (r_max−r_min)·clamp(C_fr / C_ref, 0, 1)` | **相对大小**；`C_ref` 取**固定参考**（如 `wmax/ASR` 派生的单果潜在碳）→ 果随自身碳单调长大、滑动不跳变。绝对 cm 标定留田间。 |
| 果色 | `ripe = clamp(age / te, 0, 1)` → 绿→红 ramp | `te`=481°Cd（契约参数）；`age` 逐果序列。 |
| 果显隐 | `gate < ε` → 不渲染（或缩到≈0） | 未坐果器官（预分配·门控）不画。 |
| 叶大小 | v1：常数 / `LAI / 节数` 均摊（**集总**） | 逐叶面积驱动 = 风险5+。 |
| 叶角 | 叶序 θ_k | 几何，非数据。 |

**生长动画 = 复用 B1·C 滑块**（GIS `ghScrubFrac` / `valueAt(frac)`，见 GIS `CLAUDE.md` B1·C）：拖生长进度 0→1 → 逐果取 `C_fr/age` 在该 frac 的值 → 重算半径/色（就地改 `GeometryInstance` 颜色 + 重建大小，或整体重挂）。**非烤关键帧**。

---

## 5. v1 模块设计（GIS 侧 `fspmPlant.js`，与 `greenhouse3d.js`/`interiorPlants.js` 平级）

```
入口  buildFspmPlantPrimitive(originLon, originLat, position, fruitState, topo, params)
  fruitState  进室内时拉 /api/simulate?model=tomato_fspm_organ，按 scrub frac 取每果 {C_fr, age, gate}
  topo        /api/model 的 structure（topology/instances）→ 摆放器（§3）
  params      { L_int, L_pet, θ0, r_min, r_max, leafForm }（GIS 侧，可滑块）
  → 单个 Cesium.Primitive（茎 + 24 果球 + 叶），ENU 摆放（合并 GeometryInstance，PerInstanceColor 逐果上色）
更新  updateFspmPlant(frac)   // 滑块 → 重算果半径/色
资产  ORGAN_ASSETS[crop][organ]  // v1 程序化球/叶片；预留 glTF 实现位（§1 插槽接口）
```

**接线（`CesiumMap.vue`）**：在 `renderInteriorPlants`（line ~2906）旁——温室 `crop==='tomato'` 且 FSPM 可用 → 选中/英雄株渲 FSPM 细节，其余株仍 `interiorPlants` 占位；`enterInterior` 建、`exitInterior` 清（同 `interiorPlantPrim` 生命周期）。株数/细节封顶（沿用 `CAP=600` 思路，FSPM 株另设小上限防 24×N 炸图元）。

---

## 6. 范围与边界

**v1 做**：番茄单株 → 24 单果（碳定大小、龄定色、gate 控显隐）+ 通用叶 + 锥茎；静态但 live（滑块回放）；相对大小；代表株（**清楚标注**：每 m² 集总 → 同温室各株同形）；果/叶资产插槽（程序化实现 + 预留 glTF 位）。

**v1 不做（各自留后理由）**：
- glTF 导出/加载 → 等 3D 建模师交资产时，GIS 才长第一条 `Cesium.Model` 加载路径（加载**美术资产**，非 EQC 烤的整株）。
- 逐叶碳 / 逐叶截光 → 风险5+ 科学（几何反哺光竞争，v2）。
- 去集总真个体（每株自己的微气候/竞争）→ 需 per-plant 模型实例，大科学步。
- 木质分枝（蓝莓/苹果）→ `tree` kind；草莓/蓝莓器官级 FSPM 模型 → 待建（插槽已留位）。
- 自动循环播放 → 滑块已够，循环是几行的事。

---

## 7. 施工分步（GIS 侧；每步 `npm run build` 过 + 真 `preview_screenshot` 验证 + 用户点头再提交）

> EQC 侧 v1 **零改动**（契约已够）。构建/验证走 GIS 便携 node22 + Preview MCP（见 GIS `CLAUDE.md`）；调 Cesium **必 `preview_stop`+`preview_start` 全新 server 测、不能只 reload**（HMR 残留假崩，铁律见 GIS `CLAUDE.md` Stage3）。

1. **摆放器 + 静态番茄株（§3）**：`fspmPlant.js` 从 `/api/model` structure 算骨架 + `/api/simulate` 终值建 24 果球（经纬球）+ 锥茎 + 通用叶；接 `enterInterior`（番茄房，英雄株）。验证：进番茄温室见按碳定大小、按龄定色的 24 果错峰株。
2. **资产插槽抽象（§1）**：把果/叶建几何抽成 `ORGAN_ASSETS[crop][organ]` + `renderOrgan` 接口（程序化实现）；预留 glTF 实现位（不实现）。验证：行为不变、接口就位。
3. **生长动画（§4，复用 B1·C 滑块）**：`updateFspmPlant(frac)` 接生长进度滑块 → 逐果 `valueAt(frac)` 重算大小/色。验证：拖滑块见果从小到大、绿到红、晚果后出现。
4. **打磨 + 封顶 + 占位共存**：FSPM 株与 `interiorPlants` 占位共存（番茄 FSPM、余作物占位）；株数/细节封顶；相对大小归一稳定（固定 `C_ref`）。

---

## 8. 立场与边界

- **科学诚实优先**：v1 如实显示模型真算的（24 错峰果碳）、清楚标注占位的（集总叶、相对大小、代表株）。不画无数据的逐叶几何（假精度）。
- **SSOT 三分**：EQC 生理 / GIS 摆位 / 美术资产，插槽解耦。
- **EQC 内核零改动**：几何是 viewer 侧映射；可选 EQC additive（**往后**）= 若节间伸长有科学意义则给 metamer 加"节间长"变量（温度驱动伸长才有生理依据）。
- **每步一验、绿灯 + 用户点头再提交**（项目一贯节奏）。

> **★风险5 v1 = 把器官量做成可见的真实植株**：番茄 24 果按模拟碳定大小、按热龄定色、按拓扑+叶序摆位，温室内 live 可滑块回放；果/叶都是可替换资产插槽（程序化现在、各物种 glTF 以后）。这是"地球→基地→温室→**真植株 FSPM 3D**→数字孪生"缩放主轴的最后一块几何拼图。
