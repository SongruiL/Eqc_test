<script lang="ts">
  // 进化（GP）工作区：靶点多选 → 配置 → 异步进化（进度+实时收敛）→ Pareto 前沿 → 候选详情（单/联合）。
  // 数据全来自 /api/evolve[/start|/status] 契约；前端只拼装、点选、采纳。
  import { store } from '../lib/store.svelte'
  import { startEvolve, evolveStatus } from '../lib/api'
  import type { EvolveResult, GpCandidate, GpBaseline, GpTraj } from '../lib/contract'
  import CandidateBlock from '../components/gp/CandidateBlock.svelte'

  const mod = $derived(store.modelJson?.modules?.[0])
  const targets = $derived((mod?.equations ?? []).filter((e) => e.gp_target))

  let sel = $state<string[]>([])
  let pop = $state(60)
  let gens = $state(40)
  let seed = $state(1)
  let memetic = $state(false)

  let running = $state(false)
  let status = $state('')
  let prog = $state<{ gen: number; total: number; conv: string }>({ gen: 0, total: 0, conv: '' })
  let result = $state<EvolveResult | null>(null)
  let point = $state(0)
  let poll: ReturnType<typeof setTimeout> | undefined

  // 切模型 → 清空
  let lastModel: string | null = null
  $effect(() => {
    if (store.model !== lastModel) {
      lastModel = store.model
      sel = []
      result = null
      running = false
      status = ''
      clearTimeout(poll)
    }
  })

  function toggle(id: string) {
    sel = sel.includes(id) ? sel.filter((x) => x !== id) : [...sel, id]
  }

  async function run() {
    if (!sel.length || running) return
    running = true
    result = null
    status = ''
    prog = { gen: 0, total: Number(gens), conv: '' }
    const s = await startEvolve({
      model: store.model, zone: store.zone, targets: sel,
      pop: Number(pop), gens: Number(gens), seed: Number(seed), memetic,
    })
    if (s.error) { status = '进化失败：' + s.error; running = false; return }
    if (!s.task_id) { status = '启动失败'; running = false; return }
    pollLoop(s.task_id)
  }

  function pollLoop(id: string) {
    clearTimeout(poll)
    const tick = async () => {
      let j
      try {
        j = await evolveStatus(id)
      } catch {
        poll = setTimeout(tick, 800)
        return
      }
      if (!j.status) { status = j.error ?? '状态查询失败'; running = false; return }
      prog = { gen: j.gen ?? 0, total: j.total_gens ?? 0, conv: j.convergence_svg ?? '' }
      if (j.status === 'done') {
        running = false
        if (j.result) {
          result = j.result
          point = 0
          status = '完成 · ' + (j.result.targets?.join('+') ?? j.result.target ?? '')
        }
        return
      }
      if (j.status === 'error') { status = '进化出错：' + (j.error ?? ''); running = false; return }
      poll = setTimeout(tick, 600)
    }
    tick()
  }

  // Pareto 散点点选（事件委托）：读 data-i + 高亮该点（@html 内 DOM 我自有，可手动改）
  function onPareto(e: MouseEvent) {
    const t = e.target as Element
    if (!t?.classList?.contains('pp')) return
    const svg = t.closest('svg')
    svg?.querySelectorAll('circle.pp').forEach((c) => {
      c.setAttribute('r', '4')
      c.setAttribute('fill', '#2563eb')
    })
    t.setAttribute('r', '6')
    t.setAttribute('fill', '#dc2626')
    point = +(t.getAttribute('data-i') ?? 0)
  }

  const entry = $derived(result?.pareto_front?.[point])
  function baselineFor(c: GpCandidate): GpBaseline | undefined {
    return result?.joint ? result?.baselines?.[c.target ?? ''] : result?.baseline
  }
  function observedFor(c: GpCandidate): GpTraj | null | undefined {
    return result?.joint
      ? (result?.observed as Record<string, GpTraj> | undefined)?.[c.output ?? '']
      : (result?.observed as GpTraj | undefined)
  }
  function fmt(x?: number | null): string {
    if (x == null) return '—'
    return Math.abs(x) >= 1000 || (Math.abs(x) < 0.001 && x !== 0) ? x.toExponential(3) : x.toFixed(4)
  }
</script>

<div class="ws">
  <div class="ws-head">
    <b>受约束 GP（方程结构进化）</b>
    <span class="cfg">种群 <input type="number" min="4" bind:value={pop} /></span>
    <span class="cfg">代数 <input type="number" min="1" bind:value={gens} /></span>
    <span class="cfg">种子 <input type="number" bind:value={seed} /></span>
    <label class="cfg" title="内层 DE 标定每个候选的常数：更准但更慢"><input type="checkbox" bind:checked={memetic} /> memetic</label>
    <button class="btn on" disabled={!sel.length || running} onclick={run}>开始进化</button>
  </div>

  {#if !targets.length}
    <div class="hint">本模型没有 🟠 进化靶点（gp_target）。</div>
  {:else}
    <div class="hint">在机理留白（🟠 靶点）处提议方程形式——你来挑前沿、判 rediscovery、采纳。
      观测取<b>当前处理区</b>（{store.zone}）录入数据。多选 ≥2 = 联合进化。</div>

    {#if targets.length > 1}
      <div class="seltools">
        <span class="sub">多选 = 联合（一次仿真同拟合、捕捉槽位间耦合）</span>
        <button class="btn" onclick={() => (sel = targets.map((t) => t.id))}>全选</button>
        <button class="btn" onclick={() => (sel = [])}>清空</button>
      </div>
    {/if}

    <div class="targets">
      {#each targets as t}
        {@const g = t.gp_target}
        <button class="tgt" class:sel={sel.includes(t.id)} onclick={() => toggle(t.id)}>
          <div class="tgt-name">🟠 {t.name} <span class="gram">{g?.grammar ?? ''}</span></div>
          <div class="tgt-meta">输出 {t.output}　·　输入 {(g?.inputs?.length ? g.inputs : t.refs).join(', ')}</div>
        </button>
      {/each}
    </div>

    <div class="status">
      {#if status}{status}{:else if sel.length === 0}点靶点选择（多选=联合）。{:else if sel.length === 1}已选 1 个（单靶）。点「开始进化」。{:else}已选 {sel.length} 个 = 联合进化。点「开始进化」。{/if}
    </div>

    {#if running || prog.conv}
      <div class="progress">
        <div class="sub">{running ? `⏳ 进化中… 第 ${prog.gen}/${prog.total} 代` : `✅ 完成 · ${prog.total} 代`}</div>
        {#if prog.conv}<div class="conv">{@html prog.conv}</div>{/if}
      </div>
    {/if}

    {#if result}
      <div class="grid">
        <div class="left">
          <div class="sub">Pareto 前沿 · {result.joint ? '总' : ''}复杂度 vs {result.joint ? '平均' : ''}rmse（点拐点）</div>
          <!-- 点选委托到 EQC 生成的 SVG 内 circle.pp；容器只转发点击 -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div class="pareto" role="presentation" onclick={onPareto}>{@html result.pareto_svg ?? ''}</div>
          <div class="hint">前沿 {result.pareto_front.length} 个{result.joint ? ' 套配置' : ' 候选'} · 实测 {result.n_obs ?? 0} 点。点某点看详情。</div>
        </div>
        <div class="right">
          {#if entry}
            {#if entry.slots}
              <div class="sub">整模型配置 · 总复杂度 {entry.complexity} · 平均rmse {fmt(entry.error)}</div>
              {#each entry.slots as s, k}
                <div class="slot">
                  <div class="slot-hd">槽位 {k + 1}：<b>{s.target}</b>（输出 {s.output}）</div>
                  <CandidateBlock cand={s} baseline={baselineFor(s)} observed={observedFor(s)} name={s.target ?? 'gp'} />
                </div>
              {/each}
            {:else}
              <CandidateBlock cand={entry as GpCandidate} baseline={result.baseline} observed={result.observed as GpTraj} name={result.target ?? 'gp'} autoOpenSignal={store.gpGrowSignal} />
            {/if}
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .ws { display: flex; flex-direction: column; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .cfg { font-size: 12px; color: var(--sub); display: inline-flex; align-items: center; gap: 5px; }
  .cfg input[type='number'] { width: 58px; font-size: 12px; padding: 2px 5px; border: 1px solid var(--line); border-radius: 6px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; }
  .btn:hover { background: #eef2ff; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .hint { color: var(--sub); font-size: 12px; margin: 6px 0; }
  .seltools { display: flex; align-items: center; gap: 8px; margin: 8px 0; }
  .sub { color: var(--sub); font-size: 12px; font-weight: 600; }
  .targets { display: flex; flex-wrap: wrap; gap: 8px; margin: 8px 0; }
  .tgt { text-align: left; border: 1px solid var(--line); border-radius: 8px; padding: 7px 10px; cursor: pointer; background: #fff; font-size: 12px; max-width: 340px; }
  .tgt:hover { background: #eef2ff; }
  .tgt.sel { border-color: var(--accent); background: #eef2ff; box-shadow: 0 0 0 1px var(--accent) inset; }
  .tgt-name { font-weight: 600; color: var(--ink); }
  .gram { display: inline-block; background: #fef9c3; color: #854d0e; border-radius: 10px; padding: 0 7px; font-size: 11px; }
  .tgt-meta { color: var(--sub); margin-top: 3px; }
  .status { font-size: 12px; color: var(--sub); margin: 6px 0; }
  .progress { margin: 8px 0; }
  .conv :global(svg) { width: 100%; max-width: 720px; height: auto; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; align-items: start; }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  .pareto :global(svg) { width: 100%; max-width: 700px; height: auto; }
  .pareto :global(circle.pp) { cursor: pointer; }
  .slot { border: 1px solid var(--line); border-radius: 8px; padding: 10px; margin-top: 10px; }
  .slot-hd { font-size: 12px; font-weight: 600; border-bottom: 1px solid var(--line); padding-bottom: 5px; margin-bottom: 6px; }
</style>
