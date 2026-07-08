// 进化史工作区冒烟（图论进化 arc 呈现层 Step2）。
// 草莓：版本轨迹带(chips+SVG) + 选中版本 diff + ★标定坑清单(混淆簇+阈值) + 诚实白名单。
// 番茄：cohort/箱车假象脚注可折叠展开(is_last__N 十成员簇)。温室：无 evolution.yaml → 优雅降级。
// 断言放松（有结构 + 有关键文案 + 无运行时错误），不抠具体数字（数据随模型演进会变）。
const { test, expect } = require('@playwright/test')

test('进化史工作区 · 轨迹 + 坑清单 + cohort 脚注 + 优雅降级', async ({ page }) => {
  const errors = []
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()) })
  page.on('pageerror', (e) => errors.push(String(e)))

  await page.goto('/v2')
  await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })

  const modelSel = page.locator('header select, .top select, select').first()

  // —— 草莓：完整视图 ——
  await modelSel.selectOption('strawberry')
  await page.locator('nav').getByRole('button', { name: '进化史', exact: true }).click()

  // ① 版本轨迹带：chips（s1..8.1）+ 轨迹 SVG
  await expect(page.locator('.evo .chip').first()).toBeVisible({ timeout: 15_000 })
  expect(await page.locator('.evo .chip').count(), '版本 chip 数').toBeGreaterThanOrEqual(8)
  await expect(page.locator('.evo svg.traj polyline').first()).toBeVisible()

  // ③ 标定坑清单：混淆系数簇 + 至少一条 clique 条目 + 阈值不可辨识
  await expect(page.getByText('标定坑清单')).toBeVisible()
  await expect(page.locator('.evo .pititem.clique').first()).toBeVisible()
  await expect(page.locator('.evo .pititem.thr').first()).toBeVisible()
  // ④ 诚实白名单
  await expect(page.getByText('诚实白名单可辨识性')).toBeVisible()

  // 点某个早期版本 → diff 卡更新（选 s4 之类）
  await page.locator('.evo .chip', { hasText: 's4' }).click()
  await expect(page.locator('.evo .diff')).toBeVisible()

  // 🎬 演化回放：播放 → 3D canvas 挂载 + 章节指示推进 + 退出
  await page.getByRole('button', { name: /播放进化/ }).click()
  await expect(page.locator('.evo3d canvas')).toBeVisible({ timeout: 15_000 })
  await expect(page.locator('.playbar .ch')).toBeVisible() // 章节指示 k/N
  await page.getByRole('button', { name: /退出/ }).click()

  // —— 番茄：cohort/箱车假象脚注可展开 ——
  await modelSel.selectOption('tomato')
  const foot = page.locator('.evo .foot')
  await expect(foot).toBeVisible({ timeout: 15_000 })
  await foot.click()
  await expect(page.locator('.evo .pititem.art')).toBeVisible()
  await expect(page.getByText(/is_last/)).toBeVisible()

  // —— 温室：无 evolution.yaml → 优雅降级 ——
  await modelSel.selectOption('greenhouse')
  await expect(page.locator('.evo .empty')).toBeVisible({ timeout: 15_000 })
  await expect(page.getByText(/无进化血缘清单/)).toBeVisible()

  const real = errors.filter((e) => !/webgl|gl_|swiftshader|gpu|deprecat|failed to load resource|404|favicon/i.test(e))
  expect(real, real.join('\n')).toHaveLength(0)
})
