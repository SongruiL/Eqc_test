<script lang="ts">
  // 耦合仿真/优化工作区（可仿真耦合条目，如 温室×番茄）：跑双向耦合仿真画轨迹 + 在双向模型上跑 DE。
  // 数据全来自 EQC：/api/couple（作物+温室合成轨迹）、/api/couple.svg（图）、/api/couple-optimize（DE）。
  import { store } from '../lib/store.svelte'
  import { fetchCouple, coupleChartUrl, runCoupleOptimize } from '../lib/api'
  import { fmtNum } from '../lib/format'
  import type { SimSeries, CoupleOptResult } from '../lib/contract'

  const entry = $derived(store.models.find((m) => m.id === store.model))
  const simCap = $derived(!!entry?.sim_capable)

  let sim = $state<SimSeries | null>(null)
  let keys = $state<string[]>([])
  let selected = $state<string[]>([])
  let running = $state(false)
  let status = $state('')

  async function runSim() {
    running = true; status = '⏳ 跑耦合仿真…（双向，可能数秒）'
    try {
      const j = await fetchCouple(store.model)
      if (j.error) { status = '失败：' + j.error; sim = null; return }
      sim = j
      keys = Object.keys(j.series ?? {})
      selected = keys.slice(0, 2)
      status = '✅ 完成（' + (j.steps ?? 0) + ' 步）'
    } catch (e) { status = '请求失败：' + e } finally { running = false }
  }
  function toggle(k: string) {
    selected = selected.includes(k) ? selected.filter((x) => x !== k) : [...selected, k]
  }
  const chartSrc = $derived(sim && selected.length ? coupleChartUrl(store.model, selected) : '')

  // 耦合优化
  let spec = $state('')
  let optRunning = $state(false)
  let optStatus = $state('')
  let opt = $state<CoupleOptResult | null>(null)
  async function runOpt() {
    const s = spec.trim()
    if (!s) { optStatus = '请填耦合优化 spec 路径（serve 启动目录相对）。'; return }
    optRunning = true; optStatus = '⏳ 耦合优化中…（DE 套双向模型，数十秒~分钟）'; opt = null
    try {
      const j = await runCoupleOptimize(store.model, s)
      if (j.error) { optStatus = '失败：' + j.error; return }
      opt = j; optStatus = '✅ 完成'
    } catch (e) { optStatus = '请求失败：' + e } finally { optRunning = false }
  }
  const knobEntries = $derived(opt?.best_knobs ? Object.entries(opt.best_knobs) : [])
</script>

<div class="ws">
  <div class="ws-head"><b>耦合仿真 / 优化（温室 ↔ 作物 双向）</b></div>

  {#if !simCap}
    <div class="hint">请在顶部选择<b>可仿真耦合</b>条目（清单里写了 fast/slow/weather 的，如「温室 × 番茄（可仿真）」）。</div>
  {:else}
    <div class="hint">这是双向耦合（温室气候↔作物吸收互相影响）。跑一次仿真画温室气候+作物轨迹；或在双向模型上搜最优环控。</div>

    <!-- 仿真 -->
    <section>
      <div class="sec-head"><b>耦合仿真</b>
        <button class="btn on" disabled={running} onclick={runSim}>跑耦合仿真</button>
        <span class="status">{status}</span>
      </div>
      {#if sim}
        <div class="grid">
          <div>
            {#if chartSrc}<img class="chart" src={chartSrc} alt="耦合轨迹" />{:else}<div class="hint">勾选右侧变量看轨迹。</div>{/if}
          </div>
          <div class="vars">
            <div class="sub">变量（温室: 前缀=温室侧 · {selected.length} 选中）</div>
            <div class="vlist">
              {#each keys as k}
                <label class="vrow"><input type="checkbox" checked={selected.includes(k)} onchange={() => toggle(k)} /><span>{k}</span></label>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </section>

    <!-- 优化 -->
    <section>
      <div class="sec-head"><b>耦合优化</b>
        <input class="spec" placeholder="coupled.yaml（serve 启动目录相对）" bind:value={spec} />
        <button class="btn on" disabled={optRunning} onclick={runOpt}>跑耦合优化</button>
      </div>
      {#if optStatus}<div class="status">{optStatus}</div>{/if}
      {#if opt && !opt.error}
        <div class="grid">
          <div>
            <div class="sub">目标（{opt.sense === 'max' ? '最大化' : '最小化'} {opt.objective ?? ''}）：<b>{fmtNum(opt.best_objective)}</b></div>
            <table class="tbl"><tbody>
              <tr><th>旋钮</th><th>最优值</th></tr>
              {#each knobEntries as [k, v]}<tr><td>{k}</td><td>{fmtNum(v)}</td></tr>{/each}
            </tbody></table>
          </div>
          <div class="conv">{@html opt.convergence_svg ?? ''}</div>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .ws { display: flex; flex-direction: column; }
  .ws-head { margin-bottom: 8px; }
  .hint { color: var(--sub); font-size: 12px; margin: 6px 0; }
  section { margin-top: 16px; border-top: 1px solid var(--line); padding-top: 12px; }
  .sec-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; font-size: 14px; margin-bottom: 8px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .spec { font-size: 13px; padding: 4px 8px; border: 1px solid var(--line); border-radius: 6px; min-width: 240px; }
  .status { font-size: 12px; color: var(--sub); }
  .sub { color: var(--sub); font-size: 12px; font-weight: 600; }
  .grid { display: grid; grid-template-columns: 1fr 260px; gap: 16px; align-items: start; }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
  .chart { width: 100%; max-width: 760px; height: auto; display: block; border: 1px solid var(--line); border-radius: 8px; background: #fff; }
  .vars { border: 1px solid var(--line); border-radius: 8px; padding: 8px; background: #fff; max-height: 70vh; overflow: auto; }
  .vlist { margin-top: 4px; }
  .vrow { display: flex; align-items: baseline; gap: 7px; padding: 3px 2px; font-size: 12px; border-bottom: 1px solid #f1f3f5; }
  .tbl { width: 100%; border-collapse: collapse; font-size: 12px; margin-top: 8px; }
  .tbl th, .tbl td { text-align: left; padding: 3px 8px; border-bottom: 1px solid var(--line); }
  .tbl th { color: var(--sub); font-weight: 600; }
  .conv :global(svg) { width: 100%; max-width: 720px; height: auto; }
</style>
