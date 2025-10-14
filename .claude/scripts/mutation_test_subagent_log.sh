#!/bin/bash
# Mutation Test Subagent Progress Logging
# Usage: mutation_test_subagent_log.sh <port> <action> [message]
# Actions: init, log, error, step

set -e

PORT="$1"
ACTION="$2"
MESSAGE="${3:-}"

if [[ -z "$PORT" || -z "$ACTION" ]]; then
    echo "Usage: mutation_test_subagent_log.sh <port> <action> [message]"
    echo "Actions: init, log, error, step"
    exit 1
fi

LOG_FILE="$TMPDIR/mutation_test_subagent_${PORT}_progress.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

case "$ACTION" in
    init)
        # Initialize/overwrite log file
        echo "[$TIMESTAMP] === SUBAGENT PORT $PORT START ===" > "$LOG_FILE"
        ;;
    step)
        # Log workflow step
        echo "[$TIMESTAMP] STEP: $MESSAGE" >> "$LOG_FILE"
        ;;
    log)
        # Log general message
        echo "[$TIMESTAMP] LOG: $MESSAGE" >> "$LOG_FILE"
        ;;
    error)
        # Log error
        echo "[$TIMESTAMP] ERROR: $MESSAGE" >> "$LOG_FILE"
        ;;
    tool)
        # Log tool call
        echo "[$TIMESTAMP] TOOL: $MESSAGE" >> "$LOG_FILE"
        ;;
    result)
        # Log result
        echo "[$TIMESTAMP] RESULT: $MESSAGE" >> "$LOG_FILE"
        ;;
    *)
        echo "Unknown action: $ACTION"
        exit 1
        ;;
esac
