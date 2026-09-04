# stage-tauri-key.ps1 - materialize TAURI_SIGNING_PRIVATE_KEY into a key file.
#
# GitLab CI variables must be single-line (masking rules forbid newlines), so
# the multi-line minisign secret key is stored base64-encoded in the
# TAURI_SIGNING_PRIVATE_KEY variable. tauri-cli reads that variable as either
# a file path or the RAW multi-line key content - it does NOT decode base64.
# This script decodes the variable into a temp file and re-points the
# variable at that file, so the Windows packaging jobs can call it before
# `npm run tauri build`.
#
# The key is never echoed to the job log. Only its temp path is printed.
#
# CI usage:
#   & "$env:CI_PROJECT_DIR\scripts\stage-tauri-key.ps1"
#
# Expected variable value (GitLab -> Settings -> CI/CD -> Variables):
#   base64 -w0 < tauri-updater.key     # single line
# Raw multi-line key content is also accepted for robustness.
#
# NOTE: keep this file ASCII-only. Windows PowerShell 5.1 decodes BOM-less
# UTF-8 files using the system ANSI codepage, so any non-ASCII byte breaks
# parsing on that host.

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    throw "TAURI_SIGNING_PRIVATE_KEY is not set. Add it under GitLab > Settings > CI/CD > Variables (value = base64 of the minisign secret key file)."
}

$keyPath = Join-Path ([System.IO.Path]::GetTempPath()) ("tauri-signing-key-{0}.key" -f $PID)
if ($env:TAURI_SIGNING_PRIVATE_KEY -match "untrusted comment:") {
    # Raw multi-line key content (allowed when masking is not used).
    [System.IO.File]::WriteAllText($keyPath, $env:TAURI_SIGNING_PRIVATE_KEY + "`n")
} else {
    # Single-line GitLab variables store the key base64-encoded.
    try {
        [System.IO.File]::WriteAllBytes($keyPath, [Convert]::FromBase64String($env:TAURI_SIGNING_PRIVATE_KEY))
    } catch {
        throw ("TAURI_SIGNING_PRIVATE_KEY is neither raw key content nor valid base64: {0}" -f $_.Exception.Message)
    }
}

$env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
Write-Output "Tauri signing key staged to $keyPath"
