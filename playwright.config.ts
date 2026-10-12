import { defineConfig } from '@playwright/test';

// 设置页内置模型指示 E2E：vite web 模式 + 模拟 Tauri IPC（见 tests/e2e/ui/tauri-mock.ts）。
// 使用系统 Edge（channel: msedge）免下载浏览器；端口固定 5177 避开 tauri dev 的 5173。
export default defineConfig({
  testDir: 'tests/e2e/ui',
  fullyParallel: false,
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: 'http://127.0.0.1:5177',
    locale: 'zh-CN',
    // 本机无 Edge 时可 E2E_CHANNEL=chrome 走系统 Chrome（e2e 仅本地跑，CI 未接入）
    channel: (process.env.E2E_CHANNEL ?? 'msedge') as 'msedge' | 'chrome',
    viewport: { width: 1440, height: 900 },
    trace: 'retain-on-failure',
    // 人工过目：E2E_SLOW_MS=300 让每个动作放慢，便于肉眼跟随
    launchOptions: { slowMo: Number(process.env.E2E_SLOW_MS ?? 0) || 0 },
  },
  reporter: [
    ['list'],
    ['html', { outputFolder: 'tests/e2e/ui/reports/html', open: 'never' }],
  ],
  outputDir: 'tests/e2e/ui/reports/artifacts',
  webServer: {
    command: 'npm run dev -- --port 5177 --strictPort',
    url: 'http://127.0.0.1:5177',
    reuseExistingServer: true,
    timeout: 60_000,
  },
  projects: [{ name: 'msedge', use: { browserName: 'chromium' } }],
});
