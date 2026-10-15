#!/usr/bin/env bash
# scripts/ci/generate-macos-signing-cert.sh — 生成 macOS 自签代码签名证书并导出 p12。
#
# 用途：给 CI（ci/macos/package.sh）提供**稳定的代码签名身份**。ad-hoc（-）签名的
# cdhash 每次构建都变，TCC 授权随每次发版失效——macOS 权限弹窗每版重弹；固定
# 证书签名后 TCC 按证书 designated requirement 记录授权，跨版本持久。
#
# 用法（推荐在 Mac 上跑，openssl 自带 LibreSSL 即可）：
#   P12_PASSWORD='你的导出密码' ./scripts/ci/generate-macos-signing-cert.sh
#
# 生成后按脚本输出的指引：
#   1) 把 *.b64（base64 p12）配到 GitHub 桥接仓库 CodeFuckee/gl-macos-bridge 的
#      secret `GL131_MACOS_SIGNING_P12`，导出密码配到 `GL131_MACOS_SIGNING_P12_PASSWORD`；
#   2) 桥接仓库 workflow 给打包作业注入 env：MACOS_SIGNING_P12(_PASSWORD)（映射同上）；
#   3) 本机钥匙串导入证书（脚本已自动尝试），本地 `tauri:build:local-mac` 可用
#      `APPLE_SIGNING_IDENTITY="<CN>"` 复用同一身份。
set -euo pipefail

CN="${1:-WorkReview Dev}"
DAYS="${DAYS:-3650}"
OUT_P12="${OUT_P12:-workreview-signing.p12}"
P12_PASSWORD="${P12_PASSWORD:?请设置 P12_PASSWORD 环境变量作为 p12 导出密码}"

KEY="$(mktemp -t wrcert).key"
CRT="$(mktemp -t wrcert).crt"
trap 'rm -f "$KEY" "$CRT"' EXIT

echo "==> 生成自签代码签名证书（CN=$CN, ${DAYS} 天）"
openssl req -x509 -newkey rsa:3072 -keyout "$KEY" -out "$CRT" -nodes \
  -subj "/CN=$CN/O=WorkReview/C=CN" \
  -addext "keyUsage=digitalSignature" \
  -addext "extendedKeyUsage=codeSigning"

echo "==> 导出 p12: $OUT_P12"
# OpenSSL 3 默认 AES 加密的 p12 会被旧版 `security import` 拒收（MAC 校验失败），
# 加 -legacy 回到 3DES 口径；LibreSSL（macOS 自带）无 -legacy，默认即兼容口径
LEGACY_FLAG=""
if openssl version 2>/dev/null | grep -q "OpenSSL 3"; then
  LEGACY_FLAG="-legacy"
fi
openssl pkcs12 -export $LEGACY_FLAG -out "$OUT_P12" \
  -inkey "$KEY" -in "$CRT" -password pass:"$P12_PASSWORD"

echo "==> 导入本机登录钥匙串（本地构建复用同一身份；失败不影响 CI 配置）"
security import "$OUT_P12" -k "$HOME/Library/Keychains/login.keychain-db" \
  -P "$P12_PASSWORD" -T /usr/bin/codesign || {
  echo "    自动导入失败，请手动双击 $OUT_P12 导入登录钥匙串" >&2
}

BASE64_OUT="$OUT_P12.b64"
if base64 -i "$OUT_P12" -o "$BASE64_OUT" 2>/dev/null; then
  : # macOS BSD base64
else
  base64 -w0 "$OUT_P12" > "$BASE64_OUT" # GNU base64
fi

echo
echo "✅ 完成。接下来一次性配置（见 docs/ci/macos-bridge.md「稳定签名身份」节）："
echo "  1. GitHub 桥接仓库 secrets：GL131_MACOS_SIGNING_P12 = $BASE64_OUT 文件内容（单行 base64）"
echo "     GL131_MACOS_SIGNING_P12_PASSWORD = 导出密码"
echo "  2. 桥接仓库 workflow 打包作业 env 注入 MACOS_SIGNING_P12 / MACOS_SIGNING_P12_PASSWORD"
echo "  3. 本地构建复用：APPLE_SIGNING_IDENTITY=\"$CN\" npm run tauri:build:local-mac"
echo "  注意：切换签名身份后 TCC 需重新授权一次，此后跨版本持久。"
