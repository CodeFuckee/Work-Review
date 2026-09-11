import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('关于页不应包含 GitHub 与赞助入口及其弹层死代码', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /openGitHub/);
  assert.doesNotMatch(source, /github\.com/);
  assert.doesNotMatch(source, /sponsorship|Sponsorship/i);
  assert.doesNotMatch(source, /about\.sponsorship/);
  assert.doesNotMatch(source, /zoomedQr/);
  assert.doesNotMatch(source, /docs\/sponsorship\//);
  assert.doesNotMatch(source, /plugin-shell/);
  assert.match(source, /about\.openDataDir/);
  assert.match(source, /open_data_dir/);
});

test('关于页操作区应保留数据目录入口且检查更新位于下方状态区', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.match(source, /about-action-row/);
  assert.match(source, /role="switch"/);
  assert.match(source, /aria-checked=\{autoCheckUpdate\}/);
  assert.ok(
    source.indexOf("t('about.openDataDir')")
      < source.indexOf("t('about.checkUpdates')"),
    '数据目录应先展示，检查更新应位于下方状态区'
  );
});

test('关于页应独立读取版本与更新设置，失败时不得伪造旧版本号', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.match(source, /typeof version === 'string' && version\.trim\(\) \? version : '--'/);
  assert.match(source, /settings\?\.autoCheck \?\? true/);
  assert.doesNotMatch(source, /appVersion = '1\.0\.0'/);
  assert.ok(source.indexOf("console.error('获取版本失败:'") < source.indexOf("console.error('读取更新设置失败:'"));
});

test('关于页不应恢复 Linux 会话状态查询或 Wayland 详情卡片', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.match(source, /open_data_dir/);
  assert.doesNotMatch(source, /invoke\('get_linux_session_support'\)/);
  assert.doesNotMatch(source, /about\.linuxSessionTitle/);
  assert.doesNotMatch(source, /activeWindowProvider/);
  assert.doesNotMatch(source, /browserUrlSupportLevel/);
});
