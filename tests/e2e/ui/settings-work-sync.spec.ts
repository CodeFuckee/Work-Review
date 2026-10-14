import { expect, test, type Page } from '@playwright/test';
import { baseSpec, installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：设置 → 常规 →「工作记录同步（多机）」卡 + 日报页「来源机器」标注。
// 多机同步的真实推拉链路（加密/服务端/LWW）由 tests/integration/work-sync-roundtrip.mjs
// 联调脚本与 xUnit/单测覆盖；本套只验证用户可见界面行为：
// 卡渲染与开关交互、同步就绪态提示、手动同步的完整 mock 链路、报告来源 pill。

const CARD_TITLE = '工作记录同步（多机）';
const MACHINE_NAME_LABEL = '本机名称';
const SYNC_NOW_TEXT = '立即同步';
const SYNC_SKIPPED_TEXT = '当前未启用同步或未登录，已跳过';
const SYNC_DONE_PREFIX = '同步完成：推送 0 块';
const MACHINE_ID_PREFIX = '机器标识：abcd1234';

/** 预置本地登录凭证；withSyncKey=false 时模拟「未重新派生同步密钥」的旧登录态 */
async function seedAuth(page: Page, withSyncKey = true): Promise<void> {
  await page.addInitScript((withKey: boolean) => {
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
    if (!withKey) return;
    const bytes = new Uint8Array(32).fill(7);
    let bin = '';
    bytes.forEach((b) => { bin += String.fromCharCode(b); });
    localStorage.setItem('yue-review-sync-key', btoa(bin));
  }, withSyncKey);
}

function workSyncCardSpec(overrides: MockSpec = {}, config: MockSpec = {}): MockSpec {
  return {
    ...baseSpec(),
    get_config: {
      work_sync_enabled: false,
      machine_name: '',
      ...(config as Record<string, unknown>),
    },
    work_sync_get_state: { machine_id: 'abcd1234-5678-90ef-abcd-1234567890ef' },
    ...overrides,
  };
}

async function openSettings(page: Page, spec: MockSpec, withSyncKey = true): Promise<void> {
  await installTauriMock(page, spec);
  await seedAuth(page, withSyncKey);
  await page.goto('/#/settings');
  await expect(page.getByText(CARD_TITLE)).toBeVisible();
}

test('E2E-001 同步卡渲染：关闭态隐藏明细且立即同步禁用', async ({ page }) => {
  await openSettings(page, workSyncCardSpec());

  await expect(page.getByText(MACHINE_NAME_LABEL)).toHaveCount(0, '开关关闭时不出现机器名输入');
  await expect(page.getByText(MACHINE_ID_PREFIX, { exact: false })).toHaveCount(0);
  await expect(page.getByRole('button', { name: SYNC_NOW_TEXT })).toBeDisabled();
});

test('E2E-002 已配置启用态：机器标识/回填机器名可见且立即同步可用', async ({ page }) => {
  await openSettings(
    page,
    workSyncCardSpec({}, { work_sync_enabled: true, machine_name: '公司电脑' }),
  );

  await expect(page.getByText(MACHINE_ID_PREFIX, { exact: false })).toBeVisible();
  await expect(page.locator('#work-sync-machine-name')).toHaveValue('公司电脑');
  await expect(page.getByRole('button', { name: SYNC_NOW_TEXT })).toBeEnabled();

  // 可改写：改名不丢单（blur 后保持）
  await page.locator('#work-sync-machine-name').fill('家里电脑');
  await page.locator('#work-sync-machine-name').blur();
  await expect(page.locator('#work-sync-machine-name')).toHaveValue('家里电脑');
});

test('E2E-003 同步密钥未就绪（旧登录态）时手动同步应提示跳过而非假成功', async ({ page }) => {
  await openSettings(
    page,
    workSyncCardSpec({}, { work_sync_enabled: true, machine_name: '公司电脑' }),
    false,
  );

  await page.getByRole('button', { name: SYNC_NOW_TEXT }).click();
  await expect(page.getByText(SYNC_SKIPPED_TEXT)).toBeVisible();
});

test('E2E-004 完整就绪态手动同步应走完引擎链路并提示完成', async ({ page }) => {
  // 引擎链路 mock：本地库无待推活动 + 云端无新块 + 无日报变更 → 零增量完成
  const spec = workSyncCardSpec(
    {
      work_sync_export_batch: { activities: [], ids: [], max_id: 0, has_more: false },
      get_reports_in_range_for_sync: [],
      apply_remote_report: null,
      client_http_post_json: {
        delivered: true,
        status: 200,
        body: JSON.stringify({
          data: { blocks: [], latestSeqByMachine: {}, reports: [], latestUpdatedAt: 0 },
        }),
      },
    },
    { work_sync_enabled: true, machine_name: '公司电脑' },
  );
  await openSettings(page, spec);

  await page.getByRole('button', { name: SYNC_NOW_TEXT }).click();
  await expect(page.getByText(SYNC_DONE_PREFIX, { exact: false })).toBeVisible();
  await expect(page.getByText(/上次同步/)).toBeVisible();
});

test('E2E-005 回归：云端同步落地日报应在报告页显示来源机器标注', async ({ page }) => {
  const spec: MockSpec = {
    ...baseSpec(),
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
      date: new Date().toISOString().slice(0, 10),
      locale: 'zh-CN',
      content: '# 工作日报\n\n由公司电脑生成的内容。',
      ai_mode: 'local',
      model_name: null,
      fallback_reason: null,
      created_at: Math.floor(Date.now() / 1000),
      updated_at: Date.now(),
      origin_machine: '公司电脑',
    },
  };
  await installTauriMock(page, spec);
  await seedAuth(page);
  await page.goto('/#/report');

  const pill = page.getByText('来自 公司电脑', { exact: false });
  await expect(pill).toBeVisible();
  // 本机生成的日报不显示该标注
  await expect(page.getByText('来自 家里电脑')).toHaveCount(0);
});
