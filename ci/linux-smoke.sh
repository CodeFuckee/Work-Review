#!/usr/bin/env bash
# ci/linux-smoke.sh — Linux 桥接链路冒烟脚本（由 GitHub Actions 的 Ubuntu runner 执行）。
# 验证点：GLIBC/工具链环境、dist/ 产物回传（roundtrip 在 GitLab 侧验证 Job-Token 读回）。
# 产出：dist/linux-smoke.txt（桥接 workflow 自动 PUT 回 linux-bridge 包）。
set -euo pipefail
cd "${CI_PROJECT_DIR:-$PWD}"

out=linux-smoke.txt
{
  echo "== work-review Linux bridge smoke =="
  echo "time_utc: $(date -u +%FT%TZ)"
  echo "job: ${CI_JOB_NAME:-?} @ ${CI_COMMIT_SHA:-?} (pipeline ${CI_PIPELINE_IID:-?})"
  echo "host: $(hostname)  arch: $(uname -m)  kernel: $(uname -r)"
  echo "glibc: $(ldd --version | head -1 | awk '{print $NF}')"
  echo
  echo "-- toolchain --"
  (node --version) 2>/dev/null || echo "node: 未预装"
  (command -v rustup >/dev/null && rustup show active-toolchain) || echo "rust: rustup 未预装"
  (rustc --version && cargo --version) 2>/dev/null || true
  echo
  echo "-- sudo/apt 可用性（Linux 托管 runner 红利验证）--"
  sudo -n true 2>/dev/null && echo "sudo: 可用（免密）" || echo "sudo: 不可用"
  apt-get --version | head -1
  echo
  echo "-- GitLab 回连 --"
  curl -sS -m 10 -o /dev/null -w "gitlab https: %{http_code} (%{time_total}s)\n" \
    "https://${CI_SERVER_HOST:?CI_SERVER_HOST 未注入}/users/sign_in" || true
} | tee "$out"

mkdir -p dist
cp "$out" dist/
echo "smoke: dist/ 就绪，等待桥接 workflow 回传"
