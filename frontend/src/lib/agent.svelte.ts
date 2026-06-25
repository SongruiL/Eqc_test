// 前端 LLM Agent：从命令注册表自动派生 Anthropic tools → 跑 agent loop（前端执行 handler）
// → 后端 /api/llm 只代理 Claude + 持 key。能力 = 注册表，加命令即加能力。confirm 闸护栏落盘类操作。
import { aiCommands, type Command } from './commands.svelte'
import { store } from './store.svelte'

// dev 默认 Sonnet 4.6（多轮工具往返的甜点：快/省/工具强）。后端 EQC_LLM_MODEL 可一行覆盖、无需重建。
const MODEL = 'claude-sonnet-4-6'
const MAX_TOKENS = 2048
const MAX_ITERS = 12 // agent loop 防失控上限

// —— 消息/块类型（Anthropic Messages API 形状的子集）——
export type Block =
  | { type: 'text'; text: string }
  | { type: 'tool_use'; id: string; name: string; input: Record<string, unknown> }
  | { type: 'tool_result'; tool_use_id: string; content: string; is_error?: boolean }
export type Msg = { role: 'user' | 'assistant'; content: string | Block[] }

export interface PendingConfirm {
  label: string
  summary: string
}

// 全局 Agent 状态（抽屉开关 + 对话 + 运行态 + 待确认）。
export const agent = $state({
  open: false,
  running: false,
  convo: [] as Msg[],
  pending: null as PendingConfirm | null,
  error: '' as string,
})

let confirmResolver: ((ok: boolean) => void) | null = null

// 工具名只允许 [a-zA-Z0-9_-]（Anthropic 限制），命令 id 含 `.` 须清洗 + 反查。
function toolName(id: string): string {
  return id.replace(/[^a-zA-Z0-9_-]/g, '_')
}
function buildToolMap(): Record<string, Command> {
  const m: Record<string, Command> = {}
  for (const c of aiCommands()) m[toolName(c.id)] = c
  return m
}

/** 注册表 → Anthropic tools 数组（每请求生成；会话内稳定 → 命中 prompt 缓存）。 */
function buildTools() {
  return aiCommands().map((c) => ({
    name: toolName(c.id),
    description: c.description ?? c.label,
    input_schema: {
      type: 'object',
      properties: c.params ?? {},
      required: c.required ?? [],
    },
  }))
}

const SYSTEM = `你是 EQC Studio 的内置助手。EQC 是一个农业生态系统的数学建模工具：把方程编译成可仿真的过程模型（草莓/番茄/蓝莓/温室等），并支持结构可视化、整季仿真、决策优化、参数标定、受约束遗传进化(GP)。

你通过【工具】操作整个前端——导航工作区、查询模型、调情景参数、跑仿真、写处理区设置等。原则：
- 用户能用界面做的，你基本都能通过工具做；底层代码与你无关。
- 想了解模型细节时，先用 describe_model / describe_variable，而不是凭空猜。
- 落盘类操作（写文件）会在执行前弹确认框给用户；被取消就换个方式或如实告知。
- 回答用中文，简洁。先给结论，再给细节。能直接动手就动手，不要罗列你不会做的选项。`

function modelSummary(): string {
  const j = store.modelJson
  if (!j) return '（当前未加载模型）'
  const vars = j.modules.flatMap((m) => m.variables)
  const params = j.modules.flatMap((m) => m.parameters).filter((p) => !p.values)
  const byClass: Record<string, number> = {}
  for (const v of vars) byClass[v.class] = (byClass[v.class] ?? 0) + 1
  const measurable = vars.filter((v) => v.measurable).map((v) => v.name)
  const outputs = vars.filter((v) => v.var_type === 'output').map((v) => v.name)
  const roster = store.models.map((m) => `${m.id}(${m.name})${m.coupled ? '·耦合' : ''}`).join('、')
  const lines = [
    `## 当前模型：${store.model}`,
    store.models.length > 1 ? `可切换模型（switch_model 用 id）：${roster}` : '',
    `变量 ${vars.length} 个（${Object.entries(byClass).map(([k, n]) => `${k}:${n}`).join(' ')}），标量参数 ${params.length} 个。`,
    outputs.length ? `输出变量：${outputs.slice(0, 30).join('、')}` : '',
    measurable.length ? `可测变量：${measurable.slice(0, 30).join('、')}` : '',
    `标量参数：${params.slice(0, 40).map((p) => p.name).join('、')}`,
    '（完整清单/某变量详情用 describe_model / describe_variable 工具取。）',
  ].filter(Boolean)
  return lines.join('\n')
}

function currentState(): string {
  const sc = store.scenario
  const ov = [
    ...Object.entries(sc.p).map(([k, v]) => `${k}=${v}`),
    ...Object.entries(sc.i).map(([k, v]) => `init:${k}=${v}`),
    ...Object.entries(sc.d).map(([k, v]) => `driver:${k}=${v}`),
  ]
  return [
    `## 当前界面状态`,
    `模型=${store.model}；处理区=${store.zone}；模式=${store.mode === 'park' ? '园区' : '专家'}；工作区=${store.workspace}`,
    `已选中变量：${store.selectedVars.join('、') || '（无）'}`,
    `情景覆盖：${ov.join('、') || '（无，用模型默认）'}`,
  ].join('\n')
}

function buildRequest() {
  return {
    model: MODEL,
    max_tokens: MAX_TOKENS,
    system: [
      // 稳定前缀（system + tools 一起缓存）：静态提示 + 按模型变的摘要，各打缓存断点。
      { type: 'text', text: SYSTEM, cache_control: { type: 'ephemeral' } },
      { type: 'text', text: modelSummary(), cache_control: { type: 'ephemeral' } },
      // 易变后缀（每请求重读、不缓存、很小）：当前界面状态。
      { type: 'text', text: currentState() },
    ],
    tools: buildTools(),
    messages: agent.convo,
  }
}

function needsConfirm(c: Command): boolean {
  return c.confirm ?? c.access === 'danger'
}

function summarizeCall(c: Command, input: Record<string, unknown>): string {
  const args = Object.keys(input).length ? JSON.stringify(input) : ''
  return `${c.label}${args ? ' ' + args : ''}`
}

/** UI 在 confirm 卡上点「允许/取消」时调用。 */
export function resolveConfirm(ok: boolean) {
  agent.pending = null
  const r = confirmResolver
  confirmResolver = null
  r?.(ok)
}
function askConfirm(c: Command, input: Record<string, unknown>): Promise<boolean> {
  agent.pending = { label: c.label, summary: summarizeCall(c, input) }
  return new Promise((res) => {
    confirmResolver = res
  })
}

async function callLlm(body: unknown): Promise<any> {
  const r = await fetch('/api/llm', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  })
  return r.json()
}

async function runLoop() {
  agent.running = true
  agent.error = ''
  try {
    const map = buildToolMap()
    for (let iter = 0; iter < MAX_ITERS; iter++) {
      const data = await callLlm(buildRequest())
      if (data?.type === 'error') {
        agent.error = data.error?.message ?? '调用 Claude 失败'
        break
      }
      const content: Block[] = Array.isArray(data?.content) ? data.content : []
      agent.convo.push({ role: 'assistant', content })

      if (data?.stop_reason !== 'tool_use') break // end_turn / 其它 → 结束

      const toolUses = content.filter((b): b is Extract<Block, { type: 'tool_use' }> => b.type === 'tool_use')
      const results: Block[] = []
      for (const tu of toolUses) {
        const cmd = map[tu.name]
        if (!cmd) {
          results.push({ type: 'tool_result', tool_use_id: tu.id, content: `未知命令：${tu.name}`, is_error: true })
          continue
        }
        if (needsConfirm(cmd)) {
          const ok = await askConfirm(cmd, tu.input)
          if (!ok) {
            results.push({ type: 'tool_result', tool_use_id: tu.id, content: '用户取消了该操作。' })
            continue
          }
        }
        try {
          const out = await cmd.run(tu.input ?? {})
          const text = typeof out === 'string' ? out : JSON.stringify(out)
          results.push({ type: 'tool_result', tool_use_id: tu.id, content: text || '完成' })
        } catch (e) {
          results.push({ type: 'tool_result', tool_use_id: tu.id, content: `执行失败：${e}`, is_error: true })
        }
      }
      agent.convo.push({ role: 'user', content: results })
    }
  } finally {
    agent.running = false
  }
}

/** 发一条用户消息并驱动 agent loop。 */
export async function sendMessage(text: string) {
  const t = text.trim()
  if (!t || agent.running) return
  agent.convo.push({ role: 'user', content: t })
  await runLoop()
}

export function clearConvo() {
  if (agent.running) return
  agent.convo = []
  agent.error = ''
}
