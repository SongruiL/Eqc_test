<script lang="ts">
  // 左导航 = 工作区列表（按模式给不同项）。点击切换主工作区。
  import { store, setWorkspace } from '../lib/store.svelte'

  const EXPERT = [
    { id: 'structure', label: '结构' },
    { id: 'simulate', label: '仿真' },
    { id: 'optimize', label: '优化' },
    { id: 'calibrate', label: '标定' },
    { id: 'gp', label: '进化' },
    { id: 'evolution', label: '进化史' },
    { id: 'edit', label: '编辑' },
  ]
  const PARK = [
    { id: 'understand', label: '看懂' },
    { id: 'entry', label: '录入' },
    { id: 'calibrate', label: '标定' },
  ]
  // 耦合视图条目：导航换成 结构（+ 可仿真时 耦合仿真）；否则按模式给专家/园区。
  const entry = $derived(store.models.find((m) => m.id === store.model))
  const coupledItems = $derived([
    { id: 'structure', label: '结构' },
    ...(entry?.sim_capable ? [{ id: 'couple', label: '耦合仿真' }] : []),
  ])
  const items = $derived(entry?.coupled ? coupledItems : store.mode === 'park' ? PARK : EXPERT)
</script>

<nav>
  {#each items as it}
    <button class:active={store.workspace === it.id} onclick={() => setWorkspace(it.id)}>{it.label}</button>
  {/each}
</nav>

<style>
  nav {
    width: 96px;
    flex: none;
    border-right: 1px solid var(--line);
    background: #fff;
    padding: 8px 0;
    display: flex;
    flex-direction: column;
  }
  nav button {
    border: 0;
    background: transparent;
    color: var(--sub);
    text-align: left;
    padding: 9px 16px;
    font-size: 14px;
    cursor: pointer;
    border-left: 3px solid transparent;
  }
  nav button:hover { background: #f3f4f6; }
  nav button.active { color: var(--accent); border-left-color: var(--accent); font-weight: 600; background: #eff4ff; }
</style>
