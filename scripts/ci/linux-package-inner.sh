#!/usr/bin/env bash
# 容器内执行的 linux 打包脚本（由 .tauri-package-linux 以 runner 同 UID 运行）
set -euo pipefail

npm ci --cache /build/.npmc --prefer-offline
# 打包前清理 bundle 目录：runner 构建目录跨流水线持久，不清理会让
# 历史版本的安装包累积并被 publish 的全量 glob 选中（2026-09-16 更新失效根因）
rm -rf target/release/bundle target/*/release/bundle
npm run tauri build -- $@  # 有意不加引号：TAURI_BUILD_ARGS 需按空格拆分为多个参数
