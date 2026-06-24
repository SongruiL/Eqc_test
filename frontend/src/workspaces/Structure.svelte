<script lang="ts">
  // 结构工作区：复用 EQC 自生成的报告（Forrester 图 + 二维公式），嵌进 iframe。
  import { store } from '../lib/store.svelte'
  import { reportUrl } from '../lib/api'

  let layout = $state('forrester')
  const src = $derived(reportUrl(store.model, layout))
  const layouts = [
    { id: 'forrester', label: 'Forrester' },
    { id: 'force', label: '力导向' },
    { id: 'layered', label: '分层' },
  ]
</script>

<div class="ws">
  <div class="ws-head">
    <b>模型结构</b>
    <span class="seg">
      {#each layouts as l}
        <button class:active={layout === l.id} onclick={() => (layout = l.id)}>{l.label}</button>
      {/each}
    </span>
  </div>
  <iframe title="结构图" {src}></iframe>
</div>

<style>
  .ws { display: flex; flex-direction: column; height: 100%; }
  .ws-head { display: flex; align-items: center; gap: 12px; margin-bottom: 10px; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }
  .seg button { border: 0; background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; cursor: pointer; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button.active { background: var(--accent); color: #fff; }
  iframe { flex: 1; width: 100%; border: 1px solid var(--line); border-radius: 8px; background: #fff; }
</style>
