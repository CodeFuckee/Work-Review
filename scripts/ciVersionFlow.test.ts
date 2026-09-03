import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// 版本自增与三平台 release 编译的 CI 编排：静态校验 .gitlab-ci.yml 与脚本的
// 配套关系（防止后续白话改坏自动发布链路）。
const CI_YAML = fileURLToPath(new URL('../.gitlab-ci.yml', import.meta.url));
const BUMP_SCRIPT = fileURLToPath(new URL('./bump-version.mjs', import.meta.url));

test('CI stages 应包含 compile 与 version', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(
    yaml,
    /stages: \[verify, compile, daily-package, audit, version, preflight, package, publish\]/,
  );
});

test('应存在三个平台的 release 编译 job（仅日常推送，tag 不跑）', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  for (const job of ['build-release-linux', 'build-release-macos', 'build-release-windows']) {
    assert.match(yaml, new RegExp(`^${job}:`, 'm'), `缺少 job ${job}`);
  }
  assert.match(yaml, /cargo build --release -p work-review/);
  assert.match(yaml, /if: '\$CI_COMMIT_TAG'\s+when: never/);
});

test('version-bump 应依赖全部 verify 与 compile job 且仅在 push main 运行', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /^version-bump:/m);
  for (const dep of ['frontend', 'rust-verify-linux', 'rust-verify-macos', 'rust-verify-windows', 'build-release-linux', 'build-release-macos', 'build-release-windows']) {
    assert.match(yaml, new RegExp(`- ${dep}`), `version-bump needs ${dep}`);
  }
  assert.match(yaml, /CI_PIPELINE_SOURCE == "push" && \$CI_COMMIT_BRANCH == \$CI_DEFAULT_BRANCH/);
  assert.match(yaml, /when: never/);
});

test('version-bump 使用 GIT_PUSH_TOKEN 且 bump 提交带 [skip ci] 防循环', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /GIT_PUSH_TOKEN/);
  assert.match(yaml, /Authorization: Basic \$\{PUSH_AUTH\}/);
  assert.match(yaml, /printf 'oauth2:%s'/);
  assert.match(yaml, /\[skip ci\]/);
  assert.match(yaml, /scripts\/bump-version\.mjs/);
});

test('缓存 key 不应包含 Cargo.lock（避免 bump 后缓存失效）', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /files: \[package-lock\.json, src-tauri\/Cargo\.toml\]/);
  assert.doesNotMatch(yaml, /files: \[package-lock\.json, Cargo\.lock\]/);
});

test('bump-version.mjs 应存在且本身不含破坏性 git 操作', () => {
  const script = readFileSync(BUMP_SCRIPT, 'utf8');
  assert.match(script, /computeNextVersion/);
  assert.match(script, /NEW_VERSION=/);
  // 脚本只做文件版本同步，不做 git commit/push（git 操作留在 CI job 里）
  assert.doesNotMatch(script, /\bgit push\b/);
  assert.doesNotMatch(script, /\bgit commit\b/);
});