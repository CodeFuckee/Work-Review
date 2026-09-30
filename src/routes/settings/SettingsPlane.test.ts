import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Plane 集成设置：静态校验设置页注册了新 tab、两条命令调用与 i18n 完整性。
const SETTINGS = fileURLToPath(new URL('./Settings.svelte', import.meta.url));
const PLANE_COMPONENT = fileURLToPath(new URL('./components/SettingsPlane.svelte', import.meta.url));
const LOCALES = fileURLToPath(new URL('../../lib/i18n/locales', import.meta.url));

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

test('设置页应注册 Plane 标签页并渲染组件', () => {
  const source = read(SETTINGS);
  assert.match(source, /import SettingsPlane from '.\/components\/SettingsPlane.svelte'/);
  assert.match(source, /\{ id: 'plane', labelKey: 'settings\.tabs\.plane', icon: 'plane' \}/);
  assert.match(source, /\{:else if activeTab === 'plane'\}\s*<SettingsPlane bind:config on:change=\{\(\) => dirty = true\} \/>/);
});

test('Plane 组件应调用测试连接命令并传 url/apiKey/workspaceSlug', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /plane_test_connection/);
  assert.match(source, /ok: boolean;/);
  assert.match(source, /workspace_name: string;/);
  assert.match(source, /url: current\.url \?\? ''/);
  assert.match(source, /apiKey: current\.api_key \?\? ''/);
  assert.match(source, /workspaceSlug: current\.workspace_slug \?\? ''/);
});

test('Plane 组件应以掩码形式输入 API Key 并支持切换显示', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /type=\{showKey \? 'text' : 'password'\}/);
  assert.match(source, /showKey/);
  assert.match(source, /autocomplete="off"/);
});

test('Plane 组件应提供工作项预览命令与预览列表', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /plane_fetch_preview/);
  assert.match(source, /items: PlanePreviewItem\[\]/);
  assert.match(source, /limit: 20/);
  assert.match(source, /previewItems/);
  assert.match(source, /previewEmpty/);
});

test('Plane 组件应提供启用开关并按结构化结果展示连接状态', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /role="switch"/);
  assert.match(source, /aria-checked=\{config\.plane\?\.enabled\}/);
  assert.match(source, /testResult\.ok/);
  assert.match(source, /previewError/);
});

test('Plane 开关赋值必须以 config 为根以触发 Svelte 失效（禁止 plane() 调用表达式赋值）', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /config\.plane = \{ \.\.\.current, enabled: !current\.enabled \}/);
  assert.match(source, /config\.plane = \{ \.\.\.current, allow_insecure_http: !current\.allow_insecure_http \}/);
  assert.doesNotMatch(source, /plane\(\)\.enabled = /);
  assert.doesNotMatch(source, /plane\(\)\.allow_insecure_http = /);
});

test('Plane 组件应提供项目过滤 textarea 并随保存流程持久化', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /t\('settingsPlane\.filterLabel'\)/);
  assert.match(source, /t\('settingsPlane\.filterHint'\)/);
  assert.match(source, /value=\{\(config\.plane\?\.project_filter \?\? \[\]\)\.join\('\\n'\)\}/);
  assert.match(source, /on:input=\{onFilterInput\}/);
  assert.match(source, /config\.plane = \{ \.\.\.plane\(\), project_filter: lines \};/);
});

test('设置页 Plane 图标分支应使用单枚描边 SVG', () => {
  const source = read(SETTINGS);
  assert.match(source, /\{:else if tab\.icon === 'plane'\}\s*<svg class="w-3\.5 h-3\.5" fill="none" stroke="currentColor"/);
});

test('四个语言的 i18n 都应包含 plane tab 与完整 settingsPlane 文案块', () => {
  for (const locale of ['zh-CN', 'zh-TW', 'en', 'ar']) {
    const file = read(`${LOCALES}/${locale}.ts`);
    assert.match(file, /plane: 'Plane'/);
    assert.match(file, /settingsPlane: \{/);
    for (const key of [
      'title',
      'description',
      'enabledLabel',
      'enabledHint',
      'urlLabel',
      'urlHint',
      'apiKeyLabel',
      'apiKeyHint',
      'showKey',
      'hideKey',
      'workspaceSlugLabel',
      'workspaceSlugHint',
      'filterLabel',
      'filterHint',
      'testConnection',
      'testing',
      'testFailed',
      'previewButton',
      'previewing',
      'previewEmpty',
      'previewFailed',
      'allowInsecureLabel',
      'allowInsecureHint',
      'schemeHint',
      'previewLimitHint',
    ]) {
      assert.match(file, new RegExp(`${key}: `), `locale ${locale} missing settingsPlane.${key}`);
    }
  }
});

test('Plane 组件应本地化全部字段与提示文案', () => {
  const source = read(PLANE_COMPONENT);
  assert.match(source, /t\('settingsPlane\.title'\)/);
  assert.match(source, /t\('settingsPlane\.urlLabel'\)/);
  assert.match(source, /t\('settingsPlane\.apiKeyLabel'\)/);
  assert.match(source, /t\('settingsPlane\.workspaceSlugLabel'\)/);
  assert.match(source, /t\('settingsPlane\.testConnection'\)/);
  assert.match(source, /t\('settingsPlane\.previewButton'\)/);
});
