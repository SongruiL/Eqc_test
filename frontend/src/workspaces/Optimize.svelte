<script lang="ts">
  // 优化工作区：填决策 spec → 跑 DE → 最优旋钮 + 约束 + 收敛曲线；多目标则画 Pareto 前沿、点选看该点。
  import { store, applyKnobs, setWorkspace } from '../lib/store.svelte'
  import { runOptimize } from '../lib/api'
  import type { OptResult, Knob } from '../lib/contract'
  import { fmtNum } from '../lib/format'
  import KnobTable from '../components/KnobTable.svelte'

  let spec = $state('')
  let running = $state(false)
  let status = $state('')
  let res = $state<OptResult | null>(null)
  let point = $state(0)

  async function run() {
    const s = spec.trim()
    if (!s) { status = '请填决策 spec 路径（相对模型目录）。'; return }
    running = true
    status = '⏳ 优化中…（DE 搜索，可能数十秒；release 更快）'
    res = null
    try {
      const j = await runOptimize(store.model, s)
      if (j.error) { status = '优化失败：' + j.error; return }
      res = j
      point = 0
      status = '✅ 完成' + (j.optimizer ? ` · DE pop=${j.optimizer.pop} iters=${j.optimizer.iters} seed=${j.optimizer.seed}` : '')
    } catch (e) {
      status = '请求失败：' + e
    } finally {
      running = false
    }
  }

  function onPareto(e: MouseEvent) {
    const t = e.target as Element
    if (!t?.classList?.contains('pp')) return
    t.closest('svg')?.querySelectorAll('circle.pp').forEach((c) => {
      c.setAttribute('r', '4'); c.setAttribute('fill', '#2563eb')
    })
    t.setAttribute('r', '6'); t.setAttribute('fill', '#dc2626')
    point = +(t.getAttribute('data-i') ?? 0)
  }
  const pt = $derived(res?.front?.[point])

  function overlay(knobs: Knob[]) {
    applyKnobs(knobs)
    setWorkspace('simulate') // 跳到仿真工作区，最优旋钮已叠加进情景、曲线即时重画
  }
</script>

<div class="ws">
  <div class="ws-head">
    <b>决策优化（差分进化 DE）</b>
    <input class="spec" placeholder="problem.yaml（相对模型目录）" bind:value={spec} />
    <button class="btn on" disabled={running} onclick={run}>运行优化</button>
  </div>
  <div class="hint">在前向模型上搜最优决策（旋钮空间）。spec 含 <code>objective2</code> 则多目标 Pareto。见 <code>docs/spec-optimization.md</code>。</div>
  {#if status}<div class="status">{status}</div>{/if}

  {#if res && !res.multi_objective}
    <div class="grid">
      <div>
        <div class="sub">目标值（{res.objective?.sense ?? ''}）：{fmtNum(res.objective_value)}
          {res.feasible ? '· 可行 ✓' : res.constraints?.length ? '· 违反约束 ✗' : ''}</div>
        <KnobTable knobs={res.best_knobs ?? []} />
        {#if res.constraints?.length}
          <div class="cons"><b>约束：</b>
            {#each res.constraints as c}
              <div class={c.satisfied ? 'ok' : 'bad'}>{c.satisfied ? '✓' : '✗'} {c.expr} = {fmtNum(c.value)} ≤ {fmtNum(c.max)}{c.satisfied ? '' : `（违反 ${fmtNum(c.violation)}）`}</div>
            {/each}
          </div>
        {/if}
        {#if res.best_knobs?.length}
          <button class="btn link" onclick={() => overlay(res!.best_knobs!)}>叠加最优旋钮到仿真曲线 →</button>
        {/if}
      </div>
      <div class="conv">{@html res.convergence_svg ?? ''}</div>
    </div>
  {:else if res}
    <div class="grid">
      <div>
        <div class="sub">多目标 Pareto 前沿（{res.front?.length ?? 0} 点）</div>
        <div class="cons">
          目标1（{res.objectives?.[0]?.sense ?? ''}）：{res.objectives?.[0]?.expr ?? ''}<br />
          目标2（{res.objectives?.[1]?.sense ?? ''}）：{res.objectives?.[1]?.expr ?? ''}
        </div>
        <!-- 点选委托到 EQC 生成的 SVG 内 circle.pp -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="pareto" role="presentation" onclick={onPareto}>{@html res.pareto_svg ?? ''}</div>
      </div>
      <div>
        {#if pt}
          <div class="sub">选中点</div>
          <div class="cons">{#each pt.objectives ?? [] as v, i}{res.objectives?.[i]?.expr ?? ('目标' + (i + 1))} = {fmtNum(v)}　{/each}{pt.feasible === false ? '（违反约束）' : ''}</div>
          <KnobTable knobs={pt.knobs ?? []} />
          {#if pt.knobs?.length}
            <button class="btn link" onclick={() => overlay(pt!.knobs)}>叠加该点旋钮到仿真曲线 →</button>
          {/if}
        {:else}
          <div class="hint">点前沿上的点查看该点旋钮。</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ws { display: flex; flex-direction: column; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 8px; }
  .spec { font-size: 13px; padding: 4px 8px; border: 1px solid var(--line); border-radius: 6px; min-width: 280px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .btn.link { margin-top: 10px; color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .hint { color: var(--sub); font-size: 12px; margin: 4px 0; }
  .status { font-size: 13px; margin: 8px 0; }
  .sub { color: var(--sub); font-size: 12px; font-weight: 600; }
  .cons { font-size: 12px; margin-top: 8px; }
  .cons .ok { color: #16a34a; } .cons .bad { color: #dc2626; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; align-items: start; margin-top: 8px; }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  .conv :global(svg), .pareto :global(svg) { width: 100%; max-width: 720px; height: auto; }
  .pareto :global(circle.pp) { cursor: pointer; }
</style>
