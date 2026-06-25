// 命令注册表：前端「能做的事」的单一真相源。
//   ① ⌘K 命令面板从这里渲染；② LLM Agent 从这里**自动派生 Anthropic tools**（lib/agent）。
// 加功能 = 加一条命令 = 面板按钮 + AI 能力同时获得，零额外胶水。
//
// 元数据全可选、增量加（不破坏既有面板用法）：
//   description  给 LLM 看（何时/如何用；缺省回退 label）
//   params/required  JSON Schema 的 properties/required（无参命令省略）
//   access       'read'|'write'|'danger' → confirm 默认（danger 必确认）；缺省 'read'
//   confirm      显式覆盖 access 推断
//   aiHidden     纯 UI 命令不暴露给 AI（如打开面板）
// run 现可接受 args 并返回结果（字符串/对象）= tool_result；UI 按钮不传参、忽略返回。
import { store, setWorkspace, setMode, switchModel } from './store.svelte'
import { fetchSimulate, saveZone, fetchZone } from './api'
import type { VarJson, ParamJson } from './contract'

export interface Command {
  id: string
  label: string
  group: string
  run: (args?: Record<string, unknown>) => unknown | Promise<unknown>
  keywords?: string
  description?: string
  params?: Record<string, unknown>
  required?: string[]
  access?: 'read' | 'write' | 'danger'
  confirm?: boolean
  aiHidden?: boolean
}

// 全局 UI 状态（命令面板开关）。
export const ui = $state({ palette: false })

const goto = (mode: 'expert' | 'park', ws: string) => () => {
  setMode(mode)
  setWorkspace(ws)
  return `已切换到「${ws}」视图（${mode === 'park' ? '园区' : '专家'}）`
}

// ── 静态命令（导航 + 视图切换）。面板 + AI 共用。──
export const COMMANDS: Command[] = [
  { id: 'go.structure', label: '结构', group: '专家', run: goto('expert', 'structure'), keywords: 'structure dag forrester 图 结构', access: 'read', description: '打开「结构」工作区：模型的 Forrester 库存-流量图 / DAG 依赖图。' },
  { id: 'go.simulate', label: '仿真', group: '专家', run: goto('expert', 'simulate'), keywords: 'simulate chart 轨迹 仿真', access: 'read', description: '打开「仿真」工作区：整季轨迹折线图 + 情景滑块。' },
  { id: 'go.optimize', label: '优化', group: '专家', run: goto('expert', 'optimize'), keywords: 'optimize de 决策', access: 'read', description: '打开「优化」工作区：决策优化（DE 求最优旋钮）。' },
  { id: 'go.calibrate', label: '标定', group: '专家', run: goto('expert', 'calibrate'), keywords: 'calibrate 标定', access: 'read', description: '打开「标定」工作区：用实测数据反推模型参数。' },
  { id: 'go.gp', label: '进化 (GP)', group: '专家', run: goto('expert', 'gp'), keywords: 'gp evolve 进化 遗传', access: 'read', description: '打开「进化(GP)」工作区：在 gp_target 靶点进化方程结构。' },
  { id: 'go.edit', label: '编辑器', group: '专家', run: goto('expert', 'edit'), keywords: 'edit 编辑 yaml 源码', access: 'read', description: '打开「编辑器」工作区：浏览器内编辑模型 YAML 源码。' },
  { id: 'go.understand', label: '看懂', group: '园区', run: goto('park', 'understand'), keywords: 'understand 看懂 状态', access: 'read', description: '打开园区「看懂」卡：标定徽章 + 头条 + 胁迫红绿灯。' },
  { id: 'go.entry', label: '录入', group: '园区', run: goto('park', 'entry'), keywords: 'entry 录入 观测 数据', access: 'read', description: '打开园区「录入」网格：填写本处理区的实测观测数据。' },
  { id: 'mode.expert', label: '切到专家视图', group: '视图', run: () => (setMode('expert'), '已切到专家视图'), keywords: 'expert 专家', access: 'read', aiHidden: true },
  { id: 'mode.park', label: '切到园区视图', group: '视图', run: () => (setMode('park'), '已切到园区视图'), keywords: 'park 园区', access: 'read', aiHidden: true },
]

// ── 工具：跨全部模块找变量/参数（耦合视图有多个模块）。──
function allVars(): VarJson[] {
  return (store.modelJson?.modules ?? []).flatMap((m) => m.variables)
}
function allParams(): ParamJson[] {
  return (store.modelJson?.modules ?? []).flatMap((m) => m.parameters)
}
function findVar(name: string): VarJson | undefined {
  return allVars().find((v) => v.name === name)
}

// ── Agent 专用命令（带参数；面板不收录，因需参数）。──
export const AGENT_COMMANDS: Command[] = [
  {
    id: 'describe_model',
    label: '描述当前模型',
    group: 'AI',
    access: 'read',
    description: '返回当前模型的概览：变量清单（名/中文名/类别/单位）与参数清单（名/中文名/默认值/单位）。需要某个变量的方程或物理意义时，改用 describe_variable。',
    run: () => {
      const j = store.modelJson
      if (!j) return '模型未加载'
      const vars = allVars().map((v) => ({ name: v.name, 名: v.display_name, 类别: v.class, 单位: v.unit ?? '', 可测: !!v.measurable }))
      const params = allParams().filter((p) => !p.values).map((p) => ({ name: p.name, 名: p.display_name, 默认: p.default, 单位: p.unit ?? '' }))
      return { model: store.model, 变量数: vars.length, 参数数: params.length, 变量: vars, 参数: params }
    },
  },
  {
    id: 'describe_variable',
    label: '描述某变量',
    group: 'AI',
    access: 'read',
    params: { name: { type: 'string', description: '变量代号（如 Y、LAI）' } },
    required: ['name'],
    description: '返回某变量的详情：中文名、单位、Forrester 类别、物理意义、方程（若有）、文献出处。',
    run: (args) => {
      const name = String(args?.name ?? '')
      const v = findVar(name)
      if (!v) return `未找到变量「${name}」。用 describe_model 查可用变量。`
      const eq = (store.modelJson?.modules ?? []).flatMap((m) => m.equations).find((e) => e.output === name)
      return {
        name: v.name, 中文名: v.display_name, 单位: v.unit ?? '', 类别: v.class,
        物理意义: v.description ?? '', 可测: !!v.measurable,
        方程: eq?.formula_display ?? '（无显式方程：状态量/驱动量/参数）', 出处: eq?.reference ?? '',
      }
    },
  },
  {
    id: 'run_simulation',
    label: '跑一次整季仿真',
    group: 'AI',
    access: 'read',
    description: '用当前情景覆盖跑一次整季仿真，返回各输出/可测变量的末值（整季最后一天）。用于回答“产量是多少/末了 LAI 多少”等。',
    run: async () => {
      const r = await fetchSimulate(store.model, store.scenario.p, store.scenario.d)
      if (r.error) return `仿真失败：${r.error}`
      const series = r.series ?? {}
      const interesting = allVars().filter((v) => v.measurable || v.var_type === 'output').map((v) => v.name)
      const finals: Record<string, number> = {}
      for (const k of Object.keys(series)) {
        // series 已是 name 或 name[i] 扁平键；只挑感兴趣的（或同名前缀）。
        const base = k.replace(/\[\d+\]$/, '')
        if (interesting.includes(base) || interesting.includes(k)) {
          const arr = series[k]
          if (arr?.length) finals[k] = arr[arr.length - 1]
        }
      }
      return { steps: r.steps, 末值: finals }
    },
  },
  {
    id: 'select_vars',
    label: '选中要画的变量',
    group: 'AI',
    access: 'write',
    params: { vars: { type: 'array', items: { type: 'string' }, description: '变量代号列表' } },
    required: ['vars'],
    description: '设置仿真/轨迹图要绘制的变量（替换当前选择），并切到仿真视图。',
    run: (args) => {
      const vars = (args?.vars as string[]) ?? []
      const known = new Set(allVars().map((v) => v.name))
      const ok = vars.filter((v) => known.has(v))
      const bad = vars.filter((v) => !known.has(v))
      store.selectedVars = ok
      setMode('expert'); setWorkspace('simulate')
      return ok.length ? `已选中 ${ok.join('、')}${bad.length ? `（忽略未知：${bad.join('、')}）` : ''}` : `没有有效变量${bad.length ? `（未知：${bad.join('、')}）` : ''}`
    },
  },
  {
    id: 'set_scenario_param',
    label: '调一个情景参数',
    group: 'AI',
    access: 'write',
    params: { name: { type: 'string', description: '标量参数代号' }, value: { type: 'number', description: '新值' } },
    required: ['name', 'value'],
    description: '把某标量参数设为指定值作为情景覆盖（只影响曲线，不写盘）。改完曲线自动重算。',
    run: (args) => {
      const name = String(args?.name ?? '')
      const value = Number(args?.value)
      const p = allParams().find((x) => x.name === name)
      if (!p) return `未找到参数「${name}」`
      if (p.values) return `参数「${name}」是向量参数，不支持标量覆盖`
      if (!Number.isFinite(value)) return `值无效`
      store.scenario.p = { ...store.scenario.p, [name]: value }
      setMode('expert'); setWorkspace('simulate')
      return `已把 ${p.display_name}(${name}) 设为 ${value}`
    },
  },
  {
    id: 'reset_scenario',
    label: '重置情景覆盖',
    group: 'AI',
    access: 'write',
    description: '清空所有情景覆盖（参数/初值/驱动），回到模型默认。',
    run: () => {
      store.scenario = { p: {}, i: {}, d: {} }
      return '已重置情景到模型默认'
    },
  },
  {
    id: 'switch_model',
    label: '切换模型',
    group: 'AI',
    access: 'read',
    params: { id: { type: 'string', description: '模型 id（见当前模型摘要里的「可切换模型」清单）' } },
    required: ['id'],
    description: '切换当前加载的模型/作物（顶部模型选择器的程序版）。注意：耦合视图（如温室×作物）只支持看结构图；要仿真/录入/标定/写管理，需先切到对应的单作物模型。',
    run: (args) => {
      const id = String(args?.id ?? '')
      const m = store.models.find((x) => x.id === id)
      if (!m) return `未找到模型「${id}」。可用：${store.models.map((x) => `${x.id}(${x.name})`).join('、')}`
      switchModel(id)
      return `已切换到模型 ${m.name}(${id})`
    },
  },
  {
    id: 'switch_zone',
    label: '切换处理区',
    group: 'AI',
    access: 'write',
    params: { zone: { type: 'string', description: '处理区名（如 1..6 或名称）' } },
    required: ['zone'],
    description: '切换当前处理区（影响录入/标定/看懂卡的数据来源）。',
    run: (args) => {
      const zone = String(args?.zone ?? '').trim()
      if (!zone) return '处理区名为空'
      store.zone = zone
      return `已切到处理区「${zone}」`
    },
  },
  {
    id: 'save_zone_management',
    label: '写入本区管理设置',
    group: 'AI',
    access: 'danger',
    params: { params: { type: 'object', description: '参数覆盖键值对，如 {"CO2":800}' } },
    required: ['params'],
    description: '把一组管理参数写入当前处理区的设置文件（<zone>.json，落盘）。标定/看懂卡会据此按本区处理仿真。这是落盘操作。',
    run: async (args) => {
      const params = (args?.params as Record<string, number>) ?? {}
      const cur = await fetchZone(store.model, store.zone)
      const merged = { ...(cur.params ?? {}), ...params }
      const r = await saveZone(store.model, store.zone, merged, cur.drivers ?? {})
      if (r.error) return `写入失败：${r.error}`
      return `已写入处理区「${store.zone}」管理：${Object.entries(params).map(([k, v]) => `${k}=${v}`).join('、')}`
    },
  },
]

// 动态命令（按花名册生成模型切换）——仅 ⌘K 面板用。
export function modelCommands(): Command[] {
  return store.models
    .filter((m) => !m.coupled)
    .map((m) => ({ id: 'model.' + m.id, label: '切换到：' + m.name, group: '模型', run: () => switchModel(m.id), keywords: 'model 模型 ' + m.id, access: 'read' as const }))
}

/** ⌘K 命令面板的全集（导航 + 模型切换；带参的 Agent 命令不收录）。 */
export function allCommands(): Command[] {
  return [...COMMANDS, ...modelCommands()]
}

/** 暴露给 LLM 的命令集（导航 + Agent 命令；排除 aiHidden）。 */
export function aiCommands(): Command[] {
  return [...COMMANDS.filter((c) => !c.aiHidden), ...AGENT_COMMANDS]
}
