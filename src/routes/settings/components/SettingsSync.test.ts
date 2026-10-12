import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// 被测功能：设置页「配置同步」卡的 API 认证失败处理。
// 事故（2026-10-11 用户报告）：token 失效时服务端返回 200 + {success:false, 全空}，
// 客户端只看 HTTP 状态码——push 弹「上传成功」假成功、状态恒显「云端暂无配置」，
// 用户以为上传成功实际什么都没存。修复契约：success:false 必须识别为认证失败，
// 三个入口（状态刷新/拉取/上传）都要引导重新登录，绝不能当空配置处理。

test('同步卡应识别 success:false 认证失败并在 UI 引导重新登录', async () => {
  const source = await readFile(new URL('./SettingsSync.svelte', import.meta.url), 'utf8');

  // 三个 HTTP 入口都必须先过认证失败判定（复用 configSync 的纯函数）
  assert.match(source, /import \{[\s\S]*?isSyncAuthFailure[\s\S]*?\} from '\$lib\/utils\/configSync\.ts'/);
  const refreshFn = source.indexOf('async function refreshCloudStatus');
  const pullFn = source.indexOf('async function handlePull');
  const pushFn = source.indexOf('async function handlePush');
  assert.ok(refreshFn > 0 && pullFn > refreshFn && pushFn > pullFn, '三个入口函数应按序存在');
  for (const [name, start, end] of [
    ['refreshCloudStatus', refreshFn, pullFn],
    ['handlePull', pullFn, pushFn],
    ['handlePush', pushFn, source.indexOf("function diffStateLabel")],
  ] as const) {
    const body = source.slice(start, end);
    assert.match(body, /isSyncAuthFailure/, `${name} 必须检查 success:false 认证失败（旧版服务端 200+Fail 语义）`);
    assert.match(body, /status === 401/, `${name} 必须处理 HTTP 401（新版服务端语义）`);
  }
});

test('上传遇到认证失败时禁止假成功（不得弹出上传成功）', async () => {
  const source = await readFile(new URL('./SettingsSync.svelte', import.meta.url), 'utf8');
  const pushFn = source.slice(
    source.indexOf('async function handlePush'),
    source.indexOf("function diffStateLabel"),
  );
  // 认证失败分支必须先于成功 toast 出现，且命中后直接返回
  const authFail = pushFn.indexOf('isSyncAuthFailure');
  const successToast = pushFn.indexOf("t('settingsSync.pushDone')");
  assert.ok(authFail >= 0 && successToast > authFail, '认证失败判定应先于上传成功 toast');
  assert.match(pushFn, /isSyncAuthFailure\(parsed\.data\)[\s\S]{0,200}return/, '认证失败应直接返回，不得继续走成功路径');
});

test('认证失败后状态区应显示重新登录提示而非「云端暂无配置」灰标', async () => {
  const source = await readFile(new URL('./SettingsSync.svelte', import.meta.url), 'utf8');
  // authExpired 状态存在
  assert.match(source, /let authExpired = false/);
  // 状态刷新：认证失败置位 authExpired 且不得把空 data 当作 cloudMeta
  const refreshFn = source.slice(
    source.indexOf('async function refreshCloudStatus'),
    source.indexOf('onMount('),
  );
  assert.match(refreshFn, /isSyncAuthFailure\(parsed\.data\)[\s\S]{0,200}authExpired = true/);
  assert.match(refreshFn, /authExpired = true[\s\S]{0,200}cloudMeta = null/);
  // 模板：状态行内 authExpired 提示先于灰标分支（操作按钮保留，上传可再试出警告）
  const authExpiredBranch = source.indexOf('{#if authExpired}');
  const grayTag = source.indexOf('type="gray"');
  assert.ok(authExpiredBranch > 0, '状态行应有 authExpired 独立分支');
  assert.ok(grayTag < 0 || grayTag > authExpiredBranch, '重新登录提示应优先于云端暂无配置灰标');
  assert.match(source, /\{#if authExpired\}[\s\S]{0,300}t\('settingsSync\.authExpired'\)/);
});
