<script lang="ts">
  // 看懂卡（园区简明视图）：标定徽章 + 头条（measurable 输出整季末值）+ 胁迫红绿灯。
  // 数据全来自 EQC：/api/model（变量元数据）+ /api/zone（本区管理+标定状态）+ /api/simulate（轨迹）。
  // 前端只做「取整季哪个值 + 阈值映射红绿灯」这点呈现逻辑。
  import { store } from '../lib/store.svelte'
  import { fetchSimulate, fetchZone } from '../lib/api'
  import type { SimSeries, ZoneInfo, VarJson } from '../lib/contract'
  import { fmtNum } from '../lib/format'

  let sim = $state<SimSeries | null>(null)
  let zone = $state<ZoneInfo | null>(null)
  let loading = $state(true)
  let lastKey = ''

  const mod = $derived(store.modelJson?.modules?.[0])
  const vars = $derived(mod?.variables ?? [])

  // 模型/处理区变 → 取本区管理 + 带管理跑仿真
  $effect(() => {
    const key = store.model + '|' + store.zone
    if (key === lastKey || !store.modelJson) return
    lastKey = key
    void load()
  })
  async function load() {
    loading = true
    sim = null
    try {
      const z = await fetchZone(store.model, store.zone)
      zone = z
      sim = await fetchSimulate(store.model, z.params ?? {}, z.drivers ?? {})
    } catch (e) {
      sim = { error: String(e) }
    } finally {
      loading = false
    }
  }

  // 标定徽章
  const badge = $derived.by(() => {
    const zc = zone?.calibration
    if (zc) {
      const err = zc.error != null ? '误差 ' + fmtNum(zc.error) : ''
      let dt = ''
      try {
        if (zc.at) dt = ' · ' + new Date(zc.at * 1000).toLocaleDateString()
      } catch { /* */ }
      return { cls: 'ok', text: '✓ 本区已标定' + (err ? ' · ' + err : '') + dt, note: '（模型整体仍待跨区联合标定）' }
    }
    const c = mod?.calibration
    if (c?.calibrated) return { cls: 'ok', text: '✓ 已标定' + (c.note ? ' · ' + c.note : ''), note: '' }
    return { cls: 'warn', text: '⚠ 未标定 · ' + (c?.note ?? '参数为占位值，结果仅供方向参考'), note: '' }
  })

  // 头条：measurable 输出（排除胁迫信号）的整季末值
  const headline = $derived.by(() => {
    const s = sim?.series
    if (!s) return [] as { lab: string; val: number; unit: string }[]
    let head = vars.filter((v) => v.measurable && v.var_type === 'output' && !v.stress_factor)
    if (!head.length) head = vars.filter((v) => v.var_type === 'output' && !v.stress_factor)
    return head
      .map((v) => ({ v, ser: s[v.name] }))
      .filter((x) => x.ser?.length)
      .map(({ v, ser }) => ({ lab: v.label ?? v.description ?? v.display_name, val: ser[ser.length - 1], unit: v.unit ?? '' }))
  })

  // 胁迫红绿灯
  function reduceVal(ser: number[], v: VarJson): number {
    const reduce = v.stress_reduce ?? (v.stress_factor === 'risk' ? 'max' : 'min')
    if (reduce === 'final') return ser[ser.length - 1]
    if (reduce === 'max') return Math.max(...ser)
    return Math.min(...ser)
  }
  const lights = $derived.by(() => {
    const s = sim?.series
    if (!s) return [] as { cls: string; label: string; txt: string; val: number }[]
    return vars
      .filter((v) => v.stress_factor)
      .map((v) => ({ v, ser: s[v.name] }))
      .filter((x) => x.ser?.length)
      .map(({ v, ser }) => {
        const val = reduceVal(ser, v)
        let cls = 'bad', txt = ''
        if (v.stress_factor === 'risk') {
          if (val <= 0.1) { cls = 'ok'; txt = '安全' } else if (val <= 0.4) { cls = 'warn'; txt = '注意' } else { cls = 'bad'; txt = '高风险' }
        } else {
          if (val >= 0.9) { cls = 'ok'; txt = '充足' } else if (val >= 0.6) { cls = 'warn'; txt = '偏紧' } else { cls = 'bad'; txt = '缺乏' }
        }
        return { cls, label: v.label ?? v.display_name, txt, val }
      })
  })
</script>

<div class="ws">
  <div class="ws-head"><b>模型怎么说</b>
    <span class="badge {badge.cls}">{badge.text}</span>
    {#if badge.note}<span class="note">{badge.note}</span>{/if}
    <button class="btn" onclick={load}>刷新</button>
  </div>

  {#if loading}
    <div class="hint">正在跑当前处理区（{store.zone}）整季仿真…</div>
  {:else if sim?.error || !sim?.series}
    <div class="hint">提示：{sim?.error ?? '需要启动时加 --drivers 才能预测整季'}</div>
  {:else}
    <div class="headline">
      {#each headline as h}
        <div class="hl-item">
          <div class="hl-val">{fmtNum(h.val)} <span class="hl-unit">{h.unit}</span></div>
          <div class="hl-lab">{h.lab}</div>
        </div>
      {/each}
    </div>

    <div class="lights">
      {#if lights.length}
        {#each lights as l}
          <div class="light {l.cls}"><span class="dot"></span>{l.label}：{l.txt} <small>({fmtNum(l.val)})</small></div>
        {/each}
      {:else}
        <span class="note">（模型未标注胁迫信号）</span>
      {/if}
    </div>
    <div class="hint">基于当前处理区管理 + 整季仿真{sim.steps ? `（${sim.steps} 天）` : ''}；红绿灯取各信号设定的全季最差或终值。未标定时数值仅供方向参考。</div>
  {/if}
</div>

<style>
  .ws { display: flex; flex-direction: column; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 12px; }
  .badge { font-size: 12px; font-weight: 600; padding: 2px 10px; border-radius: 12px; }
  .badge.ok { background: #dcfce7; color: #166534; }
  .badge.warn { background: #fef9c3; color: #854d0e; }
  .note { color: var(--sub); font-size: 12px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; border-radius: 7px; cursor: pointer; margin-left: auto; }
  .hint { color: var(--sub); font-size: 12px; margin-top: 10px; }
  .headline { display: flex; flex-wrap: wrap; gap: 24px; margin: 8px 0 16px; }
  .hl-val { font-size: 26px; font-weight: 700; color: var(--ink); line-height: 1.2; }
  .hl-unit { font-size: 13px; color: var(--sub); font-weight: 400; }
  .hl-lab { font-size: 13px; color: var(--sub); margin-top: 2px; }
  .lights { display: flex; flex-direction: column; gap: 6px; }
  .light { font-size: 14px; display: flex; align-items: center; gap: 8px; }
  .light .dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
  .light.ok .dot { background: #16a34a; } .light.ok { color: #166534; }
  .light.warn .dot { background: #d97706; } .light.warn { color: #854d0e; }
  .light.bad .dot { background: #dc2626; } .light.bad { color: #991b1b; }
  .light small { color: var(--sub); }
</style>
