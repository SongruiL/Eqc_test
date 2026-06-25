<script lang="ts">
  // AI 抽屉：对话 + 工具调用卡 + confirm 闸 + 运行态。右侧滑出，消费 lib/agent。
  import { agent, sendMessage, clearConvo, resolveConfirm, type Block, type Msg } from '../lib/agent.svelte'
  import { aiCommands } from '../lib/commands.svelte'

  let input = $state('')

  // 工具名 → 友好标签（卡片显示用）。
  const labelOf = (name: string) => aiCommands().find((c) => c.id.replace(/[^a-zA-Z0-9_-]/g, '_') === name)?.label ?? name

  function blocksOf(m: Msg): Block[] {
    return typeof m.content === 'string' ? [{ type: 'text', text: m.content }] : m.content
  }
  const trunc = (s: string, n = 280) => (s.length > n ? s.slice(0, n) + '…' : s)

  function submit() {
    const t = input.trim()
    if (!t || agent.running) return
    input = ''
    sendMessage(t)
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      submit()
    }
  }
</script>

{#if agent.open}
  <aside class="drawer">
    <header>
      <span>🤖 EQC 助手</span>
      <span class="sp"></span>
      <button class="ghost" onclick={clearConvo} disabled={agent.running} title="清空对话">清空</button>
      <button class="ghost" onclick={() => (agent.open = false)} title="关闭">✕</button>
    </header>

    <div class="log">
      {#if agent.convo.length === 0}
        <p class="hint">用自然语言指挥前端。例如：<br />「把草莓的 LUE 调到 5 看看产量」<br />「LAI 是什么？画出它的整季曲线」<br />「跑一次仿真，末了产量多少」</p>
      {/if}

      {#each agent.convo as m}
        {#each blocksOf(m) as b}
          {#if b.type === 'text' && b.text.trim()}
            <div class="msg {m.role}">{b.text}</div>
          {:else if b.type === 'tool_use'}
            <div class="tool">🔧 {labelOf(b.name)}{Object.keys(b.input).length ? ' · ' + JSON.stringify(b.input) : ''}</div>
          {:else if b.type === 'tool_result'}
            <div class="tres" class:err={b.is_error}>↳ {trunc(b.content)}</div>
          {/if}
        {/each}
      {/each}

      {#if agent.running}<div class="spin">思考/执行中…</div>{/if}
      {#if agent.error}<div class="errbar">⚠ {agent.error}</div>{/if}
    </div>

    {#if agent.pending}
      <div class="confirm">
        <div class="ctext">将执行落盘操作：<b>{agent.pending.summary}</b></div>
        <div class="cbtn">
          <button class="ok" onclick={() => resolveConfirm(true)}>允许</button>
          <button class="no" onclick={() => resolveConfirm(false)}>取消</button>
        </div>
      </div>
    {/if}

    <div class="composer">
      <textarea
        bind:value={input}
        onkeydown={onKey}
        placeholder={agent.running ? '执行中…' : '问我，或让我操作前端（Enter 发送 / Shift+Enter 换行）'}
        disabled={agent.running}
        rows="2"
      ></textarea>
      <button class="send" onclick={submit} disabled={agent.running || !input.trim()}>发送</button>
    </div>
  </aside>
{/if}

<style>
  .drawer {
    position: fixed; top: 0; right: 0; bottom: 0; width: 380px; max-width: 90vw;
    background: #fff; border-left: 1px solid var(--line); box-shadow: -8px 0 24px rgba(0, 0, 0, 0.06);
    display: flex; flex-direction: column; z-index: 50;
  }
  header { display: flex; align-items: center; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--line); font-weight: 600; font-size: 14px; }
  .sp { flex: 1; }
  .ghost { border: 0; background: transparent; color: var(--sub); cursor: pointer; font-size: 13px; }
  .ghost:hover { color: var(--ink); }
  .log { flex: 1; overflow: auto; padding: 12px; display: flex; flex-direction: column; gap: 8px; }
  .hint { color: var(--sub); font-size: 12.5px; line-height: 1.7; background: #f8fafc; border: 1px solid var(--line); border-radius: 8px; padding: 10px; }
  .msg { padding: 8px 11px; border-radius: 10px; font-size: 13.5px; line-height: 1.55; white-space: pre-wrap; max-width: 92%; }
  .msg.user { align-self: flex-end; background: var(--accent); color: #fff; }
  .msg.assistant { align-self: flex-start; background: #f1f5f9; color: var(--ink); }
  .tool { align-self: flex-start; font-size: 12px; color: #7c3aed; background: #f5f3ff; border: 1px solid #ddd6fe; border-radius: 7px; padding: 4px 8px; }
  .tres { align-self: flex-start; font-size: 12px; color: var(--sub); padding: 2px 8px; white-space: pre-wrap; }
  .tres.err { color: #dc2626; }
  .spin { align-self: flex-start; font-size: 12px; color: var(--sub); }
  .errbar { font-size: 12.5px; color: #dc2626; background: #fef2f2; border: 1px solid #fecaca; border-radius: 7px; padding: 6px 9px; }
  .confirm { border-top: 1px solid var(--line); background: #fffbeb; padding: 10px 12px; }
  .ctext { font-size: 13px; margin-bottom: 8px; }
  .cbtn { display: flex; gap: 8px; }
  .cbtn button { border: 0; border-radius: 7px; padding: 5px 14px; font-size: 13px; cursor: pointer; }
  .cbtn .ok { background: #16a34a; color: #fff; }
  .cbtn .no { background: #e5e7eb; color: var(--ink); }
  .composer { border-top: 1px solid var(--line); padding: 10px; display: flex; gap: 8px; align-items: flex-end; }
  textarea { flex: 1; resize: none; font-size: 13.5px; padding: 7px 9px; border: 1px solid var(--line); border-radius: 8px; font-family: inherit; color: var(--ink); }
  .send { border: 0; background: var(--accent); color: #fff; border-radius: 8px; padding: 8px 14px; font-size: 13.5px; cursor: pointer; }
  .send:disabled { background: #cbd5e1; cursor: default; }
</style>
