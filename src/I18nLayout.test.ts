import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('应用壳层应使用统一 i18n 资源并支持三种界面语言', async () => {
  const [appSource, i18nSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/i18n/index.ts', import.meta.url), 'utf8'),
  ]);

  assert.match(i18nSource, /'zh-CN'/);
  assert.match(i18nSource, /'en'/);
  assert.match(i18nSource, /'zh-TW'/);
  assert.match(i18nSource, /'ar'/);
  assert.match(i18nSource, /export function cycleLocale\(\)/);
  assert.match(appSource, /applyLocaleToDocument/);
  assert.match(appSource, /initializeLocale/);
  assert.match(appSource, /\{#key currentLocale\}/);
  assert.match(appSource, /<Router \{routes\} \/>/);
});

// MD 迁移后语言菜单为壳层自绘 md-menu-surface（App.svelte，语义与原菜单等价），
// 语言清单与顺序断言改为针对壳层实现。
test('壳层应提供语言切换菜单，并按 ZH、EN、TW、AR 顺序展示', async () => {
  const source = await readFile(new URL('./App.svelte', import.meta.url), 'utf8');

  assert.match(source, /md-menu-surface/);
  assert.match(source, /selectLocale\(option\.value\)/);
  assert.match(source, /value: 'zh-CN', label: 'ZH'/);
  assert.match(source, /value: 'en', label: 'EN'/);
  assert.match(source, /value: 'zh-TW', label: 'TW'/);
  assert.match(source, /value: 'ar', label: 'AR'/);
  assert.ok(
    source.indexOf("value: 'zh-CN', label: 'ZH'")
      < source.indexOf("value: 'en', label: 'EN'")
      && source.indexOf("value: 'en', label: 'EN'")
      < source.indexOf("value: 'zh-TW', label: 'TW'")
      && source.indexOf("value: 'zh-TW', label: 'TW'")
      < source.indexOf("value: 'ar', label: 'AR'"),
    '语言顺序应为 ZH、EN、TW、AR'
  );
});

test('后端语言切换命令应保持私有以避免 Tauri command 宏重导出冲突', async () => {
  const source = await readFile(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(source, /#\[tauri::command\]\s*async fn set_app_locale\b/);
  assert.doesNotMatch(source, /#\[tauri::command\]\s*pub async fn set_app_locale\b/);
});
