#!/usr/bin/env bash
# Cleanup stale integration-test app processes from the test config.
# Usage:
#   bash .claude/scripts/integration_tests/cleanup_stale_test_processes.sh <config_path>

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <config_path>" >&2
    exit 2
fi

config_path="$1"

if [[ ! -f "${config_path}" ]]; then
    echo "[cleanup] Config file not found: ${config_path}" >&2
    exit 2
fi

uid="$(id -u)"

app_names=()
while IFS= read -r app; do
    app_names+=("${app}")
done < <(
    jq -r '
      [
        (.[] | select(has("app_name")) | .app_name),
        (.[] | select(has("apps")) | .apps[]?.app_name)
      ]
      | flatten
      | map(select(. != null and . != "" and . != "N/A" and . != "various"))
      | unique
      | .[]
    ' "${config_path}"
)

if [[ ${#app_names[@]} -eq 0 ]]; then
    echo "[cleanup] No configured app names found in ${config_path}"
    exit 0
fi

echo "[cleanup] Target app names: ${app_names[*]}"

# Graceful pass
for app in "${app_names[@]}"; do
    pkill -TERM -x -u "${uid}" "${app}" 2>/dev/null || true
done

sleep 1

# Force-kill remaining processes
for app in "${app_names[@]}"; do
    pkill -KILL -x -u "${uid}" "${app}" 2>/dev/null || true
done

echo "[cleanup] Completed stale process cleanup"
