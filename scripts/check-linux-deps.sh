#!/usr/bin/env bash
# check-linux-deps.sh - non-sudo preflight for Linux build dependencies.
#
# The GitLab CI Linux jobs no longer run `sudo apt-get ...` (the shell
# executor has no TTY, so sudo waits for a password and the job times out).
# Instead the system packages are installed ONCE on the runner, manually, by
# an operator; this script only verifies they are present and, when they are
# missing, prints the exact command to run.
#
# Usage:
#   bash scripts/check-linux-deps.sh            # verify-stage packages
#   bash scripts/check-linux-deps.sh --for-package  # verify + package bundling packages
#
# Never invokes sudo and never reads stdin (non-interactive).

set -u

FOR_PACKAGE=0
if [[ "${1:-}" == "--for-package" ]]; then
  FOR_PACKAGE=1
fi

# Packages needed to compile/verify the Tauri app (Ubuntu 22.04 names).
# libssl-dev + pkg-config: openssl-sys（reqwest 默认 TLS）编译期定位 OpenSSL
# 全靠它们；runner 重装系统后最容易漏装，缺失时报的是晦涩的
# "Could not find directory of OpenSSL installation" 而非缺包名。
PKGS=(libssl-dev pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev)
if [[ $FOR_PACKAGE -eq 1 ]]; then
  # Additional packages needed by the release bundling stage (deb/rpm/appimage).
  # NOTE: libfuse2 is intentionally NOT required: the tauri bundler runs
  # linuxdeploy with `--appimage-extract-and-run` (and sets
  # APPIMAGE_EXTRACT_AND_RUN=1), so building the AppImage works without FUSE.
  PKGS+=(patchelf rpm rpm2cpio cpio binutils)
fi

MISSING=()
for pkg in "${PKGS[@]}"; do
  if ! dpkg -s "$pkg" >/dev/null 2>&1; then
    MISSING+=("$pkg")
  fi
done

if [[ ${#MISSING[@]} -gt 0 ]]; then
  echo "Missing Linux build dependencies: ${MISSING[*]}" >&2
  echo "" >&2
  echo "Install them once on this runner (requires root):" >&2
  echo "  sudo apt-get update" >&2
  echo "  sudo apt-get install -y ${MISSING[*]}" >&2
  exit 1
fi

echo "All required Linux build dependencies are installed."
