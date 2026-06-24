// 自画的拟合叠观测小图（候选实线 / 现有形式虚线 / 实测散点）。数据来自契约 trajectory/observed。
// 注：轨迹数据由 EQC 给（patch+仿真），前端只是把数组画成 SVG——不重算任何模型逻辑。
import type { GpTraj } from './contract'

export function fitChartSvg(cand?: GpTraj | null, base?: GpTraj | null, obs?: GpTraj | null): string {
  const W = 380, H = 210, pad = 32
  const xs: number[] = [], ys: number[] = []
  const add = (s?: GpTraj | null) => {
    if (s?.DAT) s.DAT.forEach((d, i) => { xs.push(d); ys.push(s.value[i]) })
  }
  add(cand); add(base); add(obs)
  if (!xs.length) return '<div class="hint">无轨迹数据</div>'
  const xmin = Math.min(...xs), xmax = Math.max(...xs)
  let ymin = Math.min(...ys), ymax = Math.max(...ys)
  if (ymin === ymax) { ymin -= 1; ymax += 1 }
  const px = (d: number) => pad + (xmax > xmin ? (d - xmin) / (xmax - xmin) : 0.5) * (W - 2 * pad)
  const py = (v: number) => H - pad - (v - ymin) / (ymax - ymin) * (H - 2 * pad)
  const poly = (s: GpTraj | null | undefined, color: string, dash: boolean) => {
    if (!s?.DAT?.length) return ''
    const pts = s.DAT.map((d, i) => px(d).toFixed(1) + ',' + py(s.value[i]).toFixed(1)).join(' ')
    return `<polyline points="${pts}" fill="none" stroke="${color}"${dash ? ' stroke-dasharray="5,4"' : ''} stroke-width="2"/>`
  }
  let svg = `<svg class="gp-fit" viewBox="0 0 ${W} ${H}">`
  svg += `<line x1="${pad}" y1="${H - pad}" x2="${W - pad}" y2="${H - pad}" stroke="#cbd5e1"/>`
  svg += `<line x1="${pad}" y1="${pad}" x2="${pad}" y2="${H - pad}" stroke="#cbd5e1"/>`
  svg += poly(base, '#9ca3af', true)
  svg += poly(cand, '#2563eb', false)
  if (obs?.DAT) obs.DAT.forEach((d, i) => {
    svg += `<circle cx="${px(d).toFixed(1)}" cy="${py(obs.value[i]).toFixed(1)}" r="3.2" fill="#f59e0b"/>`
  })
  svg += `<text x="${pad - 4}" y="${pad + 4}" font-size="10" fill="#6b7280" text-anchor="end">${ymax.toFixed(2)}</text>`
  svg += `<text x="${pad - 4}" y="${H - pad}" font-size="10" fill="#6b7280" text-anchor="end">${ymin.toFixed(2)}</text>`
  svg += `<text x="${W - pad}" y="${H - pad + 15}" font-size="10" fill="#6b7280" text-anchor="end">DAT ${xmax}</text>`
  return svg + '</svg>'
}
