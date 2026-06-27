// FSPM 地基 Step 3 冒烟：番茄 FSPM demo（structure: 段）→ 3D 拓扑图例显「🌿 器官结构」，
// 数据从 /api/model 的 structure 契约派生（声明一次→视图自动派生）。截图留证。
const { test, expect } = require('@playwright/test')

test('结构工作区 · 3D 拓扑显器官结构（FSPM 契约派生）', async ({ page }) => {
  const errors = []
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
  page.on('pageerror', (e) => errors.push(String(e)))

  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })

  // 切到番茄 FSPM demo
  await page.locator('header select, select').first().selectOption('tomato_fspm')
  await page.locator('nav').getByRole('button', { name: '结构' }).click()
  await page.getByRole('button', { name: '3D 拓扑' }).click()

  // 3D canvas 挂载 + 图例里「器官结构」段出现（从契约 structure 派生）
  await expect(page.locator('.topo3d canvas')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('.topo3d .leg-organ')).toBeVisible({ timeout: 10_000 })
  await expect(page.locator('.topo3d .leg-organ-t')).toContainText('器官结构')
  // metamer / fruit 两实体行
  await expect(page.locator('.topo3d .leg-organ')).toContainText('metamer')
  await expect(page.locator('.topo3d .leg-organ')).toContainText('fruit')

  await page.waitForTimeout(600)
  await page.screenshot({ path: 'e2e/__screots/fspm_structure.png', fullPage: false })

  const real = errors.filter((e) => !/webgl|gl_|swiftshader|groupmarker|fallback|gpu|deprecat|context lost|failed to load resource|404|favicon/i.test(e))
  expect(real, real.join('\n')).toHaveLength(0)
})
