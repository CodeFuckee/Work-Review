import { expect, test, type Page } from '@playwright/test';
import { installTauriMock, type MockSpec } from './tauri-mock';

// 被测功能：设置 → 常规 →「配置同步（跨设备）」卡的认证失败处理。
// 事故（2026-10-11）：token 失效时服务端返回 200 + {success:false, 全空载荷}
// （新版已改为 HTTP 401），客户端只看 HTTP 状态码把认证失败当成「云端暂无配置」
// ——上传假成功、状态恒空。修复契约：认证失败必须显示重新登录提示 + 上传弹警告，
// 真空配置（success:true）的灰标语义保持不变。

const AUTH_EXPIRED_TEXT = '登录已过期或无效，请重新登录后再同步';
const CLOUD_EMPTY_TEXT = '云端暂无配置';
const PUSH_BUTTON_TEXT = '上传到云端';
const PUSH_DONE_TEXT = '已上传到云端';

/** 预置本地登录凭证 + 同步密钥，绕过启动认证门并让同步卡可达操作态 */
async function seedSyncReady(page: Page): Promise<void> {
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
    // 32 字节 AES-256 原始密钥的 base64（getStoredSyncKey 走 importKey('raw')）
    const bytes = new Uint8Array(32).fill(7);
    let bin = '';
    bytes.forEach((b) => { bin += String.fromCharCode(b); });
    localStorage.setItem('yue-review-sync-key', btoa(bin));
  });
}

function syncSpec(statusResponse: unknown): MockSpec {
  return {
    // 含 GitLab 块：让 extractSyncPayload 有内容可传（否则上传弹「本机没有可上传」）
    get_config: { gitlab: { url: 'https://gitlab.example.com', token: 'e2e' } },
    client_http_get_json: statusResponse,
  };
}

// 旧版服务端语义：200 + success:false（未认证）
const OLD_SERVER_AUTH_FAIL = {
  delivered: true,
  status: 200,
  body: JSON.stringify({ data: { success: false, payload: null, iv: null, updatedTime: null } }),
};
// 新版服务端语义：HTTP 401
const NEW_SERVER_401 = { delivered: true, status: 401, error: '' };
// 正常：有云端配置
const NORMAL_HAS_CONFIG = {
  delivered: true,
  status: 200,
  body: JSON.stringify({
    data: { success: true, payload: 'cipher', iv: 'iv', updatedTime: '2026-10-11 12:00:00' },
  }),
};
// 正常：从未上传（真空配置）
const NORMAL_EMPTY = {
  delivered: true,
  status: 200,
  body: JSON.stringify({ data: { success: true, payload: null, iv: null, updatedTime: null } }),
};

async function openSettings(page: Page, spec: MockSpec): Promise<void> {
  await installTauriMock(page, spec);
  await seedSyncReady(page);
  await page.goto('/#/settings');
  // 等同步卡状态行渲染（general 为默认 Tab）
  await expect(page.getByText('云端配置', { exact: true })).toBeVisible();
}

test('E2E-001 旧版服务端 200+success:false 应显示重新登录提示而非云端暂无配置灰标', async ({ page }) => {
  await openSettings(page, syncSpec(OLD_SERVER_AUTH_FAIL));

  await expect(page.getByText(AUTH_EXPIRED_TEXT)).toBeVisible();
  await expect(page.getByText(CLOUD_EMPTY_TEXT)).toHaveCount(0);
});

test('E2E-002 同场景点击上传到云端应弹警告而非假成功', async ({ page }) => {
  await openSettings(page, syncSpec(OLD_SERVER_AUTH_FAIL));

  await page.getByRole('button', { name: PUSH_BUTTON_TEXT }).click();
  // toast 与状态行提示同文案，至少一处可见即可；关键是绝不能出现上传成功
  await expect(page.getByText(AUTH_EXPIRED_TEXT).first()).toBeVisible();
  await expect(page.getByText(PUSH_DONE_TEXT)).toHaveCount(0);
});

test('E2E-003 新版服务端 HTTP 401 应显示重新登录提示', async ({ page }) => {
  await openSettings(page, syncSpec(NEW_SERVER_401));

  await expect(page.getByText(AUTH_EXPIRED_TEXT)).toBeVisible();
  await expect(page.getByText(CLOUD_EMPTY_TEXT)).toHaveCount(0);
});

test('E2E-004 回归：正常有配置应显示更新时间蓝标', async ({ page }) => {
  await openSettings(page, syncSpec(NORMAL_HAS_CONFIG));

  await expect(page.getByText('更新于 2026-10-11 12:00:00')).toBeVisible();
  await expect(page.getByText(CLOUD_EMPTY_TEXT)).toHaveCount(0);
  await expect(page.getByText(AUTH_EXPIRED_TEXT)).toHaveCount(0);
});

test('E2E-005 回归：正常空配置应保留云端暂无配置灰标', async ({ page }) => {
  await openSettings(page, syncSpec(NORMAL_EMPTY));

  await expect(page.getByText(CLOUD_EMPTY_TEXT)).toBeVisible();
  await expect(page.getByText(AUTH_EXPIRED_TEXT)).toHaveCount(0);
});
