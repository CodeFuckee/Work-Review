# stage-tauri-key.ps1 - materialize TAURI_SIGNING_PRIVATE_KEY into a key file.
#
# GitLab CI variables must be single-line (masking rules forbid newlines), so
# TAURI_SIGNING_PRIVATE_KEY stores the base64-encoded minisign secret key
# (i.e. the verbatim content of tauri-updater.key, produced by
# `base64 -w0 < tauri-updater.key`).
#
# tauri-cli v2 (>= 2.10) base64-decodes the value of TAURI_SIGNING_PRIVATE_KEY
# itself (crates/tauri-cli/src/helpers/updater_signature.rs: secret_key ->
# decode_key), whether the variable is a file path or raw content. So this
# script only materializes the base64 value into a temp file (base64 text, no
# trailing newline) and re-points the variable at that file. It must NOT
# base64-decode here: that turns the value into the raw "untrusted comment:"
# key text, which tauri-cli then tries to decode as base64 and fails with
# "failed to decode base64 secret key: Invalid symbol 32, offset 9".
#
# The key is never echoed to the job log. Only its temp path is printed.
#
# CI usage:
#   & "$env:CI_PROJECT_DIR\scripts\stage-tauri-key.ps1"
#
# Expected variable value (GitLab -> Settings -> CI/CD -> Variables):
#   base64 -w0 < tauri-updater.key     # single line
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
# Write the base64 value verbatim (Trim strips any trailing newline that
# GitLab may append). tauri-cli decodes it when signing the updater artifacts.
[System.IO.File]::WriteAllText($keyPath, $env:TAURI_SIGNING_PRIVATE_KEY.Trim())

$env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
Write-Output "Tauri signing key staged to $keyPath"
