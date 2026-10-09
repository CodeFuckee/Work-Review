import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('存储设置应提供独立的启用截图开关，而不只是截图间隔', async () => {
  const source = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /settingsStorage\.screenshotsEnabled/);
  assert.match(source, /config\.storage\.screenshots_enabled/);
  assert.match(source, /settingsStorage\.screenshotsEnabledHint/);
  assert.match(source, /<Toggle/);
  assert.match(source, /labelText=\{t\('settingsStorage\.screenshotsEnabled'\)\}/);
  assert.match(source, /toggled=\{config\.storage\.screenshots_enabled\}/);
});

test('存储设置数值输入和自动导出开关应提供本地化可访问名称', async () => {
  const source = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  for (const key of ['pollingInterval', 'retentionDays', 'storageLimitLabel']) {
    assert.match(source, new RegExp(`aria-label=\\{t\\('settingsStorage\\.${key}'\\)\\}`));
  }
  // 自动导出开关已迁移 Carbon Toggle
  assert.match(source, /labelText=\{t\('settingsStorage\.autoExport'\)\}/);
  assert.match(source, /toggled=\{config\.daily_report_auto_export\}/);
  assert.equal((source.match(/settingsStorage\.(?:showSecret|hideSecret)/g) || []).length, 6);
  assert.match(source, /aria-label=\{`\$\{t\(s3AccessKeyVisible/);
  assert.match(source, /aria-label=\{`\$\{t\(webdavPasswordVisible/);
});

test('WebDAV 应提供常用配置同步开关', async () => {
  const source = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );
  assert.match(source, /settingsStorage\.syncAppConfig/);
  assert.match(source, /sync_app_config/);
});
