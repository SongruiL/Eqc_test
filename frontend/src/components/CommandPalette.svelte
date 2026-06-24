<script lang="ts">
  // ⌘K/Ctrl+K 命令面板：搜命令 → 上下键选 → 回车执行。命令来自命令注册表（Agent 将来同源）。
  import { ui, allCommands, type Command } from '../lib/commands.svelte'

  let q = $state('')
  let sel = $state(0)
  let inputEl = $state<HTMLInputElement>()

  const filtered = $derived.by(() => {
    const s = q.trim().toLowerCase()
    const list = allCommands()
    if (!s) return list
    return list.filter((c) => (c.label + ' ' + (c.keywords ?? '') + ' ' + c.id).toLowerCase().includes(s))
  })

  // 打开时聚焦输入框 + 复位
  $effect(() => {
    if (ui.palette) {
      q = ''
      sel = 0
      queueMicrotask(() => inputEl?.focus())
    }
  })
  $effect(() => { void q; sel = 0 }) // 改查询复位选择

  function run(c: Command) { c.run(); close() }
  function close() { ui.palette = false }
  function onKey(e: KeyboardEvent) {
    const f = filtered
    if (e.key === 'Escape') { close(); e.preventDefault() }
    else if (e.key === 'ArrowDown') { sel = Math.min(f.length - 1, sel + 1); e.preventDefault() }
    else if (e.key === 'ArrowUp') { sel = Math.max(0, sel - 1); e.preventDefault() }
    else if (e.key === 'Enter') { if (f[sel]) run(f[sel]); e.preventDefault() }
  }
</script>

{#if ui.palette}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="overlay" role="presentation" onclick={close}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="palette" role="presentation" onclick={(e) => e.stopPropagation()}>
      <input class="q" placeholder="输入命令…（↑↓ 选择，回车执行，Esc 关闭）" bind:value={q} bind:this={inputEl} onkeydown={onKey} />
      <div class="list">
        {#each filtered as c, i}
          <button class="row" class:sel={i === sel} onclick={() => run(c)} onmouseenter={() => (sel = i)}>
            <span class="lab">{c.label}</span><span class="grp">{c.group}</span>
          </button>
        {/each}
        {#if !filtered.length}<div class="empty">无匹配命令</div>{/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; z-index: 4000; background: rgba(15, 23, 42, 0.28); display: flex; justify-content: center; align-items: flex-start; padding-top: 12vh; }
  .palette { width: 520px; max-width: 92vw; background: #fff; border: 1px solid var(--line); border-radius: 12px; box-shadow: 0 18px 50px rgba(0, 0, 0, 0.25); overflow: hidden; }
  .q { width: 100%; box-sizing: border-box; border: 0; border-bottom: 1px solid var(--line); padding: 12px 14px; font-size: 15px; outline: none; }
  .list { max-height: 50vh; overflow: auto; padding: 4px; }
  .row { width: 100%; display: flex; align-items: center; justify-content: space-between; border: 0; background: transparent; padding: 8px 10px; border-radius: 7px; cursor: pointer; font-size: 14px; color: var(--ink); }
  .row.sel { background: #eff4ff; }
  .lab { font-weight: 500; }
  .grp { font-size: 11px; color: var(--sub); }
  .empty { padding: 14px; color: var(--sub); font-size: 13px; text-align: center; }
</style>
