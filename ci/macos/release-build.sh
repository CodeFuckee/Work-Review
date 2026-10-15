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

# 应用内更新器的 GitLab token 编译期注入（updater.rs option_env!）。桥接 runner
# 环境由桥接仓库 secrets GL131_WORK_REVIEW_UPDATER_GITLAB_TOKEN 提供；缺失时
# 产物更新器静默降级「未配置更新源」（v1.2.28-1.2.30 Mac 包实测事故），必须 fail-fast
[ -n "${WORK_REVIEW_UPDATER_GITLAB_TOKEN:-}" ] \
  || { echo "WORK_REVIEW_UPDATER_GITLAB_TOKEN 未注入（检查桥接仓库 secrets GL131_WORK_REVIEW_UPDATER_GITLAB_TOKEN 与 workflow env 映射）" >&2; exit 1; }

npm ci --prefer-offline
npm run build
cargo build --release -p work-review

# 产物自检：token 必须真实嵌入二进制（GitLab PAT 字面量 glpat- 前缀），
# 防 env 存在但 cargo 缓存/配置吞掉注入的假成功
# grep -c 读满输入，避免 grep -q 提前关管道令 macOS strings 报 SIGPIPE
if [ "$(strings -a target/release/work-review | grep -c "glpat-" || true)" -eq 0 ]; then
  echo "产物自检失败：二进制未嵌入更新器 token（glpat-）" >&2
  exit 1
fi

rm -rf dist
mkdir -p dist/target/release
# 二进制名 = cargo 包名 work-review（无 [[bin]] 重命名；tauri 打包时才用
# --binary-name 改名）。原作业声明的 Yue_Review 产物路径在 macOS 上一直是空产物
# （GitLab 对缺失路径仅警告），此处按真实产物名镜像
[ -f target/release/work-review ] || { echo "release 二进制不存在：target/release/work-review" >&2; exit 1; }
cp target/release/work-review dist/target/release/work-review
