#!/usr/bin/env bash
# ci/linux/verify.sh — rust-verify-linux 的桥接执行体（GitHub Ubuntu runner 上跑）。
# 与自建 shell runner 版的三个假设差异：
#   ① 托管 runner 可 sudo apt-get（自建版无 TTY 不能装包，靠 operator 预装 +
#      check-linux-deps.sh 校验——托管版直接安装，预检脚本退役）；
#   ② 一次性 VM 无跨流水线增量（cargo registry 缓存由桥接 workflow 的 actions/cache
#      提供，target 不缓存）；
#   ③ 工具链自装（rustup 预装；对齐 .rust-node 的钉版与重试策略，托管网络稳定
#      不需要国内镜像回退）。
# RUST_VERSION/RUSTFLAGS 由 MACOS_ENV_PASSTHROUGH 从 GitLab 变量透传。
set -euo pipefail
cd "${CI_PROJECT_DIR:?CI_PROJECT_DIR 未注入}"

: "${RUST_VERSION:?RUST_VERSION 未透传（检查作业 MACOS_ENV_PASSTHROUGH）}"

# ① 系统依赖（Ubuntu 24.04 noble 包名，对齐 scripts/check-linux-deps.sh 清单）
sudo apt-get update -qq
sudo apt-get install -y -qq libssl-dev pkg-config libwebkit2gtk-4.1-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev

# ② 工具链钉版
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

# 覆盖率（对齐原作业语义：llvm-cov 内部跑全量 cargo test，等价替代独立 test 步骤；
# GitLab 作业的 coverage 正则解析其 TOTAL 行）。cargo-llvm-cov 二进制由桥接
# workflow 的 ~/.cargo/bin 缓存跨 run 复用，首次安装约 3-6 分钟
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
cargo llvm-cov --workspace --all-targets --summary-only
