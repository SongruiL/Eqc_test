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

// 流式调用 /api/llm/stream：push 一条「活」的助手消息，边收 SSE delta 边填它（文字逐字蹦），
// message_stop 后返回组装好的 {content, stop_reason}（或 error）。
type LiveBlock = Block & { _json?: string }
async function streamLlm(body: Record<string, unknown>): Promise<{ content: Block[]; stop_reason: string; error?: string }> {
  let resp: Response
  try {
    resp = await fetch('/api/llm/stream', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ ...body, stream: true }),
    })
  } catch (e) {
    return { content: [], stop_reason: '', error: `连接失败：${e}` }
  }
  if (!resp.body) return { content: [], stop_reason: '', error: '无响应流' }

  // 关键（Svelte 5）：mutate 必须经 $state 代理才响应式——push 后用 agent.convo[idx]（代理）拿 blocks，
  // 不能用 push 进去的裸对象引用（裸引用绕过代理 → 文字不会逐字蹦）。
  const idx = agent.convo.push({ role: 'assistant', content: [] as Block[] }) - 1
  const blocks = agent.convo[idx].content as LiveBlock[]
  let stop_reason = ''
  let error = ''

  const onEvent = (ev: any) => {
    switch (ev?.type) {
      case 'content_block_start': {
        const cb = ev.content_block
        if (cb?.type === 'text') blocks[ev.index] = { type: 'text', text: cb.text ?? '' }
        else if (cb?.type === 'tool_use') blocks[ev.index] = { type: 'tool_use', id: cb.id, name: cb.name, input: {}, _json: '' }
        break
      }
      case 'content_block_delta': {
        const b = blocks[ev.index]
        if (!b) break
        if (ev.delta?.type === 'text_delta' && b.type === 'text') b.text += ev.delta.text
        else if (ev.delta?.type === 'input_json_delta' && b.type === 'tool_use') b._json = (b._json ?? '') + ev.delta.partial_json
        break
      }
      case 'content_block_stop': {
        const b = blocks[ev.index]
        if (b && b.type === 'tool_use') {
          try {
            b.input = b._json ? JSON.parse(b._json) : {}
          } catch {
            b.input = {}
          }
          delete b._json
        }
        break
      }
      case 'message_delta':
        if (ev.delta?.stop_reason) stop_reason = ev.delta.stop_reason
        break
      case 'error':
        error = ev.error?.message ?? '调用 Claude 失败'
        break
    }
  }

  const reader = resp.body.getReader()
  const dec = new TextDecoder()
  let buf = ''
  const flushLines = (final = false) => {
    let i: number
    while ((i = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, i).replace(/\r$/, '')
      buf = buf.slice(i + 1)
      if (line.startsWith('data:')) {
        const d = line.slice(5).trim()
        if (d) {
          try {
            onEvent(JSON.parse(d))
          } catch {
            /* 跨块的半行：忽略，等下一块拼齐 */
          }
        }
      }
    }
    if (final && buf.trim().startsWith('data:')) {
      try {
        onEvent(JSON.parse(buf.trim().slice(5).trim()))
      } catch {
        /* ignore */
      }
    }
  }
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buf += dec.decode(value, { stream: true })
    flushLines()
  }
  flushLines(true)

  for (const b of blocks) if (b && b._json !== undefined) delete b._json
  if (error && blocks.length === 0) agent.convo.splice(idx, 1) // 出错且没产出 → 撤掉空气泡
  return { content: blocks as Block[], stop_reason, error: error || undefined }
}

async function runLoop() {
  agent.running = true
  agent.error = ''
  try {
    const map = buildToolMap()
    for (let iter = 0; iter < MAX_ITERS; iter++) {
      const res = await streamLlm(buildRequest())
      if (res.error) {
        agent.error = res.error
        break
      }
      if (res.stop_reason !== 'tool_use') break // end_turn / 其它 → 结束

      const toolUses = res.content.filter((b): b is Extract<Block, { type: 'tool_use' }> => b.type === 'tool_use')
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
