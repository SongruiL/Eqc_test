// CommonJS（前端 "type":"module" + Node 18 下 Playwright 无法直接加载 .ts 测试 → 用 .cjs）。
const { test, expect } = require('@playwright/test')

// 前提：外部已在 baseURL 起好 serve（带真 .eqc-secret，AI 已配置）。这些用例打真 Claude。
test.describe('EQC Studio · AI 助手「问AI」端到端', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/v2')
    await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 }) // 连上后端
    await page.getByRole('button', { name: /问AI/ }).click()
    await expect(page.locator('.drawer')).toBeVisible()
  })

  test('自然语言 → 触发工具调用 + 给出回答，无报错', async ({ page }) => {
    await page.locator('.composer textarea').fill('打开仿真工作区，并把变量 Y 选中画出来')
    await page.locator('.composer .send').click()
    // 真 LLM：断言至少冒出一张工具卡 + 一条助手文本（不抠具体内容）。
    await expect(page.locator('.tool').first()).toBeVisible({ timeout: 60_000 })
    await expect(page.locator('.msg.assistant').first()).toBeVisible({ timeout: 60_000 })
    await expect(page.locator('.errbar')).toHaveCount(0)
    // 工具确实是仿真/选中类（放松断言：至少一张匹配的卡）。
    await expect(page.locator('.tool').filter({ hasText: /仿真|选中/ }).first()).toBeVisible({ timeout: 5_000 })
  })

  test('落盘命令触发 confirm 闸（点取消，不真写盘）', async ({ page }) => {
    // 默认模型「草莓 S8」是单作物，可写管理。
    await page.locator('.composer textarea').fill('把当前处理区的管理参数 N_supply 直接写成 5')
    await page.locator('.composer .send').click()
    // 应弹出确认框（danger 命令 save_zone_management 执行前）。
    await expect(page.locator('.confirm')).toBeVisible({ timeout: 60_000 })
    await expect(page.locator('.confirm')).toContainText('落盘')
    // 点「取消」——不真写盘；loop 应优雅继续到结束。
    await page.locator('.confirm .no').click()
    await expect(page.locator('.confirm')).toHaveCount(0)
    await expect(page.locator('.msg.assistant').last()).toBeVisible({ timeout: 30_000 })
  })
})
