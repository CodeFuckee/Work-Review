import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// 更新链路测试·层次2：publish-release 编排契约（结构守卫）。
// 防止"版本化选择安装包 / 产物自检"在后续修改中被删改（2026-09-16 更新失效事故）。
const CI_YAML = fileURLToPath(new URL('../.gitlab-ci.yml', import.meta.url));
const ARTIFACTS_VERIFY = fileURLToPath(new URL('./verify-update-artifacts.mjs', import.meta.url));
const GENERATOR = fileURLToPath(new URL('./generate-updater-json.mjs', import.meta.url));

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

test('publish-release 必须在收尾调用发布产物端到端自检（层次3接线防删）', () => {
  const yaml = read(CI_YAML);
  assert.match(yaml, /node scripts\/verify-update-artifacts\.mjs/);
  assert.match(yaml, /--version "\$VERSION"/);
  assert.match(
    yaml,
    /--platform "windows-x86_64=\$WIN_EXE"/,
    'Windows 平台自检必须接线（本次事故所在平台）',
  );
  assert.match(yaml, /观察模式/);
});

test('generate-updater-json 必须保留版本一致性不变式检查（观察模式）', () => {
  const source = read(GENERATOR);
  assert.match(source, /assertPlatformVersionConsistency\(manifest, manifest\.version\)/);
  assert.match(source, /观察模式/);
});

test('安装包选择必须按发布版本过滤（根因修复落地后移除 skip 启用为硬断言）', { skip: '根因修复（按版本选择安装包 + 打包前清理 bundle 目录）落地后启用' }, () => {
  const yaml = read(CI_YAML);
  // 选择器不得使用裸 glob + head -1（会选中字母序最旧的累积产物）
  assert.doesNotMatch(
    yaml,
    /WIN_EXE="\$\(ls target\/\*\*\/release\/bundle\/nsis\/\*\.exe 2>\/dev\/null \| head -n 1 \|\| true\)"/,
    'Windows 安装包选择必须匹配 $VERSION',
  );
  assert.match(
    yaml,
    /WIN_EXE="\$\(ls target\/\*\*\/release\/bundle\/nsis\/\*"\$VERSION"\*\.exe[^"]*"/,
    'Windows 安装包选择应包含 $VERSION 过滤',
  );
  // Release 资产链接不得混入其他版本的安装包
  assert.doesNotMatch(
    yaml,
    /Yue-Review_1\.1\.4[0-9]_x64-setup\.exe/,
    '发布编排不得引用硬编码的旧版本文件名',
  );
});
