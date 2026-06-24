// 对 EQC `/api/*` 契约的薄封装。前端只消费，不重实现逻辑（EQC 持有事实）。
import type { ModelsJson, ModelJson } from './contract'

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
