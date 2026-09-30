#!/usr/bin/env bash
# 检测自最新版本 tag 以来是否有「程序代码」变更（src/、src-tauri/、crates/）。
# 退出码 0 = 有代码变更（应递增版本并发版）；1 = 无（文档/CI 等变更，跳过发版）。
# 无任何 tag 时保守视为有代码变更。
set -euo pipefail

LAST_TAG="$(git describe --tags --abbrev=0 2>/dev/null || echo '')"
if [ -z "$LAST_TAG" ]; then
  echo "no tag found -> treat as code change"
  exit 0
fi

# 排除 target/ 等产物目录不必要（diff 只看已跟踪文件）
CODE_CHANGED=0
while IFS= read -r path; do
  case "$path" in
    src/*|src-tauri/*|crates/*)
      CODE_CHANGED=1
      echo "code change: $path"
      ;;
  esac
done < <(git diff --name-only "$LAST_TAG" HEAD)

if [ "$CODE_CHANGED" = 1 ]; then
  exit 0
fi
echo "no code change since $LAST_TAG"
exit 1
