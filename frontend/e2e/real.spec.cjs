// 真 LLM 冒烟（方案 A）：默认跳过；设 EQC_E2E_REAL=1 且 serve 带真 .eqc-secret 时才跑。
// 真模型非确定性 → 断言放松（有工具卡/有回答/无报错）。花几分钱，留作真链路抽检。
const { test, expect } = require('@playwright/test')

test.describe('真 LLM 冒烟（EQC_E2E_REAL=1 才跑）', () => {
  test.skip(process.env.EQC_E2E_REAL !== '1', '真 LLM 冒烟：需 EQC_E2E_REAL=1 + 真 key serve')

  test.beforeEach(async ({ page }) => {
    await page.goto('/v2')
    await expect(page.locator('.status.ok')).toBeVisible({ timeout: 20_000 })
    await page.getByRole('button', { name: /问AI/ }).click()
    await expect(page.locator('.drawer')).toBeVisible()
  })

  test('自然语言 → 触发工具调用 + 回答，无报错', async ({ page }) => {
    await page.locator('.composer textarea').fill('打开仿真工作区，并把变量 Y 选中画出来')
    await page.locator('.composer .send').click()
    await expect(page.locator('.tool').first()).toBeVisible({ timeout: 60_000 })
    await expect(page.locator('.msg.assistant').first()).toBeVisible({ timeout: 60_000 })
    await expect(page.locator('.errbar')).toHaveCount(0)
    await expect(page.locator('.tool').filter({ hasText: /仿真|选中/ }).first()).toBeVisible({ timeout: 5_000 })
  })

  test('落盘命令触发 confirm 闸（点取消，不真写盘）', async ({ page }) => {
    await page.locator('.composer textarea').fill('把当前处理区的管理参数 N_supply 直接写成 5')
    await page.locator('.composer .send').click()
    await expect(page.locator('.confirm')).toBeVisible({ timeout: 60_000 })
    await expect(page.locator('.confirm')).toContainText('落盘')
    await page.locator('.confirm .no').click()
    await expect(page.locator('.confirm')).toHaveCount(0)
    await expect(page.locator('.msg.assistant').last()).toBeVisible({ timeout: 30_000 })
  })
})
