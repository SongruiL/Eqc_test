<script lang="ts">
  // 仿真工作区：变量勾选 + 整季轨迹图 + 情景探索（参数/初值滑块）。状态在全局 store
  // （selectedVars/scenario）→ 优化工作区「叠加最优旋钮」写它、这里即时画出（跨工作区联动）。
  import { store, clearScenario } from '../lib/store.svelte'
  import { chartUrl } from '../lib/api'

  const mod = $derived(store.modelJson?.modules?.[0])
  const allVars = $derived(mod?.variables ?? [])
  const scalarParams = $derived((mod?.parameters ?? []).filter((p) => p.values == null))
  const stateInits = $derived((mod?.variables ?? []).filter((v) => v.class === 'state' && v.init != null))

  // 轨迹图：随 选中变量 / 情景覆盖 / 模型 变化防抖重取（含优化叠加旋钮触发的 scenario 变化）。
  let chartSrc = $state('')
  let timer: ReturnType<typeof setTimeout> | undefined
  $effect(() => {
    void JSON.stringify([store.selectedVars, store.scenario, store.model]) // 深度依赖
    clearTimeout(timer)
    timer = setTimeout(() => {
      chartSrc = store.selectedVars.length
        ? chartUrl(store.model, store.selectedVars, store.scenario.p, store.scenario.i, store.scenario.d)
        : ''
    }, 150)
  })

  const dOverrides = $derived(Object.entries(store.scenario.d))

  function toggle(name: string) {
    store.selectedVars = store.selectedVars.includes(name)
      ? store.selectedVars.filter((n) => n !== name)
      : [...store.selectedVars, name]
  }
  const sliderRange = (def: number) => {
    const lo = Math.min(0, 2 * def)
    const hi = Math.max(0, 2 * def) || 1
    return { lo, hi, step: (hi - lo) / 100 || 0.01 }
  }
</script>

<div class="ws">
  <div class="ws-head"><b>整季仿真轨迹</b> <span class="sub">{mod?.name_cn ?? mod?.model ?? ''}</span></div>

  <div class="grid">
    <div class="chart-col">
      {#if chartSrc}
        <img class="chart" src={chartSrc} alt="整季轨迹" />
      {:else}
        <div class="hint">勾选右侧变量以绘制轨迹（默认 Y / 输出量）。</div>
      {/if}

      {#if dOverrides.length}
        <div class="dovr">恒定驱动覆盖（来自优化）：{#each dOverrides as [k, v]}<code>{k}={v}</code> {/each}</div>
      {/if}

      <div class="scn-head">情景 <span class="sub">调参数/初值即实时重算</span>
        <button class="btn" onclick={clearScenario}>重置默认</button>
      </div>
      <div class="scn">
        {#each scalarParams as p}
          {@const r = sliderRange(p.default)}
          {@const v = store.scenario.p[p.name] ?? p.default}
          <div class="row" class:changed={store.scenario.p[p.name] != null && store.scenario.p[p.name] !== p.default}>
            <span class="lab" title={p.name}>{p.display_name}</span>
            <input type="range" min={r.lo} max={r.hi} step={r.step} value={v}
              oninput={(e) => (store.scenario.p[p.name] = +e.currentTarget.value)} />
            <input type="number" step={r.step} value={v}
              oninput={(e) => (store.scenario.p[p.name] = +e.currentTarget.value)} />
          </div>
        {/each}
        {#each stateInits as s}
          {@const def = s.init ?? 0}
          {@const r = sliderRange(def)}
          {@const v = store.scenario.i[s.name] ?? def}
          <div class="row" class:changed={store.scenario.i[s.name] != null && store.scenario.i[s.name] !== def}>
            <span class="lab" title={s.name}>{s.display_name} <em>初值</em></span>
            <input type="range" min={r.lo} max={r.hi} step={r.step} value={v}
              oninput={(e) => (store.scenario.i[s.name] = +e.currentTarget.value)} />
            <input type="number" step={r.step} value={v}
              oninput={(e) => (store.scenario.i[s.name] = +e.currentTarget.value)} />
          </div>
        {/each}
      </div>
    </div>

    <div class="vars-col">
      <div class="sub">变量（{store.selectedVars.length} 选中）</div>
      <div class="vars">
        {#each allVars as v}
          <label class="vrow">
            <input type="checkbox" checked={store.selectedVars.includes(v.name)} onchange={() => toggle(v.name)} />
            <span title={v.name}>{v.display_name}</span>
            <span class="cls">{v.class}</span>
            <span class="unit">{v.unit ?? ''}</span>
          </label>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .ws { display: flex; flex-direction: column; height: 100%; }
  .ws-head { margin-bottom: 10px; }
  .sub { color: var(--sub); font-size: 12px; }
  .grid { display: grid; grid-template-columns: 1fr 260px; gap: 16px; align-items: start; min-height: 0; }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
  .chart { width: 100%; max-width: 760px; height: auto; display: block; border: 1px solid var(--line); border-radius: 8px; background: #fff; }
  .hint { color: var(--sub); font-size: 13px; padding: 30px; border: 1px dashed var(--line); border-radius: 8px; text-align: center; }
  .dovr { font-size: 12px; color: var(--sub); margin-top: 8px; }
  .dovr code { color: var(--accent); }
  .scn-head { display: flex; align-items: center; gap: 8px; margin: 14px 0 6px; font-size: 13px; font-weight: 600; }
  .scn-head .btn { margin-left: auto; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; border-radius: 7px; cursor: pointer; }
  .btn:hover { background: #eef2ff; }
  .scn { display: flex; flex-direction: column; gap: 4px; }
  .row { display: grid; grid-template-columns: 130px 1fr 84px; gap: 8px; align-items: center; font-size: 12px; }
  .row.changed .lab { color: var(--accent); font-weight: 600; }
  .lab { color: var(--sub); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .lab em { color: #9ca3af; font-style: normal; font-size: 11px; }
  .row input[type='number'] { width: 84px; font-size: 12px; padding: 2px 5px; border: 1px solid var(--line); border-radius: 5px; }
  .vars-col { border: 1px solid var(--line); border-radius: 8px; padding: 8px; background: #fff; max-height: 70vh; overflow: auto; }
  .vars { margin-top: 4px; }
  .vrow { display: flex; align-items: baseline; gap: 7px; padding: 3px 2px; font-size: 12px; border-bottom: 1px solid #f1f3f5; }
  .vrow span[title] { color: var(--ink); }
  .cls { color: var(--accent); font-size: 10px; }
  .unit { color: #9ca3af; margin-left: auto; font-size: 11px; }
</style>
