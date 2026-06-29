// 契约的 TypeScript 镜像（子集）——对着 EQC `eqc export` / `/api/*` 写类型。
// 编译器替你抓「契约字段拼错/漂移」。P 后期改用 ts-rs 从 Rust 结构体自动生成（spec §8）。

export interface ModelEntry {
  id: string
  name: string
  has_drivers?: boolean
  coupled?: boolean
  sim_capable?: boolean
}
export interface ModelsJson {
  models: ModelEntry[]
}

export interface ParamJson {
  name: string
  name_cn: string
  display_name: string
  default: number
  unit?: string
  /** 向量参数（cohort 种子）的分量值；标量参数为 undefined。情景面板只调标量。 */
  values?: number[]
  management?: boolean
}

export interface VarJson {
  name: string
  display_name: string
  var_type: string
  /** Forrester 分类：state/rate/driving/auxiliary/parameter/control/semi_state/boundary */
  class: string
  dynamic: boolean
  unit?: string
  description?: string
  label?: string
  measurable?: boolean
  /** 胁迫/健康信号："factor"（1=好）/ "risk"（0=好）；前端据此画红绿灯。 */
  stress_factor?: string
  /** 红绿灯取整季哪个值："min"/"max"/"final"；缺省由 stress_factor 推断。 */
  stress_reduce?: string
  init?: number
  /** 积分状态量的速率来源变量名（X[n]=X[n-1]+rate·dt）；非状态量省略。镜像后端 VarJson.rate。 */
  rate?: string
  /** 延迟寄存器（semi_state）的上一步来源变量名；非延迟量省略。镜像后端 VarJson.prev。 */
  prev?: string
  /** FSPM 器官实例身份（结构/cohort 实例化变量才有）；前端「按器官上色/折叠」用。镜像后端 VarJson.instance。 */
  instance?: InstanceJson
}

// —— FSPM 器官结构（/api/model 的 structure 字段；地基风险2/3）——
export interface InstanceJson {
  entity: string
  id: string
}
export interface StructureJson {
  entities: { name: string; count: number; topology: string }[]
  instances: { id: string; entity: string; parent?: string }[]
  topology: { from: string; to: string; kind: string }[]
  // FSPM 风险3·聚合可见性：某输出变量沿拓扑邻域聚合而来（聚合已 lower 成标量，此处保留语义）。
  aggregations?: { output: string; kind: string; over: string; entity?: string }[]
}

export interface GpTargetJson {
  grammar: string
  inputs: string[]
  output_bounds?: [number, number]
  monotone?: Record<string, string>
  frozen?: boolean
}

export interface EqJson {
  id: string
  name: string
  output: string
  mathml: string
  refs: string[]
  reference?: string
  formula_display?: string
  /** GP 进化靶点标记（仅 gp_target 方程有）。 */
  gp_target?: GpTargetJson
  /** 方程来源档（出处诚实纪律）：文献/平移/推导/猜测。缺省=未标注。 */
  provenance?: '文献' | '平移' | '推导' | '猜测'
}

// —— GP（/api/evolve[/start|/status]）契约 ——
export interface GpTraj {
  DAT: number[]
  value: number[]
}
export interface GpBaseline {
  formula?: string
  formula_mathml?: string
  form?: string | null
  trajectory?: GpTraj | null
  error?: number | null
  complexity?: number | null
}
// —— 版本结构 diff（GA-4 GraphDiffJson；GP「看它长出什么」彩蛋 GA-6b Phase 3 用）——
export interface DiffNodeJson {
  id: string
  /** 角色：parameter / Forrester 类（state/rate/…）/ external。 */
  kind: string
}
export interface EqChangeJson {
  output: string
  from_id: string
  to_id: string
}
/** before→after 结构 diff（按本地名）。受约束 GP 只动一条方程 → 主要看 added_edges（长出的新依赖）
 *  + changed_equations（形式变了的方程，前端给它打脉冲）。added_nodes 一般为空（GP 不引入新变量）。 */
export interface GraphDiffJson {
  schema_version: number
  added_nodes: DiffNodeJson[]
  removed_nodes: DiffNodeJson[]
  kept_nodes: number
  added_edges: [string, string][]
  removed_edges: [string, string][]
  kept_edges: number
  added_equations?: string[]
  removed_equations?: string[]
  changed_equations?: EqChangeJson[]
  distance: number
  edge_similarity: number
}
/** 一个候选（单靶 ParetoEntry 或联合的一个 slot 都满足）。 */
export interface GpCandidate {
  target?: string
  output?: string
  complexity: number
  error?: number | null
  formula: string
  formula_mathml?: string
  mechanistic_form?: string | null
  rediscovery?: boolean
  provenance_suggestion?: string
  trajectory?: GpTraj | null
  provenance_stub?: string
  yaml_fragment?: string
  consts?: number[]
  /** 采纳此候选相对现有模型的结构 diff（GP「看它长出什么」3D 生长动画用）。 */
  structure_diff?: GraphDiffJson
}
/** 前沿一点：单靶=候选本身（扁平字段）；联合=含 slots。 */
export interface GpFrontEntry extends Partial<GpCandidate> {
  complexity: number
  error?: number | null
  slots?: GpCandidate[]
}
export interface EvolveResult {
  target?: string
  output?: string
  grammar?: string
  mode?: string
  joint?: boolean
  targets?: string[]
  n_obs?: number
  pareto_front: GpFrontEntry[]
  baseline?: GpBaseline
  baselines?: Record<string, GpBaseline>
  observed?: GpTraj | Record<string, GpTraj>
  pareto_svg?: string
}
export interface EvolveStatus {
  status?: string
  gen?: number
  total_gens?: number
  convergence_svg?: string
  result?: EvolveResult
  error?: string
  task_id?: string
}

// —— 优化 / 标定（/api/optimize、/api/calibrate）契约 ——
export interface Knob {
  var: string
  value: number
  unit?: string
  kind?: string
  bounds?: [number, number]
}
export interface Constraint {
  expr: string
  value: number
  max: number
  satisfied: boolean
  violation?: number
}
export interface ParetoPoint {
  objectives: number[]
  knobs: Knob[]
  feasible?: boolean
}
/** /api/optimize 与 /api/calibrate 共用（标定=旋钮为参数、目标为误差）。 */
export interface OptResult {
  error?: string
  multi_objective?: boolean
  objective?: { sense: string; expr: string }
  objective_value?: number | null
  feasible?: boolean
  best_knobs?: Knob[]
  constraints?: Constraint[]
  convergence_svg?: string
  optimizer?: { pop: number; iters: number; seed: number }
  // 多目标
  objectives?: { expr: string; sense: string }[]
  front?: ParetoPoint[]
  pareto_svg?: string
  // 标定附加
  zone?: string
  n_obs?: number
  observed_path?: string
  calibrated_at?: number
}

// —— 园区视图（/api/simulate、/api/zone、/api/observations）契约 ——
export interface SimSeries {
  steps?: number
  series?: Record<string, number[]>
  error?: string
}
export interface ZoneCalib {
  error?: number | null
  at?: number
  spec?: string
  n_obs?: number
}
export interface ZoneInfo {
  zone: string
  params?: Record<string, number>
  drivers?: Record<string, number>
  has_observed?: boolean
  calibration?: ZoneCalib | null
}
export interface ObservationsJson {
  zone: string
  exists: boolean
  observations: Record<string, [number, number][]>
  days: number[]
  error?: string
}

// —— 耦合仿真/优化（/api/couple、/api/couple-optimize）契约 ——
export interface CoupleOptResult {
  coupled?: boolean
  best_objective?: number
  best_knobs?: Record<string, number>
  objective?: string
  sense?: string
  convergence_svg?: string
  error?: string
}

// —— 模型编辑器（/api/source、/api/validate）契约 ——
export interface SourceJson {
  source?: string
  path?: string
  editable?: boolean
  error?: string
}
export interface ValidateJson {
  ok?: boolean
  errors?: string[]
  report_html?: string
  error?: string
}

export interface Calibration {
  calibrated?: boolean
  note?: string
  date?: string
}

export interface ModuleJson {
  id: string
  model: string
  name_cn: string
  name_en?: string
  description?: string
  calibration?: Calibration
  parameters: ParamJson[]
  variables: VarJson[]
  equations: EqJson[]
}

export interface ModelJson {
  schema_version: number
  modules: ModuleJson[]
  /** 是否有任一模块声明 meta.modules 子系统划分；前端「按子系统」配色据此启用（2D/3D 共用）。 */
  has_modules?: boolean
  /** Forrester 8 类 → 3D 鲜调颜色（Rust palette 单一真相源，与 2D 报告同源）；3D 据此上色。 */
  class_colors?: Record<string, string>
  /** FSPM 器官结构（实体/实例/拓扑）；结构/cohort 模型才有，前端「按器官折叠/上色」用。 */
  structure?: StructureJson
}

// —— 3D 拓扑布局（/api/layout3d；GA-5 力导向坐标，GA-6 前端渲染）契约 ——
export interface Node3dJson {
  /** 图节点 id，格式 `MODULE.name`（去前缀=本地名，与 data-var / selectedVars 同键）。 */
  id: string
  x: number
  y: number
  z: number
  /** ∝ 介数中心性，归一 0–1（前端定球半径）。 */
  size: number
  community: number
  depth: number
  /** 作者声明的子系统名（meta.modules 键，如「光合」「氮」）；参数/驱动/未分组或未声明 → 省略。
   *  GA-6「按子系统」配色 + 图例用（additive）。 */
  module?: string
  /** 该子系统的鲜调颜色（#rrggbb，Rust 单一真相源 palette 算、与 2D 同色相）；前端 3D 直接用。 */
  module_color?: string
}
export interface Layout3dJson {
  nodes: Node3dJson[]
  edges: [string, string][]
  /** 坐标范围 [-bound, bound]。 */
  bound: number
}

// —— 生长动画（/api/growth；GA-6b）：按子系统声明序的"章节"，2D/3D 同步逐章显形 ——
export interface GrowthChapter {
  key: string
  title: string
  /** 旁白字幕（非专家文案）。 */
  narration: string
  /** 本章揭示的节点本地名（与 data-var / 3D localName 同键）。 */
  nodes: string[]
}
export interface GrowthJson {
  chapters: GrowthChapter[]
}
