#!/usr/bin/env bash
# 容器内执行的 linux 打包脚本（由 .tauri-package-linux 以 runner 同 UID 运行）
set -euo pipefail

# npm 缓存指向 /build/.npm（GIT_CLEAN_FLAGS 排除清单内的持久缓存，与其余
# 作业一致）；此前用 /build/.npmc 不在排除清单，每次 checkout 被清、全量重下
npm ci --cache /build/.npm --prefer-offline
# 打包前清理 bundle 目录：runner 构建目录跨流水线持久，不清理会让
# 历史版本的安装包累积并被 publish 的全量 glob 选中（2026-09-16 更新失效根因）
rm -rf target/release/bundle target/*/release/bundle

# 预热 tauri 打包器的 AppImage 工具缓存（$HOME/.cache/tauri，文件名与
# tauri-bundler 约定一致）：yx runner 直连 github 下载频繁挂起/超时
# （v1.1.84/86 两次发版失败根因），走 gh-proxy 镜像优先、直连兜底。
# 单个工具下载失败不阻断：tauri 打包器缓存未命中时还会自行重试直连。
CACHE_DIR="$HOME/.cache/tauri"
mkdir -p "$CACHE_DIR"
TAURI_ARCH=x86_64
case "$*" in *aarch64*) TAURI_ARCH=aarch64 ;; esac
# 候选源：gh-proxy 镜像优先（yx runner 直连 github 频繁挂起），空串 = 直连兜底。
# 下载后校验文件头（ELF 或脚本），防止镜像返回损坏/HTML 内容被当工具缓存
fetch_tool() {
  local dest="$1"; local github_url="$2"
  local looks_valid=""
  if [ -s "$dest" ] && head -c 4 "$dest" | od -A n -t x1 | grep -q "7f 45 4c 46\|23 21"; then
    return 0
  fi
  local base
  for base in "https://gh-proxy.com/" "https://gh-proxy.cn/" ""; do
    if curl -fsSL --connect-timeout 10 --max-time 300 -o "$dest.part" "$base$github_url"; then
      looks_valid=$(head -c 4 "$dest.part" | od -A n -t x1)
      if echo "$looks_valid" | grep -q "7f 45 4c 46\|23 21"; then
        mv "$dest.part" "$dest"
        # 全部工具（含 .sh 插件脚本与 AppRun）都必须可执行：
        # linuxdeploy 发现插件时要求可执行位，否则报 Could not find plugin
        chmod +x "$dest"
        echo "已缓存打包工具: $dest"
        return 0
      fi
      echo "警告: 镜像返回内容非法（非 ELF/脚本），换下一候选: $base" >&2
    fi
    rm -f "$dest.part"
  done
  echo "警告: 打包工具下载失败（打包器将自行重试）: $dest" >&2
  return 0
}
fetch_tool "$CACHE_DIR/AppRun-$TAURI_ARCH" "https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-$TAURI_ARCH"
fetch_tool "$CACHE_DIR/linuxdeploy-$TAURI_ARCH.AppImage" "https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy/linuxdeploy-$TAURI_ARCH.AppImage"
# 注意：该插件的 bundler 缓存名不带架构后缀（tauri-bundler prepare_tools 约定）
fetch_tool "$CACHE_DIR/linuxdeploy-plugin-appimage.AppImage" "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-$TAURI_ARCH.AppImage"
fetch_tool "$CACHE_DIR/linuxdeploy-plugin-gtk.sh" "https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
fetch_tool "$CACHE_DIR/linuxdeploy-plugin-gstreamer.sh" "https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gstreamer/master/linuxdeploy-plugin-gstreamer.sh"

# 容器无 /dev/fuse，AppImage 形态的 linuxdeploy 及其插件必须走解包运行，
# 否则 tauri 打包器在 AppImage 步骤报 "failed to run linuxdeploy"（v1.1.86 根因）
export APPIMAGE_EXTRACT_AND_RUN=1
npm run tauri build -- $@  # 有意不加引号：TAURI_BUILD_ARGS 需按空格拆分为多个参数
