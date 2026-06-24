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
}
