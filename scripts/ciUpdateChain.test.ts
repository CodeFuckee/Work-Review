import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// 更新链路测试·层次2：publish-release 编排契约（结构守卫）。
// 防止"版本化选择安装包 / 产物自检"在后续修改中被删改（2026-09-16 更新失效事故）。
// 根因（打包前不清理 bundle + 裸 glob 选择）已修复，本文件断言均为阻断式。
const CI_YAML = fileURLToPath(new URL('../.gitlab-ci.yml', import.meta.url));
const ARTIFACTS_VERIFY = fileURLToPath(new URL('./verify-update-artifacts.mjs', import.meta.url));
const GENERATOR = fileURLToPath(new URL('./generate-updater-json.mjs', import.meta.url));

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

test('publish-release 必须在收尾调用发布产物端到端自检（阻断模式）', () => {
  const yaml = read(CI_YAML);
  assert.match(yaml, /node scripts\/verify-update-artifacts\.mjs/);
  assert.match(yaml, /--version "\$VERSION"/);
  assert.match(
    yaml,
    /--platform "windows-x86_64=\$WIN_EXE"/,
    'Windows 平台自检必须接线（本次事故所在平台）',
  );
  // 自检失败必须阻断发布（禁止回退到观察模式包装）
  assert.doesNotMatch(
    yaml,
    /verify-update-artifacts\.mjs[^\n]*\|\|\s*echo/,
    '自检不得再套观察模式包装',
  );
  assert.doesNotMatch(yaml, /观察模式/);
});

test('generate-updater-json 必须阻断版本不一致的清单（不得回退观察模式）', () => {
  const source = read(GENERATOR);
  assert.match(source, /assertPlatformVersionConsistency\(manifest, manifest\.version\)/);
  assert.doesNotMatch(source, /观察模式/);
});

test('打包作业必须在构建前清理 bundle 目录（防历史版本产物累积）', () => {
  const yaml = read(CI_YAML);
  const cleanupCount = (
    yaml.match(/打包前清理 bundle 目录/g) ?? []
  ).length;
  assert.equal(
    cleanupCount,
    7,
    '七个打包作业（.tauri-package/tauri-package-windows/.package-daily 系列与 windows 日常包）都应有构建前清理',
  );
  assert.match(yaml, /GIT_CLEAN_FLAGS: -ffdx --exclude=target/);
});

test('安装包选择必须按发布版本过滤（根因已修复，防回退）', () => {
  const yaml = read(CI_YAML);
  // 选择器不得使用裸 glob + head -1（会选中字母序最旧的累积产物）
  assert.doesNotMatch(
    yaml,
    /WIN_EXE="\$\(ls target\/\*\*\/release\/bundle\/nsis\/\*\.exe 2>\/dev\/null \| head -n 1 \|\| true\)"/,
    'Windows 安装包选择必须匹配 $VERSION',
  );
  assert.match(
    yaml,
    /WIN_EXE="\$\(ls target\/\*\*\/release\/bundle\/nsis\/\*"\$\{VERSION\}"\*\.exe/,
    'Windows 安装包选择应包含 $VERSION 过滤',
  );
  // 上传/资产循环不得引用其他版本的安装包
  assert.doesNotMatch(
    yaml,
    /Yue-Review_1\.1\.4[0-9]_x64-setup\.exe/,
    '发布编排不得引用硬编码的旧版本文件名',
  );
});
