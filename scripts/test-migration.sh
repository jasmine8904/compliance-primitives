#!/usr/bin/env bash
# Build and exercise an upgrade from the last released WASM to HEAD.
#
# Usage:
#   scripts/test-migration.sh [previous-release-ref]
#
# PREVIOUS_WASM_DIR may point at a directory containing the previous release
# WASM artifacts instead of checking out and building a git ref.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

fail() {
  echo "migration test failed: $*" >&2
  exit 1
}

PREVIOUS_REF="${1:-${PREVIOUS_RELEASE_REF:-}}"
PREVIOUS_WASM_DIR="${PREVIOUS_WASM_DIR:-}"
TEMP_WORKTREE=""

cleanup() {
  if [ -n "$TEMP_WORKTREE" ] && [ -d "$TEMP_WORKTREE" ]; then
    git worktree remove --force "$TEMP_WORKTREE" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [ -z "$PREVIOUS_WASM_DIR" ]; then
  if [ -z "$PREVIOUS_REF" ]; then
    PREVIOUS_REF="$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || true)"
  fi
  if [ -z "$PREVIOUS_REF" ]; then
    echo "==> Skipping migration test: no previous release tag is available"
    echo "    Pass a ref or set PREVIOUS_WASM_DIR when testing a tagged release."
    exit 0
  fi

  git rev-parse --verify "${PREVIOUS_REF}^{commit}" >/dev/null 2>&1 ||
    fail "previous release ref not found: $PREVIOUS_REF"

  TEMP_WORKTREE="$(mktemp -d)"
  git worktree add --detach "$TEMP_WORKTREE" "$PREVIOUS_REF" >/dev/null
  rustup target add wasm32v1-none >/dev/null 2>&1 || true

  echo "==> Building previous release $PREVIOUS_REF"
  (
    cd "$TEMP_WORKTREE"
    cargo build --target wasm32v1-none --release \
      -p allowlist-token -p denylist-gate -p jurisdiction-flag
  )
  PREVIOUS_WASM_DIR="$TEMP_WORKTREE/target/wasm32v1-none/release"
fi

[ -d "$PREVIOUS_WASM_DIR" ] || fail "previous WASM directory not found: $PREVIOUS_WASM_DIR"
PREVIOUS_ALLOWLIST="$PREVIOUS_WASM_DIR/allowlist_token.wasm"
[ -f "$PREVIOUS_ALLOWLIST" ] || fail "missing previous allowlist WASM: $PREVIOUS_ALLOWLIST"

echo "==> Building HEAD WASM"
rustup target add wasm32v1-none >/dev/null 2>&1 || true
cargo build --target wasm32v1-none --release \
  -p allowlist-token -p denylist-gate -p jurisdiction-flag

HEAD_ALLOWLIST="$REPO_ROOT/target/wasm32v1-none/release/allowlist_token.wasm"
[ -f "$HEAD_ALLOWLIST" ] || fail "missing HEAD allowlist WASM: $HEAD_ALLOWLIST"

echo "==> Testing previous WASM -> HEAD upgrade"
PREVIOUS_ALLOWLIST_WASM="$PREVIOUS_ALLOWLIST" \
HEAD_ALLOWLIST_WASM="$HEAD_ALLOWLIST" \
cargo test -p allowlist-token \
  test_previous_release_wasm_upgrade_preserves_state -- --exact
