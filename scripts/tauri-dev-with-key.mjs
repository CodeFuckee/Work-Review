#!/usr/bin/env node
// tauri:dev 包装：编译期注入内置兜底模型的 API Key（与 CI 变量
// WORK_REVIEW_BUILTIN_MODEL_API_KEY 同源），使本地开发构建与发布版行为一致——
// 设置页 AI 区块可见「内置模型」指示卡，AI 功能开箱即用。
//
// Key 存放在 git-ignored 的 src-tauri/.builtin-model-key.local（从 GitLab
// CI/CD 变量获取）。文件缺失时照常启动，但构建不含内置模型（设置页无指示卡，
// AI 功能退回「必须自行配置」的旧行为），并会打印本说明。
import { existsSync, readFileSync, utimesSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const keyPath = join(dirname(fileURLToPath(import.meta.url)), '../src-tauri/.builtin-model-key.local');

let builtinKey = '';
if (existsSync(keyPath)) {
  builtinKey = readFileSync(keyPath, 'utf8').trim();
}
if (!builtinKey) {
  console.warn('>>> 未找到 src-tauri/.builtin-model-key.local：本次开发构建不含内置模型。');
  console.warn('>>> 如需内置模型（设置页指示卡 + AI 开箱即用），把 GitLab CI 变量');
  console.warn('>>> WORK_REVIEW_BUILTIN_MODEL_API_KEY 的值写入该文件后重跑。');
}

// option_env! 在编译期取值，cargo 不会因环境变量变化而重编译——
// touch 入口文件强制重编 bin crate，确保 key 变更后构建即时生效。
if (builtinKey) {
  const mainRs = join(dirname(fileURLToPath(import.meta.url)), '../src-tauri/src/main.rs');
  utimesSync(mainRs, new Date(), new Date());
}

const result = spawnSync('npm', ['run', 'tauri:dev:raw'], {
  stdio: 'inherit',
  shell: true,
  env: builtinKey
    ? { ...process.env, WORK_REVIEW_BUILTIN_MODEL_API_KEY: builtinKey }
    : process.env,
});
process.exit(result.status ?? 1);
