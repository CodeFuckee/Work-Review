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

# 签名身份稳定化（可选）：桥接仓库 secrets GL131_MACOS_SIGNING_P12（base64 p12）
# / GL131_MACOS_SIGNING_P12_PASSWORD 注入后，把自签代码签名证书导入一次性 VM 的
# 临时 keychain 并用其签名。TCC 按证书 designated requirement 记录授权，跨版本
# 持久；ad-hoc 的 cdhash 每次构建都变——更新一版授权即失效，权限弹窗每版重弹
# （「已授权仍反复弹」的放大器，v1.2.31 实测）。证书生成见
# scripts/ci/generate-macos-signing-cert.sh 与 docs/ci/macos-bridge.md。
# 未注入时回退 ad-hoc（保持可出包），显式告警不静默。
SIGNING_OVERRIDE_CONFIG=""
if [ -n "${MACOS_SIGNING_P12:-}" ]; then
  : "${MACOS_SIGNING_P12_PASSWORD:=}"
  SIGN_KEYCHAIN="$PWD/tauri-signing.keychain-db"
  SIGN_KC_PWD="$(head -c 18 /dev/urandom | base64)"
  security create-keychain -p "$SIGN_KC_PWD" "$SIGN_KEYCHAIN"
  security set-keychain-settings -t 3600 "$SIGN_KEYCHAIN"
  security unlock-keychain -p "$SIGN_KC_PWD" "$SIGN_KEYCHAIN"
  printf '%s' "$MACOS_SIGNING_P12" | openssl base64 -d -A > signing-identity.p12
  security import signing-identity.p12 -k "$SIGN_KEYCHAIN" \
    -P "$MACOS_SIGNING_P12_PASSWORD" -T /usr/bin/codesign
  rm -f signing-identity.p12
  # 分区列表不放开时 codesign 取私钥会弹 GUI 授权或静默失败
  security set-key-partition-list -S apple-tool:,apple: -k "$SIGN_KC_PWD" "$SIGN_KEYCHAIN" >/dev/null
  # 临时 keychain 挂进用户搜索列表，codesign 才能按 identity 找到证书
  # （runner 路径无空格；bash 3.2 无 mapfile，tr 拼回单行按词展开）
  EXISTING_KEYCHAINS="$(security list-keychains -d user | tr -d '"' | tr '\n' ' ')"
  # shellcheck disable=SC2086
  security list-keychains -d user -s "$SIGN_KEYCHAIN" $EXISTING_KEYCHAINS
  # find-identity 行格式：`  1) <40位HEX> "Common Name"`，取第二列 SHA-1
  SIGN_ID_HASH="$(security find-identity -v -p codesigning "$SIGN_KEYCHAIN" | awk '/^[[:space:]]*[0-9]+\)[[:space:]]+[A-F0-9]{40}[[:space:]]+"/ {print $2; exit}')"
  [ -n "$SIGN_ID_HASH" ] || {
    echo "签名身份导入失败：keychain 中无可用 codesigning identity（检查 MACOS_SIGNING_P12 内容与 MACOS_SIGNING_P12_PASSWORD）" >&2
    exit 1
  }
  # tauri.conf.json 的 signingIdentity 是主配置默认值，--config deep merge 覆盖之
  # （不依赖 APPLE_SIGNING_IDENTITY env 与 config 的优先级行为）
  cat > tauri-signing-override.json <<EOF2
{ "bundle": { "macOS": { "signingIdentity": "$SIGN_ID_HASH" } } }
EOF2
  SIGNING_OVERRIDE_CONFIG="tauri-signing-override.json"
  export APPLE_SIGNING_IDENTITY="$SIGN_ID_HASH"
  echo "已启用稳定签名身份（TCC 授权跨版本持久）: $SIGN_ID_HASH"
else
  export APPLE_SIGNING_IDENTITY="-" # ad-hoc 签名（未配证书时的回退）
  echo "警告: 未配置 MACOS_SIGNING_P12，回退 ad-hoc 签名——TCC 授权无法跨版本持久，macOS 权限弹窗每版重弹（配置方法见 docs/ci/macos-bridge.md）" >&2
fi

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
if [ -n "$SIGNING_OVERRIDE_CONFIG" ]; then
  npm run tauri build -- --target "$TARGET_TRIPLE" --config "$SIGNING_OVERRIDE_CONFIG"
else
  npm run tauri build -- --target "$TARGET_TRIPLE"
fi

# 产物自检：打包出的 .app 二进制必须真实嵌入更新器 token（防编译缓存/环境
# 差异吞掉注入的假成功——更新器会静默降级为「已是最新」）
APP_BIN="target/${TARGET_TRIPLE}/release/bundle/macos/Yue-Review.app/Contents/MacOS/Yue_Review"
[ -f "$APP_BIN" ] || APP_BIN=$(find target/*/release/bundle/macos -path "*Contents/MacOS/Yue_Review" 2>/dev/null | head -1)
# grep -c 读满输入，避免 grep -q 提前关管道令 macOS strings 报 SIGPIPE
if [ -z "$APP_BIN" ] || [ "$(strings -a "$APP_BIN" | grep -c "glpat-" || true)" -eq 0 ]; then
  echo "产物自检失败：.app 二进制未嵌入更新器 token（glpat-）：$APP_BIN" >&2
  exit 1
fi

# 签名自检：配置了稳定身份时，.app 必须真实按该身份签名且签名完整——防
# --config 覆盖被静默忽略/打包后签名被破坏（签名失效 = TCC 授权失配重弹）
if [ -n "$SIGNING_OVERRIDE_CONFIG" ]; then
  APP_DIR="$(dirname "$(dirname "$APP_BIN")")"
  codesign --verify --strict "$APP_DIR" || {
    echo "产物自检失败：.app 签名校验不通过（codesign --verify）：$APP_DIR" >&2
    exit 1
  }
  APP_AUTHORITY="$(codesign -dv "$APP_DIR" 2>&1 | sed -n 's/^Authority=//p' | head -1)"
  if [ -z "$APP_AUTHORITY" ] || [ "$APP_AUTHORITY" = "-" ]; then
    echo "产物自检失败：.app 未按稳定签名身份签出（Authority 为空/ad-hoc）：$APP_DIR" >&2
    exit 1
  fi
  echo "产物签名身份自检通过: $APP_AUTHORITY"
fi

mkdir -p "dist/target/${TARGET_TRIPLE}/release"
cp -R "target/${TARGET_TRIPLE}/release/bundle" "dist/target/${TARGET_TRIPLE}/release/bundle"
