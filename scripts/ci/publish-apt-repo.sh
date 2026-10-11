#!/usr/bin/env bash
# 构建并发布 yue-review 私有 apt 仓库到 GitLab Generic Package Registry。
#
# 仓库固定地址（sources.list 指向它，客户端永远不变）：
#   https://home.chenkaidi.top:509/api/v4/projects/131/packages/generic/apt-repo/stable/
# 布局：dists/stable/main/binary-amd64/{Packages,Packages.gz,Release,Release.gpg,InRelease}
#       pool/main/y/<pkg>/<deb>  +  KEY.gpg（仓库根）
#
# 用法：
#   WR_TOKEN=<PAT(api权限)> [WR_GNUPGHOME=~/.gnupg] \
#     ./scripts/ci/publish-apt-repo.sh <deb文件> [deb文件...]
# CI（tag 流水线 publish-apt-repo 作业）把私钥 base64 存在 CI 变量
# APT_REPO_GPG_KEY，导入临时 GNUPGHOME 后调用本脚本。
#
# 发布策略：整包「先删后传」（DELETE packages?package_name=apt-repo 全部记录再重传），
# 不依赖 generic registry 的重复/覆盖语义，仓库状态确定无漂移。pool 只含传入的 deb，
# 即仓库始终只保留最新版（个人源，不留历史版本）。
#
# 认证事实（2026-10-07 实测，实例 GitLab 19.x）：
#   - generic registry 上传/下载接受 HTTP Basic（oauth2:<PAT>）→ apt 的 auth.conf.d 可用
#   - 文件名可含斜杠 → 能拼出 dists/... 仓库路径
#   - 包管理（DELETE /packages/:id）需 PRIVATE-TOKEN 头
set -euo pipefail

API_BASE="${WR_API_BASE:-https://home.chenkaidi.top:509/api/v4/projects/131}"
REG_BASE="$API_BASE/packages/generic/apt-repo/stable"
TOKEN="${WR_TOKEN:?需要 WR_TOKEN（PAT，api 权限）}"
SUITE=stable
COMP=main
# 三级回退：WR_GNUPGHOME 显式指定 > 继承外部 GNUPGHOME（CI 作业的 mktemp 导入目录）> 本机默认。
# 不能无条件覆盖：CI 作业先把私钥导入临时 GNUPGHOME，覆盖会导致签名时 "no default secret key"。
GNUPGHOME="${WR_GNUPGHOME:-${GNUPGHOME:-$HOME/.gnupg}}"
export GNUPGHOME

if [ "$#" -eq 0 ]; then
  echo "用法: $0 <deb文件>..." >&2
  exit 1
fi
for deb in "$@"; do
  [ -f "$deb" ] || { echo "deb 不存在: $deb" >&2; exit 1; }
done
# 同名不同内容的 deb 拒发：共享 runner 工作区可能残留其他流水线写入的同名
# 同版本 deb（v1.2.26 事故：夜间 daily 不带 --target 落 target/release/，与发版
# 构建一起被全量 glob 捞入——Packages 索引双条各记一份、pool 单文件取其一，
# apt 按第一条校验报「文件尺寸不符」）。同名必须逐字节一致，否则宁可失败。
declare -A seen_sha
for deb in "$@"; do
  base="$(basename "$deb")"
  cur="$(sha256sum "$deb" | cut -d' ' -f1)"
  if [ -n "${seen_sha[$base]:-}" ] && [ "${seen_sha[$base]}" != "$cur" ]; then
    echo "拒发: 同名 deb 内容不一致（跨流水线工作区残留？）: $deb" >&2
    exit 1
  fi
  seen_sha[$base]="$cur"
done
command -v gpg >/dev/null || { echo "runner 缺少 gpg（apt install gnupg）" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ROOT="$WORK/repo"
DIST="$ROOT/dists/$SUITE/$COMP/binary-amd64"
mkdir -p "$DIST"

# ---- pool + Packages 索引 ----
PKGIDX="$DIST/Packages"
: > "$PKGIDX"
for deb in "$@"; do
  pkg="$(dpkg-deb -f "$deb" Package)"
  ver="$(dpkg-deb -f "$deb" Version)"
  arch="$(dpkg-deb -f "$deb" Architecture)"
  [ "$arch" = "amd64" ] || { echo "仅支持 amd64，跳过 $arch: $deb" >&2; exit 1; }
  dest="$ROOT/pool/$COMP/${pkg:0:1}/$pkg/$(basename "$deb")"
  mkdir -p "$(dirname "$dest")"
  cp "$deb" "$dest"

  size="$(stat -c%s "$deb")"
  sha256="$(sha256sum "$deb" | cut -d' ' -f1)"
  maint="$(dpkg-deb -f "$deb" Maintainer 2>/dev/null || echo 'WorkReview Team')"
  section="$(dpkg-deb -f "$deb" Section 2>/dev/null || true)"
  priority="$(dpkg-deb -f "$deb" Priority 2>/dev/null || echo optional)"
  depends="$(dpkg-deb -f "$deb" Depends 2>/dev/null || true)"

  {
    printf 'Package: %s\n' "$pkg"
    printf 'Version: %s\n' "$ver"
    printf 'Architecture: %s\n' "$arch"
    printf 'Maintainer: %s\n' "$maint"
    printf 'Section: %s\n' "${section:-utils}"
    printf 'Priority: %s\n' "${priority:-optional}"
    [ -n "$depends" ] && printf 'Depends: %s\n' "$depends"
    printf 'Filename: %s\n' "pool/$COMP/${pkg:0:1}/$pkg/$(basename "$deb")"
    printf 'Size: %s\n' "$size"
    printf 'SHA256: %s\n' "$sha256"
    # Description 值自带续行（dpkg-deb -f 原样输出多行）
    printf 'Description: %s\n' "$(dpkg-deb -f "$deb" Description)"
    printf '\n'
  } >> "$PKGIDX"
  echo "索引收录: $pkg $ver ($arch, $((size / 1024)) KB)"
done

gzip -k9 "$PKGIDX"

# ---- Release（apt 要求含 dists 内文件的校验和段，路径相对 dists/<suite>）----
rel_dir="$ROOT/dists/$SUITE"
cd "$rel_dir"
{
  printf 'Origin: Yue Review\n'
  printf 'Label: Yue Review\n'
  printf 'Suite: %s\n' "$SUITE"
  printf 'Codename: %s\n' "$SUITE"
  printf 'Architectures: amd64\n'
  printf 'Components: %s\n' "$COMP"
  printf 'Description: Work Review (yue-review) private apt repository\n'
  printf 'Date: %s\n' "$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S UTC')"
  printf 'SHA256:\n'
  for f in "$COMP/binary-amd64/Packages" "$COMP/binary-amd64/Packages.gz"; do
    printf ' %s %8d %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$(stat -c%s "$f")" "$f"
  done
} > "$rel_dir/Release"

# ---- 签名（InRelease 内嵌清签 + Release.gpg 分离签名）----
gpg --batch --pinentry-mode loopback --passphrase '' \
  --clearsign -o "$rel_dir/InRelease" "$rel_dir/Release"
gpg --batch --pinentry-mode loopback --passphrase '' \
  --detach-sign --armor -o "$rel_dir/Release.gpg" "$rel_dir/Release"
gpg --batch --armor --export > "$ROOT/KEY.gpg"
echo "已签名 (key: $(gpg --batch --list-keys --with-colons | awk -F: '/^fpr:/ {print $10; exit}'))"

# ---- 发布：先删后传 ----
ids="$(curl -fsS -H "PRIVATE-TOKEN: $TOKEN" "$API_BASE/packages?package_name=apt-repo" \
  | python3 -c 'import json,sys; print(" ".join(str(p["id"]) for p in json.load(sys.stdin)))')"
if [ -n "${ids:-}" ]; then
  for id in $ids; do
    curl -fsS -X DELETE -H "PRIVATE-TOKEN: $TOKEN" "$API_BASE/packages/$id" >/dev/null
  done
  echo "已清空旧 apt-repo 包记录: $ids"
fi

cd "$ROOT"
mapfile -t files < <(find . -type f | sed 's|^\./||' | sort)
for rel in "${files[@]}"; do
  code="$(curl -fsS -o /dev/null -w '%{http_code}' -X PUT -u "oauth2:$TOKEN" \
    -H 'Content-Type: application/octet-stream' --data-binary "@$rel" "$REG_BASE/$rel")"
  case "$code" in 200|201) ;; *) echo "上传失败($code): $rel" >&2; exit 1;; esac
done
echo "已发布 ${#files[@]} 个文件到 $REG_BASE/"
echo "客户端源地址: deb [signed-by=/etc/apt/keyrings/yue-review.gpg] $REG_BASE/ $SUITE $COMP"
