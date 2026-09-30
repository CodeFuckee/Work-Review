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
    /stages: \[verify, compile, daily-package, audit, deploy-admin, version, preflight, package, publish\]/,
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
    // 只依赖成败不下载产物：bump 从不消费三平台二进制
    assert.match(
      yaml,
      new RegExp(`- job: ${dep}\\n\\s+artifacts: false`),
      `version-bump needs ${dep} (artifacts: false)`,
    );
  }
  assert.match(yaml, /CI_PIPELINE_SOURCE == "push" && \$CI_COMMIT_BRANCH == \$CI_DEFAULT_BRANCH/);
  assert.match(yaml, /when: never/);
});

test('version-bump 使用 GIT_PUSH_TOKEN；bump 提交由 workflow 规则排除而非 [skip ci] 防循环', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /GIT_PUSH_TOKEN/);
  // 认证走 GIT_CONFIG_* 环境变量（token 不进 git 进程 argv），且推送带 --atomic
  assert.match(yaml, /GIT_CONFIG_KEY_0="http\.extraheader"/);
  assert.match(yaml, /GIT_CONFIG_VALUE_0="Authorization: Basic \$\(printf 'oauth2:%s'/);
  assert.match(yaml, /git push --atomic origin/);
  assert.match(yaml, /printf 'oauth2:%s'/);
  // bump 提交不用 [skip ci]（会连带跳过 tag 发布流水线），改为 workflow 排除规则
  assert.doesNotMatch(yaml, /commit -m "chore\(release\): bump version to v\$\{NEW_VERSION\} \[skip ci\]"/);
  assert.match(yaml, /commit -m "chore\(release\): bump version to v\$\{NEW_VERSION\}"/);
  assert.match(
    yaml,
    /CI_COMMIT_MESSAGE =~ \/\^chore\\\(release\\\): bump version to v\//,
  );
  assert.match(yaml, /scripts\/bump-version\.mjs/);
});

test('缓存 key 固定为 npm-cache-v1（bump 改写 Cargo.toml/Cargo.lock 不失效）', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  const cache = yaml.match(/^cache:\n(?:[ \t]+[^\n]*\n)+/m)?.[0] ?? '';
  assert.match(cache, /key: npm-cache-v1/, '缓存 key 应固定为 npm-cache-v1');
  assert.match(cache, /paths: \[\.npm\/\]/, '缓存只承担 npm 下载缓存');
  assert.doesNotMatch(cache, /Cargo/, '缓存 key/路径不得包含 Cargo 相关文件（bump 后失效）');
});

test('夜间流水线应有提交门禁：无新提交时取消流水线，package-daily 依赖门禁', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /^nightly-gate:/m, '缺少 nightly-gate 门禁作业');
  assert.match(yaml, /git describe --tags/, '门禁应比较最新 tag 与 HEAD');
  assert.match(
    yaml, /pipelines\/\$CI_PIPELINE_ID\/cancel/,
    '无新提交时应通过 API 取消整条夜间流水线（避免重复打包相同产物）',
  );
  // 三个默认启用的 daily 作业都必须依赖门禁与各自平台的验证
  assert.match(yaml, /package-daily-windows:\n[\s\S]*?needs: \[nightly-gate, rust-verify-windows\]/);
  assert.match(yaml, /package-daily-linux-x64:\n[\s\S]*?needs: \[nightly-gate, rust-verify-linux\]/);
  assert.match(yaml, /package-daily-macos-arm64:\n[\s\S]*?needs: \[nightly-gate, rust-verify-macos\]/);
  // workflow 顶层必须放行 schedule，否则定时流水线在创建阶段即被过滤
  assert.match(
    yaml,
    /CI_PIPELINE_SOURCE == "schedule"/,
    'workflow/job rules 应放行 schedule 触发',
  );
});

test('bump-version.mjs 应存在且本身不含破坏性 git 操作', () => {
  const script = readFileSync(BUMP_SCRIPT, 'utf8');
  assert.match(script, /computeNextVersion/);
  assert.match(script, /NEW_VERSION=/);
  // 脚本只做文件版本同步，不做 git commit/push（git 操作留在 CI job 里）
  assert.doesNotMatch(script, /\bgit push\b/);
  assert.doesNotMatch(script, /\bgit commit\b/);
});