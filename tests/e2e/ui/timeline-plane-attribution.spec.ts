import { expect, test, type Page } from '@playwright/test';
import { baseSpec, installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：时间线页的 Plane 工作项归因徽标——应用活动行、GitLab 提交行、
// 文件事件行按「时间戳落入归因会话窗口 [start,end]」显示归属工作项徽标。
// 回归背景：提交行/文件事件行原先不做归因 join（bug 修复，2026-09-28），
// 活动行徽标为既有行为（重构后不得回归）。

// 当天本地时间 → 秒级时间戳（mock 数据全部落在今天，页面默认加载今天）
function todayAt(hour: number, minute: number): number {
  const d = new Date();
  d.setHours(hour, minute, 0, 0);
  return Math.floor(d.getTime() / 1000);
}

function attribution(
  startHour: number,
  startMinute: number,
  endHour: number,
  endMinute: number,
): Record<string, unknown> {
  return {
    ok: true,
    ai_used: false,
    message: null,
    attributions: [
      {
        session_start: todayAt(startHour, startMinute),
        session_end: todayAt(endHour, endMinute),
        plane_item_id: 'uuid-proj-12',
        display_key: 'PROJ-12',
        project_name: 'Plane演示项目',
        name: '修复登录跳转',
        score: 90,
        evidence: 'identifier:PROJ-12',
        source: 'deterministic',
      },
    ],
  };
}

function commitAt(hour: number, minute: number): MockSpec {
  return {
    get_gitlab_commits_for_date: [
      {
        sha: 'abc123def4567890abcdef1234567890abcdef12',
        date: new Date().toISOString().slice(0, 10),
        committed_at: todayAt(hour, minute),
        project: 'demo/repo',
        branch: 'main',
        short_id: 'abc123d',
        title: '修复登录页白屏',
        additions: 12,
        deletions: 3,
        author_name: 'tester',
        author_email: 'tester@example.com',
      },
    ],
  };
}

function activityAt(hour: number, minute: number): MockSpec {
  return {
    get_timeline: [
      {
        id: 1,
        timestamp: todayAt(hour, minute),
        app_name: 'VS Code',
        window_title: 'main.rs - work-review',
        screenshot_path: '',
        ocr_text: null,
        category: 'develop',
        duration: 300,
        browser_url: null,
        executable_path: null,
        semantic_category: null,
        semantic_confidence: null,
      },
    ],
  };
}

function fileEventAt(hour: number, minute: number): MockSpec {
  return {
    get_file_monitor_stats: {
      total: 1,
      events: [
        {
          path: 'C:/work/demo/src/login.ts',
          event_type: 'modified',
          size: 2048,
          occurred_at: todayAt(hour, minute),
          date: new Date().toISOString().slice(0, 10),
        },
      ],
    },
  };
}

// 背景活动：列表区渲染要求至少一条应用活动（Timeline.svelte 空态条件为
// activities.length === 0，活动为空时 GitLab 提交行等外部行一并被空态遮蔽）。
// 固定放 08:00 一条并保持它在所有归因窗口之外，避免干扰各用例的徽标断言。
function backgroundActivity(): Array<Record<string, unknown>> {
  return [
    {
      id: 9001,
      timestamp: todayAt(8, 0),
      app_name: '记事本',
      window_title: '背景活动 - 无 notable 归因',
      screenshot_path: '',
      ocr_text: null,
      category: 'others',
      duration: 120,
      browser_url: null,
      executable_path: null,
      semantic_category: null,
      semantic_confidence: null,
    },
  ];
}

function emptyBase(): MockSpec {
  return {
    ...baseSpec(),
    get_timeline: backgroundActivity(),
    get_hourly_summaries: [],
    get_gitlab_commits_for_date: [],
    get_file_monitor_stats: { total: 0, events: [] },
    get_plane_work_items_for_date: [],
  };
}

async function openTimeline(page: Page, spec: MockSpec): Promise<void> {
  await installTauriMock(page, spec);
  // 预置本地登录凭证，绕过启动认证门（App.svelte 无凭证强制跳 /auth；
  // key 对应 src/lib/stores/auth.ts 的 AUTH_KEY，expiresAt 为未来时间即视为已登录）
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

// 人工过目模式：E2E_HOLD_MS=毫秒 时用例结束后停留供肉眼核对（同 builtin-indicator 约定）
async function reviewHold(page: Page): Promise<void> {
  const holdMs = Number(process.env.E2E_HOLD_MS ?? 0) || 0;
  if (holdMs > 0) {
    console.log(`[review] 停留 ${holdMs}ms 供人工核对（当前用例页面保持打开）`);
    await page.waitForTimeout(holdMs);
  }
}

test.describe('时间线 Plane 归因徽标', () => {
  test('E2E-001 提交时间在归因窗口内：提交行显示 Plane 归因徽标', async ({ page }) => {
    // 提交 10:00 落在归因窗口 09:30-10:30 内
    await openTimeline(page, {
      ...emptyBase(),
      ...commitAt(10, 0),
      plane_attributions_for_date: attribution(9, 30, 10, 30),
    });

    // 提交行是三类外部行中唯一的 button（活动行无此修饰类，文件/Plane 行是 div）
    const commitRow = page.locator('button.timeline-entry-commit');
    await expect(commitRow).toBeVisible();

    const badge = commitRow.locator('.timeline-plane-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText('PROJ-12');
    await expect(badge).toContainText('修复登录跳转');
    // 双徽标并排：蓝色项目徽标直接显示纯项目名（不再有「· 项目名」小字）
    const projectBadge = commitRow.locator('.timeline-project-badge');
    await expect(projectBadge).toBeVisible();
    await expect(projectBadge).toHaveText('Plane演示项目');
    // 提交自身信息不受徽标影响
    await expect(commitRow).toContainText('修复登录页白屏');
    await expect(commitRow.locator('.timeline-commit-stats', { hasText: '+12/-3' })).toBeVisible();
    await reviewHold(page);
  });

  test('E2E-002 提交时间在归因窗口外：不显示徽标且提交行正常渲染', async ({ page }) => {
    // 提交 10:00，归因窗口 15:00-16:00 不覆盖
    await openTimeline(page, {
      ...emptyBase(),
      ...commitAt(10, 0),
      plane_attributions_for_date: attribution(15, 0, 16, 0),
    });

    const commitRow = page.locator('button.timeline-entry-commit');
    await expect(commitRow).toBeVisible();
    await expect(commitRow.locator('.timeline-plane-badge')).toHaveCount(0);
    await expect(commitRow).toContainText('修复登录页白屏');
    await expect(commitRow).toContainText('+12/-3');
    await reviewHold(page);
  });

  test('E2E-003 活动时间在归因窗口内：活动行徽标不回归（既有行为保持）', async ({ page }) => {
    // 活动 11:00 落在归因窗口 10:30-11:30 内
    await openTimeline(page, {
      ...emptyBase(),
      ...activityAt(11, 0),
      plane_attributions_for_date: attribution(10, 30, 11, 30),
    });

    const activityRow = page.locator('.timeline-entry-title', { hasText: 'main.rs - work-review' });
    await expect(activityRow).toBeVisible();
    const badge = activityRow.locator('.timeline-plane-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText('PROJ-12');
    const projectBadge = activityRow.locator('.timeline-project-badge');
    await expect(projectBadge).toBeVisible();
    await expect(projectBadge).toHaveText('Plane演示项目');
    await reviewHold(page);
  });

  test('E2E-004 文件事件时间在归因窗口内：文件事件行显示 Plane 归因徽标', async ({ page }) => {
    // 文件事件 12:00 落在归因窗口 11:30-12:30 内
    await openTimeline(page, {
      ...emptyBase(),
      ...fileEventAt(12, 0),
      plane_attributions_for_date: attribution(11, 30, 12, 30),
    });

    const fileRow = page.locator('div.timeline-entry-commit', { hasText: 'C:/work/demo/src/login.ts' });
    await expect(fileRow).toBeVisible();
    const badge = fileRow.locator('.timeline-plane-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText('PROJ-12');
    await expect(badge).toContainText('修复登录跳转');
    const projectBadge = fileRow.locator('.timeline-project-badge');
    await expect(projectBadge).toBeVisible();
    await expect(projectBadge).toHaveText('Plane演示项目');
    await reviewHold(page);
  });

  test('E2E-005 徽标加入后提交行仍可点击打开详情抽屉（相邻主流程回归）', async ({ page }) => {
    await openTimeline(page, {
      ...emptyBase(),
      ...commitAt(10, 0),
      plane_attributions_for_date: attribution(9, 30, 10, 30),
    });

    const commitRow = page.locator('button.timeline-entry-commit');
    await expect(commitRow).toBeVisible();
    await commitRow.click();

    const drawer = page.getByRole('dialog');
    await expect(drawer).toBeVisible();
    await expect(drawer).toContainText('修复登录页白屏');
    await reviewHold(page);
  });
});
