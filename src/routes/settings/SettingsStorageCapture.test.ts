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
  // 开关已迁移 SMUI Switch：影子变量受控 + onSMUISwitchChange + aria-label
  assert.match(source, /<Switch/);
  assert.match(source, /aria-label=\{t\('settingsStorage\.screenshotsEnabled'\)\}/);
  assert.match(source, /\$: screenshotsEnabledSwitch = config\.storage\?\.screenshots_enabled/);
  assert.match(source, /bind:checked=\{screenshotsEnabledSwitch\}/);
});

test('存储设置数值输入和自动导出开关应提供本地化可访问名称', async () => {
  const source = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  for (const key of ['pollingInterval', 'retentionDays', 'storageLimitLabel']) {
    assert.match(source, new RegExp(`aria-label=\\{t\\('settingsStorage\\.${key}'\\)\\}`));
  }
  // 自动导出开关已迁移 SMUI Switch（影子变量 + aria-label，未选目录时保持禁用）
  assert.match(source, /aria-label=\{t\('settingsStorage\.autoExport'\)\}/);
  assert.match(source, /bind:checked=\{dailyReportAutoExportSwitch\}/);
  assert.match(source, /disabled=\{!config\.daily_report_export_dir\}/);
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
