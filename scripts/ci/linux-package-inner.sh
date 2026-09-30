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
# 供应链门禁（第九轮评审 #19）：全部工具钉 sha256 白名单——镜像仅作下载
# 加速，内容必须与白名单逐字节一致，否则立即失败。发版 runner 持签名私钥，
# 未经校验的工具不得交给打包器执行并进入用户安装包；原「下载失败不阻断、
# 打包器自行重试」语义随门禁作废（打包器自行下载不经校验，等于没有门禁）。
# 上游升版流程：从 github 直连真源重算 sha256 → 更新白名单（脚本类同时把
# raw URL 钉到新 commit）→ 提交说明注明上游版本与日期。
CACHE_DIR="$HOME/.cache/tauri"
mkdir -p "$CACHE_DIR"
TAURI_ARCH=x86_64
case "$*" in *aarch64*) TAURI_ARCH=aarch64 ;; esac

# ── sha256 白名单（2026-09-24 自 github 直连真源计算，镜像交叉验证一致）──
# AppRun / linuxdeploy 来自 tauri-apps/binary-releases 的固定 tag（apprun-old/linuxdeploy）
case "$TAURI_ARCH" in
  x86_64)
    APPRUN_SHA="f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f"
    LINUXDEPLOY_SHA="e762bea85c8eb0d4b3508d46e5c1f037f717d0f9303ae3b4aafc8b04991fa1ef"
    PLUGIN_APPIMAGE_SHA="0441769ab38009504d2678c38cd7e526955388dd30a215b4a20afaa5471652f2"
    ;;
  aarch64)
    APPRUN_SHA="072f17c0895a85c490282fe5395c5007e5fc75da727e553b3b8fb680feb11578"
    LINUXDEPLOY_SHA="b12b5cc57bd0921e1f98d73f58aa364503bc1a27f54b7a69fd2870bce7fa2f55"
    PLUGIN_APPIMAGE_SHA="ce574719bcf9cc1fb12728d60b17e48cc87d9b6c40f6f48b04cff7d273b5eb24"
    ;;
esac
# 插件脚本与架构无关；raw URL 钉死 commit（原 master 分支为移动目标）
PLUGIN_GTK_SHA="7804c9eef13e59bf2783aad9882ef9db8f3f3f9e8d631874b1d348d550a3693f"
PLUGIN_GTK_URL="https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/dda522bce37387f1b853d9095713bfaa924c8423/linuxdeploy-plugin-gtk.sh"
PLUGIN_GSTREAMER_SHA="c107b49d84edbffc6ab226ed1007e0626a4f7aa2c3a36b7782bef62351d49e94"
PLUGIN_GSTREAMER_URL="https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gstreamer/2a2e67491c32995a3f279ad0ecbe77abd512b42a/linuxdeploy-plugin-gstreamer.sh"

# 候选源：gh-proxy 镜像优先（yx runner 直连 github 频繁挂起），空串 = 直连兜底。
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

# 容器无 /dev/fuse，AppImage 形态的 linuxdeploy 及其插件必须走解包运行，
# 否则 tauri 打包器在 AppImage 步骤报 "failed to run linuxdeploy"（v1.1.86 根因）
export APPIMAGE_EXTRACT_AND_RUN=1
npm run tauri build -- $@  # 有意不加引号：TAURI_BUILD_ARGS 需按空格拆分为多个参数
