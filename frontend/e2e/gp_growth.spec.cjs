// GA-6b Phase 3 冒烟：GP「看它长出什么」彩蛋。
// gpdemo3（双输入互作合成 demo）→ 进化(GP)工作区 → 跑 GP → 选互作候选 → 「🌱 看它长出什么」
// → 内联 3D 预览（Topology3d canvas）+ 旁白字幕出现，且自动播放后绿"新枝"已长出（结构 diff 动画）。
// 真 GP（无 mock），故给足超时；断言放松（有 canvas + 有旁白 + 无运行时错误），不抠具体字。
const { test, expect } = require('@playwright/test')

test('进化工作区 · GP「看它长出什么」结构生长预览（Topology3d + 旁白）', async ({ page }) => {
  const errors = []
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
  page.on('pageerror', (e) => errors.push(String(e)))

  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })

  // 切到 gpdemo3 模型（双输入互作 demo）
  await page.locator('header select, .top select, select').first().selectOption('gpdemo3')
  // 进化(GP)工作区
  await page.locator('nav').getByRole('button', { name: '进化', exact: true }).click()

  // 选靶点「门控」+ 压小规模提速（信号强，互作仍可靠胜出）
  await page.getByRole('button', { name: /门控/ }).first().click()
  await page.locator('.ws-head input[type=number]').nth(0).fill('45') // 种群
  await page.locator('.ws-head input[type=number]').nth(1).fill('30') // 代数
  await page.getByRole('button', { name: '开始进化' }).click()

  // 等结果（Pareto 前沿 SVG 出现）——异步任务，给足时间
  await expect(page.locator('.pareto svg')).toBeVisible({ timeout: 70_000 })

  // 选最高复杂度的前沿点（最右 = 互作形式，最可能长出 d2→y 边）
  const pts = page.locator('.pareto circle.pp')
  await expect(pts.first()).toBeVisible({ timeout: 10_000 })
  await pts.last().click()

  // 候选块出现「🌱 看它长出什么」→ 点开内联预览
  const growBtn = page.getByRole('button', { name: /看它长出什么/ })
  await expect(growBtn).toBeVisible({ timeout: 10_000 })
  await growBtn.click()

  // 内联 3D 预览：Topology3d canvas 挂载 + 有尺寸 + 旁白非空
  const canvas = page.locator('.gp-grow .topo3d canvas')
  await expect(canvas).toBeVisible({ timeout: 15_000 })
  const box = await canvas.boundingBox()
  expect(box && box.width, 'canvas 宽').toBeGreaterThan(50)
  expect(box && box.height, 'canvas 高').toBeGreaterThan(50)
  await expect(page.locator('.gp-grow .overlay.err')).toHaveCount(0)
  await expect(page.locator('.gp-grow .gg-text')).not.toBeEmpty()

  // 自动播放（800ms 后长出）→ 截图留证（滚到预览 + 元素级截图，确保拍到 3D 生长画面）
  await page.locator('.gp-grow').scrollIntoViewIfNeeded()
  await page.waitForTimeout(2800)
  await page.locator('.gp-grow').screenshot({ path: 'e2e/__screots/gp_growth.png' })
  await page.screenshot({ path: 'e2e/__screots/gp_growth_full.png', fullPage: false })

  // 「再播」可点（重播动画）
  await page.getByRole('button', { name: /再播/ }).click()
  await expect(canvas).toBeVisible()

  const real = errors.filter((e) => !/webgl|gl_|swiftshader|groupmarker|fallback|gpu|deprecat|context lost|failed to load resource|404|favicon/i.test(e))
  expect(real, real.join('\n')).toHaveLength(0)
})
