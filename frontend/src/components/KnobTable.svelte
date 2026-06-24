<script lang="ts">
  // 旋钮/拟合参数表（优化 + 标定共用）。
  import type { Knob } from '../lib/contract'
  import { fmtNum } from '../lib/format'

  let { knobs }: { knobs: Knob[] } = $props()
  const KIND: Record<string, string> = {
    param: '参数', init: '初值', driver_const: '恒定驱动', FastParam: '温室参', SlowParam: '作物参',
  }
</script>

<table class="tbl">
  <tbody>
    <tr><th>旋钮</th><th>值</th><th>种类</th><th>边界</th></tr>
    {#each knobs as k}
      <tr>
        <td>{k.var}</td>
        <td>{fmtNum(k.value)}{k.unit ? ' ' + k.unit : ''}</td>
        <td>{KIND[k.kind ?? ''] ?? k.kind ?? ''}</td>
        <td>{k.bounds ? `[${fmtNum(k.bounds[0])}, ${fmtNum(k.bounds[1])}]` : ''}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .tbl { width: 100%; border-collapse: collapse; font-size: 12px; margin-top: 8px; }
  .tbl th, .tbl td { text-align: left; padding: 3px 8px; border-bottom: 1px solid var(--line); }
  .tbl th { color: var(--sub); font-weight: 600; }
</style>
