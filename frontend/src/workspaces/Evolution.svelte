<script lang="ts">
  // 进化史工作区：沿模型版本血缘（如草莓 s1→s8.1）看图论演化轨迹 + 版本 diff + ★标定坑清单。
  // 数据全来自 /api/evolution 契约（沿 evolution.yaml 走 git 历史算，EQC 持有事实）；前端只拼装展示。
  import { onDestroy } from 'svelte'
  import { store, reloadModel } from '../lib/store.svelte'
  import { fetchEvolution, fetchHistory, fetchSourceAtRev, saveSource, reportUrl } from '../lib/api'
  import type { EvolutionReport, GitCommit } from '../lib/contract'
  import Topology3d from '../components/Topology3d.svelte'
  import { firstChapterMap, applyReveal2d, clearReveal2d } from '../lib/reveal2d'

  let report = $state<EvolutionReport | null>(null)
  let loading = $state(false)
  let err = $state('')
  let sel = $state(0) // 选中版本下标（驱动 diff / 轨迹高亮）
  let showArtifacts = $state(false)
  // 演化回放（Step3）：局部 reveal 状态（不占全局 store.growth，与结构视图子系统生长隔离）
  let evoPlaying = $state(false)
  let evoChapter = $state(0)
  let evoTimer: ReturnType<typeof setInterval> | undefined
  let evoView = $state<'3d' | '2d'>('3d') // 演化回放 3D 拓扑 ↔ 2D Forrester 报告
  let evoIframe = $state<HTMLIFrameElement | undefined>(undefined)
  // Tier2 回退/checkout：本文件 git 历史 + 查看历史源码 + 人在环回退（write_source 校验+备份+原子写）
  let history = $state<GitCommit[]>([])
  let viewRev = $state('') // 正在查看源码的 rev（空=未展开）
  let viewSrc = $state('')
  let confirmRev = $state<GitCommit | null>(null) // 待确认回退的 commit
  let revertStatus = $state('')
  let reverting = $state(false)
  let lastModel: string | null = null

  $effect(() => {
    if (store.model !== lastModel) {
      lastModel = store.model
      stopEvo()
      resetRevert()
      load()
      loadHistory()
    }
  })
  onDestroy(() => clearInterval(evoTimer))

  function resetRevert() {
    history = []
    viewRev = ''
    viewSrc = ''
    confirmRev = null
    revertStatus = ''
    reverting = false
  }
  async function loadHistory() {
    try {
      const h = await fetchHistory(store.model)
      history = h.commits ?? []
    } catch {
      history = []
    }
  }
  async function viewSource(rev: string) {
    if (viewRev === rev) { viewRev = ''; viewSrc = ''; return } // 再点收起
    viewRev = rev
    viewSrc = '加载中…'
    try {
      const j = await fetchSourceAtRev(store.model, rev)
      viewSrc = j.source ?? j.error ?? '(无源码)'
    } catch (e) {
      viewSrc = '加载失败：' + String(e)
    }
  }
  async function doRevert() {
    if (!confirmRev || reverting) return
    reverting = true
    revertStatus = '⏳ 取历史源码…'
    try {
      const s = await fetchSourceAtRev(store.model, confirmRev.sha)
      if (!s.source) { revertStatus = '取源码失败：' + (s.error ?? '无源码'); reverting = false; return }
      revertStatus = '⏳ 校验 + 备份 + 写回…'
      const j = await saveSource(store.model, s.source) // write_source：校验通过才备份+原子写
      if (j.error) { revertStatus = '回退失败（校验/写盘被拒）：' + j.error; reverting = false; return }
      revertStatus = `✅ 已回退到 ${confirmRev.short}（备份 ${j.backup ?? ''}）· ⚠️ 决策缓存可能过期，需重跑 eqc optimize -o`
      confirmRev = null
      await reloadModel() // 工作树已变 → 刷新当前模型
    } catch (e) {
      revertStatus = '回退请求失败：' + String(e)
    }
    reverting = false
  }

  async function load() {
    loading = true
    err = ''
    report = null
    sel = 0
    try {
      const r = await fetchEvolution(store.model)
      if (r && r.error) err = r.error
      else if (r && r.versions?.length) {
        report = r
        sel = r.versions.length - 1 // 默认选末版
      } else err = '无进化数据'
    } catch (e) {
      err = '加载失败：' + String(e)
    }
    loading = false
  }

  const versions = $derived(report?.versions ?? [])
  const selVer = $derived(versions[sel] ?? null)
  const selDiff = $derived.by(() => {
    if (!report || sel <= 0 || !versions[sel]) return null
    return report.diffs.find((d) => d.to === versions[sel].version) ?? null
  })

  // —— 指标轨迹图几何 ——
  const W = 760, H = 172, padL = 34, padR = 14, padT = 14, padB = 34
  const plotW = W - padL - padR, plotH = H - padT - padB
  const METRICS = [
    { key: 'nodes', name: '节点', color: '#3b82f6' },
    { key: 'edges', name: '边', color: '#f97316' },
    { key: 'params', name: '参数', color: '#22c55e' },
    { key: 'confounded_pairs', name: '混淆对', color: '#ef4444' },
  ] as const

  const chart = $derived.by(() => {
    const n = versions.length
    if (!n) return null
    const xat = (i: number) => padL + (n <= 1 ? plotW / 2 : (i * plotW) / (n - 1))
    const cols = versions.map((v, i) => ({ x: xat(i), label: v.version }))
    const series = METRICS.map((m) => {
      const vals = versions.map((v) => (v as unknown as Record<string, number>)[m.key])
      const min = Math.min(...vals), max = Math.max(...vals)
      const yat = (val: number) => padT + plotH - (max > min ? (val - min) / (max - min) : 0.5) * plotH
      const pts = vals.map((val, i) => `${xat(i).toFixed(1)},${yat(val).toFixed(1)}`).join(' ')
      return { ...m, pts, min, max, cur: vals[sel] }
    })
    return { n, cols, series, guideX: xat(sel), hitW: plotW / Math.max(n - 1, 1) }
  })

  // —— 坑清单分组 ——
  const cliques = $derived((report?.calibration_pitlist ?? []).filter((e) => e.kind === 'confounding-clique'))
  const thresholds = $derived((report?.calibration_pitlist ?? []).filter((e) => e.kind === 'unidentifiable-threshold'))
  const artifacts = $derived(report?.structural_artifacts ?? [])
  const honest = $derived(report?.final_honest_identifiability ?? null)

  // —— 演化回放 plan：每版一章，nodes=该版新增节点本地名（基线节点未列→Topology3d 默认第0章显）——
  const evoPlan = $derived.by(() => {
    if (!report) return null
    return report.versions.map((v, i) => {
      const d = i === 0 ? null : report!.diffs.find((dd) => dd.to === v.version)
      return { key: v.version, title: v.version, narration: v.step, nodes: d ? d.added_nodes : [] }
    })
  })
  const nCh = $derived(evoPlan?.length ?? 0)
  // reveal 始终非空（隔离 store.growth）：不播时 chapter=末章=全显；播放时=evoChapter。
  const reveal = $derived(evoPlan ? { chapters: evoPlan, chapter: evoPlaying ? evoChapter : nCh - 1 } : null)
  const curNarr = $derived(evoPlaying && evoPlan ? evoPlan[evoChapter] : null)

  function playEvo() {
    if (nCh < 2) return
    evoPlaying = true
    evoChapter = 0
    clearInterval(evoTimer)
    evoTimer = setInterval(() => {
      if (evoChapter >= nCh - 1) clearInterval(evoTimer)
      else evoChapter += 1
    }, 2200)
  }
  function stopEvo() {
    evoPlaying = false
    evoChapter = 0
    clearInterval(evoTimer)
  }
  function stepEvo(dir: number) {
    clearInterval(evoTimer)
    evoChapter = Math.max(0, Math.min(nCh - 1, evoChapter + dir))
  }

  // —— 演化回放 · 2D 同步（reveal2d 共享·与子系统生长同一套逐章揭示）——
  const evoReportSrc = $derived(reportUrl(store.model, 'forrester', 'variable', store.topoColorMode))
  function evoDoc(): Document | null {
    try { return evoIframe?.contentDocument ?? null } catch { return null }
  }
  function applyEvo2d() {
    const d = evoDoc(); if (!d) return
    if (evoView !== '2d' || !evoPlan) { clearReveal2d(d); return }
    applyReveal2d(d, firstChapterMap(evoPlan), evoPlaying ? evoChapter : nCh - 1) // 不播=末章全显
  }
  $effect(() => {
    void evoChapter; void evoPlaying; void evoView; void store.model
    applyEvo2d()
  })
</script>

<div class="evo">
  <div class="head">
    <h2>进化史</h2>
    <span class="model">{store.model}</span>
    {#if report}<span class="tag">口径 {report.measurable_convention}</span>{/if}
    {#if loading}<span class="sub">加载中…</span>{/if}
  </div>

  {#if err}
    <div class="empty">
      <p class="big">{err}</p>
      <p class="sub">进化史需要该模型目录下有 <code>evolution.yaml</code> 血缘清单（沿它走 git 历史逐版本分析）。目前草莓 / 番茄 / 蓝莓有，其它模型暂无。</p>
    </div>
  {:else if report}
    <!-- ① 版本轨迹带 -->
    <section class="card">
      <div class="chips">
        {#each versions as v, i}
          <button class="chip" class:active={i === sel} onclick={() => (sel = i)} title={v.step}>{v.version}</button>
        {/each}
      </div>
      {#if chart}
        <svg viewBox="0 0 {W} {H}" class="traj" role="img" aria-label="指标演化轨迹">
          <line x1={chart.guideX} y1={padT} x2={chart.guideX} y2={H - padB} class="guide" />
          {#each chart.series as s}
            <polyline points={s.pts} fill="none" stroke={s.color} stroke-width="2" stroke-linejoin="round" />
          {/each}
          {#each chart.cols as c, i}
            <text x={c.x} y={H - 12} class="xlab" class:on={i === sel} text-anchor="middle">{c.label}</text>
            {#if i === sel}<circle cx={c.x} cy={H - padB} r="3" class="dot" />{/if}
          {/each}
        </svg>
        <div class="legend">
          {#each chart.series as s}
            <span class="li"><i style="background:{s.color}"></i>{s.name} <b>{s.cur}</b> <em>({s.min}→{s.max})</em></span>
          {/each}
        </div>
      {/if}
    </section>

    <!-- 🎬 演化回放：沿血缘逐版本"长出"当前模型 -->
    {#if evoPlan && nCh > 1}
      <section class="card">
        <h3>🎬 演化回放 <span class="sub">沿血缘逐版本"长出"当前模型（{store.model}）</span>
          <span class="viewtog">
            <button class:on={evoView === '3d'} onclick={() => (evoView = '3d')}>3D 拓扑</button>
            <button class:on={evoView === '2d'} onclick={() => (evoView = '2d')}>2D 报告</button>
          </span>
        </h3>
        <div class="evo3d">
          {#if evoView === '3d'}
            <Topology3d contract={store.modelJson} {reveal} />
          {:else}
            <iframe class="evo2dframe" bind:this={evoIframe} title="演化回放 2D 报告" src={evoReportSrc} onload={applyEvo2d}></iframe>
          {/if}
          {#if curNarr}<div class="narr"><b>{curNarr.title}</b> · {curNarr.narration}</div>{/if}
        </div>
        <div class="playbar">
          {#if !evoPlaying}
            <button class="pbtn" onclick={playEvo}>▶ 播放进化</button>
            <span class="sub">从 {evoPlan[0].title} 逐版本长到 {evoPlan[nCh - 1].title}（{nCh} 版）</span>
          {:else}
            <button class="pbtn" onclick={() => stepEvo(-1)}>⏮</button>
            <button class="pbtn" onclick={() => stepEvo(1)}>⏭</button>
            <button class="pbtn" onclick={stopEvo}>⏹ 退出</button>
            <span class="ch">{evoChapter + 1}/{nCh} · {evoPlan[evoChapter].title}</span>
          {/if}
        </div>
        <p class="sub note">呈现「当前模型怎么一步步搭起来」（存活血缘）；中途被删的瞬态节点不在最新图、不显示。</p>
      </section>
    {/if}

    <!-- ② 选中版本 + 结构 diff -->
    {#if selVer}
      <section class="card">
        <h3>{selVer.version} <span class="step">{selVer.step}</span></h3>
        <div class="stats">
          <span>节点 <b>{selVer.nodes}</b></span>
          <span>边 <b>{selVer.edges}</b></span>
          <span>深度 <b>{selVer.depth}</b></span>
          <span>参数 <b>{selVer.params}</b></span>
          <span>混淆对 <b>{selVer.confounded_pairs}</b></span>
          <span>代数环 <b>{selVer.algebraic_loops}</b></span>
          {#if selVer.hubs?.length}<span>枢纽 <b>{selVer.hubs[0]}</b></span>{/if}
        </div>
        {#if selDiff}
          <div class="diff">
            <div class="diffhead">← 从 {selDiff.from} 的结构变化 · 距离 {selDiff.distance} · 边相似 {selDiff.edge_similarity.toFixed(2)}</div>
            {#if selDiff.added_equations.length}<div class="drow">➕ 新增方程 <b>{selDiff.added_equations.length}</b>：{selDiff.added_equations.join('、')}</div>{/if}
            {#if selDiff.added_params.length}<div class="drow">➕ 新增参数 <b>{selDiff.added_params.length}</b>：{selDiff.added_params.join('、')}</div>{/if}
            <div class="drow">➕ 新增边 <b>{selDiff.added_edges}</b>{#if selDiff.removed_edges} · 删边 {selDiff.removed_edges}{/if}</div>
            {#if selDiff.changed_equations.length}<div class="drow">🔧 形式改变方程：{selDiff.changed_equations.join('、')}</div>{/if}
            {#if selDiff.new_confounded.length}<div class="drow hot">🎯 这一步新引入混淆对：{selDiff.new_confounded.map((p) => p.join('~')).join('，')}</div>{/if}
          </div>
        {:else if sel === 0}
          <div class="sub pad">链首版本，无前序 diff。</div>
        {/if}
      </section>
    {/if}

    <!-- ③ ★标定坑清单 -->
    <section class="card pit">
      <h3>🎯 标定坑清单 <span class="sub">结构分析给标定实验设计的抓手</span></h3>
      {#if cliques.length}
        <div class="pgh">混淆系数簇 · 异参同效 → 需联合标定或加正交/多工况对照实验</div>
        {#each cliques as e}
          <div class="pititem clique">
            <div class="pp">{e.params.join(' × ')}</div>
            <div class="pm">{e.mechanism ?? '?'} {#if e.equation}<span class="eq">{e.equation}</span>{/if} <span class="ver">@{e.introduced_at}</span></div>
          </div>
        {/each}
      {/if}
      {#if thresholds.length}
        <div class="pgh sep">阈值不可辨识 · 纯阈值/临界常数、结构够不到数据 → 只能靠先验</div>
        {#each thresholds as e}
          <div class="pititem thr">
            <div class="pp">{e.params.join(' × ')}</div>
            <div class="pm">{e.mechanism ?? '?'} {#if e.equation}<span class="eq">{e.equation}</span>{/if} <span class="ver">@{e.introduced_at}</span></div>
          </div>
        {/each}
      {/if}
      {#if !cliques.length && !thresholds.length}
        <div class="sub pad">此链无异参同效簇 / 阈值不可辨识参数。</div>
      {/if}
      {#if artifacts.length}
        <button class="foot" onclick={() => (showArtifacts = !showArtifacts)}>
          {showArtifacts ? '▾' : '▸'} 结构假象脚注（{artifacts.length}）· cohort/箱车离散化的固定标记簇 · 非标定目标
        </button>
        {#if showArtifacts}
          {#each artifacts as e}
            <div class="pititem art">
              <div class="pp">{e.params.length > 4 ? e.params.slice(0, 3).join(' × ') + ` … (${e.params.length} 成员)` : e.params.join(' × ')}</div>
              <div class="pm">{e.mechanism ?? '?'} {#if e.equation}<span class="eq">{e.equation}</span>{/if} <span class="ver">@{e.introduced_at}</span></div>
            </div>
          {/each}
        {/if}
      {/if}
    </section>

    <!-- ④ 诚实白名单可辨识性 -->
    {#if honest}
      <section class="card">
        <h3>诚实白名单可辨识性 <span class="sub">{honest.version} · 真田间可测量 {honest.measurable_whitelist.length} 项</span></h3>
        {#if honest.unidentifiable.length}
          <div class="pad">结构上标不出：<b class="warn">{honest.unidentifiable.join('、')}</b></div>
        {:else}
          <div class="sub pad">白名单口径下全部参数可辨识 ✓</div>
        {/if}
        <div class="sub note">{honest.note}</div>
      </section>
    {/if}

    <!-- 📜 Tier2：本文件版本历史 + 人在环回退 -->
    {#if history.length}
      <section class="card">
        <h3>📜 本文件版本历史 <span class="sub">git 提交 · 回退=覆盖工作树（校验+备份+人在环）</span></h3>
        {#if revertStatus}<div class="rstat" class:okmsg={revertStatus.startsWith('✅')}>{revertStatus}</div>{/if}
        {#each history as c}
          <div class="hrow">
            <code class="sha">{c.short}</code>
            <span class="hdate">{c.date}</span>
            <span class="hsub" title={c.subject}>{c.subject}</span>
            <button class="hbtn" onclick={() => viewSource(c.sha)}>{viewRev === c.sha ? '收起' : '查看'}</button>
            <button class="hbtn danger" onclick={() => (confirmRev = c)}>⤺ 回退</button>
          </div>
          {#if viewRev === c.sha}<pre class="hsrc">{viewSrc}</pre>{/if}
        {/each}
        {#if confirmRev}
          <div class="confirm">
            <div class="chead"><b>确认把当前模型回退到此版本？</b></div>
            <div class="sub">{confirmRev.short} · {confirmRev.date} · {confirmRev.subject}</div>
            <div class="sub cwarn">将用该版本源码<b>覆盖当前文件</b>——write_source 自动<b>校验（不合法则拒绝）+ 备份 .bak + 原子写</b>；git 历史不变、随时可再回退。</div>
            <div class="cact">
              <button class="hbtn danger" disabled={reverting} onclick={doRevert}>确认回退</button>
              <button class="hbtn" disabled={reverting} onclick={() => (confirmRev = null)}>取消</button>
            </div>
          </div>
        {/if}
        <p class="sub note">v1 只回退本文件自己的 git 历史；跨文件的血缘版本（如 s1 是另一个文件）回退留后。</p>
      </section>
    {/if}
  {/if}
</div>

<style>
  .evo { max-width: 880px; }
  .head { display: flex; align-items: baseline; gap: 10px; margin-bottom: 12px; }
  .head h2 { margin: 0; font-size: 18px; }
  .model { font-weight: 600; color: var(--accent); }
  .tag { font-size: 12px; color: var(--sub); border: 1px solid var(--line); border-radius: 10px; padding: 1px 8px; }
  .sub { color: var(--sub); font-size: 13px; }
  .empty { border: 1px dashed var(--line); border-radius: 10px; padding: 28px; text-align: center; }
  .empty .big { font-size: 15px; margin: 0 0 8px; }
  code { background: #f3f4f6; padding: 1px 5px; border-radius: 4px; }

  .card { border: 1px solid var(--line); border-radius: 10px; background: #fff; padding: 14px 16px; margin-bottom: 12px; }
  .card h3 { margin: 0 0 10px; font-size: 15px; display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; }
  .step { font-weight: 400; color: var(--sub); font-size: 13px; }

  .chips { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 10px; }
  .chip { border: 1px solid var(--line); background: #fff; border-radius: 14px; padding: 3px 12px; font-size: 13px; cursor: pointer; color: var(--sub); }
  .chip:hover { background: #f3f4f6; }
  .chip.active { background: var(--accent); color: #fff; border-color: var(--accent); font-weight: 600; }

  .traj { width: 100%; height: auto; display: block; }
  .traj .guide { stroke: var(--accent); stroke-width: 1.5; stroke-dasharray: 3 3; opacity: 0.5; }
  .traj .xlab { font-size: 11px; fill: var(--sub); }
  .traj .xlab.on { fill: var(--accent); font-weight: 700; }
  .traj .dot { fill: var(--accent); }
  .legend { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 6px; font-size: 12px; color: var(--sub); }
  .legend .li i { display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin-right: 4px; vertical-align: middle; }
  .legend b { color: #111; }
  .legend em { font-style: normal; opacity: 0.7; }

  .stats { display: flex; flex-wrap: wrap; gap: 14px; font-size: 13px; color: var(--sub); margin-bottom: 8px; }
  .stats b { color: #111; }
  .diff { border-top: 1px solid var(--line); padding-top: 8px; margin-top: 4px; }
  .diffhead { font-size: 13px; color: var(--sub); margin-bottom: 6px; }
  .drow { font-size: 13px; padding: 2px 0; }
  .drow b { color: #111; }
  .drow.hot { color: #b45309; font-weight: 600; }
  .pad { padding: 4px 0; }

  .pit { background: #fffdf7; }
  .pgh { font-size: 12px; color: var(--sub); margin: 4px 0 6px; }
  .pgh.sep { border-top: 1px dashed var(--line); padding-top: 10px; margin-top: 12px; }
  .pititem { display: flex; align-items: baseline; gap: 10px; padding: 5px 8px; border-radius: 6px; margin-bottom: 4px; }
  .pititem .pp { font-family: ui-monospace, monospace; font-size: 13px; font-weight: 600; min-width: 200px; }
  .pititem .pm { font-size: 13px; color: var(--sub); }
  .pititem .eq { font-family: ui-monospace, monospace; color: #6366f1; }
  .pititem .ver { opacity: 0.6; }
  .pititem.clique { background: #fff3e6; }
  .pititem.thr { background: #f3e8ff; }
  .pititem.art { background: #f1f2f4; }
  .foot { border: 0; background: transparent; color: var(--sub); font-size: 12px; cursor: pointer; padding: 8px 0 4px; display: block; }
  .foot:hover { color: #111; }

  .warn { color: #7c3aed; }
  .note { margin-top: 6px; opacity: 0.85; }

  .viewtog { margin-left: auto; display: inline-flex; gap: 4px; }
  .viewtog button { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 2px 10px; border-radius: 6px; cursor: pointer; }
  .viewtog button.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .evo3d { position: relative; height: 380px; border: 1px solid var(--line); border-radius: 8px; overflow: hidden; background: #fafbfc; }
  .evo2dframe { width: 100%; height: 100%; border: 0; background: #fff; }
  .narr { position: absolute; left: 0; right: 0; bottom: 0; padding: 8px 12px; background: rgba(17, 24, 39, 0.72); color: #fff; font-size: 13px; }
  .narr b { color: #fbbf24; }
  .playbar { display: flex; align-items: center; gap: 8px; margin-top: 8px; flex-wrap: wrap; }
  .pbtn { border: 1px solid var(--line); background: #fff; border-radius: 6px; padding: 4px 12px; font-size: 13px; cursor: pointer; }
  .pbtn:hover { background: #f3f4f6; }
  .ch { font-size: 13px; color: var(--accent); font-weight: 600; }

  .rstat { font-size: 13px; padding: 6px 10px; border-radius: 6px; background: #f3f4f6; margin-bottom: 8px; }
  .rstat.okmsg { background: #f0fdf4; color: #15803d; }
  .hrow { display: flex; align-items: center; gap: 10px; padding: 5px 0; border-bottom: 1px solid #f1f2f4; font-size: 13px; }
  .sha { font-family: ui-monospace, monospace; color: #6366f1; }
  .hdate { color: var(--sub); font-size: 12px; white-space: nowrap; }
  .hsub { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hbtn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 10px; border-radius: 6px; cursor: pointer; white-space: nowrap; }
  .hbtn:hover { background: #f3f4f6; }
  .hbtn.danger { color: #b91c1c; border-color: #fca5a5; }
  .hbtn.danger:hover { background: #fef2f2; }
  .hbtn:disabled { opacity: 0.5; cursor: default; }
  .hsrc { max-height: 240px; overflow: auto; background: #0f172a; color: #e2e8f0; font-family: ui-monospace, monospace; font-size: 12px; padding: 8px 10px; border-radius: 6px; margin: 4px 0 8px; white-space: pre-wrap; }
  .confirm { border: 1px solid #fde68a; background: #fffbeb; border-radius: 8px; padding: 10px 12px; margin: 8px 0; }
  .chead { font-size: 13px; }
  .cwarn { margin-top: 4px; }
  .cact { margin-top: 8px; display: flex; gap: 8px; }
</style>
