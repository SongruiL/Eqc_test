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
  init?: number
}

export interface EqJson {
  id: string
  name: string
  output: string
  mathml: string
  refs: string[]
}

export interface ModuleJson {
  id: string
  model: string
  name_cn: string
  name_en?: string
  description?: string
  parameters: ParamJson[]
  variables: VarJson[]
  equations: EqJson[]
}

export interface ModelJson {
  schema_version: number
  modules: ModuleJson[]
}
