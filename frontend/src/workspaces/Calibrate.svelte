<script lang="ts">
  // 标定工作区：填 calib spec → 用当前处理区录入数据反推参数 → 拟合参数 + 收敛曲线。
  import { store } from '../lib/store.svelte'
  import { runCalibrate } from '../lib/api'
  import type { OptResult } from '../lib/contract'
  import { fmtNum } from '../lib/format'
  import KnobTable from '../components/KnobTable.svelte'

  let spec = $state('')
  let running = $state(false)
  let status = $state('')
  let res = $state<OptResult | null>(null)

  async function run() {
    const s = spec.trim()
    if (!s) { status = '请填标定 spec 路径（相对模型目录）。'; return }
    running = true
    status = '⏳ 标定中…（DE 搜索，可能数十秒；release 更快）'
    res = null
    try {
      const j = await runCalibrate(store.model, s, store.zone)
      if (j.error) { status = '标定失败：' + j.error; return }
      res = j
      status = `✅ 标定完成（处理区 ${store.zone}）`
    } catch (e) {
      status = '请求失败：' + e
    } finally {
      running = false
    }
  }
</script>

<div class="ws">
  <div class="ws-head">
    <b>参数标定</b>
    <input class="spec" placeholder="calib.yaml（相对模型目录）" bind:value={spec} />
    <button class="btn on" disabled={running} onclick={run}>用本区数据标定</button>
  </div>
  <div class="hint">用<b>当前处理区（{store.zone}）</b>录入的实测数据反推参数（旋钮=参数、目标=预测 vs 实测误差）。spec 见 <code>docs/spec-calibration.md</code>。</div>
  {#if status}<div class="status">{status}</div>{/if}

  {#if res}
    <div class="grid">
      <div>
        <div class="sub">拟合参数（{res.n_obs ?? 0} 观测点）· 误差 {fmtNum(res.objective_value)}</div>
        <KnobTable knobs={res.best_knobs ?? []} />
      </div>
      <div class="conv">{@html res.convergence_svg ?? ''}</div>
    </div>
  {/if}
</div>

<style>
  .ws { display: flex; flex-direction: column; }
  .ws-head { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 8px; }
  .spec { font-size: 13px; padding: 4px 8px; border: 1px solid var(--line); border-radius: 6px; min-width: 280px; }
  .btn { border: 1px solid var(--line); background: #fff; color: var(--sub); font-size: 12px; padding: 4px 12px; border-radius: 7px; cursor: pointer; }
  .btn.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .hint { color: var(--sub); font-size: 12px; margin: 4px 0; }
  .status { font-size: 13px; margin: 8px 0; }
  .sub { color: var(--sub); font-size: 12px; font-weight: 600; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; align-items: start; margin-top: 8px; }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  .conv :global(svg) { width: 100%; max-width: 720px; height: auto; }
</style>
