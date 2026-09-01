import test from 'node:test';
import assert from 'node:assert/strict';

import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Windows CI 工具链引导测试。覆盖 .gitlab-ci.yml 的 `.powershell-toolchain`
// 引用的 scripts/windows-toolchain.ps1，以及其前身（commit 703624a 内联逻辑）
// 的缺陷模式：只兜底一个写死目录，node 由 nvm-windows 等安装时找不到。
// 仅 Windows 可运行（脚本依赖 Windows 的 C:\ 布局），其它平台跳过。
const SCRIPT_PATH = fileURLToPath(new URL('./windows-toolchain.ps1', import.meta.url));

const WINDOWS_ONLY = process.platform === 'win32';

// 前身逻辑（.gitlab-ci.yml @703624a 第 68-74 行）：==NODE_DIR_PLACEHOLDER==
// 由测试注入「官方安装目录」，以隔离本机真实环境。
// 注意：模板字符串中的 PowerShell 变量（$nodeDir、$env:Path 等）必须转义，
// 避免被 TypeScript 当作插值表达式。
const LEGACY_NODE_PROBE_PS = `
\$ErrorActionPreference = "Stop"
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  \$nodeDir = "==NODE_DIR_PLACEHOLDER=="
  if (Test-Path "\$nodeDir\\node.exe") { \$env:Path = "\$nodeDir;\$env:Path" }
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  throw "node not found. Install Node.js 22 and ensure it is on the PATH of the gitlab-runner service account."
}
\$cmd = Get-Command node -ErrorAction SilentlyContinue
if (\$cmd) { Write-Output ("NODE_FOUND=" + \$cmd.Source) }
`;

interface SpawnResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

/**
 * 解析可用的 PowerShell 解释器（脚本需同时兼容 Windows PowerShell 5.1 与
 * PowerShell 7；runner 与本地开发机的发现路径可能不同）：
 * 1) 环境变量 PWSH
 * 2) PATH 中的 pwsh（PowerShell 7）
 * 3) 标准安装路径 C:\Program Files\PowerShell\7\pwsh.exe
 * 4) Windows 自带 powershell.exe（PowerShell 5.1）
 * 找不到时返回 null，测试跳过。
 */
function resolvePowerShell(): string | null {
  const candidates: string[] = [];
  if (process.env.PWSH) candidates.push(process.env.PWSH);

  const programFiles = process.env.ProgramFiles ?? 'C:\\Program Files';
  candidates.push(
    join(programFiles, 'PowerShell', '7', 'pwsh.exe'),
    join(programFiles, 'PowerShell', '7-preview', 'pwsh.exe'),
    join(
      process.env.SystemRoot ?? 'C:\\Windows',
      'System32',
      'WindowsPowerShell',
      'v1.0',
      'powershell.exe',
    ),
  );
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

const POWER_SHELL = resolvePowerShell();

function runPowerShell(args: string[], env: NodeJS.ProcessEnv): SpawnResult {
  assert.ok(POWER_SHELL, 'no PowerShell interpreter found');
  const result = spawnSync(POWER_SHELL, ['-NoProfile', '-NonInteractive', '-Command', ...args], {
    env,
    encoding: 'utf8',
    timeout: 60_000,
  });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr };
}

/** PATH 仅保留系统目录：确保待探测的 node 不在 PATH 中。 */
function minimalPathEnv(): NodeJS.ProcessEnv {
  return { ...process.env, Path: `${process.env.SystemRoot ?? 'C:\\Windows'}\\System32` };
}

function makeFixture(t: test.TestContext): string {
  const fixtureDir = mkdtempSync(join(tmpdir(), 'work-review-toolchain-'));
  t.after(() => rmSync(fixtureDir, { recursive: true, force: true }));
  return fixtureDir;
}

test('legacy probe (commit 703624a): fails to find node under nvm-windows layout', { skip: !WINDOWS_ONLY }, (t) => {
  const fixtureDir = makeFixture(t);

  // nvm-windows 布局：node.exe 位于 %APPDATA%\nvm\<version>\。
  const nvmVersionDir = join(fixtureDir, 'nvm', 'v22.20.0');
  mkdirSync(nvmVersionDir, { recursive: true });
  writeFileSync(join(nvmVersionDir, 'node.exe'), '');

  // 「官方安装目录」模拟为另一个空目录（流水线失败时该目录没有 node.exe）。
  const officialDir = join(fixtureDir, 'Program Files', 'nodejs');
  mkdirSync(officialDir, { recursive: true });

  const probe = LEGACY_NODE_PROBE_PS.replace(
    '==NODE_DIR_PLACEHOLDER==',
    officialDir.replaceAll('\\', '\\\\'),
  );
  const result = runPowerShell([probe], minimalPathEnv());
  console.log(`exit=${result.status}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);

  // 旧逻辑只查单个写死目录，因此在 nvm 布局下必然失败（非零退出）。
  // 该用例守护「定位逻辑不得退化回单目录」。
  assert.notEqual(result.status, 0, 'legacy probe should not find node in nvm-windows layout');
  assert.match(result.stderr + result.stdout, /node not found/);
});

test('bootstrap script: finds node when installed under nvm-windows layout', { skip: !WINDOWS_ONLY }, (t) => {
  const fixtureDir = makeFixture(t);
  const nvmVersionDir = join(fixtureDir, 'nvm', 'v22.20.0');
  mkdirSync(nvmVersionDir, { recursive: true });
  writeFileSync(join(nvmVersionDir, 'node.exe'), '');

  const result = runPowerShell(
    [`& '${SCRIPT_PATH}' -NodeOnly -NodeCandidateDirs '${nvmVersionDir}'`],
    minimalPathEnv(),
  );
  console.log(`exit=${result.status}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);

  assert.equal(result.status, 0, `bootstrap should succeed: ${result.stderr}`);
  assert.match(result.stdout, new RegExp(`NODE_DIR=${escapeRegExp(nvmVersionDir)}`));
});

test('bootstrap script: finds node in default official location when PATH lacks it', { skip: !WINDOWS_ONLY }, (t) => {
  const officialDir = process.env.ProgramFiles
    ? `${process.env.ProgramFiles}\\nodejs`
    : 'C:\\Program Files\\nodejs';
  if (!existsSync(join(officialDir, 'node.exe'))) {
    t.skip(`no node.exe in ${officialDir}; nothing to assert for the default candidate`);
    return;
  }

  const result = runPowerShell([`& '${SCRIPT_PATH}' -NodeOnly`], minimalPathEnv());
  console.log(`exit=${result.status}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);

  assert.equal(result.status, 0, `bootstrap should find node in ${officialDir}: ${result.stderr}`);
  assert.match(result.stdout, /NODE_DIR=/);
});

test('bootstrap script: reports a diagnostic listing searched locations when node is missing', { skip: !WINDOWS_ONLY }, (t) => {
  const fixtureDir = makeFixture(t);
  const emptyDir = join(fixtureDir, 'no-node-here');
  mkdirSync(emptyDir, { recursive: true });

  const result = runPowerShell(
    [`& '${SCRIPT_PATH}' -NodeOnly -NodeCandidateDirs '${emptyDir}'`],
    minimalPathEnv(),
  );
  console.log(`exit=${result.status}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);

  assert.notEqual(result.status, 0, 'bootstrap should fail when no node exists anywhere');
  const combined = result.stderr + result.stdout;
  assert.match(combined, /node not found/);
  // 报错必须列出实际查找过的位置（含注入的候选），方便排查。
  assert.ok(combined.includes(emptyDir), `error should mention searched dir ${emptyDir}`);
});

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}