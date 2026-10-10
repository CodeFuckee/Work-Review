#!/usr/bin/env bash
# ci/macos-smoke.sh — macOS 桥接链路冒烟脚本（由 GitHub Actions 的 macos runner 执行）。
# 验证点：GitLab 网络可达、按 SHA 克隆的环境完整性、dist/ 产物回传。
# 产出：dist/bridge-smoke.txt（由桥接 workflow 自动 PUT 回 GitLab package registry）。
set -euo pipefail
cd "${CI_PROJECT_DIR:-$PWD}"

out=bridge-smoke.txt
{
  echo "== work-review macOS bridge smoke =="
  echo "time_utc: $(date -u +%FT%TZ)"
  echo "job: ${CI_JOB_NAME:-?} @ ${CI_COMMIT_SHA:-?} (pipeline ${CI_PIPELINE_IID:-?})"
  echo "host: $(hostname)  arch: $(uname -m)  kernel: $(uname -r)"
  sw_vers
  echo
  echo "-- toolchain --"
  xcodebuild -version || true
  clang --version | head -1 || true
  (command -v rustup >/dev/null && rustup show active-toolchain) || echo "rust: rustup 未预装"
  (rustc --version && cargo --version) 2>/dev/null || true
  node --version 2>/dev/null || echo "node: 未预装"
  echo
  echo "-- GitLab 回连 --"
  # /users/sign_in 匿名可访问（200）；API 类端点匿名是 401，会污染判读
  curl -sS -m 10 -o /dev/null -w "gitlab https: %{http_code} (%{time_total}s)\n" \
    "https://${CI_SERVER_HOST:?CI_SERVER_HOST 未注入}/users/sign_in" || true
} | tee "$out"

mkdir -p dist
cp "$out" dist/
echo "smoke: dist/ 就绪，等待桥接 workflow 回传"
