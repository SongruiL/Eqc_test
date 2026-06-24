<script lang="ts">
  // 一个候选块（单靶=一个；联合=每槽一个）：rediscovery 徽章 + 2D 公式 + 与现有形式并排对比
  // + 拟合叠观测 + 采纳。单/多槽复用。
  import type { GpCandidate, GpBaseline, GpTraj } from '../../lib/contract'
  import { fitChartSvg } from '../../lib/fitChart'
  import AdoptPanel from './AdoptPanel.svelte'

  let {
    cand,
    baseline,
    observed,
    name,
  }: { cand: GpCandidate; baseline?: GpBaseline; observed?: GpTraj | null; name: string } = $props()

  let showAdopt = $state(false)
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

  <div class="leg">
    <i style="color:#2563eb">━ 候选拟合</i>　<i style="color:#9ca3af">┄ 现有形式</i>　<i style="color:#f59e0b">● 实测</i>
  </div>
  {@html fitChartSvg(cand.trajectory, b.trajectory, observed)}

  {#if b.formula}<div class="cmp">现有形式公式：<br /><code>{b.formula}</code></div>{/if}
  {#if cand.provenance_suggestion}<div class="hint">溯源建议：{cand.provenance_suggestion}</div>{/if}

  <button class="btn" onclick={() => (showAdopt = !showAdopt)}>采纳此候选 ▾</button>
  {#if showAdopt}<AdoptPanel stub={cand.provenance_stub} yaml={cand.yaml_fragment} {name} />{/if}
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
  :global(.gp-fit) { width: 100%; height: auto; display: block; border: 1px solid var(--line); border-radius: 8px; margin-top: 8px; background: #fff; max-width: 420px; }
  .cmp { font-size: 12px; color: var(--sub); margin-top: 10px; }
  .cmp code { color: var(--ink); }
  .hint { color: var(--sub); font-size: 12px; margin-top: 8px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; border-radius: 7px; cursor: pointer; margin-top: 10px; }
  .btn:hover { background: #eef2ff; }
</style>
