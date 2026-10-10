#!/usr/bin/env bash
# ci/macos/verify.sh — rust-verify-macos 的桥接执行体（GitHub macOS runner 上跑）。
# 职责对齐 .rust-node/.rust-verify：工具链钉版 + 前端构建 + fmt/clippy/test 全量。
# 差异：GitHub runner 是一次性 VM（无跨流水线增量，每次全量冷构建）；
# 数据中心直连静态 CDN 稳定，不需要 .rust-node 的国内镜像回退。
# RUST_VERSION/RUSTFLAGS 由 MACOS_ENV_PASSTHROUGH 从 GitLab 变量透传。
set -euo pipefail
cd "${CI_PROJECT_DIR:?CI_PROJECT_DIR 未注入}"

: "${RUST_VERSION:?RUST_VERSION 未透传（检查作业 MACOS_ENV_PASSTHROUGH）}"
for attempt in 1 2 3; do
  if rustup toolchain install "$RUST_VERSION" --profile minimal --component rustfmt,clippy; then
    break
  fi
  [ "$attempt" = 3 ] && { echo "rustup 工具链安装三连败（对齐 .rust-node 重试策略）" >&2; exit 1; }
  echo "rustup 第 ${attempt} 次失败，15s 后重试…"
  sleep 15
done
rustup default "$RUST_VERSION"
node --version
npm --version
cargo --version

npm ci --prefer-offline
npm run build
cargo fmt --all -- --check
# 独立的 cargo check 省略：clippy 是类型/借用检查的超集（对齐 .rust-verify）
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
