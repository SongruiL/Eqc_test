// 确定性回归（方案 B）：serve 带 EQC_LLM_MOCK=1 时跑。测试在消息里写 [[MOCK 工具 {json}]]
// 指令，mock 后端确定性返回对应 tool_use → 驱动【真】前端 loop/handler/confirm/store。
// 零成本、可重复、可精确断言「点允许后真落盘」等。设 EQC_E2E_REAL=1（真模式）时跳过。
const { test, expect } = require('@playwright/test')

test.describe('Mock LLM · 确定性回归', () => {
  test.skip(process.env.EQC_E2E_REAL === '1', 'mock 用例需 mock-serve（EQC_LLM_MOCK=1）')

  test.beforeEach(async ({ page }) => {
    await page.goto('/v2')
    await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })
    await page.getByRole('button', { name: /问AI/ }).click()
    await expect(page.locator('.drawer')).toBeVisible()
  })

  const send = async (page, text) => {
    await page.locator('.composer textarea').fill(text)
    await page.locator('.composer .send').click()
  }

  test('工具调用 → 真 handler 执行（select_vars）', async ({ page }) => {
    await send(page, '[[MOCK select_vars {"vars":["Y","LAI"]}]] 画出来')
    await expect(page.locator('.tool')).toContainText('选中要画的变量')
    await expect(page.locator('.tres')).toContainText('已选中 Y、LAI') // 真 handler 的返回
    await expect(page.locator('.msg.assistant').last()).toContainText('完成') // tool_result 轮 → end_turn
    await expect(page.locator('.errbar')).toHaveCount(0)
  })

  test('confirm 闸 · 允许 → 真落盘', async ({ page }) => {
    await page.locator('header input.zone').fill('e2etest') // 用测试处理区，不碰真数据
    await send(page, '[[MOCK save_zone_management {"params":{"N_supply":5}}]]')
    await expect(page.locator('.confirm')).toBeVisible()
    await expect(page.locator('.confirm')).toContainText('落盘')
    await page.locator('.confirm .ok').click()
    await expect(page.locator('.tres')).toContainText('已写入') // 真写盘成功
  })

  test('confirm 闸 · 取消 → 不执行', async ({ page }) => {
    await send(page, '[[MOCK save_zone_management {"params":{"N_supply":9}}]]')
    await expect(page.locator('.confirm')).toBeVisible()
    await page.locator('.confirm .no').click()
    await expect(page.locator('.tres')).toContainText('用户取消')
  })

  test('并行多工具', async ({ page }) => {
    await send(page, '[[MOCK describe_model {}]][[MOCK go_simulate {}]]')
    await expect(page.locator('.tool')).toHaveCount(2)
    await expect(page.locator('.errbar')).toHaveCount(0)
  })

  test('后端错误 → 友好提示', async ({ page }) => {
    await send(page, '[[MOCK_ERROR 测试错误信息]]')
    await expect(page.locator('.errbar')).toContainText('测试错误信息')
  })
})
