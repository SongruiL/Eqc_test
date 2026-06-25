<script lang="ts">
  // v2 应用外壳：顶栏（全局选择器）+ 左导航（工作区）+ 主内容区（当前工作区）。
  // 增量迁移中：studio.html(v1) 仍是默认 `/`；本应用挂 `/v2`。
  import { store, loadModels } from './lib/store.svelte'
  import { ui } from './lib/commands.svelte'
  import TopBar from './components/TopBar.svelte'
  import Nav from './components/Nav.svelte'
  import CommandPalette from './components/CommandPalette.svelte'
  import Structure from './workspaces/Structure.svelte'
  import Simulate from './workspaces/Simulate.svelte'
  import Optimize from './workspaces/Optimize.svelte'
  import Calibrate from './workspaces/Calibrate.svelte'
  import Gp from './workspaces/Gp.svelte'
  import Understand from './workspaces/Understand.svelte'
  import Entry from './workspaces/Entry.svelte'
  import Edit from './workspaces/Edit.svelte'
  import Couple from './workspaces/Couple.svelte'
  import Placeholder from './workspaces/Placeholder.svelte'
  import AgentChat from './components/AgentChat.svelte'

  const LABELS: Record<string, string> = {
    structure: '结构', simulate: '仿真', optimize: '优化', calibrate: '标定',
    gp: '进化', edit: '编辑', understand: '看懂', entry: '录入',
  }

  loadModels()

  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault()
      ui.palette = true
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  <TopBar />
  <div class="body">
    <Nav />
    <main>
      {#if store.workspace === 'structure'}
        <Structure />
      {:else if store.workspace === 'simulate'}
        <Simulate />
      {:else if store.workspace === 'optimize'}
        <Optimize />
      {:else if store.workspace === 'calibrate'}
        <Calibrate />
      {:else if store.workspace === 'gp'}
        <Gp />
      {:else if store.workspace === 'understand'}
        <Understand />
      {:else if store.workspace === 'entry'}
        <Entry />
      {:else if store.workspace === 'edit'}
        <Edit />
      {:else if store.workspace === 'couple'}
        <Couple />
      {:else}
        <Placeholder name={store.workspace} label={LABELS[store.workspace] ?? store.workspace} />
      {/if}
    </main>
  </div>
</div>

<CommandPalette />
<AgentChat />

<style>
  .shell { display: flex; flex-direction: column; height: 100%; }
  .body { display: flex; flex: 1; min-height: 0; }
  main { flex: 1; min-width: 0; overflow: auto; padding: 16px; }
</style>
