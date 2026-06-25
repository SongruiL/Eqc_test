<script lang="ts">
  // 顶栏（全局常驻）：模型选择器 + 处理区 + 园区/专家切换 + 命令面板 + 连接状态。
  import { store, switchModel, setMode } from '../lib/store.svelte'
  import { ui } from '../lib/commands.svelte'
  import { agent } from '../lib/agent.svelte'
</script>

<header>
  <span class="title">EQC Studio <small>v2</small></span>

  {#if store.models.length > 1}
    <label class="field">模型
      <select value={store.model} onchange={(e) => switchModel(e.currentTarget.value)}>
        {#each store.models as m}<option value={m.id}>{m.name}</option>{/each}
      </select>
    </label>
  {/if}

  <label class="field">处理区
    <input class="zone" bind:value={store.zone} title="英文/数字；6 个处理区可用 1..6 或名称" />
  </label>

  <span class="seg">
    <button class:active={store.mode === 'park'} onclick={() => setMode('park')}>园区</button>
    <button class:active={store.mode === 'expert'} onclick={() => setMode('expert')}>专家</button>
  </span>

  <button class="cmdk" title="命令面板（Ctrl/⌘ + K）" onclick={() => (ui.palette = true)}>⌘K 命令</button>
  <button class="ai" class:on={agent.open} title="AI 助手：用自然语言操作前端" onclick={() => (agent.open = !agent.open)}>🤖 问AI</button>
  <span class="status" class:ok={store.connected}>{store.connected ? '● 已连接' : '○ 未连接'}</span>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--line);
    background: #fff;
  }
  .title { font-weight: 700; font-size: 15px; }
  .title small { color: var(--sub); font-weight: 400; font-size: 11px; }
  .field { font-size: 12px; color: var(--sub); display: inline-flex; align-items: center; gap: 6px; }
  select, .zone { font-size: 13px; padding: 3px 7px; border: 1px solid var(--line); border-radius: 6px; color: var(--ink); }
  .zone { width: 80px; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }
  .seg button { border: 0; background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; cursor: pointer; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button.active { background: var(--accent); color: #fff; }
  .cmdk { margin-left: auto; border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 10px; border-radius: 7px; cursor: pointer; }
  .cmdk:hover { background: #eef2ff; }
  .ai { border: 1px solid var(--line); background: #fff; color: var(--ink); font-size: 12px; padding: 3px 10px; border-radius: 7px; cursor: pointer; }
  .ai:hover, .ai.on { background: #f5f3ff; border-color: #ddd6fe; color: #7c3aed; }
  .status { font-size: 12px; color: var(--sub); }
  .status.ok { color: #16a34a; }
</style>
