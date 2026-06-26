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
    classOf, CLS_CN, CLASS_COLOR_3D, CLASS_LEGEND, CLASS_ORDER,
    moduleColorMap, MODULE_OTHER_COLOR, MODULE_OTHER_LABEL,
  } from '../lib/annotate'

  type Props = { contract: ModelJson | null }
  let { contract }: Props = $props()

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
    // 子系统配色：按节点顺序首现定色（节点 + 图例共用同一映射）；空映射=本模型未声明子系统。
    modColor = moduleColorMap(data.nodes.map((n) => n.module))
    presentModules = [...modColor.keys()]
    hasOther = data.nodes.some((n) => !n.module)
    store.topoHasModules = presentModules.length > 0
    if (!store.topoHasModules) store.topoColorMode = 'class' // 无子系统 → 复位，切换控件与显示一致
    const pos = new Map<string, THREE.Vector3>()
    for (const n of data.nodes) {
      const ln = localName(n.id)
      // 半立体材质：emissive=自身色（loadGraph 末统一上色），高 roughness 去高光、避免洗白。
      const mat = new THREE.MeshStandardMaterial({ roughness: 0.85, metalness: 0, emissiveIntensity: 0.55 })
      const mesh = new THREE.Mesh(geo, mat)
      const r = 0.018 + n.size * 0.055     // 叶子可见的最小半径 + 介数放大
      mesh.scale.setScalar(r)
      mesh.position.set(n.x, n.y, n.z)
      // 选中描边：白色 BackSide 球放大 → 仅露轮廓=光环（区别于"亮节点"），默认隐藏。
      const halo = new THREE.Mesh(geo, haloMat)
      halo.scale.setScalar(1.5)
      halo.visible = false
      mesh.add(halo)
      mesh.userData = { id: n.id, ln, r, module: n.module, halo }
      g.add(mesh)
      nodeMeshes.push(mesh)
      pos.set(n.id, mesh.position)
    }
    recomputeClasses()   // 当前出现的 Forrester 类别（依赖 contract，晚到时由 $effect 重算）
    recolorByMode()      // 按当前配色模式上色（含 emissive 自发光）
    // 影响边（有向，本 v1 不画箭头，只画连线）
    if (data.edges.length) {
      const verts: number[] = []
      for (const [a, b] of data.edges) {
        const pa = pos.get(a), pb = pos.get(b)
        if (!pa || !pb) continue
        verts.push(pa.x, pa.y, pa.z, pb.x, pb.y, pb.z)
      }
      const eg = new THREE.BufferGeometry()
      eg.setAttribute('position', new THREE.Float32BufferAttribute(verts, 3))
      g.add(new THREE.LineSegments(eg, new THREE.LineBasicMaterial({ color: 0x64748b, transparent: true, opacity: 0.35 })))
    }
    sc.add(g)
    group = g
    if (camera && controls) {
      camera.position.set(bound * 1.1, bound * 0.9, bound * 2.6)
      controls.target.set(0, 0, 0)
      controls.update()
    }
    status = data.nodes.length ? 'ok' : 'empty'
    applySelection(store.selectedVars)
    render()
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
      mesh.scale.setScalar(on ? ud.r * 1.25 : ud.r)  // 轻微放大做冗余线索
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
        color: CLASS_COLOR_3D[c] ?? '#9ca3af',
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
  {#if status === 'ok'}
    <!-- 常驻图例（角落小卡片，可折叠）：只列当前模型实际出现的项 -->
    <div class="legend" class:closed={!legendOpen}>
      <button class="leg-head" onclick={() => (legendOpen = !legendOpen)} title="折叠/展开图例">
        <span class="leg-title">图例 · 按{legend.title}</span>
        <span class="leg-caret">{legendOpen ? '▾' : '▸'}</span>
      </button>
      {#if legendOpen}
        <div class="leg-body">
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
  .leg-note { color: #fcd34d; font-size: 11px; line-height: 1.4; padding-bottom: 2px; }
  .leg-row { display: flex; align-items: baseline; gap: 7px; line-height: 1.35; }
  .leg-dot {
    flex: 0 0 auto; width: 11px; height: 11px; border-radius: 50%;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.18); transform: translateY(1px);
  }
  .leg-name { color: #f8fafc; font-weight: 600; white-space: nowrap; }
  .leg-mean { color: #94a3b8; font-size: 11px; }
</style>
