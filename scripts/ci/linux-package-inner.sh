#!/usr/bin/env bash
# 容器内执行的 linux 打包脚本（由 .tauri-package-linux 以 runner 同 UID 运行）
set -euo pipefail

# npm 缓存指向 /build/.npm（GIT_CLEAN_FLAGS 排除清单内的持久缓存，与其余
# 作业一致）；此前用 /build/.npmc 不在排除清单，每次 checkout 被清、全量重下
npm ci --cache /build/.npm --prefer-offline
# 打包前清理 bundle 目录：runner 构建目录跨流水线持久，不清理会让
# 历史版本的安装包累积并被 publish 的全量 glob 选中（2026-09-16 更新失效根因）
rm -rf target/release/bundle target/*/release/bundle

# AppImage 工具预热与 sha256 门禁抽到共享脚本（夜间产包作业 package-daily-* 同用，
# 白名单单点维护）：见 scripts/ci/appimage-tools.sh 头注
TAURI_ARCH=x86_64
case "$*" in *aarch64*) TAURI_ARCH=aarch64 ;; esac
. "$(dirname "$0")/appimage-tools.sh" "$TAURI_ARCH"

# --verbose：tauri-bundler 会透传 linuxdeploy 的 stderr（默认被吞，
# v1.2.20/v1.2.21 两版 linuxdeploy 运行数百秒后失败均无任何工具输出可查）
npm run tauri build -- --verbose $@  # 有意不加引号：TAURI_BUILD_ARGS 需按空格拆分为多个参数
