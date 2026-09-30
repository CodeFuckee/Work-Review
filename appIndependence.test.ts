import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// 契约背景：Yue-Review 与旧版 Work-Review 是两套完全独立的软件。
// 曾经因为共用 mainBinaryName（Work_Review.exe），导致旧版运行时安装
// Yue-Review，NSIS 安装器按二进制名查杀进程，把旧版当作自身杀掉
// （提示 "yue-review is running, click ok to kill it"）。
// 本文件固化独立性契约，防止任何一项回退。

test('C-001: 安装标识必须与旧版 Work-Review 零交集', async () => {
  const conf = JSON.parse(
    await readFile(new URL('./src-tauri/tauri.conf.json', import.meta.url), 'utf8')
  );

  // 单实例互斥按 identifier 区分：两者相同则后启动的实例会被静默退出
  assert.equal(conf.identifier, 'com.yuereview.app');
  // NSIS 安装器按 mainBinaryName 查杀同名进程：两者相同则安装时杀掉旧版
  assert.equal(conf.mainBinaryName, 'Yue_Review');
  assert.equal(conf.productName, 'Yue-Review');

  const identity = JSON.stringify([
    conf.identifier,
    conf.mainBinaryName,
    conf.productName,
  ]).toLowerCase();
  assert.doesNotMatch(identity, /workreview|work_review|work-review|daily-report/);
});

test('C-001: 默认数据目录必须与旧版隔离', async () => {
  const main = await readFile(new URL('./src-tauri/src/main.rs', import.meta.url), 'utf8');
  const start = main.indexOf('fn default_data_dir');
  const prefStart = main.indexOf('fn data_dir_preference_path');
  const prefEnd = main.indexOf('fn load_data_dir_preference');
  assert.ok(start > 0 && prefStart > start && prefEnd > prefStart);

  const defaultDirFn = main.slice(start, prefStart);
  assert.match(defaultDirFn, /join\("yue-review"\)/);
  assert.doesNotMatch(defaultDirFn, /work-review/);

  const prefFn = main.slice(prefStart, prefEnd);
  assert.match(prefFn, /join\("yue-review"\)/);
  assert.doesNotMatch(prefFn, /work-review/);
});

test('C-002: CI 便携版脚本与产物应使用新二进制名', async () => {
  const ci = await readFile(new URL('./.gitlab-ci.yml', import.meta.url), 'utf8');

  assert.match(ci, /-Filter "Yue_Review\.exe"/);
  assert.match(ci, /Yue_Review_portable_x64\.zip/);
  assert.match(ci, /--binary-name Yue_Review/);
  // 旧二进制名在 CI 中不允许有任何残留依赖（查杀/打包/产物路径）
  assert.doesNotMatch(ci, /Work_Review/);
});

test('C-002: 进程自识别应认识新名并保留旧名兼容', async () => {
  const categorize = await readFile(
    new URL('./crates/core/src/categorize.rs', import.meta.url),
    'utf8'
  );
  assert.match(categorize, /"yue-review" \| "yue_review" \| "yuereview"/);
  assert.match(categorize, /=> "Yue复盘"/);
  // 旧名映射保留：历史时间线记录仍能正确归一化
  assert.match(categorize, /=> "Work Review"/);

  const appVisuals = await readFile(
    new URL('./src/lib/utils/appVisuals.ts', import.meta.url),
    'utf8'
  );
  assert.match(appVisuals, /yue-review/);
  assert.match(appVisuals, /daily-report/);
});
