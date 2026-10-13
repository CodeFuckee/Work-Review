import { expect, test, type Page } from '@playwright/test';
import { baseSpec, installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：时间线工具栏日期快捷按钮（上一天/今天/昨天，与日报页同款）。
// 断言锚点：LocalizedDatePicker 触发器显示的选中日期文本 + 按钮高亮类
// timeline-quick-date-active；E2E-003 用 invoke 记录器验证日期切换驱动数据重载。

function isoDate(offsetDays: number): string {
  const d = new Date();
  d.setDate(d.getDate() + offsetDays);
  const month = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${month}-${day}`;
}

async function openTimeline(page: Page): Promise<void> {
  const spec: MockSpec = {
    ...baseSpec(),
    get_timeline: [
      {
        id: 9001,
        timestamp: Math.floor(Date.now() / 1000) - 3600,
        app_name: '记事本',
        window_title: '背景活动',
        screenshot_path: '',
        ocr_text: null,
        category: 'others',
        duration: 120,
        browser_url: null,
        executable_path: null,
        semantic_category: null,
        semantic_confidence: null,
      },
    ],
    get_hourly_summaries: [],
    get_gitlab_commits_for_date: [],
    get_file_monitor_stats: { total: 0, events: [] },
    get_plane_work_items_for_date: [],
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
  await page.goto('/#/timeline');
}

// 日期选择器触发器（flatpickr 输入框，值为 ISO 日期如 2026-09-29）
function pickerTrigger(page: Page) {
  return page.locator('.page-toolbar .page-control-input');
}

// 人工过目模式：E2E_HOLD_MS=毫秒 时用例结束后停留供肉眼核对（同既有约定）
async function reviewHold(page: Page): Promise<void> {
  const holdMs = Number(process.env.E2E_HOLD_MS ?? 0) || 0;
  if (holdMs > 0) {
    console.log(`[review] 停留 ${holdMs}ms 供人工核对（当前用例页面保持打开）`);
    await page.waitForTimeout(holdMs);
  }
}

test.describe('时间线日期快捷按钮', () => {
  test('E2E-001 默认今天高亮；点昨天后日期切换并切换高亮', async ({ page }) => {
    await openTimeline(page);

    // 默认选中今天：[今天] 高亮、[昨天] 不高亮
    const todayBtn = page.getByRole('button', { name: '今天', exact: true });
    const yesterdayBtn = page.getByRole('button', { name: '昨天', exact: true });
    await expect(todayBtn).toBeVisible();
    await expect(todayBtn).toHaveClass(/timeline-quick-date-active/);
    await expect(yesterdayBtn).not.toHaveClass(/timeline-quick-date-active/);
    await expect(pickerTrigger(page)).toHaveValue(isoDate(0));

    // 点[昨天]：日期选择器显示昨天、高亮切换
    await yesterdayBtn.click();
    await expect(pickerTrigger(page)).toHaveValue(isoDate(-1));
    await expect(yesterdayBtn).toHaveClass(/timeline-quick-date-active/);
    await expect(todayBtn).not.toHaveClass(/timeline-quick-date-active/);
    await reviewHold(page);
  });

  test('E2E-002 上一天连点两次日期回退两天且上一天永不高亮', async ({ page }) => {
    await openTimeline(page);

    const prevBtn = page.getByRole('button', { name: '上一天', exact: true });
    await prevBtn.click();
    await expect(pickerTrigger(page)).toHaveValue(isoDate(-1));
    await prevBtn.click();
    await expect(pickerTrigger(page)).toHaveValue(isoDate(-2));
    // 相对操作按钮自身永不进入高亮态
    await expect(prevBtn).not.toHaveClass(/timeline-quick-date-active/);
    await reviewHold(page);
  });

  test('E2E-003 日期切换驱动数据重载（get_timeline 的 date 参数随选中日期变化）', async ({ page }) => {
    const dates: string[] = [];
    const spec: MockSpec = {
      ...baseSpec(),
      get_timeline: [],
    };
    await installTauriMock(page, spec);
    await page.addInitScript((recorded: string[]) => {
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
      const w = window as unknown as {
        __DATES__: string[];
        __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
      };
      w.__DATES__ = recorded;
      const raw = w.__TAURI_INTERNALS__.invoke.bind(w.__TAURI_INTERNALS__);
      w.__TAURI_INTERNALS__.invoke = (cmd: string, args?: unknown) => {
        if (cmd === 'get_timeline') {
          const date = (args as { date?: string } | undefined)?.date;
          if (date && !recorded.includes(date)) recorded.push(date);
        }
        return raw(cmd, args);
      };
    }, dates);
    await page.goto('/#/timeline');

    const yesterdayBtn = page.getByRole('button', { name: '昨天', exact: true });
    await yesterdayBtn.click();
    await expect(pickerTrigger(page)).toHaveValue(isoDate(-1));
    // 记录器（渲染层之外传引用，evaluate 侧读取）
    const recorded = await page.evaluate(() => {
      const w = window as unknown as { __DATES__?: string[] };
      return w.__DATES__ ?? [];
    });
    expect(recorded.length).toBeGreaterThanOrEqual(2);
    await reviewHold(page);
  });
});
