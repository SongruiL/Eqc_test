<script lang="ts">
  // 结构工作区：EQC 自生成报告（Forrester 图 + 二维公式）嵌 iframe + 粒度/布局/缩放 + 节点交互。
  // 报告本身零 JS（只带 data-* 属性）；交互逻辑在此，伸进同源 iframe 挂事件（移植 v1）。
  import { onMount, onDestroy } from 'svelte'
  import { store } from '../lib/store.svelte'
  import { reportUrl, fetchModel } from '../lib/api'
  import type { VarJson, ParamJson, EqJson, ModelJson } from '../lib/contract'

  let layout = $state('forrester')
  let level = $state('variable')
  let zoom = $state(1)
  let iframeEl: HTMLIFrameElement
  let tip: HTMLDivElement | undefined // 悬停注释卡：命令式挂到 document.body（v1 做法，避免定位/绑定问题）
  // 本组件自持一份契约（hover 注释用）：不依赖 store.modelJson 的时机/反应式作用域，随模型变化重取。
  let contract = $state<ModelJson | null>(store.modelJson)
  let lastModel = ''
  const src = $derived(reportUrl(store.model, layout, level))

  $effect(() => {
    if (store.model === lastModel) return
    lastModel = store.model
    fetchModel(store.model).then((j) => { contract = j }).catch(() => {})
  })

  onMount(() => {
    tip = document.createElement('div')
    tip.className = 'eqc-nodetip'
    document.body.appendChild(tip)
  })
  onDestroy(() => tip?.remove())

  const levels = [
    { id: 'variable', label: '变量' }, { id: 'equation', label: '方程' }, { id: 'module', label: '模块' },
  ]
  const layouts = [
    { id: 'forrester', label: 'Forrester' }, { id: 'force', label: '力导向' }, { id: 'layered', label: '分层' },
  ]
  function rdoc(): Document | null {
    try { return iframeEl?.contentDocument ?? null } catch { return null }
  }

  // —— 缩放：伸进 iframe 按 viewBox 比例设每张 .dag-svg 的像素宽（容器自带滚动条平移）。zoom=1=适应宽。 ——
  function applyZoom() {
    const d = rdoc(); if (!d) return
    d.querySelectorAll('.dag-svg').forEach((node) => {
      const el = node as SVGSVGElement
      const cont = el.closest('.dag') as HTMLElement | null
      const cw = (cont ? cont.clientWidth : iframeEl.clientWidth) - 26
      const vb = (el.getAttribute('viewBox') || '0 0 1000 1000').split(/\s+/).map(parseFloat)
      const W = vb[2] || 1000, H = vb[3] || 1000
      const w = Math.max(40, cw * zoom)
      el.style.minWidth = '0'
      el.style.width = w + 'px'
      el.style.height = (w * H) / W + 'px'
    })
  }
  function setZoom(z: number) { zoom = Math.max(0.2, Math.min(6, +z.toFixed(3))) }
  $effect(() => { void zoom; applyZoom() })

  // —— 节点 hover 注释（取自 /api/model 契约）+ 点选高亮（与仿真变量选择联动）——
  const CLS_CN: Record<string, string> = {
    state: '存量', rate: '速率', driving: '驱动', auxiliary: '辅助', parameter: '参数', control: '控制', semi_state: '半状态', boundary: '边界',
  }
  const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  // 遍历**所有**模块（耦合视图=温室+作物多模块；单模型=1 模块）。返回首个匹配。
  function findVar(name: string): { kind: 'var'; v: VarJson } | { kind: 'param'; p: ParamJson } | null {
    for (const m of contract?.modules ?? []) {
      const v = m.variables.find((x) => x.name === name); if (v) return { kind: 'var', v }
      const p = m.parameters.find((x) => x.name === name); if (p) return { kind: 'param', p }
    }
    return null
  }
  function findEq(name: string): EqJson | undefined {
    for (const m of contract?.modules ?? []) {
      const e = m.equations.find((x) => x.output === name); if (e) return e
    }
    return undefined
  }
  function dispName(name: string) {
    const r = findVar(name); if (!r) return name
    return (r.kind === 'var' ? r.v.display_name : r.p.display_name || r.p.name_cn) || name
  }
  function tipHtml(name: string) {
    const info = findVar(name), eq = findEq(name), disp = dispName(name)
    let h = '<div class="t-name">' + esc(disp) + '</div>'
    if (disp !== name) h += '<div class="t-id">代号 ' + esc(name) + '</div>'
    if (info?.kind === 'var') {
      const v = info.v, cls = CLS_CN[v.class] || v.class
      h += '<div class="t-sub">' + esc(cls) + (v.unit ? ' · 单位 ' + esc(v.unit) : '') + '</div>'
      if (v.description) h += '<div class="t-desc"><b>物理意义</b>：' + esc(v.description) + '</div>'
    } else if (info?.kind === 'param') {
      const p = info.p
      h += '<div class="t-sub">参数' + (p.unit ? ' · 单位 ' + esc(p.unit) : '') + '</div>'
      h += '<div class="t-desc">默认值 = ' + p.default + '</div>'
    }
    if (eq) {
      h += '<div class="t-eq">' + eq.mathml + '</div>'
      if (eq.reference) h += '<div class="t-cite">📖 ' + esc(eq.reference) + '</div>'
    } else if (info?.kind === 'var') {
      const c = info.v.class
      let why = '（外部输入）'
      if (c === 'state') why = '（状态量：值由其速率逐步积分得到，无显式方程）'
      else if (c === 'semi_state') why = '（延迟寄存器：取来源变量的上一步值）'
      else if (c === 'driving') why = '（驱动量：来自外部输入/天气数据）'
      else if (c === 'control') why = '（控制量：可由用户/环控设定）'
      h += '<div class="t-cite t-none">' + why + '</div>'
    }
    return h
  }
  function showTip(name: string, x: number, y: number) {
    if (!tip) return
    tip.innerHTML = tipHtml(name)
    tip.style.display = 'block'
    const tw = tip.offsetWidth, th = tip.offsetHeight
    let px = x + 14, py = y + 14
    if (px + tw > window.innerWidth - 8) px = x - tw - 14
    if (py + th > window.innerHeight - 8) py = window.innerHeight - th - 8
    tip.style.left = Math.max(8, px) + 'px'
    tip.style.top = Math.max(8, py) + 'px'
  }
  const hideTip = () => { if (tip) tip.style.display = 'none' }

  function setHl(name: string, on: boolean) {
    const d = rdoc(); if (!d) return
    const q = name.replace(/"/g, '')
    d.querySelectorAll('[data-var="' + q + '"]').forEach((e) => e.classList.toggle('hl', on))
    d.querySelector('.eq[data-output="' + q + '"]')?.classList.toggle('hl', on)
  }
  function syncAllHl() { for (const v of store.selectedVars) setHl(v, true) }
  function selectVar(name: string) {
    const has = store.selectedVars.includes(name)
    store.selectedVars = has ? store.selectedVars.filter((n) => n !== name) : [...store.selectedVars, name]
    setHl(name, !has)
  }

  // —— 节点交互 + 拖拽/平移（移植 v1 wireNodeClicks + wirePan）——
  function wire() {
    const d = rdoc(); if (!d) return
    d.querySelectorAll('[data-var]').forEach((node) => {
      const g = node as HTMLElement & { _wired?: boolean }
      if (g._wired) return
      g._wired = true
      const name = g.getAttribute('data-var') || ''
      g.addEventListener('mouseenter', () => {
        const fr = iframeEl.getBoundingClientRect(), nr = g.getBoundingClientRect()
        showTip(name, fr.left + nr.right, fr.top + nr.top)
      })
      g.addEventListener('mouseleave', hideTip)
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      g.addEventListener('click', () => { if (!(d as any)._suppressClick) selectVar(name) })
    })
    wirePan(d)
  }
  // 拖背景=平移画布（横滚 .dag + 纵滚 iframe 窗口）；拖节点框=移动+连线跟随（会话内、刷新复位）。
  function wirePan(d: Document) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const D = d as any
    if (D._panWired) return
    D._panWired = true
    const win = iframeEl.contentWindow!
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let mode: string | null = null, sx = 0, sy = 0, dag: any = null, moved = false
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let node: any = null, svg: SVGSVGElement | null = null, startTx = 0, startTy = 0, startU = { x: 0, y: 0 }

    const userPt = (s: SVGSVGElement, e: MouseEvent) => {
      const pt = s.createSVGPoint(); pt.x = e.clientX; pt.y = e.clientY
      const m = s.getScreenCTM(); return m ? pt.matrixTransform(m.inverse()) : { x: e.clientX, y: e.clientY }
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const center = (g: any) => ({ cx: +g.dataset.cx + (+g.dataset.tx || 0), cy: +g.dataset.cy + (+g.dataset.ty || 0), hw: +g.dataset.hw || 60, hh: +g.dataset.hh || 20 })
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const boxExit = (c: any, tx: number, ty: number) => {
      const dx = tx - c.cx, dy = ty - c.cy
      if (Math.abs(dx) < 1e-6 && Math.abs(dy) < 1e-6) return [c.cx, c.cy]
      const ex = Math.abs(dx) < 1e-6 ? Infinity : c.hw / Math.abs(dx), ey = Math.abs(dy) < 1e-6 ? Infinity : c.hh / Math.abs(dy)
      const t = Math.min(ex, ey); return [c.cx + dx * t, c.cy + dy * t]
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    function reEdges(g: any) {
      const id = (g.dataset.id || '').replace(/"/g, ''), s = g.ownerSVGElement
      if (!id || !s) return
      s.querySelectorAll('[data-from="' + id + '"],[data-to="' + id + '"]').forEach((p: Element) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const pe = p as any
        const a = s.querySelector('[data-id="' + (pe.dataset.from || '').replace(/"/g, '') + '"]')
        const b = s.querySelector('[data-id="' + (pe.dataset.to || '').replace(/"/g, '') + '"]')
        if (!a || !b) return
        const ca = center(a), cb = center(b)
        const [x1, y1] = boxExit(ca, cb.cx, cb.cy), [x2, y2] = boxExit(cb, ca.cx, ca.cy)
        const dx = x2 - x1, dy = y2 - y1, len = Math.max(1, Math.hypot(dx, dy))
        const nx = -dy / len, ny = dx / len, bow = Math.min(26, len * 0.12)
        const mx = (x1 + x2) / 2 + nx * bow, my = (y1 + y2) / 2 + ny * bow
        pe.setAttribute('d', 'M' + x1.toFixed(0) + ',' + y1.toFixed(0) + ' Q' + mx.toFixed(0) + ',' + my.toFixed(0) + ' ' + x2.toFixed(0) + ',' + y2.toFixed(0))
      })
    }
    d.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return
      moved = false; sx = e.clientX; sy = e.clientY
      const t = e.target as Element
      const g = t.closest?.('[data-var]')
      if (g) {
        mode = 'node'; node = g; svg = (g as SVGElement).ownerSVGElement
        startTx = +node.dataset.tx || 0; startTy = +node.dataset.ty || 0; startU = svg ? userPt(svg, e) : { x: 0, y: 0 }
        hideTip(); d.body.style.cursor = 'grabbing'; e.preventDefault()
      } else {
        mode = 'pan'; dag = t.closest?.('.dag')
        if (dag) { d.body.style.cursor = 'grabbing'; e.preventDefault() }
      }
    })
    d.addEventListener('mousemove', (e) => {
      if (!mode) return
      if (Math.abs(e.clientX - sx) + Math.abs(e.clientY - sy) > 4) moved = true
      if (mode === 'node' && svg) {
        const u = userPt(svg, e)
        node.dataset.tx = String(startTx + (u.x - startU.x))
        node.dataset.ty = String(startTy + (u.y - startU.y))
        node.setAttribute('transform', 'translate(' + +node.dataset.tx + ',' + +node.dataset.ty + ')')
        reEdges(node)
      } else if (mode === 'pan') {
        if (dag) dag.scrollLeft -= e.clientX - sx
        win.scrollBy(0, -(e.clientY - sy))
        sx = e.clientX; sy = e.clientY
      }
    })
    const end = () => {
      if (!mode) return
      mode = null; node = null; svg = null; d.body.style.cursor = ''
      if (moved) { D._suppressClick = true; setTimeout(() => { D._suppressClick = false }, 0) }
    }
    d.addEventListener('mouseup', end)
    d.addEventListener('mouseleave', end)
  }

  function onLoad() { hideTip(); applyZoom(); wire(); syncAllHl() }
</script>

<div class="ws">
  <div class="ws-head">
    <b>模型结构</b>
    <span class="seg" title="结构图粒度">{#each levels as l}<button class:active={level === l.id} onclick={() => (level = l.id)}>{l.label}</button>{/each}</span>
    <span class="seg" title="结构图布局">{#each layouts as l}<button class:active={layout === l.id} onclick={() => (layout = l.id)}>{l.label}</button>{/each}</span>
    <span class="seg" title="缩放（拖背景=平移、拖节点=移动）">
      <button onclick={() => setZoom(zoom / 1.25)}>−</button>
      <button onclick={() => setZoom(1)}>适应</button>
      <button onclick={() => setZoom(zoom * 1.25)}>+</button>
    </span>
    <span class="zlab">{Math.round(zoom * 100)}%</span>
    <span class="tip-note">悬停看注释 · 点选高亮 · 拖背景平移 · 拖节点移动</span>
  </div>
  <div class="frame"><iframe bind:this={iframeEl} title="结构图" {src} onload={onLoad}></iframe></div>
</div>

<style>
  .ws { display: flex; flex-direction: column; height: 100%; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }
  .seg button { border: 0; background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; cursor: pointer; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button.active { background: var(--accent); color: #fff; }
  .zlab { font-size: 12px; color: var(--sub); }
  .tip-note { font-size: 11px; color: var(--sub); margin-left: auto; }
  .frame { flex: 1; border: 1px solid var(--line); border-radius: 8px; overflow: hidden; background: #fff; }
  iframe { width: 100%; height: 100%; border: 0; }
  /* 悬停注释卡挂在 document.body（组件外）→ 全局样式 */
  :global(.eqc-nodetip) {
    position: fixed; z-index: 3000; display: none; pointer-events: none; max-width: 380px;
    background: #fff; border: 1px solid #e5e7eb; border-radius: 8px; box-shadow: 0 8px 28px rgba(0, 0, 0, 0.14);
    padding: 10px 12px; font-size: 13px; color: #1f2933;
  }
  :global(.eqc-nodetip .t-name) { font-weight: 700; font-size: 14px; }
  :global(.eqc-nodetip .t-id) { color: #6b7280; font-size: 11px; margin-top: 1px; font-family: ui-monospace, Consolas, monospace; }
  :global(.eqc-nodetip .t-sub) { color: #6b7280; font-size: 12px; margin-top: 2px; }
  :global(.eqc-nodetip .t-desc) { margin-top: 6px; line-height: 1.45; }
  :global(.eqc-nodetip .t-eq) { margin-top: 8px; overflow-x: auto; }
  :global(.eqc-nodetip .t-eq math) { font-size: 1.15em; }
  :global(.eqc-nodetip .t-cite) { margin-top: 6px; font-size: 12px; color: #1d4ed8; }
  :global(.eqc-nodetip .t-cite.t-none) { color: #6b7280; }
</style>
