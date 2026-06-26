// 对 EQC `/api/*` 契约的薄封装。前端只消费，不重实现逻辑（EQC 持有事实）。
import type {
  ModelsJson, ModelJson, EvolveStatus, OptResult, SimSeries, ZoneInfo, ObservationsJson,
  SourceJson, ValidateJson, CoupleOptResult, Layout3dJson, GrowthJson,
} from './contract'

/** `?model=` / `&model=`（id 为空则省略）。 */
export function modelQS(model: string, sep: '?' | '&' = '&'): string {
  return model ? `${sep}model=${encodeURIComponent(model)}` : ''
}

export async function fetchModels(): Promise<ModelsJson> {
  return (await fetch('/api/models', { cache: 'no-store' })).json()
}

export async function fetchModel(model: string): Promise<ModelJson> {
  return (await fetch('/api/model' + modelQS(model, '?'), { cache: 'no-store' })).json()
}

/** 3D 拓扑力导向坐标（GA-5 Rust 算、GA-6 前端渲染）。返回 {nodes,edges,bound} 或后端错误信封。 */
export async function fetchLayout3d(model: string): Promise<Layout3dJson> {
  return (await fetch('/api/layout3d?_=' + Date.now() + modelQS(model), { cache: 'no-store' })).json()
}

/** 生长动画 plan（GA-6b）：按子系统声明序的章节 + 旁白，2D/3D 同步逐章显形。 */
export async function fetchGrowth(model: string): Promise<GrowthJson> {
  return (await fetch('/api/growth?_=' + Date.now() + modelQS(model), { cache: 'no-store' })).json()
}

/** 整季轨迹图 SVG（EQC 自生成）。`p`/`init`/`d` = 情景覆盖（参数/初值/恒定驱动）。 */
export function chartUrl(
  model: string,
  vars: string[],
  p: Record<string, number>,
  init: Record<string, number>,
  d: Record<string, number> = {}
): string {
  const enc = (o: Record<string, number>) =>
    Object.entries(o)
      .map(([k, v]) => `${k}:${v}`)
      .join(',')
  let u = `/api/chart.svg?vars=${encodeURIComponent(vars.join(','))}` + modelQS(model)
  const ps = enc(p), is = enc(init), ds = enc(d)
  if (ps) u += `&p=${encodeURIComponent(ps)}`
  if (is) u += `&init=${encodeURIComponent(is)}`
  if (ds) u += `&d=${encodeURIComponent(ds)}`
  return u + `&_=${Date.now()}`
}

/** 结构图报告 HTML（iframe src）。`layout` 布局、`level` 粒度（变量/模块）、`color` 配色（类别/子系统）。 */
export function reportUrl(model: string, layout: string, level = 'variable', color = 'class'): string {
  return `/api/report?layout=${encodeURIComponent(layout)}&level=${encodeURIComponent(level)}&color=${encodeURIComponent(color)}` + modelQS(model)
}

// —— GP 异步进化 ——
export interface StartOpts {
  model: string
  zone: string
  targets: string[]
  pop: number
  gens: number
  seed: number
  memetic: boolean
}
/** 起后台进化任务（≥2 靶=联合 targets=，否则单靶 target=）→ {task_id} 或 {error}。 */
export async function startEvolve(o: StartOpts): Promise<{ task_id?: string; error?: string }> {
  const sel =
    o.targets.length >= 2
      ? 'targets=' + encodeURIComponent(o.targets.join(','))
      : 'target=' + encodeURIComponent(o.targets[0])
  const u =
    '/api/evolve/start?' +
    sel +
    modelQS(o.model) +
    `&zone=${encodeURIComponent(o.zone)}&pop=${o.pop}&gens=${o.gens}&seed=${o.seed}` +
    (o.memetic ? '&memetic=true' : '') +
    `&_=${Date.now()}`
  return (await fetch(u, { cache: 'no-store' })).json()
}
export async function evolveStatus(id: string): Promise<EvolveStatus> {
  return (
    await fetch(`/api/evolve/status?id=${encodeURIComponent(id)}&_=${Date.now()}`, { cache: 'no-store' })
  ).json()
}

// —— 优化 / 标定（同步端点：填 spec → 跑 → 结果+收敛曲线）——
export async function runOptimize(model: string, spec: string): Promise<OptResult> {
  const u = `/api/optimize?spec=${encodeURIComponent(spec)}` + modelQS(model) + `&_=${Date.now()}`
  return (await fetch(u, { cache: 'no-store' })).json()
}
export async function runCalibrate(model: string, spec: string, zone: string): Promise<OptResult> {
  const u =
    `/api/calibrate?spec=${encodeURIComponent(spec)}&zone=${encodeURIComponent(zone)}` +
    modelQS(model) +
    `&_=${Date.now()}`
  return (await fetch(u, { cache: 'no-store' })).json()
}

// —— 园区视图 ——
/** 整季仿真轨迹 JSON（{steps, series}）。`p`/`d` = 情景/处理区管理覆盖（参数/恒定驱动）。 */
export async function fetchSimulate(
  model: string,
  p: Record<string, number> = {},
  d: Record<string, number> = {}
): Promise<SimSeries> {
  const enc = (o: Record<string, number>) => Object.entries(o).map(([k, v]) => `${k}:${v}`).join(',')
  let u = '/api/simulate?_=' + Date.now() + modelQS(model)
  const ps = enc(p), ds = enc(d)
  if (ps) u += `&p=${encodeURIComponent(ps)}`
  if (ds) u += `&d=${encodeURIComponent(ds)}`
  return (await fetch(u, { cache: 'no-store' })).json()
}
// —— 耦合仿真/优化（可仿真耦合条目）——
export async function fetchCouple(model: string): Promise<SimSeries> {
  return (await fetch('/api/couple?_=' + Date.now() + modelQS(model), { cache: 'no-store' })).json()
}
export function coupleChartUrl(model: string, vars: string[]): string {
  return '/api/couple.svg?vars=' + encodeURIComponent(vars.join(',')) + modelQS(model) + '&_=' + Date.now()
}
export async function runCoupleOptimize(model: string, spec: string): Promise<CoupleOptResult> {
  const u = '/api/couple-optimize?spec=' + encodeURIComponent(spec) + modelQS(model) + '&_=' + Date.now()
  return (await fetch(u, { cache: 'no-store' })).json()
}

export async function fetchZone(model: string, zone: string): Promise<ZoneInfo> {
  return (await fetch(`/api/zone?zone=${encodeURIComponent(zone)}` + modelQS(model) + `&_=${Date.now()}`, { cache: 'no-store' })).json()
}
/** 写本区管理（param/driver 覆盖）→ <zone>.json。标定/看懂据此按本区处理仿真。 */
export async function saveZone(
  model: string,
  zone: string,
  params: Record<string, number>,
  drivers: Record<string, number>
): Promise<{ ok?: boolean; error?: string; params?: number; drivers?: number }> {
  const u = `/api/zone?zone=${encodeURIComponent(zone)}` + modelQS(model)
  return (
    await fetch(u, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ params, drivers }),
    })
  ).json()
}
export async function fetchObservations(model: string, zone: string): Promise<ObservationsJson> {
  return (await fetch(`/api/observations?zone=${encodeURIComponent(zone)}` + modelQS(model) + `&_=${Date.now()}`, { cache: 'no-store' })).json()
}
// —— 模型编辑器 ——
export async function fetchSource(model: string): Promise<SourceJson> {
  return (await fetch(`/api/source?_=${Date.now()}` + modelQS(model), { cache: 'no-store' })).json()
}
/** 受控写回模型文件（先校验+自动备份+原子写）→ {ok, path, backup} 或 {error}。 */
export async function saveSource(model: string, text: string): Promise<{ ok?: boolean; path?: string; backup?: string; error?: string }> {
  return (
    await fetch('/api/source' + modelQS(model, '?'), {
      method: 'POST', headers: { 'Content-Type': 'text/plain; charset=utf-8' }, body: text,
    })
  ).json()
}
/** 校验编辑后的 YAML 文本 → {ok, errors, report_html?}（不写盘）。layout/level 让预览与结构工作区一致。 */
export async function validateSource(
  model: string,
  text: string,
  layout = 'forrester',
  level = 'variable'
): Promise<ValidateJson> {
  const u = `/api/validate?layout=${encodeURIComponent(layout)}&level=${encodeURIComponent(level)}` + modelQS(model)
  return (await fetch(u, { method: 'POST', headers: { 'Content-Type': 'text/plain; charset=utf-8' }, body: text })).json()
}

export async function saveObservations(
  model: string,
  zone: string,
  columns: string[],
  rows: Record<string, number>[]
): Promise<{ ok?: boolean; error?: string; rows?: number }> {
  const u = `/api/observations?zone=${encodeURIComponent(zone)}` + modelQS(model)
  return (
    await fetch(u, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ columns, rows }),
    })
  ).json()
}
