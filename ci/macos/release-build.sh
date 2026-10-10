#!/usr/bin/env bash
# ci/macos/release-build.sh — build-release-macos 的桥接执行体：release 冷编译验证。
# 产物按仓库相对路径镜像进 dist/（dist/target/release/Yue_Review），桥接作业
# （MACOS_FETCH_ARTIFACTS=true）拉回解包后还原 target/release/Yue_Review 原路径，
# version-bump 等下游的 artifacts 声明零改动。
set -euo pipefail
cd "${CI_PROJECT_DIR:?CI_PROJECT_DIR 未注入}"

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

npm ci --prefer-offline
npm run build
cargo build --release -p work-review

mkdir -p dist/target/release
cp target/release/Yue_Review dist/target/release/Yue_Review
