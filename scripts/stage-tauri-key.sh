#!/usr/bin/env bash
# stage-tauri-key.sh - materialize TAURI_SIGNING_PRIVATE_KEY into a key file.
#
# GitLab CI variables must be single-line (masking rules forbid newlines), so
# the multi-line minisign secret key is stored base64-encoded in the
# TAURI_SIGNING_PRIVATE_KEY variable. tauri-cli reads that variable as either
# a file path or the RAW multi-line key content - it does NOT decode base64
# (see crates/tauri-cli/src/bundle.rs in the tauri repo). This script decodes
# the variable into a temporary file and re-points the variable at that file,
# so every packaging job (daily + tag) can simply call it before
# `npm run tauri build`.
#
# The key is never echoed to the job log. Only its temp path is printed.
#
# CI usage (bash jobs):
#   bash scripts/stage-tauri-key.sh
#
# Expected variable value (GitLab -> Settings -> CI/CD -> Variables):
#   base64 -w0 < tauri-updater.key     # single line
# Raw multi-line key content is also accepted for robustness.

set -euo pipefail

if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  echo "TAURI_SIGNING_PRIVATE_KEY is not set. Add it under GitLab -> Settings -> CI/CD -> Variables" >&2
  echo "(value = base64 of the minisign secret key file: base64 -w0 < tauri-updater.key)." >&2
  exit 1
fi

key_file="$(mktemp "${TMPDIR:-/tmp}/tauri-signing-key.XXXXXX")"

if [[ "$TAURI_SIGNING_PRIVATE_KEY" == *"untrusted comment:"* ]]; then
  # Raw multi-line key content (allowed when masking is not used).
  printf '%s\n' "$TAURI_SIGNING_PRIVATE_KEY" > "$key_file"
else
  # Single-line GitLab variables store the key base64-encoded. openssl is
  # available on both Ubuntu and macOS runners; plain `base64 -d` is not
  # portable across the two.
  if ! printf '%s' "$TAURI_SIGNING_PRIVATE_KEY" | openssl base64 -d -A > "$key_file" 2>/dev/null; then
    rm -f "$key_file"
    echo "TAURI_SIGNING_PRIVATE_KEY is neither raw key content nor valid base64." >&2
    exit 1
  fi
fi

export TAURI_SIGNING_PRIVATE_KEY="$key_file"
echo "Tauri signing key staged to $key_file"
