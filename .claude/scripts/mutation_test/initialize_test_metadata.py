#!/usr/bin/env python3
"""
Initialize or reset test metadata for types in the mutation test tracking file.

This script applies auto-pass logic to determine which types should be automatically
marked as "passed" vs "untested":
- Types with no mutation paths → "passed"
- Types with only root path that is not_mutable → "passed"
- Types with only root path with no examples → "passed"
- All other types → "untested"

Usage:
  # Reset all test metadata to defaults (auto-pass or untested)
  python3 initialize_test_metadata.py --file .claude/transient/all_types.json

  # Preview what would change without modifying the file
  python3 initialize_test_metadata.py --file .claude/transient/all_types.json --dry-run
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any, TypedDict, cast


class TestMetadata(TypedDict):
    """Test metadata fields."""

    batch_number: int | None
    test_status: str
    fail_reason: str


class TypeGuide(TypedDict, total=False):
    """Type guide structure with test metadata."""

    type: str
    mutation_paths: list[Any]  # pyright: ignore[reportExplicitAny]
    batch_number: int | None
    test_status: str
    fail_reason: str


class AllTypesFile(TypedDict):
    """Structure of all_types.json file."""

    type_guide: dict[str, TypeGuide]


def should_auto_pass(type_guide: TypeGuide) -> bool:
    """
    Determine if a type should be auto-passed based on mutation paths.

    Auto-pass criteria:
    1. No mutation paths at all
    2. Only root path ("") that is not_mutable
    3. Only root path ("") with no examples
    """
    mutation_paths = type_guide.get("mutation_paths")

    # No mutation paths → auto-pass
    if not mutation_paths or len(mutation_paths) == 0:
        return True

    # Only root path → check if testable
    if len(mutation_paths) == 1:
        # Get the single path entry (should be root with path="")
        root_path: dict[str, Any] = mutation_paths[0]  # pyright: ignore[reportExplicitAny]
        # Verify it's actually the root path
        if root_path.get("path") != "":
            return False  # Not a single root path, needs testing

        # Check mutability
        path_info: dict[str, Any] = root_path.get("path_info", {})  # pyright: ignore[reportExplicitAny]
        if path_info.get("mutability") == "not_mutable":
            return True

        # Check if has testable examples
        example: dict[str, Any] = root_path.get("example", {})  # pyright: ignore[reportExplicitAny]
        examples: list[Any] = root_path.get("examples", [])  # pyright: ignore[reportExplicitAny]

        # No examples or empty example → auto-pass
        if (not example or len(example) == 0) and (
            not examples or len(examples) == 0
        ):
            return True

    return False


def initialize_test_metadata(
    file_path: Path, dry_run: bool = False, only_new: bool = True
) -> tuple[int, int]:
    """
    Initialize test metadata for types in the file.

    Args:
        file_path: Path to all_types.json file
        dry_run: If True, don't modify the file
        only_new: If True, only process types with test_status="untested" (preserve existing)
                  If False, reset all types and apply auto-pass (init mode)

    Returns:
        Tuple of (auto_passed_count, untested_count)
    """
    # Read file
    try:
        with open(file_path, encoding="utf-8") as f:
            data: AllTypesFile = cast(AllTypesFile, json.load(f))
    except FileNotFoundError:
        print(f"Error: File not found: {file_path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in {file_path}: {e}", file=sys.stderr)
        sys.exit(1)

    type_guide = data.get("type_guide", {})
    if not type_guide:
        print("Error: No type_guide found in file", file=sys.stderr)
        sys.exit(1)

    # Initialize metadata
    auto_passed = 0
    untested = 0

    for _, type_data in type_guide.items():
        # If only_new mode, skip types that already have results
        if only_new:
            current_status = type_data.get("test_status", "untested")
            # Skip types that aren't untested (they have existing results to preserve)
            if current_status != "untested":
                # Count them for stats
                if current_status == "passed":
                    auto_passed += 1
                else:
                    untested += 1
                continue

        # Determine test status using auto-pass logic
        if should_auto_pass(type_data):
            test_status = "passed"
            auto_passed += 1
        else:
            test_status = "untested"
            untested += 1

        # Update metadata (batch_number null for new types, preserved for existing)
        if only_new:
            # For new types, just update test_status (batch_number already null from jq)
            type_data["test_status"] = test_status
        else:
            # For init mode, reset everything
            type_data["batch_number"] = None
            type_data["test_status"] = test_status
            type_data["fail_reason"] = ""

    # Write back unless dry run
    if not dry_run:
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
        print(f"✅ Initialized test metadata in {file_path}")
    else:
        print(f"🔍 Dry run - no changes made to {file_path}")

    return auto_passed, untested


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Initialize or reset test metadata for mutation testing"
    )
    _ = parser.add_argument(
        "--file",
        required=True,
        type=Path,
        help="Path to all_types.json file",
    )
    _ = parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview changes without modifying the file",
    )
    _ = parser.add_argument(
        "--reset-all",
        action="store_true",
        help="Reset all types (init mode), not just new ones",
    )

    args = parser.parse_args()

    # Initialize metadata
    file_path = cast(Path, args.file)
    dry_run = cast(bool, args.dry_run)
    reset_all = cast(bool, args.reset_all)
    only_new = not reset_all  # Invert: reset_all=False means only_new=True
    auto_passed, untested = initialize_test_metadata(file_path, dry_run, only_new)

    # Report results
    print()
    print("Test Metadata Initialization Results:")
    print(f"  Auto-passed: {auto_passed} types")
    print(f"  Untested: {untested} types")
    print(f"  Total: {auto_passed + untested} types")


if __name__ == "__main__":
    main()
