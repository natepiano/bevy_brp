#!/usr/bin/env bash
set -euo pipefail

# Usage: update_workspace_dep.sh <version> [--dry-run]
# Updates the workspace root Cargo.toml dependency for bevy_brp_mcp_macros
# to the newly published version, verifies the workspace builds, and commits.
# This runs between publish phases — after mcp_macros is published but before
# extras and mcp are published.
# Uses fibonacci backoff to wait for crates.io indexing.
# With --dry-run, reports what would happen without modifying anything.
# Exit 0 = success, Exit 1 = failure

VERSION="$1"
DRY_RUN="${2:-}"

echo "=== Update Workspace Dependency ==="

if [[ "$DRY_RUN" == "--dry-run" ]]; then
  CURRENT=$(grep '^bevy_brp_mcp_macros' Cargo.toml | head -1)
  echo "  [DRY-RUN] Would update Cargo.toml: $CURRENT → bevy_brp_mcp_macros = \"$VERSION\""
  echo "  [DRY-RUN] Would wait for crates.io indexing (fibonacci backoff)"
  echo "  [DRY-RUN] Would verify: cargo build --package bevy_brp_mcp"
  echo "  [DRY-RUN] Would commit: chore: update workspace dep to bevy_brp_mcp_macros $VERSION"
  echo ""
  echo "[DRY-RUN] Workspace dependency would be updated to bevy_brp_mcp_macros $VERSION"
  exit 0
fi

echo "  Updating bevy_brp_mcp_macros dependency to $VERSION in Cargo.toml..."
sed -i '' "s/^bevy_brp_mcp_macros = .*/bevy_brp_mcp_macros = \"$VERSION\"/" Cargo.toml

# Verify the change took
CURRENT=$(grep '^bevy_brp_mcp_macros' Cargo.toml | head -1)
echo "  Cargo.toml now has: $CURRENT"

echo ""
echo "  Waiting for crates.io to index bevy_brp_mcp_macros $VERSION..."

BACKOFF=(1 2 3 5 8 13 21 35)
BUILD_OK=false

for WAIT in "${BACKOFF[@]}"; do
  echo "    Attempting cargo build (backoff: ${WAIT}s)..."
  if cargo build --package bevy_brp_mcp 2>/dev/null; then
    BUILD_OK=true
    break
  fi
  echo "    Not indexed yet, waiting ${WAIT}s..."
  sleep "$WAIT"
done

if [[ "$BUILD_OK" != "true" ]]; then
  echo "ERROR: bevy_brp_mcp_macros $VERSION not indexed on crates.io after all retries" >&2
  exit 1
fi

echo "  Build: passed ✓"

echo ""
echo "  Committing workspace dependency update..."
git add Cargo.toml Cargo.lock
git commit -m "chore: update workspace dep to bevy_brp_mcp_macros $VERSION"

echo ""
echo "Workspace dependency updated to bevy_brp_mcp_macros $VERSION ✓"
