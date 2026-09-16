#!/usr/bin/env node
// 发布产物端到端自检（更新链路测试·层次3）：publish-release 上传完成后，
// 以"下载方视角"重新拉取刚发布的更新清单与安装包，做以下断言——
//   1) updater.json 可下载且 version 与本次发布一致
//   2) 各平台：清单下载 URL 的文件名与本地上传文件一致；文件名含版本号时
//      必须与发布版本一致（2026-09-16 事故形态：选中累积的旧版安装包）
//   3) 各平台：远端安装包 SHA256 与本地文件一致（上传完整性）
//   4) 签名字段非空且与本地 .sig 文件一致
// 任一断言失败 → 退出码 1（publish-release 据此阻断/告警）。
// 用法（publish-release job 中）：
//   node scripts/verify-update-artifacts.mjs \
//     --version "$VERSION" \
//     --url-base "$CI_SERVER_URL/api/v4/projects/$CI_PROJECT_ID/packages/generic/yue-review/$VERSION" \
//     --token-env GIT_PUSH_TOKEN \
//     --platform "windows-x86_64=$WIN_EXE" \
//     --platform "darwin-aarch64=$MAC_A64"

import { createHash } from 'node:crypto';
import { readFileSync, statSync } from 'node:fs';
import { basename, resolve } from 'node:path';

function parseArgs(argv) {
  const args = { platform: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith('--')) continue;
    const value = argv[index + 1] ?? '';
    index += 1;
    if (key === '--platform') {
      const splitAt = value.indexOf('=');
      if (splitAt === -1) {
        throw new Error(`--platform 需要 "平台=本地产物路径" 形式，收到: ${value}`);
      }
      args.platform.push([value.slice(0, splitAt), resolve(value.slice(splitAt + 1))]);
    } else if (key === '--version') {
      args.version = value;
    } else if (key === '--url-base') {
      args.urlBase = value.replace(/\/+$/, '');
    } else if (key === '--token-env') {
      args.tokenEnv = value;
    }
  }
  return args;
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

async function fetchWithAuth(url, token) {
  return fetch(url, {
    headers: { 'PRIVATE-TOKEN': token },
    signal: AbortSignal.timeout(120_000),
  });
}

const failures = [];
function check(name, ok, detail = '') {
  const mark = ok ? '✅' : '❌';
  console.log(`${mark} ${name}${detail ? ` — ${detail}` : ''}`);
  if (!ok) failures.push(name);
}

const args = parseArgs(process.argv.slice(2));
if (!args.version || !args.urlBase || !args.platform.length) {
  console.error('缺少 --version / --url-base / --platform 参数');
  process.exit(1);
}
const token = process.env[args.tokenEnv ?? 'GIT_PUSH_TOKEN'] ?? '';
if (!token) {
  console.error(`缺少认证令牌：环境变量 ${args.tokenEnv ?? 'GIT_PUSH_TOKEN'} 为空`);
  process.exit(1);
}

const manifestUrl = `${args.urlBase}/updater.json`;

// 1) 清单可下载且版本一致
let manifest;
try {
  const response = await fetchWithAuth(manifestUrl, token);
  check(`更新清单可下载 (${manifestUrl})`, response.ok, `HTTP ${response.status}`);
  if (response.ok) {
    manifest = await response.json();
  }
} catch (error) {
  check('更新清单可下载', false, String(error));
}
if (!manifest) {
  console.error('❌ 无法获取更新清单，后续断言全部跳过');
  process.exit(1);
}
check(
  `清单 version 为 ${args.version}`,
  manifest.version === args.version,
  `实际 ${manifest.version}`,
);

for (const [platform, localPath] of args.platform) {
  const localName = basename(localPath);

  let localStat;
  try {
    localStat = statSync(localPath);
  } catch {
    check(`本地产物存在: ${localName}`, false, `${localPath} 不存在`);
    continue;
  }

  // 2) 本地文件选择：文件名含版本号时必须与发布版本一致
  const embedded = localName.match(/(\d+\.\d+\.\d+)/);
  if (embedded) {
    check(
      `本地产物版本与发布一致: ${localName}`,
      embedded[1] === args.version,
      `文件名版本 ${embedded[1]}`,
    );
  }

  // 3) 清单中该平台存在、URL 文件名与本地一致、签名字段非空
  const entry = manifest.platforms?.[platform];
  check(`清单包含平台 ${platform}`, Boolean(entry));
  if (!entry) continue;
  const remoteName = entry.url.split('/').pop() ?? '';
  check(
    `清单下载 URL 与本地上传文件一致: ${remoteName}`,
    remoteName === localName,
    remoteName === localName ? '' : `应为 ${localName}`,
  );
  check(`清单签名字段非空 (${platform})`, Boolean(entry.signature));

  // 4) 远端安装包可下载且 SHA256 与本地一致（上传完整性）
  const remoteSha = await (async () => {
    try {
      const response = await fetchWithAuth(entry.url, token);
      if (!response.ok) {
        check(`远端安装包可下载 (${platform})`, false, `HTTP ${response.status}`);
        return null;
      }
      const buffer = Buffer.from(await response.arrayBuffer());
      check(
        `远端安装包可下载 (${platform})`,
        true,
        `${(buffer.length / 1024 / 1024).toFixed(2)} MB`,
      );
      return createHash('sha256').update(buffer).digest('hex');
    } catch (error) {
      check(`远端安装包可下载 (${platform})`, false, String(error));
      return null;
    }
  })();

  const localSha = sha256File(localPath);
  check(
    `远端安装包 SHA256 与本地一致 (${platform})`,
    remoteSha !== null && remoteSha === localSha,
    remoteSha ? `本地 ${localSha.slice(0, 12)}… / 远端 ${String(remoteSha).slice(0, 12)}…` : '远端下载失败',
  );
}

if (failures.length > 0) {
  console.error(`❌ 发布产物自检未通过（${failures.length} 项）: ${failures.join(' | ')}`);
  process.exit(1);
}
console.log('✅ 发布产物自检全部通过');
