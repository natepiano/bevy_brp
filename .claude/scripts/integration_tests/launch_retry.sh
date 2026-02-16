#!/usr/bin/env bash
# Calculate retry delay for BRP status checks
# Usage: integration_test_launch_retry.sh <attempt_number> [base_delay_ms]
# Returns: delay in seconds for the given attempt number

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <attempt_number> [base_delay_ms]" >&2
    exit 1
fi

attempt="$1"
base_delay_ms="${2:-500}"  # Default to 500ms

# Calculate exponential backoff: delay = base * (2 ^ (attempt - 1))
# attempt 1: 500ms, attempt 2: 1000ms, attempt 3: 2000ms, attempt 4: 4000ms
delay_ms=$((base_delay_ms * (1 << (attempt - 1))))

# Convert to seconds for output
delay_seconds=$(echo "scale=3; $delay_ms / 1000" | bc)

echo "$delay_seconds"
