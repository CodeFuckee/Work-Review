import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// 日常（每次 push main）三平台安装包产物：直接下载使用（CI artifact + 固定 latest URL）。
const CI_YAML = fileURLToPath(new URL('../.gitlab-ci.yml', import.meta.url));
const README = fileURLToPath(new URL('../README.md', import.meta.url));

test('stages 应包含 daily-package（在 compile 与 audit 之间）', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(
    yaml,
    /stages: \[verify, compile, daily-package, audit, version, preflight, package, publish\]/,
  );
});

test('应存在五个日常安装包 job（Windows/macOS×2/Linux×2），rules 收在 .package-daily 模板，均保留 30 天', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  for (const job of [
    'package-daily-windows',
    'package-daily-macos-arm64',
    'package-daily-macos-x64',
    'package-daily-linux-x64',
    'package-daily-linux-arm64',
  ]) {
    assert.match(yaml, new RegExp(`^${job}:`, 'm'), `缺少 job ${job}`);
  }
  // 模板：非 tag、on_success
  const template = yaml.match(/\.package-daily:[\s\S]*?rules:[\s\S]*?\n\n/)?.[0] ?? '';
  assert.match(template, /stage: daily-package/);
  assert.match(template, /if: '\$CI_COMMIT_TAG'\s+when: never/);
  assert.match(template, /when: on_success/);
  // 每个 job 的上传 artifact 保留 30 天
  const jobSlices = yaml.split(/^(?=package-daily-)/m);
  for (const slice of jobSlices) {
    if (/^package-daily-/.test(slice)) {
      assert.match(slice, /expire_in: 30 days/, '日常 artifact 保留 30 天');
      assert.match(slice, /npm run tauri build/, '必须产出安装包');
    }
  }
});

test('Windows 日常打包应生成便携 zip 产物', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  const windows = yaml.split(/^(?=package-daily-windows)/m).find((s) => s.startsWith('package-daily-windows'));
  assert.ok(windows);
  assert.match(windows, /Work_Review_portable_x64\.zip/);
  assert.match(windows, /target\/\*\*\/release\/bundle\/nsis\/\*\.exe/);
});

test('daily-package 应位于 compile 之后（依赖 release 编译产物语义顺序）', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  const stagesMatch = yaml.match(/stages: \[([^\]]+)\]/);
  assert.ok(stagesMatch, '应有 stages 声明');
  const list = stagesMatch[1].split(',').map((s) => s.trim());
  assert.ok(list.indexOf('compile') < list.indexOf('daily-package'), 'daily-package 应在 compile 之后');
  assert.ok(list.indexOf('daily-package') < list.indexOf('audit'), 'daily-package 应在 audit 之前');
});

test('README 应说明最新产物的固定下载链接用法', () => {
  const readme = readFileSync(README, 'utf8');
  assert.match(readme, /\/-\/jobs\/artifacts\/main\/download\?job=/);
  assert.match(readme, /package-daily-/);
});

test('发布版（tag）仍需保留原有永久 artifact 与 release 流程', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /expire_in: never/);
  assert.match(yaml, /glab release create/);
});