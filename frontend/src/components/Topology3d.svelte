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
  import { tipHtml, nodeColor3d, localName, makeTip, showTipAt, hideTip } from '../lib/annotate'

  type Props = { contract: ModelJson | null }
  let { contract }: Props = $props()

  let host: HTMLDivElement
  let tip: HTMLDivElement | undefined
  let status = $state<'loading' | 'ok' | 'empty' | 'error'>('loading')
  let errMsg = $state('')

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
  const SEL = 0xffffff      // 选中自发光提示色

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
    // 光照：环境光 + 一盏方向光（给球体明暗 = 3D 深度线索）
    scene.add(new THREE.AmbientLight(0xffffff, 0.75))
    const dir = new THREE.DirectionalLight(0xffffff, 0.7)
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
    const geo = new THREE.SphereGeometry(1, 20, 16)   // 单位球，逐节点缩放（共享几何）
    const pos = new Map<string, THREE.Vector3>()
    for (const n of data.nodes) {
      const ln = localName(n.id)
      const col = new THREE.Color(nodeColor3d(contract, ln))
      const mat = new THREE.MeshStandardMaterial({ color: col, roughness: 0.55, metalness: 0 })
      const mesh = new THREE.Mesh(geo, mat)
      const r = 0.018 + n.size * 0.055     // 叶子可见的最小半径 + 介数放大
      mesh.scale.setScalar(r)
      mesh.position.set(n.x, n.y, n.z)
      mesh.userData = { id: n.id, ln, r }
      g.add(mesh)
      nodeMeshes.push(mesh)
      pos.set(n.id, mesh.position)
    }
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

  /** 按 contract 重新着色（contract 晚于布局到达时补色，不重建几何）。 */
  function recolor(c: ModelJson | null) {
    for (const mesh of nodeMeshes) {
      const ud = mesh.userData as { ln: string }
      ;(mesh.material as THREE.MeshStandardMaterial).color.set(nodeColor3d(c, ud.ln))
    }
    render()
  }

  function applySelection(sel: string[]) {
    const set = new Set(sel)
    for (const mesh of nodeMeshes) {
      const ud = mesh.userData as { ln: string; r: number }
      const on = set.has(ud.ln)
      const mat = mesh.material as THREE.MeshStandardMaterial
      mat.emissive.set(on ? SEL : 0x000000)
      mat.emissiveIntensity = on ? 0.4 : 0
      mesh.scale.setScalar(on ? ud.r * 1.4 : ud.r)
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
  // contract 晚到 → 补色。
  $effect(() => {
    const c = contract
    if (nodeMeshes.length) recolor(c)
  })
  // 选中变化（2D/3D/仿真共享 store.selectedVars）→ 更新高亮。
  $effect(() => {
    const sel = store.selectedVars
    if (nodeMeshes.length) applySelection(sel)
  })
</script>

<div class="topo3d" bind:this={host}>
  {#if status === 'loading'}<div class="overlay">加载 3D 拓扑…</div>{/if}
  {#if status === 'error'}<div class="overlay err">3D 拓扑加载失败：{errMsg}</div>{/if}
  {#if status === 'empty'}<div class="overlay">该模型无可视节点</div>{/if}
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
</style>
