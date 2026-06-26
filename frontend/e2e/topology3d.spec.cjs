// GA-6 冒烟：结构工作区 → 3D 拓扑视图 → three.js canvas 挂载、有尺寸、无错误覆盖层、无运行时 JS 错误。
// 不依赖 LLM（mock/real serve 皆可跑），只需 serve 起在 EQC_E2E_BASE。验证组件真在浏览器里跑起来
// （svelte-check 只查类型；这条查运行时）。headless WebGL 走 SwiftShader（见 playwright.config.cjs 的旗标）。
const { test, expect } = require('@playwright/test')

test('结构工作区 · 3D 拓扑视图渲染（three.js canvas + 无运行时错误）', async ({ page }) => {
  const errors = []
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
  page.on('pageerror', (e) => errors.push(String(e)))

  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })

  // 左导航切到「结构」，再切「3D 拓扑」视图
  await page.locator('nav').getByRole('button', { name: '结构' }).click()
  await page.getByRole('button', { name: '3D 拓扑' }).click()

  // three.js canvas 挂载且有尺寸
  const canvas = page.locator('.topo3d canvas')
  await expect(canvas).toBeVisible({ timeout: 15_000 })
  const box = await canvas.boundingBox()
  expect(box && box.width, 'canvas 宽度').toBeGreaterThan(50)
  expect(box && box.height, 'canvas 高度').toBeGreaterThan(50)

  // /api/layout3d 成功 → 无错误覆盖层；加载态也应消失
  await expect(page.locator('.topo3d .overlay.err')).toHaveCount(0)
  await expect(page.locator('.topo3d .overlay')).toHaveCount(0, { timeout: 15_000 })

  // 无真实运行时 JS 错误（过滤 headless 软件 WebGL 噪声 + 浏览器自动请求 favicon 的 404 资源噪声）
  const real = errors.filter((e) => !/webgl|gl_|swiftshader|groupmarker|fallback|gpu|deprecat|context lost|failed to load resource|404|favicon/i.test(e))
  expect(real, real.join('\n')).toHaveLength(0)
})

// GA-6 配色模式 + 图例冒烟：默认按类别带含义图例；切「按子系统」（默认模型草莓 S8 有 meta.modules）
// 图例标题随之变；图例可折叠。只验存在性/交互，不验美观（无头）。
test('结构工作区 · 3D 配色模式切换 + 常驻图例', async ({ page }) => {
  const errors = []
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
  page.on('pageerror', (e) => errors.push(String(e)))

  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })
  await page.locator('nav').getByRole('button', { name: '结构' }).click()
  await page.getByRole('button', { name: '3D 拓扑' }).click()

  // 等 3D 加载完成（canvas 挂载 + 覆盖层消失）
  await expect(page.locator('.topo3d canvas')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('.topo3d .overlay')).toHaveCount(0, { timeout: 15_000 })

  // 常驻图例存在、默认「按类别」、至少一项
  const legend = page.locator('.topo3d .legend')
  await expect(legend).toBeVisible()
  await expect(legend.locator('.leg-title')).toContainText('按类别')
  expect(await legend.locator('.leg-row').count(), '按类别图例项数').toBeGreaterThan(0)

  // 切「按子系统」（草莓 S8 声明了子系统 → 按钮可用）→ 图例标题变、仍有项
  const byMod = page.getByRole('button', { name: '按子系统' })
  await expect(byMod).toBeEnabled()
  await byMod.click()
  await expect(legend.locator('.leg-title')).toContainText('按子系统')
  expect(await legend.locator('.leg-row').count(), '按子系统图例项数').toBeGreaterThan(0)

  // 折叠图例 → 内容隐藏
  await legend.locator('.leg-head').click()
  await expect(legend.locator('.leg-body')).toHaveCount(0)

  const real = errors.filter((e) => !/webgl|gl_|swiftshader|groupmarker|fallback|gpu|deprecat|context lost|failed to load resource|404|favicon/i.test(e))
  expect(real, real.join('\n')).toHaveLength(0)
})

// 2D Forrester 合并冒烟：变量级配色切换（按类别/按子系统）与 3D 共用同一开关，切换改 iframe 的
// color 参数（草莓 S8 有 meta.modules → 按子系统可用）。验证 2D/3D 配色统一的 UI 接线。
test('结构工作区 · 2D Forrester 按子系统配色切换（形状=类别·颜色=子系统）', async ({ page }) => {
  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })
  await page.locator('nav').getByRole('button', { name: '结构' }).click()
  // 默认 2D 报告 · 变量级 → 配色切换可见、按子系统可用（草莓有子系统）
  const byMod = page.getByRole('button', { name: '按子系统', exact: true })
  await expect(byMod).toBeEnabled({ timeout: 10_000 })
  await byMod.click()
  // iframe 重载为 color=module
  await expect(page.locator('.frame iframe')).toHaveAttribute('src', /color=module/, { timeout: 10_000 })
  // 切回按类别 → color=class
  await page.getByRole('button', { name: '按类别', exact: true }).click()
  await expect(page.locator('.frame iframe')).toHaveAttribute('src', /color=class/, { timeout: 10_000 })
})

// 方程级（计算骨架）复活冒烟：方程级用 Forrester 形状（与变量级同款，藏参数叶子），
// 支持力导向/分层布局 + 配色切换。验证 level/layout 经 iframe src 生效、配色开关在方程级也在。
test('结构工作区 · 2D 方程级骨架复活（Forrester 形状 · 三布局 · 可配色）', async ({ page }) => {
  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })
  await page.locator('nav').getByRole('button', { name: '结构' }).click()
  // 切方程级 → level=equation
  await page.getByRole('button', { name: '方程', exact: true }).click()
  await expect(page.locator('.frame iframe')).toHaveAttribute('src', /level=equation/, { timeout: 10_000 })
  // 方程级也有配色切换（说明走的是 Forrester 渲染、非旧纯色 DAG）
  await expect(page.getByRole('button', { name: '按子系统', exact: true })).toBeVisible()
  // 力导向 / 分层布局都能切
  await page.getByRole('button', { name: '力导向', exact: true }).click()
  await expect(page.locator('.frame iframe')).toHaveAttribute('src', /layout=force/, { timeout: 10_000 })
  await page.getByRole('button', { name: '分层', exact: true }).click()
  await expect(page.locator('.frame iframe')).toHaveAttribute('src', /layout=layered/, { timeout: 10_000 })
})

// GA-6b 生长动画冒烟：点「生长演示」→ plan 加载 + 旁白字幕出现 + 章节推进；切 3D 旁白仍在（2D/3D 同步）；退出后旁白消失。
test('结构工作区 · 生长演示（逐章显形 + 旁白 · 2D/3D 同步）', async ({ page }) => {
  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })
  await page.locator('nav').getByRole('button', { name: '结构' }).click()
  // 默认 2D 视图开演（plan 加载 → 旁白出现）
  await page.getByRole('button', { name: '▶ 生长演示' }).click()
  await expect(page.locator('.frame .narration')).toBeVisible({ timeout: 10_000 })
  await expect(page.locator('.frame .narration .ntitle')).not.toBeEmpty()
  // 章节指示推进（自动播放）
  const ind = page.locator('.ws-head .gchap')
  await expect(ind).toHaveText(/1\/\d+/, { timeout: 5_000 })
  await expect(ind).not.toHaveText(/^1\/\d+$/, { timeout: 8_000 })
  // 切 3D → 旁白仍在（共用 store.growth）+ canvas 挂载（2D/3D 同步）
  await page.getByRole('button', { name: '3D 拓扑' }).click()
  await expect(page.locator('.topo3d canvas')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('.frame .narration')).toBeVisible()
  // 退出 → 旁白消失
  await page.getByRole('button', { name: '✕ 退出' }).click()
  await expect(page.locator('.frame .narration')).toHaveCount(0)
})
