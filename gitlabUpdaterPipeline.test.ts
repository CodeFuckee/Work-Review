import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';

test('GitLab 发布流水线应在 publish-release 生成并上传 updater.json', () => {
  const source = readFileSync(new URL('./.gitlab-ci.yml', import.meta.url), 'utf8');

  assert.match(source, /scripts\/generate-updater-json\.mjs/, 'publish-release 必须调用清单生成脚本');
  assert.match(
    source,
    /--platform "darwin-aarch64=\$MAC_A64"/,
    '清单生成必须提供 darwin-aarch64 产物',
  );
  assert.match(
    source,
    /--platform "windows-x86_64=\$WIN_EXE"/,
    '清单生成必须提供 windows-x86_64 产物',
  );
  assert.match(
    source,
    /packages\/generic\/yue-review\/\$VERSION/,
    '清单下载地址必须指向自建 GitLab 包仓库',
  );
  assert.match(
    source,
    /for f in[^\n]*updater\.json/,
    'updater.json 必须进入包仓库上传与 Release 资产链接循环',
  );
  assert.doesNotMatch(source, /releases\/latest\/download\/updater/, '不应再引用 GitHub latest 下载路由');
});

test('updater 清单生成脚本应强制三平台产物与签名', () => {
  const scriptPath = new URL('./scripts/generate-updater-json.mjs', import.meta.url);
  assert.ok(existsSync(scriptPath), 'scripts/generate-updater-json.mjs 必须存在');

  const script = readFileSync(scriptPath, 'utf8');
  for (const platform of ['darwin-aarch64', 'darwin-x86_64', 'windows-x86_64']) {
    assert.match(script, new RegExp(platform), `清单必须包含 ${platform} 平台`);
  }
  assert.match(script, /\.sig/, '必须读取 .sig 签名文件');
  assert.match(script, /platforms/, '必须输出 platforms 结构');
  assert.match(script, /version/, '必须输出 version 字段');
});

test('更新端点应指向自建 GitLab 且不再包含 GitHub', () => {
  const rustSource = readFileSync(
    new URL('./src-tauri/src/commands/updater.rs', import.meta.url),
    'utf8',
  );
  assert.doesNotMatch(rustSource, /https:\/\/[^"']*github\.com/, 'Rust 更新端点不应包含 GitHub 地址');
  assert.match(rustSource, /home\.chenkaidi\.top:509/, 'Rust 更新端点应指向自建 GitLab');
  assert.match(rustSource, /UPDATER_GITLAB_TOKEN/, '更新器应使用编译期注入的 GitLab 只读令牌');
  assert.match(rustSource, /PRIVATE-TOKEN/, '更新请求应携带 PRIVATE-TOKEN 认证头');

  const conf = readFileSync(new URL('./src-tauri/tauri.conf.json', import.meta.url), 'utf8');
  assert.doesNotMatch(conf, /github\.com\/wm94i/, 'tauri.conf.json 不应再指向 GitHub 更新源');
  assert.match(conf, /pubkey/, '更新签名公钥必须保留');
});
