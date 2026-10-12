#!/usr/bin/env bash
# tauri 打包 AppImage 工具预热（发版打包与夜间打包两路共用，须以 source 方式调用：
#   . "$(dirname "$0")/appimage-tools.sh" <x86_64|aarch64>
# 结束时向调用方 shell 导出两个变量供打包器/linuxdeploy 插件读取：
#   APPIMAGE_EXTRACT_AND_RUN=1          AppImage 工具走解包运行（容器无 /dev/fuse）
#   APPIMAGE_TYPE2_RUNTIME_FILE=<路径>  预提供的 type2 runtime，插件跳过 GitHub 下载
# 预热 $HOME/.cache/tauri（文件名与 tauri-bundler 约定一致）：runner 直连 github
# 下载频繁挂起/超时——发版 v1.1.84/86、v1.2.24 与夜间产包 2026-10-11 三代失败同根因，
# 走 gh-proxy 镜像优先、直连兜底。
# 供应链门禁（第九轮评审 #19）：全部工具钉 sha256 白名单——镜像仅作下载
# 加速，内容必须与白名单逐字节一致，否则立即失败。发版 runner 持签名私钥，
# 未经校验的工具不得交给打包器执行并进入用户安装包；原「下载失败不阻断、
# 打包器自行重试」语义随门禁作废（打包器自行下载不经校验，等于没有门禁）。
# 上游升版流程：从 github 直连真源重算 sha256 → 更新白名单（脚本类同时把
# raw URL 钉到新 commit）→ 提交说明注明上游版本与日期。
set -euo pipefail

case "${1:-}" in
  x86_64 | aarch64) TAURI_ARCH="$1" ;;
  *)
    echo "用法: source appimage-tools.sh <x86_64|aarch64>" >&2
    exit 1
    ;;
esac

CACHE_DIR="$HOME/.cache/tauri"
mkdir -p "$CACHE_DIR"

# ── sha256 白名单（2026-09-24 自 github 直连真源计算，镜像交叉验证一致）──
# 上游升版：linuxdeploy/linuxdeploy-plugin-appimage 的 continuous 资产 2026-10-01
# 被上游重发，plugin-appimage 双架构哈希 2026-10-11 自 api.github.com 真源重算
# （type2-runtime 2026-09-28 重发为相同字节，哈希不变）；AppRun/linuxdeploy 来自
# tauri-apps/binary-releases 的固定 tag（apprun-old/linuxdeploy，不可变无漂移）
case "$TAURI_ARCH" in
  x86_64)
    APPRUN_SHA="f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f"
    LINUXDEPLOY_SHA="e762bea85c8eb0d4b3508d46e5c1f037f717d0f9303ae3b4aafc8b04991fa1ef"
    PLUGIN_APPIMAGE_SHA="49d6a17160675a6bd1781699aae6bdf7692d98552e02a3671d2183d10547842e"
    # AppImage type2 runtime（linuxdeploy-plugin-appimage 生成镜像时使用；
    # 不预提供时插件自行从 GitHub 下载——家庭 runner 对 Hub 系两连败根因）
    RUNTIME_SHA="156f4bdbde9c52d01814600013e0a273f0118dc2de98975f3c8c63427ec79074"
    ;;
  aarch64)
    APPRUN_SHA="072f17c0895a85c490282fe5395c5007e5fc75da727e553b3b8fb680feb11578"
    LINUXDEPLOY_SHA="b12b5cc57bd0921e1f98d73f58aa364503bc1a27f54b7a69fd2870bce7fa2f55"
    PLUGIN_APPIMAGE_SHA="518f4b49a561c157b23fd1ba4e857b8fa2271a038ac711bdedab6f018acbe7e5"
    RUNTIME_SHA="b4ff0030242d0c3bb12ce40541828303cf167493f4793456f0436edd6255c39d"
    ;;
esac
# 插件脚本与架构无关；raw URL 钉死 commit（原 master 分支为移动目标）
PLUGIN_GTK_SHA="7804c9eef13e59bf2783aad9882ef9db8f3f3f9e8d631874b1d348d550a3693f"
PLUGIN_GTK_URL="https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/dda522bce37387f1b853d9095713bfaa924c8423/linuxdeploy-plugin-gtk.sh"
PLUGIN_GSTREAMER_SHA="c107b49d84edbffc6ab226ed1007e0626a4f7aa2c3a36b7782bef62351d49e94"
PLUGIN_GSTREAMER_URL="https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gstreamer/2a2e67491c32995a3f279ad0ecbe77abd512b42a/linuxdeploy-plugin-gstreamer.sh"

# 候选源：gh-proxy 镜像优先（runner 直连 github 频繁挂起），空串 = 直连兜底。
# 收货唯一标准 = sha256 与白名单一致（缓存命中同样全量校验，防缓存被篡改/损坏）
fetch_tool() {
  local dest="$1"; local github_url="$2"; local expected="$3"
  if [ -s "$dest" ]; then
    if echo "$expected  $dest" | sha256sum -c - >/dev/null 2>&1; then
      # 历史缓存可能缺执行位（插件脚本/AppRun 必须可执行），兜底恢复
      chmod +x "$dest" 2>/dev/null || true
      echo "打包工具缓存 sha256 校验通过: $dest"
      return 0
    fi
    echo "警告: 缓存内容与白名单不符，删除后重新下载: $dest" >&2
    rm -f "$dest"
  fi
  local base
  for base in "https://gh-proxy.com/" "https://gh-proxy.cn/" ""; do
    if curl -fsSL --connect-timeout 10 --max-time 300 -o "$dest.part" "$base$github_url" \
       && echo "$expected  $dest.part" | sha256sum -c - >/dev/null 2>&1; then
      mv "$dest.part" "$dest"
      # 全部工具（含 .sh 插件脚本与 AppRun）都必须可执行：
      # linuxdeploy 发现插件时要求可执行位，否则报 Could not find plugin
      chmod +x "$dest"
      echo "已缓存打包工具（sha256 校验通过）: $dest"
      return 0
    fi
    echo "警告: 候选源下载失败或内容与白名单不符，换下一候选: $base" >&2
    rm -f "$dest.part"
  done
  echo "错误: 打包工具无法通过 sha256 白名单校验，拒绝打包: $dest" >&2
  echo "错误: 期望 sha256 = $expected" >&2
  echo "错误: 若为上游正常升版，请从 github 直连重算哈希并更新白名单后再发版" >&2
  exit 1
}
fetch_tool "$CACHE_DIR/AppRun-$TAURI_ARCH" "https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-$TAURI_ARCH" "$APPRUN_SHA"
fetch_tool "$CACHE_DIR/linuxdeploy-$TAURI_ARCH.AppImage" "https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy/linuxdeploy-$TAURI_ARCH.AppImage" "$LINUXDEPLOY_SHA"
# 注意：该插件的 bundler 缓存名不带架构后缀（tauri-bundler prepare_tools 约定）
fetch_tool "$CACHE_DIR/linuxdeploy-plugin-appimage.AppImage" "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-$TAURI_ARCH.AppImage" "$PLUGIN_APPIMAGE_SHA"
fetch_tool "$CACHE_DIR/linuxdeploy-plugin-gtk.sh" "$PLUGIN_GTK_URL" "$PLUGIN_GTK_SHA"
fetch_tool "$CACHE_DIR/linuxdeploy-plugin-gstreamer.sh" "$PLUGIN_GSTREAMER_URL" "$PLUGIN_GSTREAMER_SHA"
# type2 runtime 预提供：经 APPIMAGE_TYPE2_RUNTIME_FILE 传给 linuxdeploy-plugin-appimage，
# 插件不再自行访问 GitHub（v1.2.24 两连败根因：runtime 下载超时且插件无法走镜像）
fetch_tool "$CACHE_DIR/runtime-$TAURI_ARCH" "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-$TAURI_ARCH" "$RUNTIME_SHA"
# tauri 2.10.3 的 bundler 改读 commit 钉版名（linuxdeploy-07333c6-<arch>.AppImage），
# 白名单基准仍为 continuous 构建：把校验过的已知良好二进制复制为钉版名供其读取，
# 避免 bundler 绕过 sha 门禁自行从 GitHub 下载（家庭 runner 对 Hub 系两连败根因）
if [ -s "$CACHE_DIR/linuxdeploy-$TAURI_ARCH.AppImage" ]; then
  cp -f "$CACHE_DIR/linuxdeploy-$TAURI_ARCH.AppImage" "$CACHE_DIR/linuxdeploy-07333c6-$TAURI_ARCH.AppImage"
  chmod +x "$CACHE_DIR/linuxdeploy-07333c6-$TAURI_ARCH.AppImage"
  echo "已为 tauri 钉版名提供 linuxdeploy 副本: linuxdeploy-07333c6-$TAURI_ARCH.AppImage"
fi

# 容器无 /dev/fuse，AppImage 形态的 linuxdeploy 及其插件必须走解包运行，
# 否则 tauri 打包器在 AppImage 步骤报 "failed to run linuxdeploy"（v1.1.86 根因）
export APPIMAGE_EXTRACT_AND_RUN=1
# 预提供的 type2 runtime：插件据此给 appimagetool 传 --runtime-file，跳过 GitHub 下载。
# 变量名必须双发：新版 linuxdeploy-plugin-appimage（2026-10-01 continuous 重发起）只认
# LDAI_RUNTIME_FILE（见其 src/main.cpp：LDAI_RUNTIME_FILE → --runtime-file，无旧名回退）；
# APPIMAGE_TYPE2_RUNTIME_FILE 为旧名保留。教训：v1.2.24 起只 export 旧名，v1.2.25/26 与
# 夜间多次「成功」实为当时 GitHub 恰可达、appimagetool 自行下载得手（3833/25969 日志
# 均有 Downloading runtime 行）——预提供从未真正生效，GitHub 断连即现形（3890 夜间失败）
export APPIMAGE_TYPE2_RUNTIME_FILE="$CACHE_DIR/runtime-$TAURI_ARCH"
export LDAI_RUNTIME_FILE="$CACHE_DIR/runtime-$TAURI_ARCH"
