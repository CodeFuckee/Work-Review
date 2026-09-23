#!/bin/bash
# Uninstaller for Yue Review (legacy name: Work Review).
# Removes everything install.sh set up. User data (~/.local/share/yue-review/,
# legacy ~/.local/share/work-review/) is PRESERVED by default — pass --purge to
# wipe it. apt packages are never removed (they're generic tools — remove with
# apt manually if you want).
#
# 安装脚本按 AppImage 产物名参数化（Yue_Review / Yue-Review / Work_Review 全兼容），
# 本脚本同样按多名字探测，新名旧名资产都清。
#
# Usage:
#   bash scripts/appimage/uninstall.sh            # keep user data (default)
#   bash scripts/appimage/uninstall.sh --purge    # also delete DB, screenshots, config
#   bash scripts/appimage/uninstall.sh --dry-run  # show what would be removed
#
set -euo pipefail

APPS_DIR="$HOME/Applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
DESKTOP_DIR="$HOME/.local/share/applications"
BIN_DIR="$HOME/bin"
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/focused-window-dbus@flexagoon.com"
DATA_DIR="$HOME/.local/share/yue-review"
LEGACY_DATA_DIR="$HOME/.local/share/work-review"

PURGE=0
DRY=0
for arg in "$@"; do
    case "$arg" in
        --purge)   PURGE=1 ;;
        --dry-run) DRY=1 ;;
        -h|--help)
            sed -n '2,14p' "$0"; exit 0 ;;
        *) echo "unknown flag: $arg (see --help)" >&2; exit 2 ;;
    esac
done

log()  { printf '\033[1;34m[uninstall]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[uninstall]\033[0m %s\n' "$*" >&2; }

# Show action; actually do it unless --dry-run.
do_rm() {
    local target="$1"
    if [ ! -e "$target" ] && [ ! -L "$target" ]; then
        log "  (skip, not present) $target"
        return 0
    fi
    if [ "$DRY" -eq 1 ]; then
        log "  would remove: $target"
    else
        log "  rm $target"
        rm -rf -- "$target"
    fi
}

# ---- 0. stop the app if running ------------------------------------------
log "0/6 stop running Work_Review/Yue_Review processes"
# 双名探测：旧版二进制 Work_Review，改名后 Yue_Review（AppImage 由启动器以
# "<产物名> --no-sandbox" 启动，按 -f 匹配）
STOPPED=0
for proc in Work_Review Yue_Review Yue-Review; do
    if pgrep -f "$proc --no-sandbox" >/dev/null; then
        if [ "$DRY" -eq 1 ]; then
            log "  would: pkill -f '$proc --no-sandbox'"
        else
            pkill -f "$proc --no-sandbox" || true
            sleep 1
            pkill -9 -f "$proc --no-sandbox" 2>/dev/null || true
            log "  stopped ($proc)"
        fi
        STOPPED=1
    fi
done
[ "$STOPPED" -eq 1 ] || log "  not running"

# ---- 1. desktop entry -----------------------------------------------------
log "1/6 .desktop entry"
# 新名 yue-review（LAUNCH_ID 随产物名推导），旧名 work-review 兜底
do_rm "$DESKTOP_DIR/yue-review.desktop"
do_rm "$DESKTOP_DIR/work-review.desktop"
[ "$DRY" -eq 1 ] || update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

# ---- 2. icon --------------------------------------------------------------
log "2/6 icon"
# 图标名跟随 AppImage 内部 png（安装脚本动态发现），新旧名都按 glob 清
shopt -s nullglob
for icon in "$ICON_DIR"/Work_Review*.png "$ICON_DIR"/Yue_Review*.png "$ICON_DIR"/Yue-Review*.png "$ICON_DIR/Yue Review.png"; do
    do_rm "$icon"
done
shopt -u nullglob
[ "$DRY" -eq 1 ] || gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# ---- 3. shims + launcher --------------------------------------------------
log "3/6 shims + launcher in $BIN_DIR"
for f in work-review yue-review gdbus gnome-screenshot grim spectacle tesseract; do
    do_rm "$BIN_DIR/$f"
done
# If ~/bin is now empty, offer to remove it (non-destructive — we only rmdir,
# which fails silently if the dir still has other files).
if [ -d "$BIN_DIR" ] && [ "$DRY" -eq 0 ]; then
    rmdir "$BIN_DIR" 2>/dev/null && log "  removed empty $BIN_DIR" || true
fi

# ---- 4. AppImage ----------------------------------------------------------
log "4/6 AppImage(s) in $APPS_DIR"
shopt -s nullglob
for ai in "$APPS_DIR"/Yue_Review_*.AppImage "$APPS_DIR"/Yue-Review_*.AppImage "$APPS_DIR"/Work_Review_*.AppImage; do
    do_rm "$ai"
done
shopt -u nullglob
# Only rmdir if empty — don't nuke other AppImages the user put there.
if [ -d "$APPS_DIR" ] && [ "$DRY" -eq 0 ]; then
    rmdir "$APPS_DIR" 2>/dev/null && log "  removed empty $APPS_DIR" || true
fi

# ---- 5. GNOME extension ---------------------------------------------------
log "5/6 GNOME extension focused-window-dbus"
if [ -d "$EXT_DIR" ]; then
    if [ "$DRY" -eq 1 ]; then
        log "  would disable + remove $EXT_DIR"
    else
        gnome-extensions disable focused-window-dbus@flexagoon.com 2>/dev/null || true
        rm -rf "$EXT_DIR"
        log "  removed $EXT_DIR (effective after next Shell reload / relogin)"
    fi
else
    log "  not installed"
fi

# ---- 6. user data ---------------------------------------------------------
log "6/6 user data ($DATA_DIR / legacy $LEGACY_DATA_DIR)"
if [ "$PURGE" -eq 1 ]; then
    do_rm "$DATA_DIR"
    do_rm "$LEGACY_DATA_DIR"
else
    for dir in "$DATA_DIR" "$LEGACY_DATA_DIR"; do
        if [ -d "$dir" ]; then
            SIZE=$(du -sh "$dir" 2>/dev/null | cut -f1)
            log "  KEPT: $dir ($SIZE). Pass --purge to remove."
        else
            log "  not present: $dir"
        fi
    done
fi

echo
log "done."
cat <<NOTES

Not touched (on purpose):
  - apt packages (libfuse2, gnome-screenshot, grim, kde-spectacle,
    tesseract-ocr*, desktop-file-utils, git). Remove manually if desired:
      sudo apt-get remove <package>
NOTES
if [ "$PURGE" -eq 0 ]; then
    for dir in "$DATA_DIR" "$LEGACY_DATA_DIR"; do
        if [ -d "$dir" ]; then
            echo "  - User data at $dir (rerun with --purge to delete)."
        fi
    done
fi
