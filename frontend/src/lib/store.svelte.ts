// 全局响应式状态（Svelte 5 runes in .svelte.ts）：模型/处理区/模式/当前工作区。
// 组件 import 这个 store 读写即响应式联动——比 v1 的全局变量 + 手动 DOM 同步干净得多。
import type { ModelEntry, ModelJson } from './contract'
import { fetchModels, fetchModel } from './api'

export const EXPERT_WS = ['structure', 'simulate', 'optimize', 'calibrate', 'gp', 'edit'] as const
export const PARK_WS = ['understand', 'entry', 'calibrate'] as const

export const store = $state({
  models: [] as ModelEntry[],
  model: '', // 当前模型 id（工作区花名册）；单模型为 'default'
  zone: '1',
  mode: 'expert' as 'expert' | 'park',
  workspace: 'simulate' as string,
  modelJson: null as ModelJson | null,
  connected: false,
})

function ls(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}
function save(key: string, val: string) {
  try {
    localStorage.setItem(key, val)
  } catch {
    /* ignore */
  }
}

export async function loadModels() {
  try {
    const j = await fetchModels()
    store.models = j.models ?? []
    const saved = ls('eqc_v2_model')
    store.model = store.models.some((m) => m.id === saved) ? saved! : store.models[0]?.id ?? ''
    store.mode = ls('eqc_v2_mode') === 'park' ? 'park' : 'expert'
    const ws = ls('eqc_v2_ws')
    if (ws) store.workspace = ws
    await reloadModel()
  } catch {
    store.connected = false
  }
}

export async function reloadModel() {
  try {
    store.modelJson = await fetchModel(store.model)
    store.connected = true
  } catch {
    store.connected = false
  }
}

export function switchModel(id: string) {
  if (!id || id === store.model) return
  store.model = id
  save('eqc_v2_model', id)
  reloadModel()
}

export function setWorkspace(w: string) {
  store.workspace = w
  save('eqc_v2_ws', w)
}

export function setMode(m: 'expert' | 'park') {
  store.mode = m
  save('eqc_v2_mode', m)
  const allowed: readonly string[] = m === 'park' ? PARK_WS : EXPERT_WS
  if (!allowed.includes(store.workspace)) setWorkspace(m === 'park' ? 'understand' : 'simulate')
}
