// 2D Forrester 报告的「逐章揭示」——单一真相源，子系统生长(Structure) + 版本演化回放(Evolution) 共用。
// 报告 SVG 由 EQC 生成：节点带 [data-var](本地名) + data-id(全id)；边 .fedge/.edge 带 data-from/data-to(全id)。
// 揭示语义：节点在「首次出现章节 ≤ 当前章节」时显形；**不在任何章节的节点 = 基线(默认第0章)、一开始就显**
// （对子系统生长——所有节点都在某章——default-0 从不触发，行为等同旧「并集」；对版本演化——基线节点未列——正好显）。

/** 从章节列表建「本地名 → 首次出现章节」映射。 */
export function firstChapterMap(chapters: { nodes: string[] }[]): Map<string, number> {
  const m = new Map<string, number>()
  chapters.forEach((ch, i) => ch.nodes.forEach((n) => { if (!m.has(n)) m.set(n, i) }))
  return m
}

/** 伸进 iframe 文档：按 firstChapter + 当前章节逐节点/边显形（opacity 0.5s 过渡）。边=两端都已显形才出现。 */
export function applyReveal2d(doc: Document, firstCh: Map<string, number>, chapter: number): void {
  const shownIds = new Set<string>()
  doc.querySelectorAll('[data-var]').forEach((node) => {
    const el = node as HTMLElement
    const dv = el.getAttribute('data-var') || ''
    const on = (firstCh.get(dv) ?? 0) <= chapter // 不在 map=基线=0
    el.style.transition = 'opacity 0.5s ease'
    el.style.opacity = on ? '1' : '0'
    if (on) { const id = el.getAttribute('data-id'); if (id) shownIds.add(id) }
  })
  doc.querySelectorAll('.fedge, .edge').forEach((edge) => {
    const el = edge as HTMLElement
    const on = shownIds.has(el.getAttribute('data-from') || '') && shownIds.has(el.getAttribute('data-to') || '')
    el.style.transition = 'opacity 0.5s ease'
    el.style.opacity = on ? '1' : '0'
  })
}

/** 复原：清掉内联 opacity/transition，回报告默认全显。 */
export function clearReveal2d(doc: Document): void {
  doc.querySelectorAll('[data-var], .fedge, .edge').forEach((n) => {
    const e = n as HTMLElement
    e.style.opacity = ''
    e.style.transition = ''
  })
}
