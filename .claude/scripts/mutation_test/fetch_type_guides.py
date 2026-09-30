#!/usr/bin/env python3
"""
Fetch every type guide for /create_mutation_test_json through the MCP stdio client.

Builds bevy_brp_mcp from this checkout, launches extras_plugin on port 22222, calls
brp_all_type_guides, writes the result to .claude/transient/all_types.json (a fresh file
with no test metadata; prepare.py initializes that), and shuts the app down.

Usage:
  python3 .claude/scripts/mutation_test/fetch_type_guides.py

Output:
  JSON on stdout with the file written and the number of type guides.
"""

import json
import shutil
import sys
from pathlib import Path
from typing import cast

# Add script directory to path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from mcp_client import REPO_ROOT, McpClient, McpClientError, saved_result_path  # noqa: E402
from run import launch_apps, shutdown_apps  # noqa: E402

APP_PORT = 22222
TARGET_FILE = REPO_ROOT / ".claude/transient/all_types.json"
# Seconds allowed for brp_all_type_guides, which gathers every registered type's guide
TYPE_GUIDES_TIMEOUT = 600.0


def fetch(client: McpClient) -> int:
    """Write every type guide to TARGET_FILE and return how many there are."""
    launch_apps(client, [APP_PORT])
    try:
        outcome = client.call_tool("brp_all_type_guides", {"port": APP_PORT}, TYPE_GUIDES_TIMEOUT)
    finally:
        shutdown_apps(client, [APP_PORT])
    if outcome.is_error:
        raise McpClientError(f"brp_all_type_guides failed: {outcome.response().get('message')}")

    TARGET_FILE.parent.mkdir(parents=True, exist_ok=True)
    raw_result = outcome.response().get("result")
    saved_path = saved_result_path(raw_result)
    if saved_path is not None:
        _ = shutil.copyfile(saved_path, TARGET_FILE)
    else:
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            json.dump(raw_result, f, indent=2)

    with open(TARGET_FILE, encoding="utf-8") as f:
        guides = cast(dict[str, object], json.load(f))
    return len(cast(dict[str, object], guides.get("type_guide", {})))


def main() -> None:
    """Build, fetch, and report."""
    try:
        with McpClient.build_and_start() as client:
            count = fetch(client)
    except (McpClientError, TimeoutError, RuntimeError, OSError) as e:
        print(json.dumps({"status": "error", "message": str(e)}, indent=2))
        sys.exit(1)
    print(
        json.dumps(
            {
                "status": "success",
                "file": str(TARGET_FILE.relative_to(REPO_ROOT)),
                "type_guides": count,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
