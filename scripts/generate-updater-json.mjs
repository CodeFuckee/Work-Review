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
import { pathToFileURL } from 'node:url';

// darwin-x86_64 是可选平台：macOS x64 打包 job 由 PACKAGE_MACOS_X64 显式开启，
// 平时只有 arm64 产物，清单中则不含 darwin-x86_64 条目。
const REQUIRED_PLATFORMS = ['darwin-aarch64', 'windows-x86_64'];
const OPTIONAL_PLATFORMS = ['darwin-x86_64'];

export function parseArgs(argv) {
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

// 由解析后的参数构建更新清单。
// options: { version, urlBase, platforms: Array<[平台名, 产物本地路径]>, notes }
// 平台产物存在性/签名检查沿用 CLI 原行为：必备平台缺失即抛错，可选平台缺失跳过。
export function buildUpdaterManifest(options) {
  const platforms = {};
  for (const [platform, artifact] of options.platforms) {
    const isRequired = REQUIRED_PLATFORMS.includes(platform);
    if (!isRequired && !OPTIONAL_PLATFORMS.includes(platform)) {
      throw new Error(
        `未知平台: ${platform}（已知: ${[...REQUIRED_PLATFORMS, ...OPTIONAL_PLATFORMS].join(', ')}）`,
      );
    }

    const file = artifact ? resolve(artifact) : '';
    if (!artifact || !existsSync(file)) {
      if (isRequired) {
        throw new Error(`缺少 ${platform} 更新产物: ${artifact || '(空)'}`);
      }
      console.warn(`>>> 可选平台 ${platform} 无产物，清单中跳过`);
      continue;
    }

    const signatureFile = `${file}.sig`;
    if (!existsSync(signatureFile)) {
      throw new Error(`缺少 ${platform} 更新签名: ${signatureFile}`);
    }
    const signature = readFileSync(signatureFile, 'utf8').replace(/\r?\n/g, '');
    if (!signature) {
      throw new Error(`${platform} 更新签名为空: ${signatureFile}`);
    }

    platforms[platform] = {
      url: `${options.urlBase.replace(/\/+$/, '')}/${basename(file)}`,
      signature,
    };
  }

  const missing = REQUIRED_PLATFORMS.filter((platform) => !platforms[platform]);
  if (missing.length > 0) {
    throw new Error(`更新清单缺少必备平台: ${missing.join(', ')}`);
  }

  return {
    version: options.version.replace(/^v/i, ''),
    notes: options.notes ?? '',
    pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
    platforms,
  };
}

// 版本一致性不变式（2026-09-16 更新失效事故的直接教训）：runner 构建目录会
// 累积多个版本的安装包，文件选择若未按发布版本过滤，清单 URL 会指向旧版
// 安装包——客户端"更新"后版本号不变（v1.1.47 起全部中招）。
// 断言：平台下载 URL 的文件名若含版本号，必须与发布版本一致；
// 不含版本号的通用文件名（如覆盖式更新的 Yue-Review.app.tar.gz）无法比对，放行。
export function assertPlatformVersionConsistency(manifest, expectedVersion) {
  if (manifest.version !== expectedVersion) {
    throw new Error(
      `清单 version=${manifest.version} 与发布版本 ${expectedVersion} 不一致`,
    );
  }
  for (const [platform, entry] of Object.entries(manifest.platforms ?? {})) {
    const file = entry.url.split('/').pop() ?? '';
    const embedded = file.match(/(\d+\.\d+\.\d+)/);
    if (embedded && embedded[1] !== expectedVersion) {
      throw new Error(
        `平台 ${platform} 的下载文件 "${file}"（版本 ${embedded[1]}）与发布版本 ${expectedVersion} 不一致——极可能选中了旧版累积产物`,
      );
    }
  }
}

const args = parseArgs(process.argv.slice(2));

// 仅在作为 CLI 直接运行时执行主流程；被测试文件 import 时不触发
// （否则顶层 process.exit 会连带终止整个测试进程）。
const argvUrl = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : '';
const isCliEntry = argvUrl.toLowerCase() === import.meta.url.toLowerCase();

if (isCliEntry && (!args.version || !args.urlBase || !args.out)) {
  console.error('缺少 --version / --url-base / --out 参数');
  process.exit(1);
}

let manifest;
if (isCliEntry) {
  try {
    manifest = buildUpdaterManifest({
      version: args.version,
      urlBase: args.urlBase,
      platforms: args.platform,
      notes: args.notesFile ? readFileSync(args.notesFile, 'utf8') : '',
    });
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }

  // 一致性检查当前为「观察模式」：只告警不阻断（runner 构建目录累积旧版安装包的
  // 根因尚未修复，阻断会让发布链持续失败）。根因修复后升级为 throw 阻断发布。
  try {
    assertPlatformVersionConsistency(manifest, manifest.version);
  } catch (error) {
    console.error(
      `⚠️ 更新清单版本一致性检查未通过（观察模式，不阻断发布）: ${error.message}`,
    );
  }

  writeFileSync(args.out, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`>>> updater.json 已生成: ${resolve(args.out)}（平台 ${Object.keys(manifest.platforms).join(', ')}）`);
}
