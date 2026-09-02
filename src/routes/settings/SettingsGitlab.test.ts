import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// GitLab 集成设置：静态校验设置页注册了新 tab、测试连接命令调用与 i18n 完整性。
const SETTINGS = fileURLToPath(new URL('./Settings.svelte', import.meta.url));
const GITLAB_COMPONENT = fileURLToPath(new URL('./components/SettingsGitlab.svelte', import.meta.url));
const LOCALES = fileURLToPath(new URL('../../lib/i18n/locales', import.meta.url));

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

test('设置页应注册 GitLab 标签页并渲染组件', () => {
  const source = read(SETTINGS);
  assert.match(source, /import SettingsGitlab from '.\/components\/SettingsGitlab.svelte'/);
  assert.match(source, /\{ id: 'gitlab', labelKey: 'settings\.tabs\.gitlab', icon: 'gitlab' \}/);
  assert.match(source, /\{:else if activeTab === 'gitlab'\}\s*<SettingsGitlab bind:config on:change=\{\(\) => dirty = true\} \/>/);
});

test('GitLab 组件应调用测试连接命令并传 url/token/过滤列表', () => {
  const source = read(GITLAB_COMPONENT);
  assert.match(source, /gitlab_test_connection/);
  assert.match(source, /ok: boolean;/);
  assert.match(source, /user_name: string;/);
  assert.match(source, /url: current\.url \?\? ''/);
  assert.match(source, /token: current\.token \?\? ''/);
  assert.match(source, /projectFilter: current\.project_filter \?\? \[\]/);
});

test('GitLab 组件应以掩码形式输入令牌并支持切换显示', () => {
  const source = read(GITLAB_COMPONENT);
  assert.match(source, /type=\{showToken \? 'text' : 'password'\}/);
  assert.match(source, /showToken/);
  assert.match(source, /autocomplete="off"/);
});

test('GitLab 组件应本地化令牌展示与连接结果文案', () => {
  const source = read(GITLAB_COMPONENT);
  assert.match(source, /t\('settingsGitlab\.title'\)/);
  assert.match(source, /t\('settingsGitlab\.testConnection'\)/);
  assert.match(source, /t\('settingsGitlab\.testProjectCount'/);
});

test('四个语言的 i18n 都应包含 gitlab tab 与完整文案块', () => {
  for (const locale of ['zh-CN', 'zh-TW', 'en', 'ar']) {
    const file = read(`${LOCALES}/${locale}.ts`);
    assert.match(file, /gitlab: 'GitLab'/);
    assert.match(file, /settingsGitlab: \{/);
    for (const key of ['title', 'urlLabel', 'tokenLabel', 'filterLabel', 'enabledLabel', 'testConnection', 'testFailed']) {
      assert.match(file, new RegExp(`${key}: `), `locale ${locale} missing settingsGitlab.${key}`);
    }
  }
});

test('项目不得残留对旧 GitLab 配置键的错误引用', () => {
  const source = read(GITLAB_COMPONENT);
  assert.doesNotMatch(source, /gitlab_url/);
  assert.doesNotMatch(source, /gitlab_token/);
});