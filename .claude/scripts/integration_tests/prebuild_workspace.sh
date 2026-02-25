#!/usr/bin/env bash
# Strict prebuild script for integration test runs.
# Usage:
#   bash .claude/scripts/integration_test/prebuild_workspace.sh
#   bash .claude/scripts/integration_test/prebuild_workspace.sh --include-wasm

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: prebuild_workspace.sh [--include-wasm]

Options:
  --include-wasm   Also build the wasm test package.
  -h, --help       Show this help text.
EOF
}

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

include_wasm=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --include-wasm)
            include_wasm=true
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

run_build "workspace + examples (dev profile)" \
    cargo build --workspace --examples --profile dev

if [[ "${include_wasm}" == "true" ]]; then
    run_build "wasm package bevy_brp_test_wasm (dev profile)" \
        cargo build --target wasm32-unknown-unknown -p bevy_brp_test_wasm --profile dev
fi

