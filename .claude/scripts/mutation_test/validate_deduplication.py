#!/usr/bin/env python3
"""
Validate mutation path deduplication in all_types.json.

Returns exit code 0 if validation passes, 1 if validation fails.

Checks:
1. Each deduplicated type has exactly one representative path
2. Representative paths are not marked as duplicate
3. All duplicate paths reference a valid representative or are child duplicates
"""

import json
import sys
from pathlib import Path
from typing import Any, cast

# Add script directory to path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from config import AllTypesData


def validate_deduplication_data(data: AllTypesData) -> tuple[bool, list[str]]:
    """
    Validate deduplication in all_types data.

    Returns: (success: bool, errors: list[str])
    """
    errors: list[str] = []

    # Track representatives for each type
    type_representatives: dict[str, list[tuple[str, str]]] = {}
    # Track all paths for validation
    all_paths: set[str] = set()
    # Track duplicate references
    duplicate_references: dict[str, str] = {}

    # First pass: collect representatives and duplicates
    for parent_type_name, type_data in data['type_guide'].items():
        mutation_paths_obj = type_data.get('mutation_paths')
        if not mutation_paths_obj:
            continue
        mutation_paths = cast(list[dict[str, Any]], mutation_paths_obj)  # pyright: ignore[reportAny]

        for mp in mutation_paths:
            path_info_obj = mp.get('path_info', {})  # pyright: ignore[reportAny]
            path_info = cast(dict[str, Any], path_info_obj)  # pyright: ignore[reportAny]
            tested_type = path_info.get('type')
            if not tested_type:
                continue
            path_obj = mp.get('path', '')  # pyright: ignore[reportAny]
            path = str(path_obj) if path_obj else ''  # pyright: ignore[reportAny]

            # Skip primitives (not deduplicated)
            if tested_type in ['f32', 'f64', 'bool', 'u32', 'i32', 'u64', 'i64', 'u8', 'i8', 'u16', 'i16', 'usize', 'isize', 'String', 'char']:
                continue

            full_path = f"{parent_type_name}{path}"
            all_paths.add(full_path)

            duplicate_of = mp.get('duplicate_of')
            if duplicate_of:
                duplicate_references[full_path] = duplicate_of
            else:
                # This is a representative
                if tested_type not in type_representatives:
                    type_representatives[tested_type] = []
                type_representatives[tested_type].append((parent_type_name, path))

    # Validation 1: Each deduplicated type has exactly one representative
    for tested_type, representatives in type_representatives.items():
        if len(representatives) != 1:
            errors.append(
                f"Type '{tested_type}' has {len(representatives)} representatives, expected 1:\n"
                + "\n".join(f"  - {parent}{path}" for parent, path in representatives)
            )

    # Validation 2: All duplicate references are valid
    for dup_path, ref in duplicate_references.items():
        if ref != "child_of_duplicate":
            # Should reference a valid representative path
            if ref not in all_paths:
                errors.append(
                    f"Duplicate path '{dup_path}' references non-existent representative '{ref}'"
                )

    # Validation 3: Types with duplicates must have representatives
    types_with_duplicates: set[str] = set()
    for parent_type_name, type_data in data['type_guide'].items():
        mutation_paths_obj = type_data.get('mutation_paths')
        if not mutation_paths_obj:
            continue
        mutation_paths = cast(list[dict[str, Any]], mutation_paths_obj)  # pyright: ignore[reportAny]

        for mp in mutation_paths:
            if 'duplicate_of' in mp:  # pyright: ignore[reportAny]
                path_info_obj = mp.get('path_info', {})  # pyright: ignore[reportAny]
                path_info = cast(dict[str, Any], path_info_obj)  # pyright: ignore[reportAny]
                tested_type = path_info.get('type')
                if tested_type:
                    if tested_type not in ['f32', 'f64', 'bool', 'u32', 'i32', 'u64', 'i64', 'u8', 'i8', 'u16', 'i16', 'usize', 'isize', 'String', 'char']:
                        types_with_duplicates.add(tested_type)  # pyright: ignore[reportAny]

    for tested_type in types_with_duplicates:
        if tested_type not in type_representatives:
            errors.append(
                f"Type '{tested_type}' has duplicates but no representative path"
            )

    return len(errors) == 0, errors


def validate_deduplication_file(filepath: str) -> tuple[bool, list[str]]:
    """
    Validate deduplication in all_types.json file.

    Returns: (success: bool, errors: list[str])
    """
    errors: list[str] = []

    try:
        with open(filepath, 'r') as f:
            data: AllTypesData = json.load(f)  # pyright: ignore[reportAny]
    except FileNotFoundError:
        errors.append(f"File not found: {filepath}")
        return False, errors
    except json.JSONDecodeError as e:
        errors.append(f"Invalid JSON: {e}")
        return False, errors

    return validate_deduplication_data(data)


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: python3 validate_deduplication.py <all_types.json>", file=sys.stderr)
        sys.exit(1)

    filepath = sys.argv[1]
    success, errors = validate_deduplication_file(filepath)

    if success:
        print("✓ Deduplication validation passed", file=sys.stderr)
        sys.exit(0)
    else:
        print("✗ Deduplication validation FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
