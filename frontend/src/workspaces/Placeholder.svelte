<script lang="ts">
  // 尚未搬迁的工作区占位（P2..Pn 逐个做实）。
  let { name, label }: { name: string; label: string } = $props()
</script>

<div class="ph">
  <h2>{label}</h2>
  <p>这个工作区（<code>{name}</code>）还在 v1（<a href="/" target="_blank">旧版 Studio</a>）里——前端 v2 迁移中，将逐个搬过来（见 <code>docs/spec-studio-frontend-v2.md</code> P2..Pn）。</p>
</div>

<style>
  .ph { max-width: 560px; margin: 40px auto; text-align: center; color: var(--sub); }
  h2 { color: var(--ink); }
  code { background: #f3f4f6; padding: 1px 5px; border-radius: 4px; }
</style>
