// CommonJS 配置（前端 package.json 是 "type":"module"，Node 会把 .ts/.js 当 ESM 加载、
// Playwright 配置加载会炸；用 .cjs 绕开。测试文件 .ts 由 Playwright 自带的 TS 转译处理，不受此限）。
const { defineConfig } = require('@playwright/test')

// e2e：默认用系统 Edge（channel=msedge，免下 ~150MB 浏览器，适配 Windows 无管理员+网络受限机器）。
// Linux/CI 无 Edge 时设 `EQC_E2E_CHANNEL=none` 走 Playwright 自带 chromium（先 npx playwright install chromium）。
// serve 由外部脚本预先在 7884 起好（带真 .eqc-secret），这里只连。全程 localhost、--no-proxy-server
// 绕开机器那个破 TLS 的系统代理。真 LLM → 断言放松（断有工具卡/有回答/无报错，不抠具体字）。
const CH = process.env.EQC_E2E_CHANNEL ?? 'msedge' // 设 'none' = 用自带 chromium（不设 channel）
module.exports = defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: process.env.EQC_E2E_BASE || 'http://localhost:7884',
    ...(CH === 'none' ? {} : { channel: CH }),
    headless: true,
    actionTimeout: 15_000,
    // --no-proxy-server 绕机器破代理；--enable-unsafe-swiftshader 让 headless 软件 WebGL 可用（GA-6 3D 视图冒烟）。
    launchOptions: { args: ['--no-proxy-server', '--enable-unsafe-swiftshader'] },
  },
})
