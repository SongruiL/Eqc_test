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
