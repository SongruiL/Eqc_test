<script lang="ts">
  // GP「看它长出什么」内联预览（GA-6b Phase 3）：把某候选相对现有模型的结构 diff，
  // 在现有结构的 3D 拓扑上播"长出新枝"动画——added 边绿色伸出 + changed 方程节点脉冲 + 旁白。
  // 复用 Topology3d（渲染 before 模型 layout，新边/脉冲叠加其上）；2D 留 follow-up（报告无新边）。
  import { store } from '../../lib/store.svelte'
  import type { GpCandidate, GpBaseline, ModelJson } from '../../lib/contract'
  import { dispName } from '../../lib/annotate'
  import Topology3d, { type GpDiffView } from '../Topology3d.svelte'

  let { cand, baseline, onclose }: { cand: GpCandidate; baseline?: GpBaseline; onclose: () => void } = $props()

  const contract: ModelJson | null = store.modelJson // 现有(before)模型契约（节点注释/配色用）
  const diff = $derived(cand.structure_diff)
  const addedEdges = $derived((diff?.added_edges ?? []) as [string, string][])
  const removedEdges = $derived((diff?.removed_edges ?? []) as [string, string][])
  // 脉冲节点 = 形式变了的方程输出 ∪ 新增方程输出（本地名）。
  const pulseOutputs = $derived([
    ...(diff?.changed_equations ?? []).map((c) => c.output),
    ...(diff?.added_equations ?? []),
  ])

  let phase = $state(0) // 0=现有结构 / 1=长出
  let nonce = $state(0) // 每次"再播"自增，强制 Topology3d 重跑动画
  const view = $derived<GpDiffView>({ addedEdges, pulseOutputs, phase, nonce })

  // 进入即自动播放：先停在"现有结构"一拍，再长出（给观众看清 before）。
  let timer: ReturnType<typeof setTimeout> | undefined
  $effect(() => {
    timer = setTimeout(() => (phase = 1), 800)
    return () => clearTimeout(timer)
  })
  function replay() {
    clearTimeout(timer)
    phase = 0
    nonce += 1
    timer = setTimeout(() => (phase = 1), 450)
  }

  // —— 旁白文案（前端组合，纯展示）——
  const changedName = $derived(
    (diff?.changed_equations ?? []).map((c) => dispName(contract, c.output)).join('、') ||
      dispName(contract, cand.output ?? ''),
  )
  const edgeText = $derived(
    addedEdges.map(([a, b]) => `${dispName(contract, a)} → ${dispName(contract, b)}`).join('、'),
  )
  const newDeps = $derived([...new Set(addedEdges.map(([a]) => dispName(contract, a)))].join('、'))
  const fromForm = $derived(baseline?.form ?? '现有形式')
  const toForm = $derived(cand.mechanistic_form ?? '自定义结构')
  const narration = $derived.by(() => {
    const head = `把「${changedName}」从 ${fromForm} 改成 ${toForm}。`
    const body = addedEdges.length
      ? `长出新枝：${edgeText}——${changedName} 现在还依赖 ${newDeps}。`
      : `形式改变（未重连结构）：给「${changedName}」方程打个脉冲。`
    const tail = cand.rediscovery ? ' GP 复原了现有机理形式 ✓' : ''
    return head + body + tail
  })
</script>

<div class="gp-grow">
  <div class="gg-head">
    <span class="gg-title">🌱 看它长出什么</span>
    <span class="gg-sub">在现有 3D 结构上播放采纳此候选的结构变化</span>
    <span class="gg-spacer"></span>
    <button class="gg-btn" onclick={replay} title="重新播放生长动画">↻ 再播</button>
    <button class="gg-btn" onclick={onclose} title="关闭预览">✕ 关闭</button>
  </div>

  <div class="gg-stage">
    <Topology3d {contract} gpDiff={view} />
    <div class="gg-cap">
      <span class="gg-badge" class:redisc={cand.rediscovery}>
        {cand.rediscovery ? '🟢 rediscovery' : '🟠 新形式假设'}
      </span>
      <span class="gg-text">{narration}</span>
      {#if removedEdges.length}
        <span class="gg-rm">（弃用依赖：{removedEdges.map(([a, b]) => `${dispName(contract, a)}→${dispName(contract, b)}`).join('、')}）</span>
      {/if}
    </div>
  </div>
  <div class="gg-foot">
    结构编辑距离 {diff?.distance ?? 0} · 新增边 {addedEdges.length} · 改形式方程 {(diff?.changed_equations ?? []).length}
    {#if addedEdges.length === 0}　·　受约束 GP 只换了方程形式、没重连结构（脉冲示意）{/if}
  </div>
</div>

<style>
  .gp-grow { border: 1px solid var(--line); border-radius: 10px; margin-top: 10px; overflow: hidden; background: #0f172a; }
  .gg-head { display: flex; align-items: center; gap: 10px; padding: 7px 10px; background: #111827; border-bottom: 1px solid #1f2937; }
  .gg-title { color: #4ade80; font-weight: 700; font-size: 13px; white-space: nowrap; }
  .gg-sub { color: #94a3b8; font-size: 11px; }
  .gg-spacer { flex: 1; }
  .gg-btn { border: 1px solid #334155; background: #1e293b; color: #e2e8f0; font-size: 12px; padding: 3px 10px; border-radius: 6px; cursor: pointer; }
  .gg-btn:hover { background: #334155; }
  .gg-stage { position: relative; height: 360px; }
  .gg-cap {
    position: absolute; left: 12px; right: 12px; bottom: 12px; z-index: 5;
    display: flex; align-items: baseline; flex-wrap: wrap; gap: 8px;
    background: rgba(15, 23, 42, 0.86); border: 1px solid #334155; border-radius: 9px;
    padding: 8px 12px; box-shadow: 0 6px 22px rgba(0, 0, 0, 0.4); backdrop-filter: blur(2px);
  }
  .gg-badge { font-size: 11px; font-weight: 700; color: #fcd34d; white-space: nowrap; }
  .gg-badge.redisc { color: #4ade80; }
  .gg-text { color: #f1f5f9; font-size: 13px; line-height: 1.5; }
  .gg-rm { color: #94a3b8; font-size: 12px; }
  .gg-foot { padding: 6px 10px; background: #111827; color: #94a3b8; font-size: 11px; border-top: 1px solid #1f2937; }
</style>
