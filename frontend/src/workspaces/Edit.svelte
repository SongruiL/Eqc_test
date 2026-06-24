<script lang="ts">
  // 模型编辑器（C1）：CodeMirror 编辑 .eq.yaml → 实时校验 + 结构预览。v1 不写盘（编辑浏览器副本）。
  // EQC 持有校验/渲染（/api/validate 返 ok/errors/结构图 HTML），前端只递交文本 + 显示。
  import { onMount } from 'svelte'
  import { basicSetup } from 'codemirror'
  import { EditorView } from '@codemirror/view'
  import { yaml } from '@codemirror/lang-yaml'
  import { store } from '../lib/store.svelte'
  import { fetchSource, validateSource } from '../lib/api'
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
  let timer: ReturnType<typeof setTimeout> | undefined
  let lastModel = ''

  onMount(() => {
    view = new EditorView({
      doc: '',
      extensions: [basicSetup, yaml(), EditorView.updateListener.of((u) => { if (u.docChanged) scheduleValidate() })],
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
    valStatus = ''; errors = []; preview = ''; ok = true; srcError = ''
    const j = await fetchSource(store.model)
    path = j.path ?? ''
    editable = j.editable ?? false
    if (!editable) { srcError = j.error ?? '该模型不可编辑'; setDoc(''); return }
    setDoc(j.source ?? '')
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
</script>

<div class="ws">
  <div class="ws-head">
    <b>模型编辑器</b> <span class="sub" title={path}>{path.split(/[\\/]/).pop() ?? ''}</span>
    <span class="status" class:ok class:bad={!ok && !!valStatus}>{valStatus}</span>
    <button class="btn" disabled={!editable} onclick={download}>下载 .eq.yaml</button>
  </div>
  <div class="note">编辑 YAML → 实时校验 + 结构预览。<b>不写盘</b>——满意了下载替换（EQC 持有模型、人决定）。</div>
  {#if srcError}<div class="hint err">{srcError}</div>{/if}

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
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; margin-left: auto; }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .note { color: var(--sub); font-size: 12px; margin-bottom: 8px; }
  .hint { color: var(--sub); font-size: 12px; padding: 12px; }
  .err { color: #dc2626; }
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
