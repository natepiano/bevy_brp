#!/usr/bin/env python3
"""
Initialize test metadata and deduplicate mutation paths for all_types.json.

This script performs one-time initialization on fresh all_types.json files:
1. Deduplicates mutation paths (marks representatives)
2. Initializes test metadata (test_status, batch_number, fail_reason)

It's idempotent: if already initialized, exits early with success.
Always called by prepare.py before batch processing.

Usage:
  python3 initialize_test_metadata.py --file .claude/transient/all_types.json
  python3 initialize_test_metadata.py --file .claude/transient/all_types.json --dry-run
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast

# Add the script directory to Python path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from config import AllTypesData, TypeData
from validate_deduplication import validate_deduplication_data


def is_already_initialized(data: AllTypesData) -> bool:
    """
    Check if data has already been initialized.

    Checks for valid test_status, batch_number, and fail_reason fields.
    These fields are added by initialization and indicate the file has been processed.
    """
    type_guide = data.get("type_guide", {})

    # Check first 5 types for initialized metadata
    checked = 0
    for _, type_data in list(type_guide.items())[:5]:
        checked += 1

        # All three fields must exist and be the correct type
        test_status = type_data.get("test_status")
        batch_number = type_data.get("batch_number")
        fail_reason = type_data.get("fail_reason")

        # If any field is missing or wrong type, not initialized
        if not isinstance(test_status, str):
            return False
        if batch_number is not None and not isinstance(batch_number, int):
            return False
        if not isinstance(fail_reason, str):
            return False

    # If we checked at least one type and all passed, it's initialized
    return checked > 0


def deduplicate_mutation_paths(data: AllTypesData) -> tuple[AllTypesData, int]:
    """
    Deduplicate mutation paths across all types based on path_info.type.

    For each unique type being tested, the first occurrence becomes the representative.
    All subsequent occurrences are marked as duplicates and will be skipped during
    test generation. Child paths of duplicates are also marked to ensure hierarchical
    propagation.

    Args:
        data: AllTypesData containing type_guide

    Returns:
        Tuple of (modified data, number of duplicates marked)
    """
    type_guide = data["type_guide"]
    type_representatives: dict[str, tuple[str, str]] = {}  # tested_type -> (parent_type, path)
    duplicates_marked = 0

    # First pass: identify representatives and mark duplicates
    for parent_type_name, type_data in type_guide.items():
        mutation_paths = type_data.get("mutation_paths")
        if not mutation_paths:
            continue

        duplicate_prefixes: set[str] = set()  # Track paths that are duplicates

        for mp in mutation_paths:
            # Cast to dict for proper type checking
            mp_dict = cast(dict[str, object], mp)
            path_info_obj = mp_dict.get("path_info")
            if not path_info_obj:
                continue

            path_info = cast(dict[str, object], path_info_obj)
            tested_type_obj = path_info.get("type")
            if not tested_type_obj or not isinstance(tested_type_obj, str):
                continue

            tested_type = tested_type_obj
            mp_path_obj = mp_dict.get("path", "")
            mp_path = str(mp_path_obj) if mp_path_obj else ""

            # Check if this path is a child of a duplicate path
            is_child_of_duplicate = any(
                mp_path.startswith(dup_prefix + ".") or mp_path.startswith(dup_prefix + "[")
                for dup_prefix in duplicate_prefixes
            )

            if is_child_of_duplicate:
                # Child of duplicate - mark it
                mp_dict["duplicate_of"] = "child_of_duplicate"
                duplicates_marked += 1
                continue

            # Skip primitives - we want hierarchical deduplication, not primitive deduplication
            # Primitives (f32, bool, u32, i32, etc.) should not be deduplicated
            # because they're part of the mutation path hierarchy we're testing
            if tested_type in [
                "f32",
                "f64",
                "bool",
                "u32",
                "i32",
                "u64",
                "i64",
                "u8",
                "i8",
                "u16",
                "i16",
                "usize",
                "isize",
                "String",
                "char",
            ]:
                continue

            if tested_type not in type_representatives:
                # First occurrence - this is the representative
                type_representatives[tested_type] = (parent_type_name, mp_path)
            else:
                # Duplicate - mark it and track prefix for child propagation
                rep_parent, rep_path = type_representatives[tested_type]
                mp_dict["duplicate_of"] = f"{rep_parent}{rep_path}"
                duplicate_prefixes.add(mp_path)
                duplicates_marked += 1

    return data, duplicates_marked


def should_auto_pass(type_data: TypeData) -> bool:
    """
    Determine if a type should be auto-passed based on mutation paths.

    Auto-pass criteria:
    1. No mutation paths at all
    2. Only root path ("") that is not_mutable
    3. Only root path ("") with no examples
    """
    mutation_paths = type_data.get("mutation_paths")

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
        if (not example or len(example) == 0) and (not examples or len(examples) == 0):
            return True

    return False


def initialize_test_metadata(data: AllTypesData) -> tuple[int, int]:
    """
    Initialize test metadata for all types.

    Args:
        data: AllTypesData containing type_guide

    Returns:
        Tuple of (auto_passed_count, untested_count)
    """
    type_guide = data["type_guide"]
    auto_passed = 0
    untested = 0

    for _, type_data in type_guide.items():
        # Determine test status using auto-pass logic
        if should_auto_pass(type_data):
            test_status = "passed"
            auto_passed += 1
        else:
            test_status = "untested"
            untested += 1

        # Initialize metadata
        type_data["batch_number"] = None
        type_data["test_status"] = test_status
        type_data["fail_reason"] = ""

    return auto_passed, untested


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Initialize test metadata and deduplicate mutation paths"
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

    args = parser.parse_args()
    file_path = cast(Path, args.file)
    dry_run = cast(bool, args.dry_run)

    # Read file
    try:
        with open(file_path, encoding="utf-8") as f:
            data: AllTypesData = cast(AllTypesData, json.load(f))
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

    # Early exit if already initialized
    if is_already_initialized(data):
        print("✓ Already initialized (skipping)", file=sys.stderr)
        sys.exit(0)

    print("Initializing test metadata...", file=sys.stderr)

    # Step 1: Deduplicate mutation paths
    data, duplicates_marked = deduplicate_mutation_paths(data)
    if duplicates_marked > 0:
        print(
            f"✓ Deduplicated {duplicates_marked} mutation paths (testing representatives only)",
            file=sys.stderr,
        )

    # Step 2: Validate deduplication
    validation_success, validation_errors = validate_deduplication_data(data)
    if not validation_success:
        print("✗ Deduplication validation FAILED:", file=sys.stderr)
        for error in validation_errors:
            print(f"  {error}", file=sys.stderr)
        sys.exit(1)

    # Step 3: Initialize test metadata
    auto_passed, untested = initialize_test_metadata(data)

    # Write back unless dry run
    if not dry_run:
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
        print(f"✅ Initialized test metadata in {file_path}", file=sys.stderr)
    else:
        print(f"🔍 Dry run - no changes made to {file_path}", file=sys.stderr)

    # Report results
    print("", file=sys.stderr)
    print("Test Metadata Initialization Results:", file=sys.stderr)
    print(f"  Auto-passed: {auto_passed} types", file=sys.stderr)
    print(f"  Untested: {untested} types", file=sys.stderr)
    print(f"  Total: {auto_passed + untested} types", file=sys.stderr)


if __name__ == "__main__":
    main()
