<script lang="ts">
  // 园区「本区管理与看懂」：看懂卡（标定徽章+头条+红绿灯）+ 本区管理编辑器 + 管理建议（求最优）。
  // 三者共享本区管理与仿真：改管理 → 即时重算看懂卡；采纳建议 → 喂回管理 → 看懂卡刷新。
  // 数据全来自 EQC：/api/model、/api/zone、/api/simulate、/api/optimize。前端只取数+呈现。
  import { store } from '../lib/store.svelte'
  import { fetchSimulate, fetchZone, saveZone, runOptimize } from '../lib/api'
  import type { SimSeries, ZoneInfo, VarJson, OptResult, Knob } from '../lib/contract'
  import { fmtNum } from '../lib/format'

  let sim = $state<SimSeries | null>(null)
  let zone = $state<ZoneInfo | null>(null)
  let mgmt = $state<{ params: Record<string, number>; drivers: Record<string, number> }>({ params: {}, drivers: {} })
  let mgmtStatus = $state('')
  let loading = $state(true)
  let lastKey = ''

  const mod = $derived(store.modelJson?.modules?.[0])
  const vars = $derived(mod?.variables ?? [])

  // 管理输入：参数(management && 非向量) + control 类变量(驱动)
  type MgmtItem = { name: string; label: string; unit: string; kind: 'param' | 'driver'; def: number | null }
  const mgmtItems = $derived.by<MgmtItem[]>(() => {
    const out: MgmtItem[] = []
    for (const p of mod?.parameters ?? [])
      if (p.management && p.values == null) out.push({ name: p.name, label: p.display_name, unit: p.unit ?? '', kind: 'param', def: p.default })
    for (const v of mod?.variables ?? [])
      if (v.class === 'control') out.push({ name: v.name, label: v.label ?? v.display_name, unit: v.unit ?? '', kind: 'driver', def: null })
    return out
  })

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
      mgmt = { params: { ...(z.params ?? {}) }, drivers: { ...(z.drivers ?? {}) } }
      await resim()
    } catch (e) {
      sim = { error: String(e) }
    } finally {
      loading = false
    }
  }

  let resimTimer: ReturnType<typeof setTimeout> | undefined
  let saveTimer: ReturnType<typeof setTimeout> | undefined
  async function resim() {
    sim = await fetchSimulate(store.model, mgmt.params, mgmt.drivers)
  }
  function scheduleResim() { clearTimeout(resimTimer); resimTimer = setTimeout(resim, 250) }
  async function saveMgmt() {
    try {
      const j = await saveZone(store.model, store.zone, mgmt.params, mgmt.drivers)
      mgmtStatus = j.ok ? `已保存本区管理 ✓（${(j.params ?? 0) + (j.drivers ?? 0)} 项）` : j.error ?? ''
    } catch { /* */ }
  }
  function scheduleSave() { clearTimeout(saveTimer); saveTimer = setTimeout(saveMgmt, 400) }

  function editMgmt(it: MgmtItem, raw: string) {
    const s = raw.trim()
    const target = it.kind === 'param' ? mgmt.params : mgmt.drivers
    if (s === '' || (it.def != null && +s === it.def)) delete target[it.name]
    else if (Number.isFinite(+s)) target[it.name] = +s
    scheduleSave()
    scheduleResim()
  }
  const mgmtVal = (it: MgmtItem) => {
    const v = it.kind === 'param' ? mgmt.params[it.name] : mgmt.drivers[it.name]
    return v ?? (it.def ?? '')
  }

  // —— 标定徽章 ——
  const badge = $derived.by(() => {
    const zc = zone?.calibration
    if (zc) {
      const err = zc.error != null ? '误差 ' + fmtNum(zc.error) : ''
      let dt = ''
      try { if (zc.at) dt = ' · ' + new Date(zc.at * 1000).toLocaleDateString() } catch { /* */ }
      return { cls: 'ok', text: '✓ 本区已标定' + (err ? ' · ' + err : '') + dt, note: '（模型整体仍待跨区联合标定）' }
    }
    const c = mod?.calibration
    if (c?.calibrated) return { cls: 'ok', text: '✓ 已标定' + (c.note ? ' · ' + c.note : ''), note: '' }
    return { cls: 'warn', text: '⚠ 未标定 · ' + (c?.note ?? '参数为占位值，结果仅供方向参考'), note: '' }
  })

  // —— 头条：measurable 输出（排除胁迫信号）整季末值 ——
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

  // —— 胁迫红绿灯 ——
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

  // —— 管理建议（求最优）——
  let adviceSpec = $state('')
  let adviceStatus = $state('')
  let advice = $state<OptResult | null>(null)
  let adviceRunning = $state(false)
  async function runAdvice() {
    const s = adviceSpec.trim()
    if (!s) { adviceStatus = '请填优化 spec 路径（相对模型目录）。'; return }
    adviceRunning = true
    adviceStatus = '⏳ 求最优中…（DE 搜索，可能数十秒）'
    advice = null
    try {
      const j = await runOptimize(store.model, s)
      if (j.error) { adviceStatus = '求最优失败：' + j.error; return }
      if (j.multi_objective) { adviceStatus = '该 spec 是多目标，请到专家视图看 Pareto 前沿。'; return }
      advice = j
      adviceStatus = '✅ 完成'
    } catch (e) {
      adviceStatus = '请求失败：' + e
    } finally {
      adviceRunning = false
    }
  }
  function adoptAdvice(knobs: Knob[]) {
    for (const k of knobs) {
      if (k.kind === 'param') mgmt.params[k.var] = k.value
      else if (k.kind === 'driver_const') mgmt.drivers[k.var] = k.value
    }
    saveMgmt()
    resim()
    adviceStatus = '已采纳建议——本区管理已更新、上方「模型怎么说」已刷新。'
  }
  const knobLabel = (name: string) => mod?.parameters?.find((p) => p.name === name)?.display_name ?? name
</script>

<div class="ws">
  <!-- 看懂卡 -->
  <div class="card-head"><b>模型怎么说</b>
    <span class="badge {badge.cls}">{badge.text}</span>
    {#if badge.note}<span class="note">{badge.note}</span>{/if}
  </div>

  {#if loading}
    <div class="hint">正在跑当前处理区（{store.zone}）整季仿真…</div>
  {:else if sim?.error || !sim?.series}
    <div class="hint">提示：{sim?.error ?? '需要启动时加 --drivers 才能预测整季'}</div>
  {:else}
    <div class="headline">
      {#each headline as h}
        <div class="hl-item"><div class="hl-val">{fmtNum(h.val)} <span class="hl-unit">{h.unit}</span></div><div class="hl-lab">{h.lab}</div></div>
      {/each}
    </div>
    <div class="lights">
      {#if lights.length}
        {#each lights as l}<div class="light {l.cls}"><span class="dot"></span>{l.label}：{l.txt} <small>({fmtNum(l.val)})</small></div>{/each}
      {:else}<span class="note">（模型未标注胁迫信号）</span>{/if}
    </div>
    <div class="hint">基于本区管理 + 整季仿真{sim.steps ? `（${sim.steps} 天）` : ''}；红绿灯取各信号设定的全季最差或终值。未标定时数值仅供方向参考。</div>
  {/if}

  <!-- 本区管理编辑器 -->
  <div class="sec">
    <div class="sec-head">本区管理 <span class="note">{mgmtStatus}</span></div>
    {#if !mgmtItems.length}
      <span class="note">（模型未标注管理输入：给参数加 management 标志、或用 control 类变量）</span>
    {:else}
      <div class="mgmt">
        {#each mgmtItems as it}
          <div class="m-row">
            <span class="m-lab" title={it.name}>{it.label}{it.unit ? ' · ' + it.unit : ''}</span>
            <input type="number" step="any" placeholder="默认" value={mgmtVal(it)}
              oninput={(e) => editMgmt(it, e.currentTarget.value)} />
            <span class="m-tag">{it.kind === 'param' ? '参数' : '控制'}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 管理建议（求最优）-->
  <div class="sec">
    <div class="sec-head">管理建议
      <input class="spec" placeholder="optimize.yaml（相对模型目录）" bind:value={adviceSpec} />
      <button class="btn on" disabled={adviceRunning} onclick={runAdvice}>求最优管理</button>
    </div>
    {#if adviceStatus}<div class="status">{adviceStatus}</div>{/if}
    {#if advice && !advice.multi_objective}
      <div class="advice-lead">为达成目标，建议这样调整管理：</div>
      <ul class="advice-list">
        {#each advice.best_knobs ?? [] as k}
          <li>把 <b>{knobLabel(k.var)}</b> 调到 <b>{fmtNum(k.value)}{k.unit ? ' ' + k.unit : ''}</b></li>
        {/each}
      </ul>
      <div class="note">预计目标（{advice.objective?.sense === 'max' ? '最大化' : '最小化'}）≈ <b>{fmtNum(advice.objective_value)}</b>
        {advice.feasible ? '· 满足约束 ✓' : advice.constraints?.length ? '· 违反约束 ✗' : ''}</div>
      {#if advice.best_knobs?.length}
        <button class="btn" onclick={() => adoptAdvice(advice!.best_knobs!)}>采纳建议 → 喂回本区管理</button>
      {/if}
    {/if}
  </div>
</div>

<style>
  .ws { display: flex; flex-direction: column; max-width: 900px; }
  .card-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 12px; }
  .badge { font-size: 12px; font-weight: 600; padding: 2px 10px; border-radius: 12px; }
  .badge.ok { background: #dcfce7; color: #166534; }
  .badge.warn { background: #fef9c3; color: #854d0e; }
  .note { color: var(--sub); font-size: 12px; }
  .hint { color: var(--sub); font-size: 12px; margin-top: 8px; }
  .headline { display: flex; flex-wrap: wrap; gap: 24px; margin: 8px 0 14px; }
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
  .sec { margin-top: 20px; border-top: 1px solid var(--line); padding-top: 14px; }
  .sec-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; font-size: 14px; font-weight: 600; margin-bottom: 8px; }
  .mgmt { display: flex; flex-direction: column; gap: 4px; }
  .m-row { display: grid; grid-template-columns: 220px 130px auto; gap: 10px; align-items: center; font-size: 12px; }
  .m-lab { color: var(--sub); }
  .m-row input { font-size: 12px; padding: 3px 6px; border: 1px solid var(--line); border-radius: 6px; }
  .m-tag { color: var(--sub); font-size: 11px; }
  .spec { font-size: 13px; padding: 4px 8px; border: 1px solid var(--line); border-radius: 6px; min-width: 240px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; margin-top: 8px; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); margin-top: 0; }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .status { font-size: 13px; margin: 6px 0; }
  .advice-lead { font-size: 13px; margin-top: 8px; }
  .advice-list { font-size: 13px; margin: 6px 0; }
</style>
