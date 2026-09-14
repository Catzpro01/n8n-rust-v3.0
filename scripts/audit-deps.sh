#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Dependency vulnerability audit for the editor (npm) and the Rust workspace
# (cargo-audit / RustSec). This is intentionally separate from
# scripts/build-release.sh: the release build must stay hermetic and
# reproducible, while an audit needs to query live advisory databases over
# the network. Keeping them apart means a registry outage fails the *audit*
# step with a clear message instead of masquerading as a broken release build.
#
# Exit codes:
#   0  no vulnerabilities at or above the configured level
#   1  vulnerabilities found (or the audit tooling itself errored)
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
export PATH="$HOME/.cargo/bin:$HOME/.local/node-v22.19.0-linux-x64/bin:$PATH"

AUDIT_LEVEL=${AUDIT_LEVEL:-high}
failed=0

echo "==> npm audit (editor, --audit-level=$AUDIT_LEVEL)"
if [ ! -d editor/node_modules ]; then
  (cd editor && npm ci --ignore-scripts)
fi
if ! (cd editor && npm audit --audit-level="$AUDIT_LEVEL"); then
  echo "npm audit reported vulnerabilities or could not reach the registry" >&2
  failed=1
fi

echo "==> cargo audit (Rust workspace)"
if command -v cargo-audit >/dev/null 2>&1; then
  if ! cargo audit; then
    echo "cargo audit reported RustSec advisories" >&2
    failed=1
  fi
else
  echo "cargo-audit is not installed; skipping RustSec audit." >&2
  echo "Install with: cargo install cargo-audit --locked" >&2
  if [ "${REQUIRE_CARGO_AUDIT:-0}" = "1" ]; then
    failed=1
  fi
fi

if [ "$failed" -ne 0 ]; then
  echo "dependency audit FAILED" >&2
  exit 1
fi
echo "dependency audit passed"
