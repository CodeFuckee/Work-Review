#!/usr/bin/env bash
# ci/bridge.sh — 把当前 GitLab 作业桥接到 GitHub Actions 的 macOS runner 上执行。
#
# 用法：在 .gitlab-ci.yml 里定义一个跑在 Linux shell runner 上的作业，
#   variables 里指定 MACOS_SCRIPT=（仓库内脚本路径），script 调本脚本。
#   MACOS_SCRIPT 必须自包含（自己装依赖、构建），产出物放 dist/，会由
#   GitHub 侧自动 PUT 回本项目的 generic package registry
#   （包名 macos-bridge，版本号 = correlation_id）。
#
# 依赖：bash、curl、python3（Ubuntu 22.04 runner 自带；刻意不依赖 jq）。
# 环境变量：
#   GH_BRIDGE_REPO      (必填) GitHub 桥接仓库 owner/repo，如 CodeFuckee/gl-macos-bridge
#   GH_BRIDGE_TOKEN     (必填) 对该仓库有 contents:write 的 GitHub PAT
#                        （GitLab 项目级 masked+protected CI 变量）
#   MACOS_SCRIPT        (必填) 要在 macOS 上执行的脚本（本仓库内路径）
#   MACOS_EXTRA_ENV     (可选) 附带传给 macOS 侧的变量 "K1=V1 K2=V2"（值不含空格）
#   GH_POLL_INTERVAL    (可选，默认 15) 轮询间隔秒
#   GH_POLL_TIMEOUT_MIN (可选，默认 40) 等待 GitHub run 的分钟上限，须小于作业 timeout
#
# 原理：组装 payload（脚本 base64 + 白名单 CI 变量）→ POST repository_dispatch
#   → 按 run-name 后缀（correlation_id）轮询终态 → 镜像为退出码；
#   本作业被取消时 trap 会同步取消 GitHub 侧 run。
# 安全：payload 不携带任何凭据；GH_BRIDGE_TOKEN 是 masked 变量，本脚本不回显。
set -euo pipefail

fail() { echo "bridge: $*" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || fail "runner 缺 curl"
command -v python3 >/dev/null 2>&1 || fail "runner 缺 python3（本脚本刻意不依赖 jq）"

GH_BRIDGE_REPO="${GH_BRIDGE_REPO:?GH_BRIDGE_REPO 未设置}"
GH_BRIDGE_TOKEN="${GH_BRIDGE_TOKEN:?GH_BRIDGE_TOKEN 未设置（见 docs/ci/macos-bridge.md）}"
MACOS_SCRIPT="${MACOS_SCRIPT:?MACOS_SCRIPT 未设置}"
[ -f "$MACOS_SCRIPT" ] || fail "脚本不存在: $MACOS_SCRIPT"

CORR="gl${CI_PROJECT_ID}-p${CI_PIPELINE_ID}j${CI_JOB_ID}"
INTERVAL="${GH_POLL_INTERVAL:-15}"
DEADLINE=$(( $(date +%s) + ${GH_POLL_TIMEOUT_MIN:-40} * 60 ))
API="https://api.github.com/repos/${GH_BRIDGE_REPO}"
# CI_SERVER_HOST 不含端口：非标端口实例须拼 CI_SERVER_PORT（与 payload 构造保持一致）
if [ "${CI_SERVER_PORT:-443}" = "443" ]; then
  GL_SERVER="${CI_SERVER_HOST}"
else
  GL_SERVER="${CI_SERVER_HOST}:${CI_SERVER_PORT}"
fi

gh_curl() {
  curl -sS -m 30 \
    -H "Authorization: Bearer ${GH_BRIDGE_TOKEN}" \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" "$@"
}

# 从 run 列表里按 run-name 后缀=correlation_id 定位本次 run：id status conclusion url
find_run() {
  gh_curl "${API}/actions/runs?event=repository_dispatch&per_page=30" | python3 -c '
import json, sys
corr = sys.argv[1]
try:
    runs = json.load(sys.stdin).get("workflow_runs", [])
except Exception:
    sys.exit(0)
for r in runs:
    if r.get("name", "").endswith(corr):
        print(r["id"], r["status"], r["conclusion"] or "-", r["html_url"])
        break
' "$CORR"
}

cancel_run() {
  local out rid
  out=$(find_run 2>/dev/null || true)
  rid=$(awk '{print $1}' <<<"${out:-}")
  if [ -n "$rid" ]; then
    echo "bridge: 取消 GitHub run ${rid}"
    gh_curl -X POST "${API}/actions/runs/${rid}/cancel" >/dev/null 2>&1 || true
  fi
}
trap cancel_run EXIT

# ---- ① 组装 payload（python3 保证 JSON 转义正确）----
PAYLOAD=$(python3 - <<'PY'
import base64, json, os

def env(k, d=""):
    return os.environ.get(k, d)

# CI_SERVER_HOST 不含端口（非标端口实例必须拼上 CI_SERVER_PORT，否则
# GitHub 侧克隆/回传会打到 443 上 404——首条真实流水线实测踩坑）
host = env("CI_SERVER_HOST")
port = env("CI_SERVER_PORT", "443")
server = host if port == "443" else f"{host}:{port}"

env_whitelist = {
    "CI_PROJECT_PATH": env("CI_PROJECT_PATH"),
    "CI_PROJECT_NAME": env("CI_PROJECT_NAME"),
    "CI_COMMIT_REF_NAME": env("CI_COMMIT_REF_NAME"),
    "CI_COMMIT_SHA": env("CI_COMMIT_SHA"),
    "CI_PIPELINE_IID": env("CI_PIPELINE_IID"),
    "CI_JOB_NAME": env("CI_JOB_NAME"),
    # 传修正后的带端口地址，macOS 侧脚本用它拼 URL
    "CI_SERVER_HOST": server,
}
# MACOS_EXTRA_ENV="K1=V1 K2=V2"（值不含空格；复杂需求改此处白名单）
for kv in os.environ.get("MACOS_EXTRA_ENV", "").split():
    if "=" in kv:
        k, _, v = kv.partition("=")
        env_whitelist[k] = v

payload = {
    "correlation_id": "gl{pid}-p{pl}j{jb}".format(
        pid=env("CI_PROJECT_ID"), pl=env("CI_PIPELINE_ID"), jb=env("CI_JOB_ID")),
    # GitHub repository_dispatch 的 client_payload 顶层属性上限 10 个：
    # GitLab 上下文收进嵌套对象（嵌套算 1 个属性），顶层固定 4 个。
    "gl": {
        "server": server,
        "project": env("CI_PROJECT_PATH"),
        "project_id": env("CI_PROJECT_ID"),
        "ref": env("CI_COMMIT_REF_NAME"),
        "sha": env("CI_COMMIT_SHA"),
        "pipeline": env("CI_PIPELINE_ID"),
        "job": env("CI_JOB_ID"),
        "job_name": env("CI_JOB_NAME"),
    },
    "env": env_whitelist,
    "script_b64": base64.b64encode(open(os.environ["MACOS_SCRIPT"], "rb").read()).decode(),
}
print(json.dumps(payload))
PY
) || fail "payload 组装失败"

# ---- ② 触发 repository_dispatch ----
echo "bridge: dispatch → ${GH_BRIDGE_REPO}（corr=${CORR}）"
gh_curl -f -X POST "${API}/dispatches" \
  -d "{\"event_type\":\"gl-macos-job\",\"client_payload\":${PAYLOAD}}" >/dev/null \
  || fail "repository_dispatch 失败：检查 GH_BRIDGE_REPO 名称、GH_BRIDGE_TOKEN 权限（需 contents:write）与有效期"

# ---- ③ 轮询终态并镜像 ----
echo "bridge: 轮询 GitHub run（每 ${INTERVAL}s，上限 ${GH_POLL_TIMEOUT_MIN:-40} 分钟）"
while :; do
  out=$(find_run || true)
  if [ -n "${out:-}" ]; then
    read -r rid state concl url <<<"$out"
    echo "  GitHub run ${rid}: ${state}/${concl}  ${url}"
    if [ "$state" = "completed" ]; then
      trap - EXIT
      if [ "$concl" = "success" ]; then
        echo "bridge: ✓ macOS 作业成功；产物（如有）已回传 package registry："
        echo "  https://${GL_SERVER}/api/v4/projects/${CI_PROJECT_ID}/packages/generic/macos-bridge/${CORR}/${CORR}.tgz"
        exit 0
      fi
      echo "bridge: ✗ macOS 作业失败（${concl}），日志：${url}" >&2
      exit 1
    fi
  fi
  [ "$(date +%s)" -gt "$DEADLINE" ] && fail "等待 GitHub run 超时（run 可能仍在排队/执行）"
  sleep "$INTERVAL"
done
