// 节点注释 / 配色 / 悬停卡：2D 结构图（Structure.svelte）与 3D 拓扑视图（Topology3d.svelte）
// 共用的单一真相源。从 Structure 抽出，避免 CLS_CN / tipHtml 两份漂移（项目「单一真相源」约定）。
import type { ModelJson, VarJson, ParamJson, EqJson } from './contract'

/** Forrester 分类 → 中文名。 */
export const CLS_CN: Record<string, string> = {
  state: '存量', rate: '速率', driving: '驱动', auxiliary: '辅助',
  parameter: '参数', control: '控制', semi_state: '半状态', boundary: '边界',
}

/** Forrester 分类 → 3D 节点球颜色。源自 2D Forrester 报告类配色（src/report/mod.rs §forr），
 *  但 GA-6 为 3D 可读性把**辅助/参数两种近似灰拉开明度**（辅助更亮、参数中灰，都在深底可见）：
 *  此处是 3D 专用副本，与 2D 报告在这两类上有意轻微不同（2D 有文字标签不靠颜色区分）。 */
export const CLASS_COLOR_3D: Record<string, string> = {
  state: '#3b82f6', semi_state: '#60a5fa', semistate: '#60a5fa', rate: '#f97316',
  driving: '#22c55e', auxiliary: '#cbd5e1', parameter: '#94a3b8',
  control: '#a855f7', boundary: '#0ea5e9',
}

/** 图例排序：Forrester 阅读顺序（存量→速率→派生→外部→配置）。 */
export const CLASS_ORDER = [
  'state', 'rate', 'auxiliary', 'driving', 'control', 'semi_state', 'parameter', 'boundary',
] as const

/** Forrester 分类 → 一句话含义（非专家能懂）。3D 图例「按类别」用；单一真相源。 */
export const CLASS_LEGEND: Record<string, string> = {
  state: '会随时间累积的量（如生物量、水量），像水箱里的水',
  rate: '存量每天增减的速度，像水龙头的流量',
  auxiliary: '由别的量当场算出的中间量（不累积）',
  driving: '来自外部的输入（天气等），模型只接收不计算',
  parameter: '固定常数（设定/标定值），不随时间变',
  semi_state: '某个量"上一时刻"的值（延迟）',
  control: '人为可调控的量（如灌溉、CO₂ 设定）',
  boundary: '边界条件',
}

/** 「按子系统」分类调色板：鲜亮、相互可分、避开灰（灰留给「其他」）；深底 #0f172a 上都醒目。
 *  子系统是模型相关、数量不定（草莓 9、温室 5），故按出现顺序循环取色，不写死映射。 */
export const MODULE_PALETTE = [
  '#4f9dff', '#ff8c42', '#3ddc97', '#c77dff', '#ff5d8f', '#ffd23f',
  '#2ec4b6', '#ff6b6b', '#a3e635', '#bdb2ff', '#f0a6ca', '#7dd3fc',
]
/** 非作者子系统（参数/驱动/未分组）并成一行「其他」用的中性灰。 */
export const MODULE_OTHER_COLOR = '#64748b'
export const MODULE_OTHER_LABEL = '其他'

/** 由「当前模型实际出现的子系统名」（按节点顺序，含重复）构造 名→颜色 映射（首次出现定色）。 */
export function moduleColorMap(modulesInOrder: (string | undefined)[]): Map<string, string> {
  const m = new Map<string, string>()
  let i = 0
  for (const name of modulesInOrder) {
    if (name && !m.has(name)) {
      m.set(name, MODULE_PALETTE[i % MODULE_PALETTE.length])
      i++
    }
  }
  return m
}

export const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')

/** 图节点 id（`MODULE.name`）→ 本地名（去模块前缀）；与 2D 报告 data-var / store.selectedVars 同键。 */
export const localName = (id: string) => {
  const i = id.indexOf('.')
  return i >= 0 ? id.slice(i + 1) : id
}

/** 遍历所有模块（耦合=多模块、单模型=1 模块），返回首个匹配的变量/参数。 */
export function findVar(
  contract: ModelJson | null,
  name: string,
): { kind: 'var'; v: VarJson } | { kind: 'param'; p: ParamJson } | null {
  for (const m of contract?.modules ?? []) {
    const v = m.variables.find((x) => x.name === name); if (v) return { kind: 'var', v }
    const p = m.parameters.find((x) => x.name === name); if (p) return { kind: 'param', p }
  }
  return null
}

export function findEq(contract: ModelJson | null, name: string): EqJson | undefined {
  for (const m of contract?.modules ?? []) {
    const e = m.equations.find((x) => x.output === name); if (e) return e
  }
  return undefined
}

export function dispName(contract: ModelJson | null, name: string): string {
  const r = findVar(contract, name); if (!r) return name
  return (r.kind === 'var' ? r.v.display_name : r.p.display_name || r.p.name_cn) || name
}

/** 节点的 Forrester 分类（变量取其 class、参数为 parameter、未知回退 auxiliary）。 */
export function classOf(contract: ModelJson | null, name: string): string {
  const r = findVar(contract, name)
  if (!r) return 'auxiliary'
  return r.kind === 'var' ? r.v.class : 'parameter'
}

/** 3D 节点球颜色（按分类，复用 2D 配色）。 */
export function nodeColor3d(contract: ModelJson | null, name: string): string {
  return CLASS_COLOR_3D[classOf(contract, name)] ?? '#9ca3af'
}

/** 悬停注释卡 HTML：显示名 + 分类/单位 + 物理意义 + 方程 MathML + 出处。2D/3D 共用。 */
export function tipHtml(contract: ModelJson | null, name: string): string {
  const info = findVar(contract, name), eq = findEq(contract, name), disp = dispName(contract, name)
  let h = '<div class="t-name">' + esc(disp) + '</div>'
  if (disp !== name) h += '<div class="t-id">代号 ' + esc(name) + '</div>'
  if (info?.kind === 'var') {
    const v = info.v, cls = CLS_CN[v.class] || v.class
    h += '<div class="t-sub">' + esc(cls) + (v.unit ? ' · 单位 ' + esc(v.unit) : '') + '</div>'
    if (v.description) h += '<div class="t-desc"><b>物理意义</b>：' + esc(v.description) + '</div>'
  } else if (info?.kind === 'param') {
    const p = info.p
    h += '<div class="t-sub">参数' + (p.unit ? ' · 单位 ' + esc(p.unit) : '') + '</div>'
    h += '<div class="t-desc">默认值 = ' + p.default + '</div>'
  }
  if (eq) {
    h += '<div class="t-eq">' + eq.mathml + '</div>'
    if (eq.reference) h += '<div class="t-cite">📖 ' + esc(eq.reference) + '</div>'
  } else if (info?.kind === 'var') {
    const c = info.v.class
    let why = '（外部输入）'
    if (c === 'state') why = '（状态量：值由其速率逐步积分得到，无显式方程）'
    else if (c === 'semi_state') why = '（延迟寄存器：取来源变量的上一步值）'
    else if (c === 'driving') why = '（驱动量：来自外部输入/天气数据）'
    else if (c === 'control') why = '（控制量：可由用户/环控设定）'
    h += '<div class="t-cite t-none">' + why + '</div>'
  }
  return h
}

/** 在 document.body 上创建一个悬停注释卡元素（命令式，定位自由；样式见 Structure.svelte 的 :global(.eqc-nodetip)）。 */
export function makeTip(): HTMLDivElement {
  const t = document.createElement('div')
  t.className = 'eqc-nodetip'
  document.body.appendChild(t)
  return t
}

/** 把注释卡填充 HTML 并定位到 (x,y)（自动避让视口右/下边界）。 */
export function showTipAt(tip: HTMLDivElement, html: string, x: number, y: number) {
  tip.innerHTML = html
  tip.style.display = 'block'
  const tw = tip.offsetWidth, th = tip.offsetHeight
  let px = x + 14, py = y + 14
  if (px + tw > window.innerWidth - 8) px = x - tw - 14
  if (py + th > window.innerHeight - 8) py = window.innerHeight - th - 8
  tip.style.left = Math.max(8, px) + 'px'
  tip.style.top = Math.max(8, py) + 'px'
}

export const hideTip = (tip: HTMLDivElement | undefined) => { if (tip) tip.style.display = 'none' }
