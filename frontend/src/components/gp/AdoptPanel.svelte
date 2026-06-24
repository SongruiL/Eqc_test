<script lang="ts">
  // 采纳区（人在环裁决 · 只产文本、不写盘）：溯源草稿（可编辑分类）+ 新方程文本，复制/下载。
  import { copyText, downloadText } from '../../lib/download'

  let { stub = '', yaml = '', name = 'gp' }: { stub?: string; yaml?: string; name?: string } = $props()

  let stubText = $state('')
  let yamlText = $state('')
  let note = $state('')

  // 同步并在切候选（传入 stub/yaml 变）时重置编辑框
  $effect(() => {
    stubText = stub
    yamlText = yaml
  })

  async function copy(t: string) {
    try {
      await copyText(t)
      note = '已复制到剪贴板 ✓'
    } catch {
      note = '复制失败'
    }
  }
  function dl(n: string, t: string, mime: string) {
    downloadText(n, t, mime)
    note = '已下载 ' + n
  }
</script>

<div class="adopt">
  <div class="sub">采纳（人在环裁决 · 只产文本、不写盘）</div>
  <div class="hint">复核/编辑分类后复制或下载。EQC 持有模型，是否写回由你决定（v1 不自动改盘上模型）。</div>

  <div class="lab">溯源条目草稿（可编辑分类 🟢/🔵/🟠）</div>
  <textarea class="ta" rows="12" bind:value={stubText}></textarea>
  <div class="btns">
    <button class="btn" onclick={() => copy(stubText)}>复制</button>
    <button class="btn" onclick={() => dl(name + '_provenance.md', stubText, 'text/markdown')}>下载 .md</button>
  </div>

  <div class="lab">新方程文本（.eq.yaml 片段，常数已代回字面值）</div>
  <textarea class="ta" rows="6" bind:value={yamlText}></textarea>
  <div class="btns">
    <button class="btn" onclick={() => copy(yamlText)}>复制</button>
    <button class="btn" onclick={() => dl(name + '.eq.yaml', yamlText, 'text/yaml')}>下载 .yaml</button>
  </div>

  {#if note}<div class="note">{note}</div>{/if}
</div>

<style>
  .adopt { margin-top: 10px; }
  .sub { font-size: 12px; font-weight: 600; color: var(--sub); }
  .hint { color: var(--sub); font-size: 12px; margin-top: 4px; }
  .lab { font-size: 12px; font-weight: 600; color: var(--ink); margin-top: 8px; }
  .ta { width: 100%; box-sizing: border-box; font-family: ui-monospace, Consolas, monospace; font-size: 12px;
    border: 1px solid var(--line); border-radius: 6px; padding: 6px; margin-top: 4px; resize: vertical; color: var(--ink); }
  .btns { margin-top: 4px; display: flex; gap: 6px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 3px 11px; border-radius: 7px; cursor: pointer; }
  .btn:hover { background: #eef2ff; }
  .note { font-size: 12px; color: #16a34a; margin-top: 6px; }
</style>
