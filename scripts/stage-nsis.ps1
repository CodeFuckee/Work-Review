# stage-nsis.ps1 - seed tauri's local NSIS toolset from a pre-downloaded cache
# so Windows packaging never downloads nsis-3.11.zip / nsis_tauri_utils.dll
# from GitHub on the Windows runner.
#
# The Windows runner (yx-pc, tags "windows, rust") executes jobs as LocalSystem.
# Pre-download ONCE on that machine and extract into C:\ci-cache\tauri\NSIS:
#   1. https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
#      (extract so that Bin\makensis.exe, Include\, Plugins\, Stubs\ sit directly
#       under C:\ci-cache\tauri\NSIS; a single nested folder like nsis-3.11\ is
#       flattened automatically by this script)
#   2. https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll
#      (drop it into C:\ci-cache\tauri\NSIS\Plugins\x86-unicode\; this script also
#       mirrors it to the "additional" subfolder that newer tauri-bundler checks)
#
# src-tauri/tauri.windows.conf.json sets bundle.useLocalToolsDir=true, so tauri
# caches tools under <project>/target/.tauri (or src-tauri/target/.tauri for some
# tauri-cli versions). This script copies the cache to BOTH candidate locations.
#
# NOTE: keep this file ASCII-only. Windows PowerShell 5.1 decodes BOM-less
# UTF-8 files using the system ANSI codepage, so any non-ASCII byte breaks
# parsing on that host.

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$cacheDir = "C:\ci-cache\tauri\NSIS"

function Assert-NsisToolset($dir) {
    $required = @(
        "Bin\makensis.exe",
        "Include\MUI2.nsh",
        "Include\FileFunc.nsh",
        "Include\x64.nsh",
        "Include\nsDialogs.nsh",
        "Include\WinMessages.nsh",
        "Stubs\lzma-x86-unicode",
        "Stubs\lzma_solid-x86-unicode"
    )
    $missing = @($required | Where-Object { -not (Test-Path (Join-Path $dir $_)) })
    if ($missing.Count -gt 0) {
        throw "NSIS toolset at $dir is incomplete; missing: $($missing -join ', '). " +
              "Fix C:\ci-cache\tauri\NSIS (see header of scripts/stage-nsis.ps1)."
    }
}

if (-not (Test-Path (Join-Path $cacheDir "makensis.exe")) -and
    -not (Test-Path (Join-Path $cacheDir "Bin\makensis.exe"))) {
    Write-Output "NSIS cache not found at $cacheDir; tauri will download (see header of scripts/stage-nsis.ps1)."
    return
}

# Flatten a single nested top-level folder (e.g. nsis-3.11\) so the required
# files sit directly under $cacheDir, matching tauri's expected layout.
$topDirs = @(Get-ChildItem -Path $cacheDir -Directory -ErrorAction SilentlyContinue)
if ($topDirs.Count -eq 1 -and
    -not (Test-Path (Join-Path $cacheDir "Bin")) -and
    (Test-Path (Join-Path $topDirs[0].FullName "Bin"))) {
    Write-Output "Flattening $($topDirs[0].FullName) -> $cacheDir"
    Get-ChildItem -Path $topDirs[0].FullName | Move-Item -Destination $cacheDir -Force
    Remove-Item -Path $topDirs[0].FullName -Force -ErrorAction SilentlyContinue
}

# Mirror nsis_tauri_utils.dll to both plugin locations: older tauri-bundler
# checks Plugins\x86-unicode\, newer checks Plugins\x86-unicode\additional\.
$dll = "nsis_tauri_utils.dll"
$pluginDirs = @(
    (Join-Path $cacheDir "Plugins\x86-unicode"),
    (Join-Path $cacheDir "Plugins\x86-unicode\additional")
)
foreach ($p in $pluginDirs) {
    if (-not (Test-Path (Join-Path $p $dll))) {
        $src = @($pluginDirs | Where-Object { Test-Path (Join-Path $_ $dll) } | Select-Object -First 1)
        if ($src.Count -gt 0) {
            New-Item -ItemType Directory -Force -Path $p | Out-Null
            Copy-Item -Path (Join-Path $src[0] $dll) -Destination (Join-Path $p $dll) -Force
            Write-Output "Mirrored $dll -> $p"
        } else {
            Write-Output "WARNING: $dll missing in cache; tauri will re-download plugins (network may fail)."
        }
    }
}

Assert-NsisToolset $cacheDir

foreach ($targetDir in @(
        (Join-Path $env:CI_PROJECT_DIR "target\.tauri\NSIS"),
        (Join-Path $env:CI_PROJECT_DIR "src-tauri\target\.tauri\NSIS"))) {
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
    Copy-Item -Path (Join-Path $cacheDir "*") -Destination $targetDir -Recurse -Force
    Assert-NsisToolset $targetDir
    Write-Output "Staged NSIS toolset: $cacheDir -> $targetDir"
}
