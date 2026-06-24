// 契约的 TypeScript 镜像（子集）——对着 EQC 的 `eqc export` / `/api/model` 写类型。
// 价值：编译器替你抓「契约字段拼错/漂移」。理想下一步可由 Rust 契约自动生成此文件。

export interface VarJson {
  name: string
  display_name: string
  var_type: string
  unit?: string
  class?: string
  description?: string
}

export interface ModuleJson {
  id: string
  name_cn?: string
  model?: string
  variables: VarJson[]
}

export interface ModelJson {
  schema_version: number
  modules: ModuleJson[]
}

export interface ModelEntry {
  id: string
  name: string
}

export interface ModelsJson {
  models: ModelEntry[]
}
