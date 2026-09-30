import { expect, test, type Page } from '@playwright/test';
import { baseSpec, installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：时间线「今日项目」归因卡（TimelineProjectsCard）——auto chip（蓝）
// 来自归因聚合、manual chip（紫）来自手动添加；删除 = 当日屏蔽；未配置 Plane
// （下拉源空且无归因）整卡隐藏。
// mock 策略：get/set_daily_project_adjustment 在 init script 内维护可变数组
// （set 更新状态 → 组件重查 get 得到新清单），驱动真实交互闭环。

const PLANE_PROJECTS = ['Plane演示项目', '测试项目', '新项目候选'];
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
  {
    session_start: 0,
    session_end: 4102444800,
    plane_item_id: 'uuid-2',
    display_key: 'TEST-1',
    project_name: '测试项目',
    name: '任务二',
    score: 90,
    evidence: 'identifier',
    source: 'deterministic',
  },
];

interface CardMockOptions {
  withAttributions: boolean;
  planeProjects: string[];
}

async function openTimelineWithCard(
  page: Page,
  options: CardMockOptions = { withAttributions: true, planeProjects: PLANE_PROJECTS },
): Promise<void> {
  const spec: MockSpec = {
    ...baseSpec(),
    get_timeline: [],
    get_hourly_summaries: [],
    get_gitlab_commits_for_date: [],
    get_file_monitor_stats: { total: 0, events: [] },
    get_plane_work_items_for_date: [],
    plane_attributions_for_date: {
      ok: true,
      ai_used: false,
      message: null,
      attributions: options.withAttributions ? ATTRIBUTIONS : [],
    },
    list_plane_projects: options.planeProjects,
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
    // 动态调整状态：set 更新数组，get 返回快照（模拟按日表）
    const w = window as unknown as {
      __ADJ__: Array<{ project_name: string; removed: boolean }>;
      __SET_CALLS__: Array<{ date: string; projectName: string; removed: boolean }>;
      __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
    };
    w.__ADJ__ = [];
    w.__SET_CALLS__ = [];
    const raw = w.__TAURI_INTERNALS__.invoke.bind(w.__TAURI_INTERNALS__);
    w.__TAURI_INTERNALS__.invoke = (cmd: string, args?: unknown) => {
      if (cmd === 'get_daily_project_adjustments') {
        return Promise.resolve([...w.__ADJ__]);
      }
      if (cmd === 'set_daily_project_adjustment') {
        const a = (args ?? {}) as { date?: string; projectName?: string; removed?: boolean };
        w.__SET_CALLS__.push({
          date: a.date ?? '',
          projectName: a.projectName ?? '',
          removed: Boolean(a.removed),
        });
        w.__ADJ__ = w.__ADJ__.filter((row) => row.project_name !== a.projectName);
        w.__ADJ__.push({ project_name: a.projectName ?? '', removed: Boolean(a.removed) });
        return Promise.resolve(null);
      }
      return raw(cmd, args);
    };
  });
  await page.goto('/#/timeline');
}

function card(page: Page) {
  return page.locator('.timeline-projects-card');
}

// 人工过目模式：E2E_HOLD_MS=毫秒 时用例结束后停留供肉眼核对（同既有约定）
async function reviewHold(page: Page): Promise<void> {
  const holdMs = Number(process.env.E2E_HOLD_MS ?? 0) || 0;
  if (holdMs > 0) {
    console.log(`[review] 停留 ${holdMs}ms 供人工核对（当前用例页面保持打开）`);
    await page.waitForTimeout(holdMs);
  }
}

test.describe('时间线今日项目卡', () => {
  test('E2E-001 归因聚合出两枚 auto chip（蓝色）且卡可见', async ({ page }) => {
    await openTimelineWithCard(page, { withAttributions: true, planeProjects: PLANE_PROJECTS });

    await expect(card(page)).toBeVisible();
    await expect(card(page)).toContainText('今日项目');
    const autoChips = card(page).locator('.timeline-projects-chip[data-source="auto"]');
    await expect(autoChips).toHaveCount(2);
    await expect(card(page)).toContainText('Plane演示项目');
    await expect(card(page)).toContainText('测试项目');
    await reviewHold(page);
  });

  test('E2E-002 添加项目：调 set(removed=false) 且紫色 manual chip 出现', async ({ page }) => {
    await openTimelineWithCard(page, { withAttributions: true, planeProjects: PLANE_PROJECTS });

    await card(page).getByRole('button', { name: /添加项目/ }).click();
    const select = card(page).locator('.timeline-projects-select');
    await expect(select).toBeVisible();
    await select.selectOption('新项目候选');
    await card(page).locator('.timeline-projects-editor-confirm').click();

    const manualChip = card(page).locator('.timeline-projects-chip[data-source="manual"]');
    await expect(manualChip).toHaveCount(1);
    await expect(manualChip).toContainText('新项目候选');

    const calls = await page.evaluate(() => {
      const w = window as unknown as { __SET_CALLS__: Array<{ removed: boolean; projectName: string }> };
      return w.__SET_CALLS__;
    });
    expect(calls).toContainEqual(
      expect.objectContaining({ projectName: '新项目候选', removed: false }),
    );
    await reviewHold(page);
  });

  test('E2E-003 删除 auto 项目：调 set(removed=true) 且 chip 消失', async ({ page }) => {
    await openTimelineWithCard(page, { withAttributions: true, planeProjects: PLANE_PROJECTS });

    const chip = card(page).locator('.timeline-projects-chip', { hasText: '测试项目' });
    await chip.hover();
    await chip.getByLabel('删除 测试项目').click();

    await expect(
      card(page).locator('.timeline-projects-chip', { hasText: '测试项目' }),
    ).toHaveCount(0);
    // 归因仍在（mock 未变），但清单被 removed 屏蔽——证明生效的是调整而非归因消失
    await expect(card(page)).toContainText('Plane演示项目');

    const calls = await page.evaluate(() => {
      const w = window as unknown as { __SET_CALLS__: Array<{ removed: boolean; projectName: string }> };
      return w.__SET_CALLS__;
    });
    expect(calls).toContainEqual(expect.objectContaining({ projectName: '测试项目', removed: true }));
    await reviewHold(page);
  });

  test('E2E-004 未配置 Plane（下拉源空且无归因）时整卡隐藏', async ({ page }) => {
    await openTimelineWithCard(page, { withAttributions: false, planeProjects: [] });

    await expect(card(page)).toHaveCount(0);
    await reviewHold(page);
  });
});
