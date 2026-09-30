import { expect, test, type Page } from '@playwright/test';
import { baseSpec, installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：「今日项目」卡手动归因按钮——点击只重算当日归因（重发
// plane_attributions_for_date），spinner 禁用与失败提示由卡内展示。
// mock 策略：plane_attributions_for_date 动态可变（第 1 次空、之后按窗口内
// __ATTR_MODE__ 返回 filled/failed/pending），驱动真实交互闭环。

const ATTRIBUTIONS = [
  {
    session_start: 0,
    session_end: 4102444800,
    plane_item_id: 'uuid-1',
    display_key: 'PROJ-1',
    project_name: 'Plane演示项目',
    name: '任务一',
    score: 90,
    evidence: 'identifier',
    source: 'deterministic',
  },
];

async function openTimeline(page: Page): Promise<void> {
  const spec: MockSpec = {
    ...baseSpec(),
    get_timeline: [],
    get_hourly_summaries: [],
    get_gitlab_commits_for_date: [],
    get_file_monitor_stats: { total: 0, events: [] },
    get_plane_work_items_for_date: [],
    list_plane_projects: ['Plane演示项目'],
  };
  await installTauriMock(page, spec);
  await page.addInitScript((attributions: unknown[]) => {
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
    // 归因动态 mock：首次调用返回空；点击后按 __ATTR_MODE__ 切换响应
    const w = window as unknown as {
      __ATTR_CALLS__: number;
      __ATTR_MODE__: 'empty' | 'filled' | 'failed' | 'pending';
      __ATTR_RESOLVE__: (() => void) | null;
      __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
    };
    w.__ATTR_CALLS__ = 0;
    w.__ATTR_MODE__ = 'empty';
    w.__ATTR_RESOLVE__ = null;
    const raw = w.__TAURI_INTERNALS__.invoke.bind(w.__TAURI_INTERNALS__);
    w.__TAURI_INTERNALS__.invoke = (cmd: string, args?: unknown) => {
      if (cmd === 'plane_attributions_for_date') {
        w.__ATTR_CALLS__ += 1;
        const mode = w.__ATTR_MODE__;
        if (mode === 'empty') {
          return Promise.resolve({ ok: true, ai_used: false, message: null, attributions: [] });
        }
        if (mode === 'failed') {
          return Promise.resolve({ ok: false, ai_used: false, message: 'boom', attributions: [] });
        }
        if (mode === 'pending') {
          return new Promise((resolve) => {
            w.__ATTR_RESOLVE__ = () =>
              resolve({ ok: true, ai_used: false, message: null, attributions: [] });
          });
        }
        return Promise.resolve({
          ok: true,
          ai_used: false,
          message: null,
          attributions,
        });
      }
      return raw(cmd, args);
    };
  }, ATTRIBUTIONS);
  await page.goto('/#/timeline');
}

function recomputeBtn(page: Page) {
  return page.getByRole('button', { name: '重新归因' });
}

// 人工过目模式：E2E_HOLD_MS=毫秒 时用例结束后停留供肉眼核对（同既有约定）
async function reviewHold(page: Page): Promise<void> {
  const holdMs = Number(process.env.E2E_HOLD_MS ?? 0) || 0;
  if (holdMs > 0) {
    console.log(`[review] 停留 ${holdMs}ms 供人工核对（当前用例页面保持打开）`);
    await page.waitForTimeout(holdMs);
  }
}

test.describe('今日项目卡手动归因按钮', () => {
  test('E2E-001 点击重发归因请求且新归因落到卡上（auto chip 出现）', async ({ page }) => {
    await openTimeline(page);

    // 首次（自动链路）返回空：卡可见（下拉源非空）但无 auto chip
    const card = page.locator('.timeline-projects-card');
    await expect(card).toBeVisible();
    await expect(card.locator('.timeline-projects-chip[data-source="auto"]')).toHaveCount(0);
    const before = await page.evaluate(() => (window as unknown as { __ATTR_CALLS__: number }).__ATTR_CALLS__);

    // 切到 filled 模拟归因更新，点按钮手动重算
    await page.evaluate(() => {
      (window as unknown as { __ATTR_MODE__: string }).__ATTR_MODE__ = 'filled';
    });
    await recomputeBtn(page).click();
    await expect(card.locator('.timeline-projects-chip[data-source="auto"]')).toHaveCount(1);
    await expect(card).toContainText('Plane演示项目');

    const after = await page.evaluate(() => (window as unknown as { __ATTR_CALLS__: number }).__ATTR_CALLS__);
    expect(after).toBeGreaterThan(before);
    await reviewHold(page);
  });

  test('E2E-002 计算中按钮禁用（mock 挂起未返回期间 disabled）', async ({ page }) => {
    await openTimeline(page);

    await page.evaluate(() => {
      (window as unknown as { __ATTR_MODE__: string }).__ATTR_MODE__ = 'pending';
    });
    await recomputeBtn(page).click();
    // 挂起期间按钮禁用且 spinner 出现
    await expect(recomputeBtn(page)).toBeDisabled();
    await expect(page.locator('.timeline-projects-recompute-spinner')).toBeVisible();

    // 放行后恢复可用
    await page.evaluate(() => {
      const w = window as unknown as { __ATTR_RESOLVE__: (() => void) | null };
      w.__ATTR_RESOLVE__?.();
    });
    await expect(recomputeBtn(page)).toBeEnabled();
    await reviewHold(page);
  });

  test('E2E-003 归因失败时失败提示行出现且自动消失', async ({ page }) => {
    await openTimeline(page);

    await page.evaluate(() => {
      (window as unknown as { __ATTR_MODE__: string }).__ATTR_MODE__ = 'failed';
    });
    await recomputeBtn(page).click();
    await expect(page.getByText('归因失败，请稍后重试')).toBeVisible();
    // 4 秒后自动消失
    await expect(page.getByText('归因失败，请稍后重试')).toBeHidden({ timeout: 7000 });
    await reviewHold(page);
  });
});
