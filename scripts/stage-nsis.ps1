# stage-nsis.ps1 - seed tauri's local NSIS toolset from a pre-downloaded cache
# so Windows packaging never downloads nsis-3.11.zip / nsis_tauri_utils.dll
# from GitHub on the Windows runner.
#
# The Windows runner (yx-pc, tags "windows, rust") executes jobs as LocalSystem.
# Pre-download ONCE on that machine:
#   1. https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
#   2. https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll
#   Extract the zip so that makensis.exe and plugins/ sit under
#   C:\ci-cache\tauri\NSIS, and drop nsis_tauri_utils.dll into that folder.
#
# src-tauri/tauri.windows.conf.json sets bundle.useLocalToolsDir=true, so tauri
# expects the toolset at <project>/target/.tauri/NSIS. This script copies the
# cache there when present; otherwise tauri falls back to downloading.
#
# NOTE: keep this file ASCII-only. Windows PowerShell 5.1 decodes BOM-less
# UTF-8 files using the system ANSI codepage, so any non-ASCII byte breaks
# parsing on that host.

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$cacheDir = "C:\ci-cache\tauri\NSIS"
$targetDir = Join-Path $env:CI_PROJECT_DIR "target\.tauri\NSIS"

if (Test-Path (Join-Path $cacheDir "makensis.exe")) {
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
    Copy-Item -Path (Join-Path $cacheDir "*") -Destination $targetDir -Recurse -Force
    Write-Output "Staged NSIS toolset: $cacheDir -> $targetDir"
} else {
    Write-Output "NSIS cache not found at $cacheDir; tauri will download (see header of scripts/stage-nsis.ps1)."
}
