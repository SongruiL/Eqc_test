<script lang="ts">
  // 田间数据录入网格（园区）：列=模型 measurable 变量、行=观测日 DAT、空格=没测（稀疏）。
  // GET /api/observations 载已存 → POST 保存（EQC 写规范稀疏 CSV，即标定输入）。
  import { store } from '../lib/store.svelte'
  import { fetchObservations, saveObservations } from '../lib/api'

  type Row = { dat: number; vals: Record<string, string> }
  let rows = $state<Row[]>([])
  let status = $state('')
  let loading = $state(true)
  let lastKey = ''

  const mod = $derived(store.modelJson?.modules?.[0])
  const cols = $derived((mod?.variables ?? []).filter((v) => v.measurable))

  $effect(() => {
    const key = store.model + '|' + store.zone
    if (key === lastKey || !store.modelJson) return
    lastKey = key
    void load()
  })
  async function load() {
    loading = true
    status = ''
    try {
      const j = await fetchObservations(store.model, store.zone)
      const obs = j.observations ?? {}
      const days = j.days ?? []
      const at = (col: string, day: number) => {
        const hit = obs[col]?.find(([d]) => d === day)
        return hit ? String(hit[1]) : ''
      }
      rows = days.map((d) => ({ dat: d, vals: Object.fromEntries(cols.map((c) => [c.name, at(c.name, d)])) }))
    } catch (e) {
      status = '读取失败：' + e
    } finally {
      loading = false
    }
  }

  function addRow() {
    const nextDat = rows.length ? Math.max(...rows.map((r) => r.dat)) + 1 : 1
    rows = [...rows, { dat: nextDat, vals: Object.fromEntries(cols.map((c) => [c.name, ''])) }]
  }
  function delRow(i: number) {
    rows = rows.filter((_, k) => k !== i)
  }

  async function save() {
    status = '⏳ 保存中…'
    // 组装 POST 行：DAT + 各非空数值列
    const payload = rows
      .filter((r) => Number.isInteger(r.dat) && r.dat > 0)
      .map((r) => {
        const o: Record<string, number> = { DAT: r.dat }
        for (const c of cols) {
          const s = r.vals[c.name]?.trim()
          if (s !== '' && s != null && Number.isFinite(+s)) o[c.name] = +s
        }
        return o
      })
    try {
      const j = await saveObservations(store.model, store.zone, cols.map((c) => c.name), payload)
      status = j.ok ? `✅ 已保存处理区 ${store.zone}（${j.rows ?? 0} 行）` : '保存失败：' + (j.error ?? '')
    } catch (e) {
      status = '请求失败：' + e
    }
  }
</script>

<div class="ws">
  <div class="ws-head"><b>田间数据录入</b> <span class="sub">处理区 {store.zone}</span>
    <button class="btn" onclick={addRow}>+ 加一行</button>
    <button class="btn on" onclick={save}>保存</button>
  </div>
  <div class="hint">列 = 模型可测变量；行 = 观测日（DAT，第几天）；空格 = 没测（稀疏）。保存即写成标定输入。</div>
  {#if status}<div class="status">{status}</div>{/if}

  {#if loading}
    <div class="hint">载入中…</div>
  {:else if !cols.length}
    <div class="hint">本模型未标注可测变量（measurable）。</div>
  {:else}
    <div class="table-wrap">
      <table class="grid">
        <thead>
          <tr>
            <th>DAT</th>
            {#each cols as c}<th title={c.name}>{c.display_name}<div class="u">{c.unit ?? ''}</div></th>{/each}
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each rows as r, i}
            <tr>
              <td><input class="dat" type="number" min="1" bind:value={r.dat} /></td>
              {#each cols as c}
                <td><input type="text" inputmode="decimal" bind:value={r.vals[c.name]} /></td>
              {/each}
              <td><button class="del" onclick={() => delRow(i)} title="删除该行">✕</button></td>
            </tr>
          {/each}
          {#if !rows.length}
            <tr><td colspan={cols.length + 2} class="empty">还没有数据 —— 点「+ 加一行」开始录入</td></tr>
          {/if}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .ws { display: flex; flex-direction: column; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 8px; }
  .sub { color: var(--sub); font-size: 12px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .btn:hover { background: #eef2ff; }
  .btn.on:hover { background: var(--accent); }
  .hint { color: var(--sub); font-size: 12px; margin: 4px 0; }
  .status { font-size: 13px; margin: 8px 0; }
  .table-wrap { overflow: auto; margin-top: 8px; }
  .grid { border-collapse: collapse; font-size: 13px; }
  .grid th, .grid td { border: 1px solid var(--line); padding: 2px; text-align: center; }
  .grid th { background: #f8fafc; color: var(--ink); font-weight: 600; font-size: 12px; padding: 4px 8px; }
  .grid th .u { color: var(--sub); font-weight: 400; font-size: 10px; }
  .grid input { width: 84px; border: 0; padding: 4px 6px; font-size: 13px; text-align: center; background: transparent; }
  .grid input:focus { outline: 1px solid var(--accent); border-radius: 3px; }
  .dat { font-weight: 600; }
  .del { border: 0; background: transparent; color: var(--sub); cursor: pointer; font-size: 12px; }
  .del:hover { color: #dc2626; }
  .empty { color: var(--sub); padding: 16px; }
</style>
