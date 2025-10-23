#!/usr/bin/env python3
"""
Get the file path for a mutation test plan file given a port number.

Usage:
  python3 get_plan_file_path.py --port 30001

Output:
  /var/folders/.../mutation_test_30001.json
"""

import argparse
from typing import cast


def get_plan_file_path(port: int) -> str:
    """
    Construct the test plan file path for a given port.

    Args:
        port: The port number

    Returns:
        Absolute path to the test plan file
    """
    return f"/tmp/mutation_test_{port}.json"


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Get test plan file path for a given port"
    )
    _ = parser.add_argument(
        "--port", type=int, required=True, help="Port number"
    )

    args = parser.parse_args()
    port = cast(int, args.port)

    file_path = get_plan_file_path(port)
    print(file_path)


if __name__ == "__main__":
    main()
