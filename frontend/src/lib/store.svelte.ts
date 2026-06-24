// 全局响应式状态（Svelte 5 runes in .svelte.ts）：模型/处理区/模式/当前工作区。
// 组件 import 这个 store 读写即响应式联动——比 v1 的全局变量 + 手动 DOM 同步干净得多。
import type { ModelEntry, ModelJson, Knob } from './contract'
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
  // 仿真视图状态（提到全局：跨工作区共享——优化「叠加最优旋钮」要写它、仿真工作区读它）。
  selectedVars: [] as string[],
  scenario: { p: {}, i: {}, d: {} } as {
    p: Record<string, number> // 参数覆盖
    i: Record<string, number> // 初值覆盖
    d: Record<string, number> // 恒定驱动覆盖（driver_const，无滑块、只影响曲线）
  },
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
    fixWorkspaceForModel()
    await reloadModel()
  } catch {
    store.connected = false
  }
}

export async function reloadModel() {
  try {
    store.modelJson = await fetchModel(store.model)
    store.connected = true
    resetSimView()
  } catch {
    store.connected = false
  }
}

/** 切模型后重置仿真视图：默认勾选（Y 优先，否则所有 output）+ 清情景覆盖。 */
function resetSimView() {
  const vs = store.modelJson?.modules?.[0]?.variables ?? []
  const hasY = vs.some((v) => v.name === 'Y')
  store.selectedVars = vs.filter((v) => (hasY ? v.name === 'Y' : v.var_type === 'output')).map((v) => v.name)
  store.scenario = { p: {}, i: {}, d: {} }
}

export function clearScenario() {
  store.scenario = { p: {}, i: {}, d: {} }
}

/** 把一组旋钮（优化/标定结果）叠加进情景覆盖（param→p、init→i、driver_const→d）。 */
export function applyKnobs(knobs: Knob[]) {
  for (const k of knobs) {
    if (k.kind === 'param') store.scenario.p[k.var] = k.value
    else if (k.kind === 'init') store.scenario.i[k.var] = k.value
    else if (k.kind === 'driver_const') store.scenario.d[k.var] = k.value
  }
}

/** 切到耦合条目时把工作区收敛到 结构/耦合（其它工作区对耦合条目会报错）。 */
function fixWorkspaceForModel() {
  const e = store.models.find((m) => m.id === store.model)
  if (!e?.coupled) return
  const allowed = e.sim_capable ? ['structure', 'couple'] : ['structure']
  if (!allowed.includes(store.workspace)) store.workspace = 'structure'
}

export function switchModel(id: string) {
  if (!id || id === store.model) return
  store.model = id
  save('eqc_v2_model', id)
  fixWorkspaceForModel()
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
