// 命令注册表：前端「能做的事」的单一真相源。当下供 ⌘K 命令面板用；
// 将来 LLM Agent（spec §7.2）从这里自动派生工具——加功能=加一条命令=面板+AI 同时获得能力。
// 最小形态 {id,label,run}；Agent 阶段再加 params schema / confirm 闸 / description（增量、不返工）。
import { store, setWorkspace, setMode, switchModel } from './store.svelte'

export interface Command {
  id: string
  label: string
  group: string
  run: () => void
  keywords?: string
}

// 全局 UI 状态（命令面板开关）——TopBar 按钮 + 全局快捷键 + 面板组件共用。
export const ui = $state({ palette: false })

const goto = (mode: 'expert' | 'park', ws: string) => () => { setMode(mode); setWorkspace(ws) }

// 静态命令（导航 + 视图切换）。
export const COMMANDS: Command[] = [
  { id: 'go.structure', label: '结构', group: '专家', run: goto('expert', 'structure'), keywords: 'structure dag forrester 图 结构' },
  { id: 'go.simulate', label: '仿真', group: '专家', run: goto('expert', 'simulate'), keywords: 'simulate chart 轨迹 仿真' },
  { id: 'go.optimize', label: '优化', group: '专家', run: goto('expert', 'optimize'), keywords: 'optimize de 决策' },
  { id: 'go.calibrate', label: '标定', group: '专家', run: goto('expert', 'calibrate'), keywords: 'calibrate 标定' },
  { id: 'go.gp', label: '进化 (GP)', group: '专家', run: goto('expert', 'gp'), keywords: 'gp evolve 进化 遗传' },
  { id: 'go.edit', label: '编辑器', group: '专家', run: goto('expert', 'edit'), keywords: 'edit 编辑 yaml 源码' },
  { id: 'go.understand', label: '看懂', group: '园区', run: goto('park', 'understand'), keywords: 'understand 看懂 状态' },
  { id: 'go.entry', label: '录入', group: '园区', run: goto('park', 'entry'), keywords: 'entry 录入 观测 数据' },
  { id: 'mode.expert', label: '切到专家视图', group: '视图', run: () => setMode('expert'), keywords: 'expert 专家' },
  { id: 'mode.park', label: '切到园区视图', group: '视图', run: () => setMode('park'), keywords: 'park 园区' },
]

// 动态命令（按花名册生成模型切换）。
export function modelCommands(): Command[] {
  return store.models
    .filter((m) => !m.coupled)
    .map((m) => ({ id: 'model.' + m.id, label: '切换到：' + m.name, group: '模型', run: () => switchModel(m.id), keywords: 'model 模型 ' + m.id }))
}

export function allCommands(): Command[] {
  return [...COMMANDS, ...modelCommands()]
}
