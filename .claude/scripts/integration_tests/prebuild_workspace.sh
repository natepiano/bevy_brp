#!/usr/bin/env bash
# Strict prebuild script for integration test runs.
# Usage:
#   bash .claude/scripts/integration_tests/prebuild_workspace.sh

set -euo pipefail

# Write logs into the repo's gitignored transient dir so this works in both
# the Claude Code sandbox (which restricts writes outside the project) and
# Codex (which has no such restriction). Resolved relative to this script so
# it doesn't depend on the caller's CWD.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="${SCRIPT_DIR}/../../transient"
mkdir -p "${LOG_DIR}"

run_build() {
    local label="$1"
    shift

    local logfile
    logfile="$(mktemp "${LOG_DIR}/bevy_brp_prebuild.XXXXXX.log")"

    echo "[prebuild] ${label}"

    if "$@" >"${logfile}" 2>&1; then
        tail -n 5 "${logfile}"
        rm -f "${logfile}"
        echo "[prebuild] ${label}: OK"
        return 0
    fi

    local status=$?
    echo "[prebuild] ${label}: FAILED (exit ${status})"
    tail -n 50 "${logfile}"
    rm -f "${logfile}"
    exit "${status}"
}

run_build "workspace + examples (dev profile)" \
    cargo build --workspace --examples --profile dev
