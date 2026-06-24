// 数值显示：大/极小用科学计数，否则 4 位小数；空值 → 「—」。
export function fmtNum(x?: number | null): string {
  if (x == null) return '—'
  if (!Number.isFinite(x)) return String(x)
  return Math.abs(x) >= 1000 || (Math.abs(x) < 0.001 && x !== 0) ? x.toExponential(3) : x.toFixed(4)
}
