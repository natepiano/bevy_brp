#!/usr/bin/env bash
# Clean up a test screenshot file by absolute path
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <absolute_path_to_screenshot>" >&2
    exit 2
fi

rm -f "$1"
