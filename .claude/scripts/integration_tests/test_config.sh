#!/usr/bin/env bash
# Query helper for the integration-test config JSON.
#
# Wraps the handful of jq queries the /integration_tests orchestration needs,
# so they can be covered by a single permission-allowlist entry
# (Bash(bash .claude/scripts/integration_tests/test_config.sh:*)) instead of
# triggering one-off prompts for each ad-hoc jq pipeline.

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"

usage() {
    cat <<EOF
Usage: $SCRIPT_NAME <subcommand> [config-file]

Subcommands:
  counts           Print total, individual_only, and batch counts as shell vars.
                   Output: total=N individual_only=M batch=K
  batch-names      Print test_name of every test included in batch execution
                   (one per line, in config order).
  batch-entries    Print every batch-included test as a JSON line with fields
                   {test_name, test_file, app_name, app_type, apps}.
  find-test NAME   Print the full JSON entry for the test with the given name,
                   or exit 1 if not found.
  list-all-names   Print test_name of every test in the config (one per line,
                   in config order). Useful for "test not found" error messages.

The config file defaults to .claude/config/integration_tests.json if omitted.
EOF
}

if [[ $# -lt 1 ]]; then
    usage >&2
    exit 2
fi

SUBCOMMAND="$1"
shift

# Optional positional args come from the subcommand's parser below.
case "$SUBCOMMAND" in
    counts | batch-names | batch-entries | list-all-names)
        CONFIG_FILE="${1:-.claude/config/integration_tests.json}"
        ;;
    find-test)
        if [[ $# -lt 1 ]]; then
            echo "error: find-test requires a test name" >&2
            exit 2
        fi
        TEST_NAME="$1"
        CONFIG_FILE="${2:-.claude/config/integration_tests.json}"
        ;;
    -h | --help | help)
        usage
        exit 0
        ;;
    *)
        echo "error: unknown subcommand '$SUBCOMMAND'" >&2
        usage >&2
        exit 2
        ;;
esac

if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "error: config file not found: $CONFIG_FILE" >&2
    exit 1
fi

case "$SUBCOMMAND" in
    counts)
        total=$(jq '.tests | length' "$CONFIG_FILE")
        individual_only=$(jq '[.tests[] | select(.individual_only == true)] | length' "$CONFIG_FILE")
        batch=$((total - individual_only))
        printf 'total=%s individual_only=%s batch=%s\n' "$total" "$individual_only" "$batch"
        ;;
    batch-names)
        jq -r '.tests[] | select(.individual_only == true | not) | .test_name' "$CONFIG_FILE"
        ;;
    batch-entries)
        jq -c '.tests[] | select(.individual_only == true | not) | {test_name, test_file, app_name, app_type, apps}' "$CONFIG_FILE"
        ;;
    list-all-names)
        jq -r '.tests[].test_name' "$CONFIG_FILE"
        ;;
    find-test)
        result=$(jq -c --arg name "$TEST_NAME" '.tests[] | select(.test_name == $name)' "$CONFIG_FILE")
        if [[ -z "$result" ]]; then
            echo "error: test '$TEST_NAME' not found in $CONFIG_FILE" >&2
            exit 1
        fi
        printf '%s\n' "$result"
        ;;
esac
