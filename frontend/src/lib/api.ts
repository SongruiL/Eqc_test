// 对 EQC `/api/*` 契约的薄封装。前端只消费，不重实现逻辑（EQC 持有事实）。
import type { ModelsJson, ModelJson, EvolveStatus, OptResult } from './contract'

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

/** 整季轨迹图 SVG（EQC 自生成）。`p`/`init` = 情景覆盖（参数/初值）。 */
export function chartUrl(
  model: string,
  vars: string[],
  p: Record<string, number>,
  init: Record<string, number>
): string {
  const enc = (o: Record<string, number>) =>
    Object.entries(o)
      .map(([k, v]) => `${k}:${v}`)
      .join(',')
  let u = `/api/chart.svg?vars=${encodeURIComponent(vars.join(','))}` + modelQS(model)
  const ps = enc(p)
  const is = enc(init)
  if (ps) u += `&p=${encodeURIComponent(ps)}`
  if (is) u += `&init=${encodeURIComponent(is)}`
  return u + `&_=${Date.now()}`
}

/** 结构图报告 HTML（iframe src）。 */
export function reportUrl(model: string, layout: string): string {
  return `/api/report?layout=${encodeURIComponent(layout)}` + modelQS(model)
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
