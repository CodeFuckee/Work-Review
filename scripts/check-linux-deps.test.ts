import test from 'node:test';
import assert from 'node:assert/strict';

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// 回归：GitLab CI 的 Linux job 不得执行需要 sudo 的命令（shell executor
// 无 TTY，sudo 会等待密码直到 job 超时——rust-verify-linux 曾因此卡死
// 1 小时被 kill）。系统依赖改为在 runner 上手动一次性安装（脚本
// scripts/check-linux-deps.sh 只做非 sudo 的 dpkg 预检并打印安装指引）。
const CI_YAML = fileURLToPath(new URL('../.gitlab-ci.yml', import.meta.url));
const DEPS_SCRIPT = fileURLToPath(new URL('./check-linux-deps.sh', import.meta.url));

test('gitlab-ci: Linux jobs must not invoke sudo', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  const lines = yaml.split(/\r?\n/);
  const offending = lines
    .map((line, index) => ({ line, index: index + 1 }))
    .filter(
      ({ line }) =>
        !/^\s*#/.test(line) && // 忽略注释（说明文字可能提到 sudo）
        (/^\s*-?\s*sudo\b/.test(line) || /\bsudo\s+(apt|apt-get|install|update)/.test(line)),
    );
  assert.deepEqual(
    offending.map((o) => `L${o.index}: ${o.line.trim()}`),
    [],
    'sudo must not appear in any job command (deps are configured manually on the runner)',
  );
});

test('gitlab-ci: Linux jobs run the non-sudo dependency precheck', () => {
  const yaml = readFileSync(CI_YAML, 'utf8');
  assert.match(yaml, /scripts\/check-linux-deps\.sh/);
  // verify 与 package 两条路径都要预检依赖
  assert.match(yaml, /check-linux-deps\.sh/);
});

test('check-linux-deps.sh must only verify, never run sudo/apt itself', () => {
  const script = readFileSync(DEPS_SCRIPT, 'utf8');
  const lines = script.split(/\r?\n/);

  // 只允许 sudo/apt-get 出现在注释或 echo 的「安装指引」文本里，绝不作为命令执行。
  const realSudo = lines.filter((l) => /^\s*sudo\s/.test(l));
  assert.deepEqual(realSudo, [], 'script must not invoke sudo as a command');

  const realApt = lines.filter((l) => /^\s*(sudo\s+)?apt(-get)?\s/.test(l));
  assert.deepEqual(realApt, [], 'script must not run apt/apt-get as a command');

  // 非交互：不得读取 TTY 输入。
  assert.doesNotMatch(script, /read\s/);
});