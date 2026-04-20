#!/usr/bin/env bash
# Strict prebuild script for integration test runs.
# Usage:
#   bash .claude/scripts/integration_tests/prebuild_workspace.sh

set -euo pipefail

run_build() {
    local label="$1"
    shift

    local logfile
    logfile="$(mktemp -t bevy_brp_prebuild.XXXXXX.log)"

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
