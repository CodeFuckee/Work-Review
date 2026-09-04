#!/usr/bin/env node
// 版本自增脚本（GitLab CI version-bump job 与本地共用，Node 跨平台）。
//
// 规则：末位 +1；末位逢 100 进位（中位 +1、末位归 0）；中位逢 100 再进位到首位。
// 同步五个版本源：
//   package.json / package-lock.json（顶层 + packages[""].version）/
//   src-tauri/tauri.conf.json / src-tauri/Cargo.toml / Cargo.lock（work-review 包）
//
// 用法：
//   node scripts/bump-version.mjs [--root DIR] [--dry-run]
// 成功输出：NEW_VERSION=x.y.z；--dry-run 只打印不写文件。
// 任何一处版本源缺失/不一致都会非零退出并给出提示。

import { readFileSync, writeFileSync, existsSync, realpathSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

/** 末位自增，逢 100 进位；非法输入返回 null。 */
export function computeNextVersion(current) {
  if (typeof current !== 'string') return null;
  if (!/^\d+\.\d+\.\d+$/.test(current)) return null;
  const [major, minor, patch] = current.split('.').map(Number);
  let nextPatch = patch + 1;
  let nextMinor = minor;
  let nextMajor = major;
  if (nextPatch >= 100) {
    nextPatch = 0;
    nextMinor += 1;
  }
  if (nextMinor >= 100) {
    nextMinor = 0;
    nextMajor += 1;
  }
  return `${nextMajor}.${nextMinor}.${nextPatch}`;
}

/** 把文本里所有 `"<oldVersion>"` 替换为 `"<newVersion>"`（用于 JSON 版本段）。 */
function replaceAllVersion(text, oldVersion, newVersion) {
  if (!text.includes(`"${oldVersion}"`)) {
    throw new Error(`未找到版本字段 ${oldVersion}`);
  }
  return text.split(`"${oldVersion}"`).join(`"${newVersion}"`);
}

/** package-lock.json 需要精确替换两处：顶层与 packages[""]（依赖恰好同版本时不得误伤）。 */
function replacePackageLockSimple(text, oldVersion, newVersion) {
  const needle = `"version": "${oldVersion}"`;
  const topIdx = text.indexOf(needle);
  if (topIdx < 0) throw new Error('package-lock.json 顶层 version 未找到');
  const packagesIdx = text.indexOf('"packages": {');
  if (packagesIdx < 0) throw new Error('package-lock.json packages 块未找到');
  const rootIdx = text.indexOf(needle, packagesIdx);
  if (rootIdx < 0) throw new Error('package-lock.json packages 根条目 version 未找到');

  const replacement = `"version": "${newVersion}"`;
  let out = text;
  // 先替换靠后的 packages[""] 处，再替换顶层
  out = out.slice(0, rootIdx) + replacement + out.slice(rootIdx + needle.length);
  out = out.slice(0, topIdx) + replacement + out.slice(topIdx + needle.length);
  return out;
}

/** Cargo.lock：只替换 work-review 包块的版本。 */
function replaceCargoLock(text, oldVersion, newVersion) {
  const pattern = `[[package]]\nname = "work-review"\nversion = "${oldVersion}"`;
  if (!text.includes(pattern)) {
    // 兼容 CRLF 与尾随注释
    const loose = /\[\[package\]\]\r?\nname = "work-review"\r?\nversion = "([^"]+)"/;
    const match = text.match(loose);
    if (!match) throw new Error(`Cargo.lock 未找到 work-review 包 (当前 ${match ? match[1] : '?'})`);
    if (match[1] !== oldVersion) {
      throw new Error(`Cargo.lock work-review 版本 ${match[1]} 与 package.json ${oldVersion} 不一致`);
    }
    return text.replace(match[0], `[[package]]\nname = "work-review"\nversion = "${newVersion}"`);
  }
  return text.split(pattern).join(`[[package]]\nname = "work-review"\nversion = "${newVersion}"`);
}

/** Cargo.toml：替换与旧版本一致的 `version = "x.y.z"` 行（依赖的 `version = "2"` 等不会误伤）。 */
function replaceCargoToml(text, oldVersion, newVersion) {
  const pattern = `version = "${oldVersion}"`;
  if (!text.includes(pattern)) {
    const match = text.match(/^version\s*=\s*"([^"]+)"/m);
    throw new Error(
      `src-tauri/Cargo.toml 未找到版本 ${oldVersion}${match ? `（当前 ${match[1]}）` : ''}`,
    );
  }
  return text.split(pattern).join(`version = "${newVersion}"`);
}

function readWithCheck(root, relative) {
  const path = join(root, relative);
  if (!existsSync(path)) throw new Error(`缺少版本源：${relative}`);
  return readFileSync(path, 'utf8');
}

/** 原地同步五个版本源；返回新版本。 */
export function bumpVersion(root) {
  const pkgPath = join(root, 'package.json');
  let packageJson = readWithCheck(root, 'package.json');
  let pkg;
  try {
    pkg = JSON.parse(packageJson);
  } catch (error) {
    throw new Error(`package.json 解析失败: ${error.message}`);
  }
  const oldVersion = pkg.version;
  if (typeof oldVersion !== 'string' || oldVersion.trim() === '') {
    throw new Error('package.json 缺少版本号');
  }
  const newVersion = computeNextVersion(oldVersion);
  if (!newVersion) throw new Error(`非法版本号：${oldVersion}`);

  // package.json：替换根级第一个 "version" 字段
  packageJson = replaceAllVersion(packageJson, oldVersion, newVersion);
  writeFileSync(pkgPath, packageJson, 'utf8');

  // package-lock.json：顶层 + packages[""]
  const lockPath = join(root, 'package-lock.json');
  const lockText = readWithCheck(root, 'package-lock.json');
  writeFileSync(lockPath, replacePackageLockSimple(lockText, oldVersion, newVersion), 'utf8');

  // src-tauri/tauri.conf.json
  const tauriPath = join(root, 'src-tauri', 'tauri.conf.json');
  writeFileSync(
    tauriPath,
    replaceAllVersion(readWithCheck(root, 'src-tauri/tauri.conf.json'), oldVersion, newVersion),
    'utf8',
  );

  // src-tauri/Cargo.toml
  const cargoPath = join(root, 'src-tauri', 'Cargo.toml');
  writeFileSync(
    cargoPath,
    replaceCargoToml(readWithCheck(root, 'src-tauri/Cargo.toml'), oldVersion, newVersion),
    'utf8',
  );

  // Cargo.lock（work-review 包块）
  const cargoLockPath = join(root, 'Cargo.lock');
  writeFileSync(
    cargoLockPath,
    replaceCargoLock(readWithCheck(root, 'Cargo.lock'), oldVersion, newVersion),
    'utf8',
  );

  return newVersion;
}

// ---- CLI ----
function parseArgs(argv) {
  const args = { root: process.cwd(), dryRun: false };
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--root') {
      args.root = argv[i + 1];
      i += 1;
    } else if (arg === '--dry-run') {
      args.dryRun = true;
    } else {
      throw new Error(`未知参数：${arg}`);
    }
  }
  return args;
}

// 仅在作为脚本直接执行时运行（node scripts/bump-version.mjs ...）。
// 之前没有这个守卫：bump-version.test.ts 为复用 computeNextVersion 而
// import 本模块，会连带执行下方 CLI——parseArgs 默认 root=process.cwd()，
// 于是在 tag 流水线的 package 作业里 `npm run test:frontend` 一跑，
// 工作区的五个版本源就被原地 +1，产出比 tag 版本号 +1 的安装包。
function isDirectInvocation() {
  try {
    return import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href;
  } catch {
    return false;
  }
}

if (isDirectInvocation()) {
  try {
    const { root, dryRun } = parseArgs(process.argv);
    const packageJsonRaw = readFileSync(join(root, 'package.json'), 'utf8');
    const current = JSON.parse(packageJsonRaw).version;
    const next = computeNextVersion(current);
    if (!next) {
      console.error(`非法版本号：${current}`);
      process.exit(1);
    }
    if (dryRun) {
      console.log(`NEW_VERSION=${current} -> ${next}`);
      process.exit(0);
    }
    const bumped = bumpVersion(root);
    console.log(`NEW_VERSION=${bumped}`);
  } catch (error) {
    console.error(`版本自增失败：${error.message}`);
    process.exit(1);
  }
}