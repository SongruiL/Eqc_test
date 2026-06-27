<script lang="ts">
  // 3D 拓扑视图（GA-6）：three.js 渲染 GA-5 力导向坐标 + GA-3 指标。
  // 节点球（size∝介数、色=Forrester 类）+ 影响边 + 轨道（转/缩/平）+ hover 注释 + 点选联动 store.selectedVars。
  // 坐标 Rust 算（/api/layout3d）、前端只渲染——守单一真相源。坐标确定性，无 RNG。
  import { onMount, onDestroy } from 'svelte'
  import * as THREE from 'three'
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
  import { store } from '../lib/store.svelte'
  import { fetchLayout3d } from '../lib/api'
  import type { ModelJson, Layout3dJson } from '../lib/contract'
  import {
    tipHtml, nodeColor3d, localName, makeTip, showTipAt, hideTip,
    classOf, classColor3d, CLS_CN, CLASS_LEGEND, CLASS_ORDER,
    moduleColorMap, MODULE_OTHER_COLOR, MODULE_OTHER_LABEL,
  } from '../lib/annotate'

  // GP「看它长出什么」彩蛋（GA-6b Phase 3）：把候选相对现有模型的结构 diff 当 3D 生长动画播。
  // 受约束 GP 不长新节点 → 主要是 added 边（绿"新枝"从源点伸到目标）+ changed 方程节点脉冲。
  // 渲染的是 before（现有）模型的 layout（所有节点都在，含被新边接上的输入），新边叠加其上。
  export type GpDiffView = {
    addedEdges: [string, string][]   // 本地名对：要"长出"的新边（绿）
    pulseOutputs: string[]           // 本地名：形式变了的方程输出（节点脉冲）
    phase: number                    // 0=现有结构（新边藏）/ 1=长出（新边伸出 + 脉冲）
    nonce: number                    // 变化即触发重播（父组件每次"再播"自增）
  }
  type Props = { contract: ModelJson | null; gpDiff?: GpDiffView | null }
  let { contract, gpDiff = null }: Props = $props()

  let host: HTMLDivElement
  let tip: HTMLDivElement | undefined
  let status = $state<'loading' | 'ok' | 'empty' | 'error'>('loading')
  let errMsg = $state('')
  // 图例 / 配色（GA-6）：当前模型实际出现的类别 / 子系统（只列出现的项）。
  let presentClasses = $state<string[]>([])
  let presentModules = $state<string[]>([])
  let hasOther = $state(false)          // 有「其他」（参数/驱动/未分组）节点
  let legendOpen = $state(true)         // 图例卡折叠态
  let modColor = new Map<string, string>() // 子系统名 → 颜色（首现定色，节点+图例共用）

  // three 对象（普通变量，非响应式）
  let renderer: THREE.WebGLRenderer | null = null
  let scene: THREE.Scene | null = null
  let camera: THREE.PerspectiveCamera | null = null
  let controls: OrbitControls | null = null
  let group: THREE.Group | null = null      // 当前图（节点+边），重建时整体替换
  let nodeMeshes: THREE.Mesh[] = []         // 拾取 / 选中 / 释放用
  let raycaster: THREE.Raycaster | null = null
  let ro: ResizeObserver | null = null
  let hovered: THREE.Mesh | null = null
  let curModel = ''
  // 生长动画（GA-6b）：边列表 + 节点位置 + 章节映射 + 补间帧（逐章把节点显形）。
  let nodePos = new Map<string, THREE.Vector3>()   // 节点 id → 位置（重建边用）
  let edgeList: [string, string][] = []            // 全部边（节点 id 对），按揭示集重建
  let edgeMesh: THREE.LineSegments | null = null   // 边对象引用（重建几何）
  let growthCh = new Map<string, number>()         // 本地名 → 章节序（store.growth.plan）
  let growthRAF = 0                                // 补间 requestAnimationFrame 句柄
  // GP 彩蛋：本地名→位置（新边端点查位）+ 绿新边对象 + 生长/脉冲补间句柄。
  let localPos = new Map<string, THREE.Vector3>()
  let gpEdgeMesh: THREE.LineSegments | null = null
  let gpRAF = 0
  const GP_EDGE = 0x22c55e   // 新枝绿（= driving 类绿，"新长出"语义）

  const BG = 0x0f172a       // 视口深色：让节点/边/深度更可读（仅画布内，不动全局浅色主题）
  const HALO = 0xffffff     // 选中描边色（白描边圈，区别于"亮节点"）

  function render() { if (renderer && scene && camera) renderer.render(scene, camera) }

  function initThree() {
    const w = host.clientWidth || 600, h = host.clientHeight || 400
    scene = new THREE.Scene()
    scene.background = new THREE.Color(BG)
    camera = new THREE.PerspectiveCamera(45, w / h, 0.01, 100)
    camera.position.set(0, 0, 3.2)
    renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.setSize(w, h)
    host.appendChild(renderer.domElement)
    // 光照（半立体方案，GA-6）：球材质自发光=自身色（底色永不洗白），灯只补很弱的明暗当立体线索。
    // 故环境光 + 方向光都调暗——颜色一眼对应类别，不被光照冲淡；景深/尺寸补立体感。
    scene.add(new THREE.AmbientLight(0xffffff, 0.3))
    const dir = new THREE.DirectionalLight(0xffffff, 0.25)
    dir.position.set(1, 1, 1)
    scene.add(dir)
    controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = false
    controls.addEventListener('change', render)    // 按需渲染：轨道变化才重绘
    raycaster = new THREE.Raycaster()
    const el = renderer.domElement
    el.addEventListener('pointermove', onPointerMove)
    el.addEventListener('pointerdown', onPointerDown)
    el.addEventListener('pointerup', onPointerUp)
    el.addEventListener('pointerleave', () => { hideTip(tip); setHover(null) })
    ro = new ResizeObserver(() => {
      if (!renderer || !camera) return
      const W = host.clientWidth || 600, H = host.clientHeight || 400
      camera.aspect = W / H; camera.updateProjectionMatrix(); renderer.setSize(W, H); render()
    })
    ro.observe(host)
  }

  function disposeGroup() {
    if (!group || !scene) return
    scene.remove(group)
    group.traverse((o) => {
      const m = o as THREE.Mesh
      if (m.geometry) m.geometry.dispose()
      const mat = m.material as THREE.Material | THREE.Material[] | undefined
      if (Array.isArray(mat)) mat.forEach((x) => x.dispose())
      else mat?.dispose()
    })
    group = null
    nodeMeshes = []
    hovered = null
    edgeMesh = null
    edgeList = []
    nodePos.clear()
    localPos.clear()
    gpEdgeMesh = null
    if (growthRAF) { cancelAnimationFrame(growthRAF); growthRAF = 0 }
    if (gpRAF) { cancelAnimationFrame(gpRAF); gpRAF = 0 }
  }

  async function loadGraph(model: string) {
    status = 'loading'; errMsg = ''
    let data: Layout3dJson
    try {
      data = await fetchLayout3d(model)
    } catch (e) {
      status = 'error'; errMsg = String(e); return
    }
    if (!scene) return
    // 后端错误信封 {type:error,...} 没有 nodes 数组
    if (!data || !Array.isArray(data.nodes)) {
      const er = (data as unknown as { error?: { message?: string } | string }).error
      status = 'error'
      errMsg = (typeof er === 'object' ? er.message : er) || '后端未返回坐标'
      return
    }
    disposeGroup()
    const sc = scene
    const g = new THREE.Group()
    const bound = data.bound || 1
    const geo = new THREE.SphereGeometry(1, 20, 16)   // 单位球，逐节点缩放（本次加载内共享）
    const haloMat = new THREE.MeshBasicMaterial({ color: HALO, side: THREE.BackSide, transparent: true, opacity: 0.85 })
    // 子系统配色：优先用契约 module_color（Rust 单一真相源、与 2D 同色相）；老契约无此字段则回退本地调色板。
    if (data.nodes.some((n) => n.module_color)) {
      modColor = new Map()
      for (const n of data.nodes) if (n.module && n.module_color && !modColor.has(n.module)) modColor.set(n.module, n.module_color)
    } else {
      modColor = moduleColorMap(data.nodes.map((n) => n.module)) // 老契约回退
    }
    presentModules = [...modColor.keys()]
    hasOther = data.nodes.some((n) => !n.module)
    // 本模型有无子系统：写完即用的局部判定（禁用「按子系统」由 Structure 直接读 contract.has_modules，
    // 故无需提到 store——见单一真相源审计 F3）。无子系统则复位配色模式，控件与显示一致。
    if (presentModules.length === 0) store.topoColorMode = 'class'
    nodePos = new Map<string, THREE.Vector3>()
    for (const n of data.nodes) {
      const ln = localName(n.id)
      // 半立体材质：emissive=自身色（loadGraph 末统一上色），高 roughness 去高光、避免洗白。
      const mat = new THREE.MeshStandardMaterial({ roughness: 0.85, metalness: 0, emissiveIntensity: 0.55 })
      const mesh = new THREE.Mesh(geo, mat)
      const r = 0.018 + n.size * 0.055     // 叶子可见的最小半径 + 介数放大
      mesh.scale.setScalar(r)
      mesh.position.set(n.x, n.y, n.z)
      // 选中描边：白色 BackSide 球略放大 → 仅露轮廓=光环（区别于"亮节点"），默认隐藏。
      const halo = new THREE.Mesh(geo, haloMat)
      halo.scale.setScalar(1.25)
      halo.visible = false
      mesh.add(halo)
      mesh.userData = { id: n.id, ln, r, module: n.module, halo }
      g.add(mesh)
      nodeMeshes.push(mesh)
      nodePos.set(n.id, mesh.position)
      localPos.set(ln, mesh.position)   // GP 新边按本地名查端点位置
    }
    recomputeClasses()   // 当前出现的 Forrester 类别（依赖 contract，晚到时由 $effect 重算）
    recolorByMode()      // 按当前配色模式上色（含 emissive 自发光）
    // 影响边（有向，本 v1 不画箭头，只画连线）；存 edgeList + edgeMesh 引用供生长动画按章重建。
    edgeList = data.edges
    const eg = new THREE.BufferGeometry()
    eg.setAttribute('position', new THREE.Float32BufferAttribute(edgeVerts(edgeList), 3))
    edgeMesh = new THREE.LineSegments(eg, new THREE.LineBasicMaterial({ color: 0x64748b, transparent: true, opacity: 0.35 }))
    g.add(edgeMesh)
    sc.add(g)
    group = g
    if (camera && controls) {
      camera.position.set(bound * 1.1, bound * 0.9, bound * 2.6)
      controls.target.set(0, 0, 0)
      controls.update()
    }
    status = data.nodes.length ? 'ok' : 'empty'
    buildGrowthMap()
    if (store.growth.active) applyGrowth(true) // 演示进行中切模型/重载 → 直接套用当前章节
    if (gpDiff) { ensureGpEdgeMesh(); runGpAnim() } // GP 彩蛋：重载后套用当前 diff 动画
    applySelection(store.selectedVars)
    render()
  }

  /** 由边列表 + 节点位置生成 LineSegments 顶点数组。 */
  function edgeVerts(edges: [string, string][]): number[] {
    const v: number[] = []
    for (const [a, b] of edges) {
      const pa = nodePos.get(a), pb = nodePos.get(b)
      if (!pa || !pb) continue
      v.push(pa.x, pa.y, pa.z, pb.x, pb.y, pb.z)
    }
    return v
  }

  // —— 生长动画（GA-6b）：逐章把节点显形（缩放 0→r 补间）+ 按揭示集重建边 —— //
  /** 从 store.growth.plan 建「本地名 → 章节序」映射。 */
  function buildGrowthMap() {
    growthCh = new Map()
    ;(store.growth.plan?.chapters ?? []).forEach((ch, i) =>
      ch.nodes.forEach((ln) => { if (!growthCh.has(ln)) growthCh.set(ln, i) }),
    )
  }
  /** 某本地名当前是否已揭示（演示关=全显；不在 plan 的随第 0 章）。 */
  function revealed(ln: string): boolean {
    if (!store.growth.active) return true
    return (growthCh.get(ln) ?? 0) <= store.growth.chapter
  }
  /** 套用当前章节：各节点目标缩放（已揭示=r / 未揭示=0）+ 重建可见边 + 启动补间。 */
  function applyGrowth(instant = false) {
    for (const mesh of nodeMeshes) {
      const ud = mesh.userData as { ln: string; r: number; target?: number }
      ud.target = revealed(ud.ln) ? ud.r : 0
      if (instant) mesh.scale.setScalar(ud.target)
    }
    rebuildEdges()
    if (instant) render()
    else startTween()
  }
  /** 重建边几何：只画两端都已揭示的边（演示关=全部）。 */
  function rebuildEdges() {
    if (!edgeMesh) return
    const vis = store.growth.active
      ? edgeList.filter(([a, b]) => revealed(localName(a)) && revealed(localName(b)))
      : edgeList
    edgeMesh.geometry.setAttribute('position', new THREE.Float32BufferAttribute(edgeVerts(vis), 3))
    edgeMesh.geometry.computeBoundingSphere()
  }
  /** 补间循环：各节点缩放渐近目标（"长出"），逐帧渲染，全到位即停。 */
  function startTween() {
    if (growthRAF) cancelAnimationFrame(growthRAF)
    const step = () => {
      let moving = false
      for (const mesh of nodeMeshes) {
        const ud = mesh.userData as { target?: number }
        const t = ud.target ?? mesh.scale.x
        const next = mesh.scale.x + (t - mesh.scale.x) * 0.18
        const snap = Math.abs(t - next) <= 1e-4
        mesh.scale.setScalar(snap ? t : next)
        if (!snap) moving = true
      }
      render()
      growthRAF = moving ? requestAnimationFrame(step) : 0
    }
    growthRAF = requestAnimationFrame(step)
  }

  // —— GP「看它长出什么」彩蛋（GA-6b Phase 3）：在现有结构上叠加 diff 动画 —— //
  /** 懒建 GP 绿新边对象（独立 LineSegments，叠在现有图上；初始空+透明）。 */
  function ensureGpEdgeMesh() {
    if (gpEdgeMesh || !group || !gpDiff) return
    const eg = new THREE.BufferGeometry()
    eg.setAttribute('position', new THREE.Float32BufferAttribute([], 3))
    gpEdgeMesh = new THREE.LineSegments(eg, new THREE.LineBasicMaterial({ color: GP_EDGE, transparent: true, opacity: 0 }))
    group.add(gpEdgeMesh)
  }
  /** added 边的端点位置对（按本地名查；端点缺失则跳过=优雅降级）。 */
  function gpEdgePairs(): { a: THREE.Vector3; b: THREE.Vector3 }[] {
    const out: { a: THREE.Vector3; b: THREE.Vector3 }[] = []
    for (const [la, lb] of gpDiff?.addedEdges ?? []) {
      const a = localPos.get(la), b = localPos.get(lb)
      if (a && b) out.push({ a, b })
    }
    return out
  }
  /** 重建绿边几何：每条从源点伸到 lerp(源,目标,t)（t=生长进度，0=一点、1=整条）。 */
  function setGpEdges(t: number) {
    if (!gpEdgeMesh) return
    const v: number[] = []
    for (const { a, b } of gpEdgePairs())
      v.push(a.x, a.y, a.z, a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t)
    gpEdgeMesh.geometry.setAttribute('position', new THREE.Float32BufferAttribute(v, 3))
    gpEdgeMesh.geometry.computeBoundingSphere()
  }
  /** 形式变了的方程输出节点（要脉冲的球）。 */
  function pulseMeshes(): THREE.Mesh[] {
    const set = new Set(gpDiff?.pulseOutputs ?? [])
    return nodeMeshes.filter((m) => set.has((m.userData as { ln: string }).ln))
  }
  const GP_BASE_EMIS = 0.55
  /** GP 生长动画：phase 1 时绿边伸出(0→1) + 脉冲节点 emissive 衰减起伏，约 1.7s；phase 0 复位。 */
  function runGpAnim() {
    if (gpRAF) { cancelAnimationFrame(gpRAF); gpRAF = 0 }
    const pulses = pulseMeshes()
    const setEmis = (val: number) => { for (const m of pulses) (m.material as THREE.MeshStandardMaterial).emissiveIntensity = val }
    if (gpDiff?.phase !== 1) { // 复位到"现有结构"：新边收回、节点回基线
      setGpEdges(0)
      if (gpEdgeMesh) (gpEdgeMesh.material as THREE.LineBasicMaterial).opacity = 0
      setEmis(GP_BASE_EMIS); render(); return
    }
    const DUR = 1700
    let start = -1
    const step = (ts: number) => {
      if (start < 0) start = ts
      const p = Math.min(1, (ts - start) / DUR)
      setGpEdges(p) // 绿边伸出
      if (gpEdgeMesh) (gpEdgeMesh.material as THREE.LineBasicMaterial).opacity = 0.35 + 0.55 * p
      const osc = Math.abs(Math.sin(p * Math.PI * 3)) * (1 - p) // 3 个衰减脉冲，收尾回基线
      setEmis(GP_BASE_EMIS + 0.85 * osc)
      render()
      gpRAF = p < 1 ? requestAnimationFrame(step) : 0
      if (p >= 1) setEmis(GP_BASE_EMIS)
    }
    gpRAF = requestAnimationFrame(step)
  }

  /** 当前模型实际出现的 Forrester 类别（按 CLASS_ORDER 排序，只列出现的；依赖 contract）。 */
  function recomputeClasses() {
    const seen = new Set<string>()
    for (const mesh of nodeMeshes) seen.add(classOf(contract, (mesh.userData as { ln: string }).ln))
    presentClasses = CLASS_ORDER.filter((c) => seen.has(c))
  }

  /** 一个节点在当前配色模式下的颜色。按子系统：命名子系统取调色板、其余=「其他」灰；
   *  本模型未声明子系统（modColor 空）则优雅回退按类别。 */
  function colorForMesh(ud: { ln: string; module?: string }): string {
    if (store.topoColorMode === 'module' && modColor.size) {
      return ud.module ? modColor.get(ud.module)! : MODULE_OTHER_COLOR
    }
    return nodeColor3d(contract, ud.ln)
  }

  /** 按当前配色模式给所有节点上色（color + emissive 同色=半立体底色，永不洗白）。 */
  function recolorByMode() {
    for (const mesh of nodeMeshes) {
      const ud = mesh.userData as { ln: string; module?: string }
      const col = colorForMesh(ud)
      const mat = mesh.material as THREE.MeshStandardMaterial
      mat.color.set(col)
      mat.emissive.set(col)
    }
    render()
  }

  function applySelection(sel: string[]) {
    const set = new Set(sel)
    for (const mesh of nodeMeshes) {
      const ud = mesh.userData as { ln: string; r: number; halo: THREE.Mesh }
      const on = set.has(ud.ln)
      ud.halo.visible = on               // 描边光环：选中显形（区别于"亮节点"）
      // 生长演示进行中由 applyGrowth 掌管缩放（0→r），选中只切光环、不抢缩放。
      if (!store.growth.active) mesh.scale.setScalar(on ? ud.r * 1.15 : ud.r)
    }
    render()
  }

  function pick(e: PointerEvent): THREE.Mesh | null {
    if (!raycaster || !camera || !renderer) return null
    const rect = renderer.domElement.getBoundingClientRect()
    const ndc = new THREE.Vector2(
      ((e.clientX - rect.left) / rect.width) * 2 - 1,
      -((e.clientY - rect.top) / rect.height) * 2 + 1,
    )
    raycaster.setFromCamera(ndc, camera)
    const hit = raycaster.intersectObjects(nodeMeshes, false)
    return hit.length ? (hit[0].object as THREE.Mesh) : null
  }

  function setHover(mesh: THREE.Mesh | null) {
    if (hovered === mesh) return
    hovered = mesh
    document.body.style.cursor = mesh ? 'pointer' : ''
  }

  function onPointerMove(e: PointerEvent) {
    const mesh = pick(e)
    setHover(mesh)
    if (mesh && tip) showTipAt(tip, tipHtml(contract, (mesh.userData as { ln: string }).ln), e.clientX, e.clientY)
    else hideTip(tip)
  }

  let downX = 0, downY = 0
  function onPointerDown(e: PointerEvent) { downX = e.clientX; downY = e.clientY }
  function onPointerUp(e: PointerEvent) {
    if (Math.abs(e.clientX - downX) + Math.abs(e.clientY - downY) > 4) return  // 拖动=轨道，不算点选
    const mesh = pick(e)
    if (!mesh) return
    const ln = (mesh.userData as { ln: string }).ln
    const has = store.selectedVars.includes(ln)
    store.selectedVars = has ? store.selectedVars.filter((n) => n !== ln) : [...store.selectedVars, ln]
    // 高亮由下方 $effect(store.selectedVars) 触发（2D/3D/仿真共享同一选择）
  }

  onMount(() => {
    tip = makeTip()
    initThree()
    curModel = store.model
    loadGraph(store.model)
  })
  onDestroy(() => {
    ro?.disconnect()
    if (growthRAF) cancelAnimationFrame(growthRAF)
    hideTip(tip); tip?.remove()
    disposeGroup()
    controls?.dispose()
    renderer?.dispose()
    renderer?.domElement.remove()
    document.body.style.cursor = ''
    renderer = null; scene = null; camera = null; controls = null
  })

  // 模型切换 → 重建图（onMount 已加载初始模型，这里只接后续变化）。
  $effect(() => {
    const m = store.model
    if (m !== curModel) { curModel = m; loadGraph(m) }
  })
  // contract 晚到 → 补色 + 重算出现的类别（按类别图例依赖 contract）。
  $effect(() => {
    void contract
    if (nodeMeshes.length) { recomputeClasses(); recolorByMode() }
  })
  // 配色模式切换（按类别 ↔ 按子系统）→ 重新上色。
  $effect(() => {
    void store.topoColorMode
    if (nodeMeshes.length) recolorByMode()
  })
  // 选中变化（2D/3D/仿真共享 store.selectedVars）→ 更新高亮。
  $effect(() => {
    const sel = store.selectedVars
    if (nodeMeshes.length) applySelection(sel)
  })
  // 生长演示（GA-6b）：active/章节/plan 变 → 重建章节映射 + 套用揭示（补间"长出"）。
  // GP 彩蛋模式（gpDiff 非空）下不跑子系统生长——节点已全显，由 GP 动画掌管叠加层。
  $effect(() => {
    const g = store.growth
    void g.active; void g.chapter; void g.plan
    if (!nodeMeshes.length || gpDiff) return
    buildGrowthMap()
    applyGrowth()
  })
  // GP「看它长出什么」（GA-6b Phase 3）：phase/nonce/addedEdges 变 → 套用 diff 动画（绿边伸出+脉冲）。
  $effect(() => {
    void gpDiff?.phase; void gpDiff?.nonce; void gpDiff?.addedEdges
    if (!nodeMeshes.length || !gpDiff) return
    ensureGpEdgeMesh()
    runGpAnim()
  })

  // —— 图例数据（只列当前模型出现的项；按类别带一句话含义、按子系统列子系统名）——
  type LegendItem = { color: string; name: string; meaning?: string }
  const legend = $derived.by<{ title: string; items: LegendItem[]; note?: string }>(() => {
    if (store.topoColorMode === 'module') {
      if (!presentModules.length)
        return { title: '子系统', items: [], note: '本模型未声明子系统，已按类别上色' }
      const items: LegendItem[] = presentModules.map((m) => ({ color: modColor.get(m)!, name: m }))
      if (hasOther) items.push({ color: MODULE_OTHER_COLOR, name: MODULE_OTHER_LABEL, meaning: '参数 / 驱动 / 未分组' })
      return { title: '子系统', items }
    }
    return {
      title: '类别',
      items: presentClasses.map((c) => ({
        color: classColor3d(contract, c),
        name: CLS_CN[c] ?? c,
        meaning: CLASS_LEGEND[c],
      })),
    }
  })
</script>

<div class="topo3d" bind:this={host}>
  {#if status === 'loading'}<div class="overlay">加载 3D 拓扑…</div>{/if}
  {#if status === 'error'}<div class="overlay err">3D 拓扑加载失败：{errMsg}</div>{/if}
  {#if status === 'empty'}<div class="overlay">该模型无可视节点</div>{/if}
  {#if status === 'ok' && !gpDiff}
    <!-- 常驻图例（角落小卡片，可折叠）：只列当前模型实际出现的项。GP 生长预览里隐藏（让位旁白字幕）。 -->
    <div class="legend" class:closed={!legendOpen}>
      <button class="leg-head" onclick={() => (legendOpen = !legendOpen)} title="折叠/展开图例">
        <span class="leg-title">图例 · 按{legend.title}</span>
        <span class="leg-caret">{legendOpen ? '▾' : '▸'}</span>
      </button>
      {#if legendOpen}
        <div class="leg-body">
          {#if contract?.structure?.entities?.length}
            <!-- FSPM 器官结构（从契约 structure 派生；声明一次→视图自动显示，零模型专属代码）-->
            <div class="leg-organ">
              <span class="leg-organ-t">🌿 器官结构</span>
              {#each contract.structure.entities as e (e.name)}
                <div class="leg-row"><span class="leg-name">{e.name}</span><span class="leg-mean">×{e.count} · {e.topology}</span></div>
              {/each}
            </div>
          {/if}
          {#if legend.note}<div class="leg-note">{legend.note}</div>{/if}
          {#each legend.items as it (it.name)}
            <div class="leg-row">
              <span class="leg-dot" style="background:{it.color}"></span>
              <span class="leg-name">{it.name}</span>
              {#if it.meaning}<span class="leg-mean">{it.meaning}</span>{/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .topo3d { position: relative; width: 100%; height: 100%; background: #0f172a; overflow: hidden; }
  .topo3d :global(canvas) { display: block; }
  .overlay {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%);
    color: #cbd5e1; font-size: 13px; background: rgba(15, 23, 42, 0.6);
    padding: 8px 14px; border-radius: 8px; pointer-events: none;
  }
  .overlay.err { color: #fca5a5; max-width: 80%; text-align: center; }

  /* 常驻图例：左下角小卡片，半透明深色，可折叠 */
  .legend {
    position: absolute; left: 12px; bottom: 12px; z-index: 5;
    background: rgba(15, 23, 42, 0.82); border: 1px solid #334155; border-radius: 8px;
    color: #e2e8f0; font-size: 12px; max-width: 280px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35); backdrop-filter: blur(2px); overflow: hidden;
  }
  .leg-head {
    display: flex; align-items: center; justify-content: space-between; gap: 10px; width: 100%;
    background: transparent; border: 0; color: #f1f5f9; cursor: pointer;
    padding: 7px 10px; font-size: 12px;
  }
  .legend.closed .leg-head { padding: 6px 10px; }
  .leg-title { font-weight: 600; }
  .leg-caret { color: #94a3b8; font-size: 11px; }
  .leg-body { padding: 2px 10px 9px; display: flex; flex-direction: column; gap: 5px; }
  .leg-organ { display: flex; flex-direction: column; gap: 4px; padding-bottom: 6px; margin-bottom: 4px; border-bottom: 1px solid #334155; }
  .leg-organ-t { color: #4ade80; font-weight: 700; font-size: 11px; }
  .leg-note { color: #fcd34d; font-size: 11px; line-height: 1.4; padding-bottom: 2px; }
  .leg-row { display: flex; align-items: baseline; gap: 7px; line-height: 1.35; }
  .leg-dot {
    flex: 0 0 auto; width: 11px; height: 11px; border-radius: 50%;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.18); transform: translateY(1px);
  }
  .leg-name { color: #f8fafc; font-weight: 600; white-space: nowrap; }
  .leg-mean { color: #94a3b8; font-size: 11px; }
</style>
