#!/usr/bin/env bash
# ci/macos/package.sh — 全部 macOS 打包作业的桥接执行体（tag 发版 + 夜间产包，
# arm64/x64 共用，目标三元组经 MACOS_ENV_PASSTHROUGH 的 TAURI_TARGET_TRIPLE
# 传入，缺省 aarch64）。签名：TAURI_SIGNING_PRIVATE_KEY(_PASSWORD) 由桥接
# workflow 从 GitHub secrets（GL131_ 前缀）注入环境——tauri-cli v2.12 直接读
# 这两个环境变量（名字与 GitLab 变量一致），密钥绝不进 payload（公开数据）。
# 产物按仓库相对路径镜像进 dist/target/<triple>/release/bundle，拉回还原后
# publish-release 的 target/**/release/bundle 全量 glob 原样命中，publish 零改动。
set -euo pipefail
cd "${CI_PROJECT_DIR:?CI_PROJECT_DIR 未注入}"

TARGET_TRIPLE="${TAURI_TARGET_TRIPLE:-aarch64-apple-darwin}"
export APPLE_SIGNING_IDENTITY="-"      # ad-hoc 签名（对齐原作业变量）

: "${RUST_VERSION:?RUST_VERSION 未透传（检查作业 MACOS_ENV_PASSTHROUGH）}"
for attempt in 1 2 3; do
  if rustup toolchain install "$RUST_VERSION" --profile minimal; then
    break
  fi
  [ "$attempt" = 3 ] && { echo "rustup 工具链安装三连败（对齐 .rust-node 重试策略）" >&2; exit 1; }
  echo "rustup 第 ${attempt} 次失败，15s 后重试…"
  sleep 15
done
rustup default "$RUST_VERSION"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
# GitHub 托管 runner 的 SDK 由 xcode-select 统一管理，无 CLT 残留 SDKROOT 问题；
# 导出对齐 xcrun 当前选择（对齐原作业 2026-09-14 修复，无害）
export SDKROOT="$(xcrun --show-sdk-path)"

[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ] \
  || { echo "TAURI_SIGNING_PRIVATE_KEY 未注入（检查桥接仓库 secrets GL131_TAURI_SIGNING_*）" >&2; exit 1; }

# 应用内更新器 token：缺失时产物更新器静默「未配置更新源」（v1.2.28-1.2.30
# Mac 包实测事故——Windows 本机 runner 有、macOS 桥接没有，差异只在透传）
[ -n "${WORK_REVIEW_UPDATER_GITLAB_TOKEN:-}" ] \
  || { echo "WORK_REVIEW_UPDATER_GITLAB_TOKEN 未注入（检查桥接仓库 secrets GL131_WORK_REVIEW_UPDATER_GITLAB_TOKEN 与 workflow env 映射）" >&2; exit 1; }

npm ci --prefer-offline
# 打包前清理 bundle：publish 的全量 glob 防历史版本包混入（2026-09-16 更新失效根因；
# 一次性 VM 本无残留，保留防线对齐 .tauri-package）
rm -rf target/release/bundle target/*/release/bundle
npm run tauri build -- --target "$TARGET_TRIPLE"

# 产物自检：打包出的 .app 二进制必须真实嵌入更新器 token（防编译缓存/环境
# 差异吞掉注入的假成功——更新器会静默降级为「已是最新」）
APP_BIN="target/${TARGET_TRIPLE}/release/bundle/macos/Yue-Review.app/Contents/MacOS/Yue_Review"
[ -f "$APP_BIN" ] || APP_BIN=$(find target/*/release/bundle/macos -path "*Contents/MacOS/Yue_Review" 2>/dev/null | head -1)
if [ -z "$APP_BIN" ] || ! strings -a "$APP_BIN" | grep -q "glpat-"; then
  echo "产物自检失败：.app 二进制未嵌入更新器 token（glpat-）：$APP_BIN" >&2
  exit 1
fi

mkdir -p "dist/target/${TARGET_TRIPLE}/release"
cp -R "target/${TARGET_TRIPLE}/release/bundle" "dist/target/${TARGET_TRIPLE}/release/bundle"
