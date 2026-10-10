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
    5,
    'yaml 内五个打包作业（.tauri-package/tauri-package-windows/.package-daily 系列与 windows 日常包）都应有构建前清理；桥接的 mac 打包作业清理收进 ci/macos/package.sh',
  );
  const packageSh = read(fileURLToPath(new URL('../ci/macos/package.sh', import.meta.url)));
  assert.match(
    packageSh,
    /rm -rf target\/release\/bundle target\/\*\/release\/bundle/,
    '桥接 mac 打包脚本同样构建前清理（一次性 VM 虽无残留，保留防线对齐 publish 全量 glob）',
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

test('publish-release 不得在 artifact 解压后整目录删除 bundle（v1.1.55 发布失败回归）', () => {
  const yaml = read(CI_YAML);
  // 只检查 publish-release 作业段：打包作业里的同款 rm -rf 是构建前清理，
  // 在 tauri build 之前执行，合法且受上一条测试锁定
  const sectionStart = yaml.indexOf('publish-release:');
  assert.notEqual(sectionStart, -1, 'publish-release 作业必须存在');
  const sectionEnd = yaml.indexOf('\n  rules:', sectionStart);
  const section = yaml.slice(sectionStart, sectionEnd);
  // publish-release 的产物由 runner 在脚本执行前解压到 target/*/release/bundle，
  // 整目录 rm -rf 会把刚解压的本版产物一并删光，清单生成缺产物直接失败
  assert.doesNotMatch(
    section,
    /rm -rf target\/release\/bundle target\/\*\/release\/bundle/,
    'artifact 解压发生在脚本之前，盲删 bundle 目录 = 删光本版产物（v1.1.55 事故）',
  );
  // 残留清理必须按当前版本过滤（rpm 等文件名用连字符，不能锚定下划线），
  // 且保留通用名的 macOS updater 产物（*.app.tar.gz，本次解压刚覆盖）
  assert.match(
    section,
    /find target\/release\/bundle target\/\*\/release\/bundle -type f ! -name "\*\$\{VERSION\}\*" ! -name '\*\.app\.tar\.gz' ! -name '\*\.app\.tar\.gz\.sig' -delete/,
    '应按版本定向清理上一版本残留，保留本版与通用名 updater 产物',
  );
  // mac runner 丢 .sig 时旧 sig 残留会造成"新包配旧签名"：签名早于对应
  // 产物的必须删除，交给补签循环重签
  assert.match(
    section,
    /\[ "\$gz" -nt "\$sig" \]/,
    '应删除早于对应产物的残留 macOS 签名（强制重签）',
  );
});
