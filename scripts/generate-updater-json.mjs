#!/usr/bin/env node
// 生成 Tauri 自动更新清单 updater.json（自建 GitLab 包仓库直链版）。
// 与旧 GitHub Actions 流程的差异：下载地址指向 Generic Package Registry，
// 客户端以编译期注入的只读 token（PRIVATE-TOKEN 头）拉取清单与安装包。
// 用法（publish-release job 中，globstar 已开启）：
//   node scripts/generate-updater-json.mjs \
//     --version "$VERSION" \
//     --notes-file release_notes.md \
//     --url-base "$CI_SERVER_URL/api/v4/projects/$CI_PROJECT_ID/packages/generic/yue-review/$VERSION" \
//     --out updater.json \
//     --platform "darwin-aarch64=$MAC_A64" \
//     --platform "darwin-x86_64=$MAC_X64" \
//     --platform "windows-x86_64=$WIN_EXE"

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { basename, resolve } from 'node:path';

// darwin-x86_64 是可选平台：macOS x64 打包 job 由 PACKAGE_MACOS_X64 显式开启，
// 平时只有 arm64 产物，清单中则不含 darwin-x86_64 条目。
const REQUIRED_PLATFORMS = ['darwin-aarch64', 'windows-x86_64'];
const OPTIONAL_PLATFORMS = ['darwin-x86_64'];

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
        throw new Error(`--platform 需要 "平台=产物路径" 形式，收到: ${value}`);
      }
      args.platform.push([value.slice(0, splitAt), value.slice(splitAt + 1)]);
    } else if (key === '--version') {
      args.version = value;
    } else if (key === '--notes-file') {
      args.notesFile = value;
    } else if (key === '--url-base') {
      args.urlBase = value;
    } else if (key === '--out') {
      args.out = value;
    }
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));

if (!args.version || !args.urlBase || !args.out) {
  console.error('缺少 --version / --url-base / --out 参数');
  process.exit(1);
}

const platforms = {};
for (const [platform, artifact] of args.platform) {
  const isRequired = REQUIRED_PLATFORMS.includes(platform);
  if (!isRequired && !OPTIONAL_PLATFORMS.includes(platform)) {
    console.error(`未知平台: ${platform}（已知: ${[...REQUIRED_PLATFORMS, ...OPTIONAL_PLATFORMS].join(', ')}）`);
    process.exit(1);
  }

  const file = artifact ? resolve(artifact) : '';
  if (!artifact || !existsSync(file)) {
    if (isRequired) {
      console.error(`缺少 ${platform} 更新产物: ${artifact || '(空)'}`);
      process.exit(1);
    }
    console.warn(`>>> 可选平台 ${platform} 无产物，清单中跳过`);
    continue;
  }

  const signatureFile = `${file}.sig`;
  if (!existsSync(signatureFile)) {
    console.error(`缺少 ${platform} 更新签名: ${signatureFile}`);
    process.exit(1);
  }
  const signature = readFileSync(signatureFile, 'utf8').replace(/\r?\n/g, '');
  if (!signature) {
    console.error(`${platform} 更新签名为空: ${signatureFile}`);
    process.exit(1);
  }

  platforms[platform] = {
    url: `${args.urlBase.replace(/\/+$/, '')}/${basename(file)}`,
    signature,
  };
}

const missing = REQUIRED_PLATFORMS.filter((platform) => !platforms[platform]);
if (missing.length > 0) {
  console.error(`更新清单缺少必备平台: ${missing.join(', ')}`);
  process.exit(1);
}

const notes = args.notesFile ? readFileSync(args.notesFile, 'utf8') : '';
const manifest = {
  version: args.version.replace(/^v/i, ''),
  notes,
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  platforms,
};

writeFileSync(args.out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`>>> updater.json 已生成: ${resolve(args.out)}（平台 ${Object.keys(platforms).join(', ')}）`);
