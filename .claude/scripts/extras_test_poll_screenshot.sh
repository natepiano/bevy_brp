#!/usr/bin/env bash
# Poll for screenshot file completion (screenshot I/O is asynchronous)
# Usage: extras_test_poll_screenshot.sh <path_to_screenshot>

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <path_to_screenshot>" >&2
    exit 1
fi

path="$1"
timeout=10

for i in $(seq 1 $timeout); do
    if [ -f "$path" ] && [ -s "$path" ]; then
        echo "Screenshot ready: $path"
        exit 0
    fi
    sleep 0.5
done

if [ -f "$path" ] && [ -s "$path" ]; then
    echo "Screenshot ready: $path"
    exit 0
else
    echo "Screenshot not ready after ${timeout}s: $path" >&2
    exit 1
fi
