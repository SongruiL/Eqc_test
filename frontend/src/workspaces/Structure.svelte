<script lang="ts">
  // 结构工作区：EQC 自生成的报告（Forrester 图 + 二维公式）嵌 iframe + 粒度/布局/缩放控制。
  // 缩放：同源 iframe，直接缩放其中的 <svg> 宽度（容器横向滚动平移），与 v1 一致。
  import { store } from '../lib/store.svelte'
  import { reportUrl } from '../lib/api'

  let layout = $state('forrester')
  let level = $state('variable')
  let zoom = $state(1)
  let iframeEl = $state<HTMLIFrameElement>()
  const src = $derived(reportUrl(store.model, layout, level))

  const levels = [
    { id: 'variable', label: '变量' },
    { id: 'equation', label: '方程' },
    { id: 'module', label: '模块' },
  ]
  const layouts = [
    { id: 'forrester', label: 'Forrester' },
    { id: 'force', label: '力导向' },
    { id: 'layered', label: '分层' },
  ]

  function applyZoom() {
    try {
      const doc = iframeEl?.contentDocument
      if (!doc) return
      doc.querySelectorAll('svg').forEach((s) => {
        const el = s as SVGElement
        el.style.width = zoom * 100 + '%'
        el.style.maxWidth = 'none'
        el.style.height = 'auto'
      })
    } catch {
      /* 跨域不可达——此处同源，正常不触发 */
    }
  }
  $effect(() => {
    void zoom // zoom 变即重应用（iframe 重载由 onload 兜）
    applyZoom()
  })
</script>

<div class="ws">
  <div class="ws-head">
    <b>模型结构</b>
    <span class="seg" title="结构图粒度">
      {#each levels as l}<button class:active={level === l.id} onclick={() => (level = l.id)}>{l.label}</button>{/each}
    </span>
    <span class="seg" title="结构图布局">
      {#each layouts as l}<button class:active={layout === l.id} onclick={() => (layout = l.id)}>{l.label}</button>{/each}
    </span>
    <span class="seg" title="缩放（拖动滚动条平移）">
      <button onclick={() => (zoom = Math.max(0.3, +(zoom * 0.8).toFixed(2)))}>−</button>
      <button onclick={() => (zoom = 1)}>适应</button>
      <button onclick={() => (zoom = Math.min(4, +(zoom * 1.25).toFixed(2)))}>+</button>
    </span>
    <span class="zlab">{Math.round(zoom * 100)}%</span>
  </div>
  <div class="frame">
    <iframe bind:this={iframeEl} title="结构图" {src} onload={applyZoom}></iframe>
  </div>
</div>

<style>
  .ws { display: flex; flex-direction: column; height: 100%; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }
  .seg button { border: 0; background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; cursor: pointer; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button.active { background: var(--accent); color: #fff; }
  .zlab { font-size: 12px; color: var(--sub); }
  .frame { flex: 1; border: 1px solid var(--line); border-radius: 8px; overflow: hidden; background: #fff; }
  iframe { width: 100%; height: 100%; border: 0; }
</style>
