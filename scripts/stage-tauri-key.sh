#!/usr/bin/env bash
# stage-tauri-key.sh - materialize TAURI_SIGNING_PRIVATE_KEY into a key file.
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
# CI usage (bash jobs):
#   bash scripts/stage-tauri-key.sh
#
# Expected variable value (GitLab -> Settings -> CI/CD -> Variables):
#   base64 -w0 < tauri-updater.key     # single line

set -euo pipefail

if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  echo "TAURI_SIGNING_PRIVATE_KEY is not set. Add it under GitLab -> Settings -> CI/CD -> Variables" >&2
  echo "(value = base64 of the minisign secret key file: base64 -w0 < tauri-updater.key)." >&2
  exit 1
fi

key_file="$(mktemp "${TMPDIR:-/tmp}/tauri-signing-key.XXXXXX")"

# Write the base64 value verbatim (strip any trailing CR/LF that GitLab may
# append; base64 has no newlines). tauri-cli decodes it when signing the
# updater artifacts.
printf '%s' "$TAURI_SIGNING_PRIVATE_KEY" | tr -d '\r\n' > "$key_file"

export TAURI_SIGNING_PRIVATE_KEY="$key_file"
echo "Tauri signing key staged to $key_file"
