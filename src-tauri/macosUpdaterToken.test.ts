import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// 被测问题：macOS 安装包「检查更新」永远显示「已是最新」（v1.2.28-1.2.30 实测）。
// 根因：macOS 打包走 GitHub 桥接，GitLab protected 变量 WORK_REVIEW_UPDATER_GITLAB_TOKEN
// 不进桥接 payload，桥接 runner 编译时 env 无此变量 → updater.rs option_env! 为空
// → 更新器静默降级「未配置更新源」。Windows 本机 runner 变量直传，同版产物正常。
// 防线：① 编译脚本 fail-fast + 产物 strings 自检；② 更新器 UI 对未配置态显式提示。

test('macOS 编译脚本必须 fail-fast 校验 updater token 注入', async () => {
  const releaseBuild = await readFile(new URL('../ci/macos/release-build.sh', import.meta.url), 'utf8');
  assert.match(
    releaseBuild,
    /\[ -n "\$\{WORK_REVIEW_UPDATER_GITLAB_TOKEN:-\}" \]/,
    '编译前必须校验 WORK_REVIEW_UPDATER_GITLAB_TOKEN 存在',
  );
  assert.match(
    releaseBuild,
    /grep -c \"glpat-\" || true/,
    '编译后必须产物自检 token 真实嵌入（GitLab PAT 字面量）',
  );

  const packageSh = await readFile(new URL('../ci/macos/package.sh', import.meta.url), 'utf8');
  assert.match(
    packageSh,
    /\[ -n "\$\{WORK_REVIEW_UPDATER_GITLAB_TOKEN:-\}" \]/,
    '打包前必须校验 WORK_REVIEW_UPDATER_GITLAB_TOKEN 存在',
  );
  assert.match(
    packageSh,
    /grep -c \"glpat-\" || true/,
    '打包后必须对 .app 二进制自检 token 嵌入',
  );
});

test('更新器 UI 对「未配置更新源」不得显示已是最新', async () => {
  const source = await readFile(new URL('../src/lib/utils/updater.ts', import.meta.url), 'utf8');
  // available:false 且 source=gitlab-unconfigured 时必须走显式提示分支，
  // 防止环境缺陷静默成「已是最新版本」的假成功
  assert.match(
    source,
    /releaseInfo\.source === 'gitlab-unconfigured'/,
    '前端必须区分「未配置更新源」与「真的已是最新」',
  );
  assert.match(
    source,
    /updater\.sourceUnconfigured/,
    '未配置态必须展示专用提示文案',
  );
});
