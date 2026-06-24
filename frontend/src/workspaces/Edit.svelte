<script lang="ts">
  // 模型编辑器（C1）：CodeMirror 编辑 .eq.yaml → 实时校验 + 结构预览 + 受控保存（校验通过+备份+确认才写盘）。
  // EQC 持有校验/渲染/写盘（/api/validate、/api/source POST），前端只递交文本 + 显示 + 裁决。
  import { onMount } from 'svelte'
  import { basicSetup } from 'codemirror'
  import { EditorView } from '@codemirror/view'
  import { yaml } from '@codemirror/lang-yaml'
  import { store } from '../lib/store.svelte'
  import { fetchSource, validateSource, saveSource } from '../lib/api'
  import { downloadText } from '../lib/download'

  let editorEl: HTMLDivElement
  let view: EditorView | undefined
  let editable = $state(true)
  let srcError = $state('')
  let ok = $state(true)
  let errors = $state<string[]>([])
  let preview = $state('') // report_html
  let valStatus = $state('')
  let path = $state('')
  let original = $state('') // 加载时的原文（算 diff / 脏标记）
  let dirty = $state(false)
  let confirmSave = $state(false)
  let saveStatus = $state('')
  let timer: ReturnType<typeof setTimeout> | undefined
  let lastModel = ''

  function onDocChange() {
    dirty = (view?.state.doc.toString() ?? '') !== original
    scheduleValidate()
  }

  onMount(() => {
    view = new EditorView({
      doc: '',
      extensions: [basicSetup, yaml(), EditorView.updateListener.of((u) => { if (u.docChanged) onDocChange() })],
      parent: editorEl,
    })
    lastModel = store.model
    void loadSource()
    return () => view?.destroy()
  })

  // 切模型 → 重载源码
  $effect(() => {
    if (view && store.model !== lastModel) {
      lastModel = store.model
      void loadSource()
    }
  })

  async function loadSource() {
    valStatus = ''; errors = []; preview = ''; ok = true; srcError = ''; saveStatus = ''; confirmSave = false
    const j = await fetchSource(store.model)
    path = j.path ?? ''
    editable = j.editable ?? false
    if (!editable) { srcError = j.error ?? '该模型不可编辑'; original = ''; dirty = false; setDoc(''); return }
    original = j.source ?? ''
    dirty = false
    setDoc(original)
    scheduleValidate()
  }
  function setDoc(text: string) {
    view?.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } })
  }
  function scheduleValidate() {
    clearTimeout(timer)
    timer = setTimeout(validate, 450)
  }
  async function validate() {
    if (!view || !editable) return
    valStatus = '校验中…'
    try {
      const j = await validateSource(store.model, view.state.doc.toString())
      ok = !!j.ok
      errors = j.errors ?? (j.error ? [j.error] : [])
      if (j.ok && j.report_html) preview = j.report_html
      valStatus = j.ok ? '✓ 校验通过' : '✗ 有错误'
    } catch (e) {
      valStatus = '校验请求失败：' + e
    }
  }
  function download() {
    if (!view) return
    const name = path.split(/[\\/]/).pop() || 'model.eq.yaml'
    downloadText(name, view.state.doc.toString(), 'text/yaml')
  }

  // —— 受控保存：紧凑行级 diff（公共前后缀外的改动块）——
  function lineDiff(a: string, b: string) {
    const al = a.split('\n'), bl = b.split('\n')
    let p = 0
    while (p < al.length && p < bl.length && al[p] === bl[p]) p++
    let s = 0
    while (s < al.length - p && s < bl.length - p && al[al.length - 1 - s] === bl[bl.length - 1 - s]) s++
    return { removed: al.slice(p, al.length - s), added: bl.slice(p, bl.length - s), atLine: p + 1 }
  }
  const diff = $derived(confirmSave && view ? lineDiff(original, view.state.doc.toString()) : null)
  async function doSave() {
    if (!view) return
    saveStatus = '⏳ 保存中…'
    const text = view.state.doc.toString()
    try {
      const j = await saveSource(store.model, text)
      if (j.error) { saveStatus = '保存失败：' + j.error; return }
      original = text; dirty = false; confirmSave = false
      saveStatus = '✅ 已保存到 ' + (j.path ?? '') + '（备份 ' + (j.backup ?? '') + '）'
    } catch (e) {
      saveStatus = '保存请求失败：' + e
    }
  }
</script>

<div class="ws">
  <div class="ws-head">
    <b>模型编辑器</b> <span class="sub" title={path}>{path.split(/[\\/]/).pop() ?? ''}</span>
    <span class="status" class:ok class:bad={!ok && !!valStatus}>{valStatus}</span>
    {#if dirty}<span class="dirty">● 未保存改动</span>{/if}
    <button class="btn" disabled={!editable} onclick={download}>下载</button>
    <button class="btn on" disabled={!editable || !ok || !dirty} onclick={() => (confirmSave = true)}>保存到文件…</button>
  </div>
  <div class="note">编辑 YAML → 实时校验 + 结构预览。保存须<b>校验通过</b>，写盘前自动<b>备份</b>到 <code>.bak</code> 并要你确认（git 仍是真正的版本史）。</div>
  {#if saveStatus}<div class="hint" class:ok-msg={saveStatus.startsWith('✅')}>{saveStatus}</div>{/if}
  {#if srcError}<div class="hint err">{srcError}</div>{/if}

  {#if confirmSave && diff}
    <div class="confirm">
      <div class="c-head"><b>确认写回模型文件？</b> <span class="sub">{path}</span></div>
      <div class="c-diff">改动（约第 {diff.atLine} 行 · −{diff.removed.length} / +{diff.added.length}）：
        <div class="d-body">
          {#each diff.removed.slice(0, 12) as l}<div class="d-rm">- {l}</div>{/each}
          {#each diff.added.slice(0, 12) as l}<div class="d-add">+ {l}</div>{/each}
          {#if diff.removed.length + diff.added.length > 24}<div class="sub">…（仅显示前若干行）</div>{/if}
        </div>
      </div>
      <div class="c-act">
        <button class="btn on" onclick={doSave}>确认写回（自动备份）</button>
        <button class="btn" onclick={() => (confirmSave = false)}>取消</button>
      </div>
    </div>
  {/if}

  <div class="grid">
    <div class="ed" bind:this={editorEl}></div>
    <div class="prev">
      {#if errors.length}
        <div class="errs"><b>校验错误</b>{#each errors as e}<div class="e">• {e}</div>{/each}</div>
      {/if}
      {#if preview}
        <iframe class="pv" title="结构预览" srcdoc={preview}></iframe>
      {:else if !errors.length}
        <div class="hint">（结构预览将在校验通过后显示）</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .ws { display: flex; flex-direction: column; height: 100%; }
  .ws-head { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
  .sub { color: var(--sub); font-size: 12px; }
  .status { font-size: 12px; color: var(--sub); }
  .status.ok { color: #16a34a; } .status.bad { color: #dc2626; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; }
  .ws-head .btn:first-of-type { margin-left: auto; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .dirty { font-size: 12px; color: #d97706; }
  .note { color: var(--sub); font-size: 12px; margin-bottom: 8px; }
  .note code { background: #f3f4f6; padding: 0 4px; border-radius: 3px; }
  .hint { color: var(--sub); font-size: 12px; padding: 6px 0; }
  .hint.ok-msg { color: #16a34a; }
  .err { color: #dc2626; }
  .confirm { border: 1px solid #fde68a; background: #fffbeb; border-radius: 8px; padding: 10px 12px; margin-bottom: 10px; }
  .c-head { font-size: 13px; } .c-head .sub { color: var(--sub); font-size: 12px; }
  .c-diff { font-size: 12px; color: var(--sub); margin-top: 6px; }
  .d-body { margin-top: 4px; font-family: ui-monospace, Consolas, monospace; max-height: 200px; overflow: auto; background: #fff; border: 1px solid var(--line); border-radius: 6px; padding: 6px; }
  .d-rm { color: #b91c1c; background: #fef2f2; white-space: pre-wrap; }
  .d-add { color: #15803d; background: #f0fdf4; white-space: pre-wrap; }
  .c-act { margin-top: 8px; display: flex; gap: 8px; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; flex: 1; min-height: 0; }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  .ed { border: 1px solid var(--line); border-radius: 8px; overflow: auto; background: #fff; min-height: 0; }
  .ed :global(.cm-editor) { height: 100%; font-size: 13px; }
  .ed :global(.cm-scroller) { font-family: ui-monospace, Consolas, monospace; }
  .prev { display: flex; flex-direction: column; min-height: 0; gap: 8px; }
  .errs { border: 1px solid #fecaca; background: #fef2f2; border-radius: 8px; padding: 8px 10px; font-size: 12px; color: #991b1b; }
  .errs .e { margin-top: 4px; font-family: ui-monospace, Consolas, monospace; }
  .pv { flex: 1; width: 100%; border: 1px solid var(--line); border-radius: 8px; background: #fff; min-height: 240px; }
</style>
