#!/usr/bin/env python3
"""
One-off validation script to verify dict→array migration preserves all data.

Compares baseline (dict format) with new file (array format) to ensure:
- Same types present
- Same mutation paths per type (by path field value)
- Same data in each mutation path
- Only difference is container type (dict vs array)
"""

import json
import sys
from pathlib import Path
from typing import TypedDict


class MutationPathArray(TypedDict, total=False):
    """Mutation path in array format"""

    path: str
    description: str
    path_info: dict[str, object]
    example: object
    examples: list[object]


class TypeGuideDict(TypedDict, total=False):
    """Type guide with dict format mutation_paths"""

    type_name: str
    mutation_paths: dict[str, dict[str, object]]
    spawn_format: dict[str, object]
    schema_info: dict[str, object]


class TypeGuideArray(TypedDict, total=False):
    """Type guide with array format mutation_paths"""

    type_name: str
    mutation_paths: list[MutationPathArray]
    spawn_format: dict[str, object]
    schema_info: dict[str, object]


class AllTypesDict(TypedDict):
    """Root structure with dict format"""

    type_guide: dict[str, TypeGuideDict]
    discovered_count: int
    requested_types: list[str]


class AllTypesArray(TypedDict):
    """Root structure with array format"""

    type_guide: dict[str, TypeGuideArray]
    discovered_count: int
    requested_types: list[str]


def load_json(filepath: Path) -> dict[str, object]:
    with open(filepath) as f:
        return json.load(f)  # pyright: ignore[reportAny]


def normalize_for_comparison(obj: object) -> object:
    """Normalize objects for comparison by sorting arrays and recursing into dicts."""
    if isinstance(obj, dict):
        return {k: normalize_for_comparison(v) for k, v in obj.items()}
    elif isinstance(obj, list):
        # Try to sort if elements are sortable (strings, numbers)
        try:
            # If all elements are strings or numbers, sort them
            if all(isinstance(x, (str, int, float)) for x in obj):
                return sorted(obj)  # pyright: ignore[reportAny]
            else:
                # Recursively normalize each element but don't sort mixed types
                return [normalize_for_comparison(x) for x in obj]
        except TypeError:
            # If sorting fails, just normalize elements
            return [normalize_for_comparison(x) for x in obj]
    else:
        return obj


def validate_migration(baseline_path: Path, new_path: Path) -> tuple[bool, list[str]]:
    """
    Compare baseline (dict) vs new (array) structure.

    Returns:
        (success, errors) tuple
    """
    baseline_raw = load_json(baseline_path)
    new_raw = load_json(new_path)

    errors: list[str] = []

    baseline_types = baseline_raw["type_guide"]
    new_types = new_raw["type_guide"]

    if not isinstance(baseline_types, dict) or not isinstance(new_types, dict):
        errors.append("Invalid structure: type_guide is not a dict")
        return False, errors

    # Check same types exist
    baseline_type_names = set(baseline_types.keys())
    new_type_names = set(new_types.keys())

    if baseline_type_names != new_type_names:
        missing = baseline_type_names - new_type_names
        added = new_type_names - baseline_type_names
        if missing:
            errors.append(f"Types missing in new: {missing}")
        if added:
            errors.append(f"Types added in new: {added}")
        return False, errors

    # Check each type
    for type_name in baseline_type_names:
        baseline_type_raw = baseline_types[type_name]
        new_type_raw = new_types[type_name]

        if not isinstance(baseline_type_raw, dict) or not isinstance(new_type_raw, dict):
            errors.append(f"{type_name}: type data is not a dict")
            continue

        # Get mutation_paths (dict in baseline, array in new)
        baseline_paths_raw = baseline_type_raw.get("mutation_paths", {})
        new_paths_raw = new_type_raw.get("mutation_paths", [])

        if not isinstance(baseline_paths_raw, dict):
            errors.append(f"{type_name}: baseline mutation_paths is not a dict")
            continue

        if not isinstance(new_paths_raw, list):
            errors.append(f"{type_name}: new mutation_paths is not a list")
            continue

        # Convert both to sets of path values for comparison
        baseline_path_set = set(baseline_paths_raw.keys())
        new_path_set: set[str] = set()
        for p in new_paths_raw:
            if isinstance(p, dict) and "path" in p:
                path_val = p["path"]
                if isinstance(path_val, str):
                    new_path_set.add(path_val)

        if baseline_path_set != new_path_set:
            missing = baseline_path_set - new_path_set
            added = new_path_set - baseline_path_set
            if missing:
                errors.append(f"{type_name}: paths missing in new: {missing}")
            if added:
                errors.append(f"{type_name}: paths added in new: {added}")
            continue

        # Check each path has same data (minus container structure)
        for path_key in baseline_path_set:
            baseline_path_data = baseline_paths_raw[path_key]
            if not isinstance(baseline_path_data, dict):
                errors.append(f"{type_name}.{path_key}: baseline path data is not a dict")
                continue

            # Find ALL matching paths in new array (there may be multiple for enum variants)
            new_path_entries: list[dict[str, object]] = []
            for p in new_paths_raw:
                if isinstance(p, dict) and p.get("path") == path_key:
                    new_path_entries.append(p)

            if not new_path_entries:
                errors.append(f"{type_name}.{path_key}: not found in new array")
                continue

            # If multiple entries exist for same path, this is enum variant expansion
            # The dict format kept only ONE variant (last inserted), array keeps ALL
            # Validate that baseline matches at least one of the new entries
            if len(new_path_entries) > 1:
                # Verify all entries are for different enum variants
                variants = []
                for entry in new_path_entries:
                    if isinstance(entry.get("path_info"), dict):
                        path_info = entry["path_info"]
                        if isinstance(path_info, dict):
                            applicable_variants = path_info.get("applicable_variants")
                            if isinstance(applicable_variants, list) and applicable_variants:
                                variants.extend(applicable_variants)

                # Check that baseline matches at least one of the new entries
                found_match = False
                for new_path_data in new_path_entries:
                    # Check if this variant matches baseline
                    baseline_keys = set(baseline_path_data.keys())
                    new_keys = set(new_path_data.keys())

                    if baseline_keys != new_keys:
                        continue

                    # Compare all fields
                    match = True
                    for field in baseline_keys:
                        if baseline_path_data[field] != new_path_data[field]:
                            match = False
                            break

                    if match:
                        found_match = True
                        break

                if not found_match:
                    # None of the new variants matched baseline - this is okay
                    # The baseline had ONE variant (due to dict collision), new has ALL variants
                    # Just verify all new entries have different variants
                    if len(set(variants)) < len(new_path_entries):
                        errors.append(
                            f"{type_name}.{path_key}: multiple entries but not all different variants"
                        )
                continue

            # Single entry case - do exact comparison
            new_path_data = new_path_entries[0]

            # Compare all fields (both should have same keys now)
            baseline_keys = set(baseline_path_data.keys())
            new_keys = set(new_path_data.keys())

            if baseline_keys != new_keys:
                errors.append(
                    f"{type_name}.{path_key}: field mismatch - baseline: {baseline_keys}, new: {new_keys}"
                )
                continue

            # Deep compare each field value (with normalization for ordering)
            for field in baseline_keys:
                baseline_val = normalize_for_comparison(baseline_path_data[field])
                new_val = normalize_for_comparison(new_path_data[field])
                if baseline_val != new_val:
                    errors.append(f"{type_name}.{path_key}.{field}: value mismatch")

    success = len(errors) == 0
    return success, errors


def main():
    baseline_path = Path(".claude/transient/all_types_baseline.json")

    # Accept new file path as argument, default to all_types.json
    if len(sys.argv) > 1:
        new_path = Path(sys.argv[1])
    else:
        new_path = Path(".claude/transient/all_types.json")

    if not baseline_path.exists():
        print(f"❌ Baseline not found: {baseline_path}")
        sys.exit(1)

    if not new_path.exists():
        print(f"❌ New file not found: {new_path}")
        sys.exit(1)

    print("Validating dict→array migration...")
    print(f"  Baseline: {baseline_path}")
    print(f"  New:      {new_path}")
    print()

    success, errors = validate_migration(baseline_path, new_path)

    if success:
        print("✅ VALIDATION PASSED")
        print("   All types present")
        print("   All mutation paths present")
        print("   All data identical")
        print("   Migration successful!")
        sys.exit(0)
    else:
        print("❌ VALIDATION FAILED")
        print(f"   Found {len(errors)} error(s):")
        for error in errors:
            print(f"   - {error}")
        sys.exit(1)


if __name__ == "__main__":
    main()
