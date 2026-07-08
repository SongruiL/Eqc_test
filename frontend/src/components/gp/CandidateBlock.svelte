<script lang="ts">
  // 一个候选块（单靶=一个；联合=每槽一个）：rediscovery 徽章 + 2D 公式 + 与现有形式并排对比
  // + 拟合叠观测 + 采纳。单/多槽复用。
  import type { GpCandidate, GpBaseline, GpTraj } from '../../lib/contract'
  import { fitChartSvg } from '../../lib/fitChart'
  import AdoptPanel from './AdoptPanel.svelte'
  import GrowthPreview from './GrowthPreview.svelte'

  let {
    cand,
    baseline,
    observed,
    name,
    autoOpenSignal = 0,
  }: { cand: GpCandidate; baseline?: GpBaseline; observed?: GpTraj | null; name: string; autoOpenSignal?: number } = $props()

  let showAdopt = $state(false)
  let showGrow = $state(false)
  // 彩蛋按钮仅在后端带了结构 diff 时出现（GA-6b Phase 3）。
  const hasDiff = $derived(!!cand.structure_diff)
  // 命令 preview_gp_growth 自增 store.gpGrowSignal → Gp.svelte 透传给选中候选 → 自动展开预览。
  // 边沿触发：首跑只记基线（不自动开），之后信号变化才展开。
  let seenSignal = $state(-1)
  $effect(() => {
    const s = autoOpenSignal
    if (seenSignal < 0) { seenSignal = s; return }
    if (s !== seenSignal) { seenSignal = s; if (hasDiff) showGrow = true }
  })
  const b = $derived(baseline ?? {})
  const cls = $derived(cand.rediscovery ? 'redisc' : cand.mechanistic_form ? 'newform' : 'custom')
  const badge = $derived(
    cand.rediscovery
      ? '🟢 rediscovery（复原现有形式 = 机理验证）'
      : cand.mechanistic_form
        ? '🟠 新形式假设（语法内另一种机理形式，待田间证伪）'
        : '🟠 自定义结构（不在标准形式集合，需人工审）'
  )
  function fmt(x?: number | null): string {
    if (x == null) return '—'
    return Math.abs(x) >= 1000 || (Math.abs(x) < 0.001 && x !== 0) ? x.toExponential(3) : x.toFixed(4)
  }
  // 带符号的结构增量（+1 / 0 / -2）。
  function sd(x: number): string {
    return x > 0 ? `+${x}` : `${x}`
  }
  const g = $derived(cand.graph_evidence)
</script>

<div class="block">
  <div><span class="badge {cls}">{badge}</span></div>
  <div class="fml">{@html cand.formula_mathml ?? `<code>${cand.formula}</code>`}</div>

  <table class="tbl">
    <tbody>
      <tr><th></th><th>现有形式</th><th>GP 候选</th></tr>
      <tr><td>机理形式</td><td>{b.form ?? '—'}</td><td>{cand.mechanistic_form ?? '自定义'}</td></tr>
      <tr><td>rmse（观测日）</td><td>{fmt(b.error)}</td><td>{fmt(cand.error)}</td></tr>
      <tr><td>复杂度（节点）</td><td>{fmt(b.complexity)}</td><td>{cand.complexity}</td></tr>
    </tbody>
  </table>

  {#if g}
    <div class="hf {g.passes_hard_filters ? 'ok' : 'bad'}">
      <div class="hf-head">
        <span class="hf-verdict">{g.passes_hard_filters ? '✓ 过结构硬过滤' : '✗ 硬过滤淘汰'}</span>
        {#if !g.passes_hard_filters}<span class="hf-why">{g.reject_reasons.join('；')}</span>{/if}
      </div>
      <div class="hf-chips">
        {#if g.added_edges.length}
          <span class="chip grow" title="候选新用了这些输入 → 依赖图长出新边（= 看它长出什么）">🌿 长出 {g.added_edges.map((e) => `${e[0]}→${e[1]}`).join('、')}</span>
        {/if}
        {#if g.coefficient_cluster.length}
          <span class="chip cluster" title="这 {g.coefficient_cluster.length} 个系数同进本方程、结构上异参同效（混淆团=经验式系数簇）→ 需一起标定，或加正交/多工况对照实验拆共线：{g.coefficient_cluster.join('、')}">⚠️ 系数簇 {g.coefficient_cluster.length}·需联合标定</span>
        {/if}
        {#if g.conservation.length}
          <span class="chip cons" title={g.conservation.map((c) => `${c.name}：patched 残差 ${fmt(c.patched_resid)} / tol ${fmt(c.tol)}（${c.patched_ok ? '守' : '破'}）`).join('\n')}>⚖️ 守恒 {g.conservation.filter((c) => c.patched_ok).length}/{g.conservation.length} 守</span>
        {/if}
        <span class="chip delta" title="采纳此候选相对现有模型的结构规模增量（机理视图·常数折回字面值）">结构Δ 点 {sd(g.complexity_delta.nodes)}·边 {sd(g.complexity_delta.edges)}</span>
      </div>
    </div>
  {/if}

  <div class="leg">
    <i style="color:#2563eb">━ 候选拟合</i>　<i style="color:#9ca3af">┄ 现有形式</i>　<i style="color:#f59e0b">● 实测</i>
  </div>
  {@html fitChartSvg(cand.trajectory, b.trajectory, observed)}

  {#if b.formula}<div class="cmp">现有形式公式：<br /><code>{b.formula}</code></div>{/if}
  {#if cand.provenance_suggestion}<div class="hint">溯源建议：{cand.provenance_suggestion}</div>{/if}

  <div class="acts">
    <button class="btn" onclick={() => (showAdopt = !showAdopt)}>采纳此候选 ▾</button>
    {#if hasDiff}
      <button class="btn grow" onclick={() => (showGrow = !showGrow)} title="在 3D 结构里看采纳此候选会长出什么（结构 diff 动画）">
        🌱 看它长出什么 {showGrow ? '▴' : '▾'}
      </button>
    {/if}
  </div>
  {#if showAdopt}<AdoptPanel stub={cand.provenance_stub} yaml={cand.yaml_fragment} {name} />{/if}
  {#if showGrow && cand.structure_diff}<GrowthPreview {cand} {baseline} onclose={() => (showGrow = false)} />{/if}
</div>

<style>
  .badge { font-size: 12px; font-weight: 600; padding: 2px 10px; border-radius: 12px; }
  .badge.redisc { background: #dcfce7; color: #166534; }
  .badge.newform { background: #fef9c3; color: #854d0e; }
  .badge.custom { background: #f1f5f9; color: #475569; }
  .fml { margin: 8px 0; overflow-x: auto; font-size: 15px; }
  .tbl { width: 100%; border-collapse: collapse; font-size: 12px; margin-top: 8px; }
  .tbl th, .tbl td { text-align: left; padding: 3px 8px; border-bottom: 1px solid var(--line); }
  .tbl th { color: var(--sub); font-weight: 600; }
  .tbl td:first-child { color: var(--sub); }
  .leg { font-size: 11px; color: var(--sub); margin-top: 8px; }
  .leg i { font-style: normal; font-weight: 600; }
  /* 结构硬过滤体检块（Tier3·graph_evidence） */
  .hf { margin-top: 10px; padding: 7px 10px; border-radius: 8px; border: 1px solid var(--line); border-left-width: 3px; }
  .hf.ok  { border-left-color: #16a34a; background: #f0fdf4; }
  .hf.bad { border-left-color: #dc2626; background: #fef2f2; }
  .hf-head { display: flex; flex-wrap: wrap; align-items: baseline; gap: 8px; }
  .hf-verdict { font-size: 12px; font-weight: 700; }
  .hf.ok  .hf-verdict { color: #166534; }
  .hf.bad .hf-verdict { color: #b91c1c; }
  .hf-why { font-size: 11px; color: #b91c1c; }
  .hf-chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .chip { font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 10px; cursor: help; white-space: nowrap; }
  .chip.grow    { background: #dcfce7; color: #15803d; }
  .chip.cluster { background: rgba(20, 184, 166, 0.16); color: #0f766e; } /* GP 青·系数簇标定信号 */
  .chip.cons    { background: #dbeafe; color: #1d4ed8; }
  .chip.delta   { background: #f1f5f9; color: #475569; }
  :global(.gp-fit) { width: 100%; height: auto; display: block; border: 1px solid var(--line); border-radius: 8px; margin-top: 8px; background: #fff; max-width: 420px; }
  .cmp { font-size: 12px; color: var(--sub); margin-top: 10px; }
  .cmp code { color: var(--ink); }
  .hint { color: var(--sub); font-size: 12px; margin-top: 8px; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 10px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; border-radius: 7px; cursor: pointer; }
  .btn:hover { background: #eef2ff; }
  .btn.grow { color: #16a34a; border-color: #bbf7d0; font-weight: 600; }
  .btn.grow:hover { background: #f0fdf4; }
</style>
