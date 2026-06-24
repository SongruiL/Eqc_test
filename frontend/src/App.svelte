<script lang="ts">
  // Spike：从 /api/models + /api/model 拉 live 契约 → 渲染模型名 + 变量列表（响应式筛选）。
  // 演示：① TS 类型对契约 ② 组件 ③ 响应式（$state/$derived）④ 消费 EQC live API。
  import type { ModelsJson, ModelJson } from './contract'

  let models = $state<{ id: string; name: string }[]>([])
  let current = $state('')
  let model = $state<ModelJson | null>(null)
  let filter = $state('')
  let err = $state('')

  async function loadModels() {
    try {
      const j: ModelsJson = await (await fetch('/api/models', { cache: 'no-store' })).json()
      models = j.models ?? []
      current = models[0]?.id ?? ''
      await loadModel()
    } catch (e) {
      err = String(e)
    }
  }
  async function loadModel() {
    try {
      const q = current ? '?model=' + encodeURIComponent(current) : ''
      model = await (await fetch('/api/model' + q, { cache: 'no-store' })).json()
    } catch (e) {
      err = String(e)
    }
  }

  const mod = $derived(model?.modules?.[0])
  const vars = $derived(
    (mod?.variables ?? []).filter(
      (v) => (v.display_name + ' ' + v.name).toLowerCase().includes(filter.toLowerCase())
    )
  )

  loadModels()
</script>

<main>
  <h1>EQC Studio · spike <small>(Vite + Svelte + TS → 单 HTML，仍 include_str! 进 eqc.exe)</small></h1>
  {#if err}<p class="err">加载失败：{err}</p>{/if}
  <div class="bar">
    <label>模型
      <select bind:value={current} onchange={loadModel}>
        {#each models as m}<option value={m.id}>{m.name}</option>{/each}
      </select>
    </label>
    <input placeholder="筛选变量…" bind:value={filter} />
  </div>
  <p class="sub">{mod?.name_cn ?? mod?.model ?? ''} · 显示 {vars.length} 个变量</p>
  <ul>
    {#each vars as v}
      <li>
        <b>{v.display_name}</b>
        <code>{v.name}</code>
        <span class="cls">{v.class ?? ''}</span>
        <span class="unit">{v.unit ?? ''}</span>
      </li>
    {/each}
  </ul>
</main>

<style>
  main { font-family: system-ui, -apple-system, sans-serif; max-width: 760px; margin: 28px auto; color: #1f2933; padding: 0 16px; }
  h1 { font-size: 18px; } small { color: #6b7280; font-weight: 400; font-size: 12px; }
  .bar { display: flex; gap: 12px; align-items: center; margin: 12px 0; }
  select, input { font-size: 13px; padding: 4px 8px; border: 1px solid #e5e7eb; border-radius: 6px; }
  .sub { color: #6b7280; font-size: 13px; }
  ul { list-style: none; padding: 0; margin: 0; }
  li { padding: 5px 0; border-bottom: 1px solid #eef1f4; font-size: 13px; display: flex; gap: 10px; align-items: baseline; }
  code { color: #6b7280; } .cls { color: #2563eb; font-size: 11px; } .unit { color: #9ca3af; margin-left: auto; }
  .err { color: #dc2626; }
</style>
