import { expect, test, type Page } from '@playwright/test';
import { baseSpec, installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：日报「文件修改」小节（与 GitLab 今日提交平级的独立块
// FILE_EVENTS_TODAY）——渲染、与 GitLab 小节的顺序、空态隐藏、隐藏-恢复交互。
// 后端聚合逻辑由 crates/core/src/file_events_report.rs 单测覆盖，本套只验证
// 前端渲染层（mock get_saved_report 返回带块标记的日报 markdown）。

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function fileBlock(section = true): string {
  if (!section) return '';
  return `<!-- WR_BLOCK_START:FILE_EVENTS_TODAY -->
## 文件修改

新建 2 · 修改 5，共 6 个文件

- \`C:/codes/work-review/src\` — 5 次变动
- 其他 2 个目录 — 2 次变动
<!-- WR_BLOCK_END:FILE_EVENTS_TODAY -->
`;
}

function reportMarkdown(withFileBlock = true): string {
  return `# 工作日报

**日期：${today()}**

<!-- WR_BLOCK_START:LOCAL_OVERVIEW -->
## 今日概览

今日记录 3 条活动。
<!-- WR_BLOCK_END:LOCAL_OVERVIEW -->
<!-- WR_BLOCK_START:GITLAB_TODAY -->
## GitLab 今日提交

- [demo/repo] [main] abc1234 fix: 示例提交
<!-- WR_BLOCK_END:GITLAB_TODAY -->
${fileBlock(withFileBlock)}`;
}

async function openReport(page: Page, withFileBlock = true): Promise<void> {
  const spec: MockSpec = {
    ...baseSpec(),
    // 形状须过 isReportConfig 校验（缺字段时 config 为 null，钉选/隐藏静默失效）
    get_config: {
      ai_mode: 'local',
      daily_report_custom_prompt: '',
      daily_report_prompt_presets: [],
      daily_report_pinned_blocks: [],
      daily_report_hidden_blocks: [],
      daily_report_export_dir: null,
    },
    get_daily_stats: {
      total_duration: 3600,
      screenshot_count: 3,
      app_usage: [],
      category_usage: [],
      browser_duration: 0,
      url_usage: [],
      domain_usage: [],
      domain_total_count: 0,
      browser_usage: [],
      work_time_duration: 3600,
      overtime_duration: 0,
      hourly_activity_distribution: [],
    },
    get_saved_report: {
      date: today(),
      locale: 'zh-CN',
      content: reportMarkdown(withFileBlock),
      ai_mode: 'local',
      model_name: null,
      fallback_reason: null,
      created_at: Math.floor(Date.now() / 1000),
    },
  };
  await installTauriMock(page, spec);
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
  await page.goto('/#/report');
}

// 人工过目模式：E2E_HOLD_MS=毫秒 时用例结束后停留供肉眼核对（同既有约定）
async function reviewHold(page: Page): Promise<void> {
  const holdMs = Number(process.env.E2E_HOLD_MS ?? 0) || 0;
  if (holdMs > 0) {
    console.log(`[review] 停留 ${holdMs}ms 供人工核对（当前用例页面保持打开）`);
    await page.waitForTimeout(holdMs);
  }
}

test.describe('日报文件修改小节', () => {
  test.beforeEach(({ page }) => {
    page.on('console', (msg) => {
      if (msg.type() === 'error') console.log('[console.error]', msg.text());
    });
  });
  test('E2E-001 日报含文件块：渲染出文件修改小节（标题+统计行+目录行）', async ({ page }) => {
    await openReport(page, true);

    const heading = page.getByRole('heading', { name: '文件修改' });
    await expect(heading).toBeVisible();
    // 统计行（仅非零类型 + 去重文件数）
    await expect(page.getByText('新建 2 · 修改 5，共 6 个文件')).toBeVisible();
    // 目录 Top 行与其余合并行
    await expect(page.getByText('C:/codes/work-review/src').first()).toBeVisible();
    await expect(page.getByText(/其他 2 个目录 — 2 次变动/)).toBeVisible();
    await reviewHold(page);
  });

  test('E2E-002 文件修改小节应显示在 GitLab 今日提交之后', async ({ page }) => {
    await openReport(page, true);

    const order = await page.evaluate(() => {
      const headings = Array.from(document.querySelectorAll('h2'));
      const gitlab = headings.findIndex((h) => h.textContent?.includes('GitLab 今日提交'));
      const files = headings.findIndex((h) => h.textContent?.includes('文件修改'));
      return { gitlab, files };
    });
    expect(order.gitlab).toBeGreaterThanOrEqual(0);
    expect(order.files).toBeGreaterThan(order.gitlab);
    await reviewHold(page);
  });

  test('E2E-003 日报无文件块：不出现文件修改小节（空态隐藏）', async ({ page }) => {
    await openReport(page, false);

    await expect(page.getByRole('heading', { name: 'GitLab 今日提交' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '文件修改' })).toHaveCount(0);
    await reviewHold(page);
  });

  test('E2E-004 隐藏文件修改小节后：抽屉恢复列表出现该条目，点击恢复', async ({ page }) => {
    await openReport(page, true);

    const heading = page.getByRole('heading', { name: '文件修改' });
    await expect(heading).toBeVisible();

    // 悬停小节让操作按钮组显现，点「隐藏此段」
    const section = page.locator('.report-section', { has: heading });
    await section.hover();
    await section.getByTitle('隐藏此段').click();
    await expect(page.getByRole('heading', { name: '文件修改' })).toHaveCount(0);

    // 隐藏块后「生成设置」按钮出现，展开抽屉查看恢复列表
    await page.getByRole('button', { name: /生成设置/ }).click();
    await expect(page.getByText('已隐藏的段落')).toBeVisible();
    const restore = page.getByRole('button', { name: /文件修改/ });
    await expect(restore).toBeVisible();

    // 点击恢复：小节回到正文
    await restore.click();
    await expect(page.getByRole('heading', { name: '文件修改' })).toBeVisible();
    await reviewHold(page);
  });
});
