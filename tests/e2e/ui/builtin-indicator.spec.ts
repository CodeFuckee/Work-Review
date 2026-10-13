import { expect, test, type Page } from '@playwright/test';
import {
  baseSpec,
  customTextModel,
  factoryDefaultTextModel,
  factoryDefaultTextModelOllama,
  installTauriMock,
  OLLAMA_PROVIDER_ENTRY_ONLY,
  type MockSpec,
} from './tauri-mock';

// 被测功能：设置 → AI 模型分区顶部的「内置模型指示卡」。
// 卡片显隐由 has_builtin_model（编译期注入）决定；文案由「是否正在使用出厂默认配置」切换。
// 本地开发二进制未注入 key 时 has_builtin_model=false —— E2E-003 即对该场景的正确复现。

const CARD_TEXT_ACTIVE = '软件已内置 AI 模型，无需配置即可使用 AI 功能';
const CARD_TEXT_AVAILABLE = '当前优先使用你配置的模型';
const PROVIDER_LABEL = '提供商';
const MODE_LEGEND = '日报模式';

// 预置本地登录凭证绕过启动认证门（App.svelte 无凭证强制跳 /auth；该门非 IPC，
// mock 拦不住——缺种子时用例依赖 onMount 前完成断言的时序竞态，冷启动必挂）
async function seedAuth(page: Page): Promise<void> {
  await page.addInitScript(() => {
    localStorage.setItem(
      'yue-review-auth',
      JSON.stringify({
        token: 'e2e-fake-token',
        userId: 1,
        userName: 'e2e',
        name: 'E2E',
        expiresAt: Math.floor(Date.now() / 1000) + 3600,
      }),
    );
  });
}

async function openAiTab(page: Page): Promise<void> {
  await seedAuth(page);
  await page.goto('/#/settings');
  // MD 迁移：设置页签为 SMUI Tab（role=tab 而非 button）
  await expect(page.getByRole('tab', { name: 'AI 模型' })).toBeVisible();
  await page.getByRole('tab', { name: 'AI 模型' }).click();
}

// 人工过目模式：E2E_HOLD_MS=毫秒 时，用例结束后浏览器停留指定时长供肉眼核对卡片。
// 默认 0（直接跑完，供 CI/无头自测）。示例：E2E_HOLD_MS=12000 npx playwright test --headed
async function reviewHold(page: Page): Promise<void> {
  const holdMs = Number(process.env.E2E_HOLD_MS ?? 0) || 0;
  if (holdMs > 0) {
    console.log(`[review] 停留 ${holdMs}ms 供人工核对（当前用例页面保持打开）`);
    await page.waitForTimeout(holdMs);
  }
}

test.describe('设置页内置模型指示卡', () => {
  test('E2E-001 内置模型+出厂默认配置：卡片可见且显示"无需配置"文案', async ({ page }) => {
    const spec: MockSpec = { ...baseSpec(), ...factoryDefaultTextModel() };
    await installTauriMock(page, spec);

    await openAiTab(page);

    const card = page.getByText(CARD_TEXT_ACTIVE, { exact: false });
    await expect(card).toBeVisible();
    await expect(page.getByText(/第三方/)).toBeVisible();
    // 卡片应位于「日报模式」切换之上：用户进 AI 设置第一眼就能看到
    const cardBox = await card.boundingBox();
    const legendBox = await page.getByText(MODE_LEGEND, { exact: true }).boundingBox();
    expect(cardBox && legendBox && cardBox.y < legendBox.y).toBe(true);
    // 内置供应商卡片可见且选中，字段隐藏（只显示说明，端点输入不存在）
    await expect(page.getByText('内置模型', { exact: true })).toBeVisible();
    await expect(page.getByText('内置模型无需任何配置')).toBeVisible();
    await expect(page.locator('#ai-endpoint')).toHaveCount(0);
    await reviewHold(page);
  });

  test('E2E-002 内置模型+已配置自定义模型（AI 增强分支）：卡片可见且显示"优先使用你的"文案', async ({ page }) => {
    const spec: MockSpec = { ...baseSpec(), ...customTextModel() };
    await installTauriMock(page, spec);

    await openAiTab(page);

    await expect(page.getByText(CARD_TEXT_AVAILABLE, { exact: false })).toBeVisible();
    // 此时不应显示"正在使用内置"版本文案
    await expect(page.getByText(CARD_TEXT_ACTIVE, { exact: false })).toHaveCount(0);
    // AI 增强分支：字段可见（端点输入），内置说明不显示
    await expect(page.locator('#ai-endpoint')).toBeVisible();
    await expect(page.getByText('内置模型无需任何配置')).toHaveCount(0);
    await reviewHold(page);
  });

  test('E2E-003 复现：内置模型不可用（本地开发未注入 key 的真实场景）时卡片不可见', async ({ page }) => {
    const spec: MockSpec = {
      ...baseSpec({
        has_builtin_model: false,
        get_ai_providers: [OLLAMA_PROVIDER_ENTRY_ONLY],
      }),
      ...factoryDefaultTextModelOllama(),
    };
    await installTauriMock(page, spec);

    await openAiTab(page);

    await expect(page.getByText(CARD_TEXT_ACTIVE, { exact: false })).toHaveCount(0);
    await expect(page.getByText(CARD_TEXT_AVAILABLE, { exact: false })).toHaveCount(0);
    // 供应商网格中不应出现内置模型卡片
    await expect(page.getByText('内置模型', { exact: true })).toHaveCount(0);
    // 页面其余部分正常：日报模式切换仍在
    await expect(page.getByText(MODE_LEGEND, { exact: true })).toBeVisible();
    await reviewHold(page);
  });

  test('E2E-004 纯 web 降级（无任何 Tauri IPC）：配置加载失败且卡片不可见', async ({ page }) => {
    // 不注入 mock：web 模式下 get_config 失败，设置页进入「加载失败」错误态（设计内降级），
    // 分区不渲染，指示卡自然不存在
    await seedAuth(page);
    await page.goto('/#/settings');
    await expect(page.getByText('加载配置失败')).toBeVisible({ timeout: 10_000 });

    await expect(page.getByText(CARD_TEXT_ACTIVE, { exact: false })).toHaveCount(0);
    await expect(page.getByText(CARD_TEXT_AVAILABLE, { exact: false })).toHaveCount(0);
    await reviewHold(page);
  });
});
