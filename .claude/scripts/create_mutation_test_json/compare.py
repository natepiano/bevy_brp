#!/usr/bin/env python3
"""
Simple comparison tool for mutation test JSON files.
Outputs raw differences without categorization.
"""

import json
import sys
import os
from pathlib import Path
from datetime import datetime
from typing import TypedDict, cast

# Type definitions for structured data
JsonValue = str | int | float | bool | None | dict[str, "JsonValue"] | list["JsonValue"]


class DifferenceDict(TypedDict):
    path: str
    change_type: str
    baseline: JsonValue
    current: JsonValue
    description: str
    type_name: str
    mutation_path: str | None


class TypeStatsDict(TypedDict):
    baseline_only: list[str]
    current_only: list[str]
    modified: list[str]


class ComparisonResultDict(TypedDict):
    all_changes: list[DifferenceDict]
    type_stats: TypeStatsDict


class MetadataDict(TypedDict):
    generated_at: str
    output_version: str


class SummaryDict(TypedDict):
    total_changes: int
    types_modified: int
    types_added: int
    types_removed: int


class FileStatsDict(TypedDict):
    total_types: int
    spawn_supported: int
    types_with_mutations: int
    total_mutation_paths: int


class OutputDict(TypedDict):
    metadata: MetadataDict
    current_file_stats: FileStatsDict
    comparison_summary: SummaryDict
    all_changes: list[DifferenceDict]


def get_output_path() -> Path:
    """Get the output path using TMPDIR environment variable."""
    tmpdir = os.environ.get("TMPDIR", "/tmp")
    return Path(tmpdir) / "mutation_comparison_full.json"


def load_files(
    baseline_path: str, current_path: str
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    """Load baseline and current JSON files."""
    with open(baseline_path) as f:
        baseline_data: JsonValue = cast(JsonValue, json.load(f))

    with open(current_path) as f:
        current_data: JsonValue = cast(JsonValue, json.load(f))

    # Extract type guides - safely handle the case where data might not be a dict
    if isinstance(baseline_data, dict):
        baseline = baseline_data.get("type_guide", baseline_data)
    else:
        baseline = baseline_data

    if isinstance(current_data, dict):
        current = current_data.get("type_guide", current_data)
    else:
        current = current_data

    # Ensure we return the correct type
    if not isinstance(baseline, dict):
        baseline = {}
    if not isinstance(current, dict):
        current = {}

    return baseline, current


def extract_mutation_path(
    path: str,
    type_name: str = "",
    baseline_data: dict[str, JsonValue] | None = None,
    current_data: dict[str, JsonValue] | None = None,
) -> str | None:
    """Extract the mutation path key from a JSON path if it's within mutation_paths.

    With array format, paths look like: mutation_paths[INDEX].field
    We extract the actual mutation path by looking at the .path field in the comparison data.

    Args:
        path: JSON path string (e.g., "mutation_paths[0].example.player")
        type_name: Name of the type being compared (optional, for array resolution)
        baseline_data: Full baseline type guide data (optional, for array resolution)
        current_data: Full current type guide data (optional, for array resolution)
    """
    if not path.startswith("mutation_paths"):
        return None

    # Handle array format: mutation_paths[INDEX] or mutation_paths[INDEX].field
    if path.startswith("mutation_paths["):
        # Extract the index from path like "mutation_paths[0]" or "mutation_paths[0].field"
        # Format: mutation_paths[INDEX] or mutation_paths[INDEX].rest
        rest = path[len("mutation_paths") :]

        # Extract the index number
        if "[" in rest and "]" in rest:
            index_str = rest[rest.find("[") + 1 : rest.find("]")]
            try:
                index = int(index_str)
            except ValueError:
                return "__ARRAY_ELEMENT__"

            # Try to resolve the actual mutation path from the data
            if type_name and (baseline_data or current_data):
                # Try current data first, then baseline
                for data in [current_data, baseline_data]:
                    if not data or type_name not in data:
                        continue

                    type_data = data[type_name]
                    if not isinstance(type_data, dict):
                        continue

                    mutation_paths = type_data.get("mutation_paths")
                    if not isinstance(mutation_paths, list):
                        continue

                    # Check if index is valid
                    if 0 <= index < len(mutation_paths):
                        path_obj = mutation_paths[index]
                        if isinstance(path_obj, dict) and "path" in path_obj:
                            path_value = path_obj["path"]
                            # The path field is always a string in our schema
                            if isinstance(path_value, str):
                                return path_value

            # Couldn't resolve - return placeholder
            return "__ARRAY_ELEMENT__"

        # Just mutation_paths[INDEX] with no field - this is the whole element
        return "__ARRAY_ELEMENT__"

    # Handle legacy double-dot case for backward compatibility (shouldn't occur with array format)
    if path.startswith("mutation_paths.."):
        # Get what comes after the double dots
        rest_after_dots = path[len("mutation_paths..") :]

        # We need to find where the mutation path ends and the nested field begins
        # Mutation paths can contain dots (like .z_config.far_z_mode)
        # The nested field is what comes after the mutation path

        # Check if it starts with a root-level metadata field (no mutation path)
        root_metadata_fields = [
            "path_info",
            "description",
            "mutability",
            "example",
            "examples",
            "type",
            "type_kind",
            "enum_variant_path",
            "applicable_variants",
            "signature",
        ]

        first_field = (
            rest_after_dots.split(".", 1)[0]
            if "." in rest_after_dots
            else rest_after_dots
        )
        if first_field in root_metadata_fields:
            # This is a root-level metadata change, empty mutation path
            return ""

        # Known nested field patterns that indicate end of mutation path (with leading dot)
        nested_field_indicators = [
            ".example",
            ".examples",
            ".path_info",
            ".description",
            ".mutability",
            ".type",
            ".type_kind",
            ".enum_variant_path",
            ".applicable_variants",
            ".signature",
        ]

        # Find the first occurrence of a nested field indicator
        for indicator in nested_field_indicators:
            if indicator in rest_after_dots:
                # Everything before this indicator is the mutation path
                mutation_path = rest_after_dots.split(indicator)[0]
                return f".{mutation_path}" if mutation_path else ""

        # If no nested field indicators found, the entire rest is the mutation path
        return f".{rest_after_dots}" if rest_after_dots else ""

    # Normal case: mutation_paths.some_key.rest -> ".some_key"
    parts = path.split(
        ".", 3
    )  # Split into max 4 parts: ["mutation_paths", "key", "rest", ...]
    if len(parts) >= 2 and parts[1]:  # Check that the key part exists and is not empty
        mutation_key = parts[1]
        return f".{mutation_key}"

    return ""


def describe_value(val: JsonValue) -> str:
    """Create a concise description of a value."""
    match val:
        case None:
            return "null"
        case bool():
            return str(val).lower()
        case int() | float():
            return str(val)
        case str():
            if len(val) > 50:
                return f'"{val[:47]}..."'
            return f'"{val}"'
        case list():
            return f"array[{len(val)}]"
        case dict():
            keys = list(val.keys())[:3]
            if len(keys) < len(val):
                keys.append("...")
            return f"object{{{','.join(str(k) for k in keys)}}}"


def deep_compare_values(
    path: str,
    baseline_val: JsonValue,
    current_val: JsonValue,
    type_name: str = "",
    baseline_data: dict[str, JsonValue] | None = None,
    current_data: dict[str, JsonValue] | None = None,
) -> list[DifferenceDict]:
    """Recursively compare values and return all differences.

    Args:
        path: JSON path to the current value
        baseline_val: Value from baseline
        current_val: Value from current
        type_name: Name of the type being compared (for mutation path resolution)
        baseline_data: Full baseline type guide data (for mutation path resolution)
        current_data: Full current type guide data (for mutation path resolution)
    """
    differences: list[DifferenceDict] = []

    # Check if both are None
    if baseline_val is None and current_val is None:
        return []

    # Check if one is None
    if baseline_val is None:
        differences.append(
            DifferenceDict(
                path=path,
                change_type="added",
                baseline=None,
                current=current_val,
                description=f"Added: {describe_value(current_val)}",
                type_name="",
                mutation_path=extract_mutation_path(path, type_name, baseline_data, current_data),
            )
        )
        return differences

    if current_val is None:
        differences.append(
            DifferenceDict(
                path=path,
                change_type="removed",
                baseline=baseline_val,
                current=None,
                description=f"Removed: {describe_value(baseline_val)}",
                type_name="",
                mutation_path=extract_mutation_path(path, type_name, baseline_data, current_data),
            )
        )
        return differences

    # Check if types differ
    if type(baseline_val) != type(current_val):
        differences.append(
            DifferenceDict(
                path=path,
                change_type="type_changed",
                baseline=baseline_val,
                current=current_val,
                description=f"Type changed: {type(baseline_val).__name__} → {type(current_val).__name__}",
                type_name="",
                mutation_path=extract_mutation_path(path, type_name, baseline_data, current_data),
            )
        )
        return differences

    # Compare based on type
    if isinstance(baseline_val, dict) and isinstance(current_val, dict):
        all_keys = set(baseline_val.keys()) | set(current_val.keys())

        # Test metadata fields to ignore during comparison
        test_metadata_fields = {"batch_number", "test_status", "fail_reason"}

        for key in sorted(all_keys):
            new_path = f"{path}.{key}" if path else str(key)

            # Skip test metadata fields at type level (when path is empty string)
            # These fields are managed by the mutation test system, not part of BRP data
            if not path and key in test_metadata_fields:
                continue

            # Check if key exists in each dict to distinguish None value from missing key
            base_has_key = key in baseline_val
            curr_has_key = key in current_val

            if base_has_key and curr_has_key:
                # Both have the key, compare values (even if both are None)
                differences.extend(
                    deep_compare_values(
                        new_path,
                        baseline_val[key],
                        current_val[key],
                        type_name,
                        baseline_data,
                        current_data,
                    )
                )
            elif base_has_key and not curr_has_key:
                # Key exists in baseline but not current (field removed)
                # Directly create the difference entry to avoid (None, None) comparison
                differences.append(
                    DifferenceDict(
                        path=new_path,
                        change_type="removed",
                        baseline=baseline_val[key],
                        current=None,
                        description=f"Removed: {describe_value(baseline_val[key])}",
                        type_name="",
                        mutation_path=extract_mutation_path(new_path, type_name, baseline_data, current_data),
                    )
                )
            elif not base_has_key and curr_has_key:
                # Key exists in current but not baseline (field added)
                # Directly create the difference entry to avoid (None, None) comparison
                differences.append(
                    DifferenceDict(
                        path=new_path,
                        change_type="added",
                        baseline=None,
                        current=current_val[key],
                        description=f"Added: {describe_value(current_val[key])}",
                        type_name="",
                        mutation_path=extract_mutation_path(new_path, type_name, baseline_data, current_data),
                    )
                )

    elif isinstance(baseline_val, list) and isinstance(current_val, list):
        # Check if arrays have different lengths
        if len(baseline_val) != len(current_val):
            # Different lengths - always report as change
            max_len = max(len(baseline_val), len(current_val))
            for i in range(max_len):
                new_path = f"{path}[{i}]"
                base_item = baseline_val[i] if i < len(baseline_val) else None
                curr_item = current_val[i] if i < len(current_val) else None

                if base_item is None and curr_item is not None:
                    differences.append(
                        DifferenceDict(
                            path=new_path,
                            change_type="added",
                            baseline=None,
                            current=curr_item,
                            description=f"Added element at index {i}",
                            type_name="",
                            mutation_path=extract_mutation_path(new_path, type_name, baseline_data, current_data),
                        )
                    )
                elif base_item is not None and curr_item is None:
                    differences.append(
                        DifferenceDict(
                            path=new_path,
                            change_type="removed",
                            baseline=base_item,
                            current=None,
                            description=f"Removed element at index {i}",
                            type_name="",
                            mutation_path=extract_mutation_path(new_path, type_name, baseline_data, current_data),
                        )
                    )
                else:
                    differences.extend(
                        deep_compare_values(new_path, base_item, curr_item, type_name, baseline_data, current_data)
                    )
        else:
            # Same length - check if arrays contain primitive values that can be compared as sets
            def is_primitive(val: JsonValue) -> bool:
                return isinstance(val, (str, int, float, bool, type(None)))

            all_primitives = all(is_primitive(item) for item in baseline_val) and all(
                is_primitive(item) for item in current_val
            )

            if all_primitives and set(baseline_val) == set(current_val):  # type: ignore[arg-type]
                # Same elements, different order - ignore this difference
                pass
            else:
                # Either not all primitives, or different elements - compare by index
                for i in range(len(baseline_val)):
                    new_path = f"{path}[{i}]"
                    differences.extend(
                        deep_compare_values(new_path, baseline_val[i], current_val[i], type_name, baseline_data, current_data)
                    )

    elif baseline_val != current_val:
        # Primitive values that differ
        differences.append(
            DifferenceDict(
                path=path,
                change_type="value_changed",
                baseline=baseline_val,
                current=current_val,
                description=f"Value changed: {describe_value(baseline_val)} → {describe_value(current_val)}",
                type_name="",
                mutation_path=extract_mutation_path(path, type_name, baseline_data, current_data),
            )
        )

    return differences


def compare_types(
    baseline: dict[str, JsonValue], current: dict[str, JsonValue]
) -> ComparisonResultDict:
    """Compare all types and collect ALL differences."""
    all_changes: list[DifferenceDict] = []
    type_stats: TypeStatsDict = {
        "baseline_only": [],
        "current_only": [],
        "modified": [],
    }

    all_types = set(baseline.keys()) | set(current.keys())

    for type_name in sorted(all_types):
        if type_name not in current:
            type_stats["baseline_only"].append(type_name)
            continue
        if type_name not in baseline:
            type_stats["current_only"].append(type_name)
            continue

        # Compare the type entries
        baseline_entry = baseline[type_name]
        current_entry = current[type_name]

        type_differences = deep_compare_values(
            "", baseline_entry, current_entry, type_name, baseline, current
        )

        if type_differences:
            type_stats["modified"].append(type_name)
            for diff in type_differences:
                diff["type_name"] = type_name
                all_changes.append(diff)

    return ComparisonResultDict(all_changes=all_changes, type_stats=type_stats)


def calculate_file_statistics(data: dict[str, JsonValue]) -> FileStatsDict:
    """Calculate statistics for a mutation test file."""
    total_types = len(data)
    spawn_supported = sum(
        1
        for t in data.values()
        if isinstance(t, dict) and t.get("spawn_format") is not None
    )
    types_with_mutations = sum(
        1 for t in data.values() if isinstance(t, dict) and t.get("mutation_paths")
    )

    # Count mutation paths - handle both dict (old) and list (new) formats
    total_paths = 0
    for t in data.values():
        if isinstance(t, dict):
            mutation_paths = t.get("mutation_paths")
            if isinstance(mutation_paths, dict):
                # Old dict format
                total_paths += len(mutation_paths)
            elif isinstance(mutation_paths, list):
                # New array format
                total_paths += len(mutation_paths)

    return FileStatsDict(
        total_types=total_types,
        spawn_supported=spawn_supported,
        types_with_mutations=types_with_mutations,
        total_mutation_paths=total_paths,
    )


def main() -> None:
    if len(sys.argv) != 3:
        print("Usage: compare.py <baseline.json> <current.json>")
        sys.exit(1)

    baseline_path = sys.argv[1]
    current_path = sys.argv[2]

    # Load files
    baseline, current = load_files(baseline_path, current_path)

    # Calculate statistics for current file
    current_stats = calculate_file_statistics(current)

    # Compare types and get ALL changes
    comparison_result = compare_types(baseline, current)
    all_changes = comparison_result["all_changes"]
    type_stats = comparison_result["type_stats"]

    # Extract stats safely
    modified_types = type_stats["modified"]
    added_types = type_stats["current_only"]
    removed_types = type_stats["baseline_only"]

    print("🔍 MUTATION TEST COMPARISON COMPLETE")
    print("=" * 60)
    print()
    print("Current File Statistics:")
    print(f"  Types registered in Bevy: {current_stats['total_types']}")
    print(f"  Spawn-supported types: {current_stats['spawn_supported']}")
    print(f"  Types with mutations: {current_stats['types_with_mutations']}")
    print(f"  Total mutation paths: {current_stats['total_mutation_paths']}")
    print()
    print("Comparison Results:")
    print("=" * 60)
    print(f"Total changes: {len(all_changes)}")
    print(f"Types modified: {len(modified_types)}")
    print(f"Types added: {len(added_types)}")
    print(f"Types removed: {len(removed_types)}")
    print()

    if len(all_changes) > 0:
        print(f"⚠️  CHANGES DETECTED: {len(all_changes)}")
        print("   Use comparison_review to examine them")
    else:
        print("✅ No changes detected!")

    print()
    print("Detailed results saved to comparison file")

    # Save results
    output_path = get_output_path()
    output_result: OutputDict = {
        "metadata": {
            "generated_at": datetime.now().isoformat(),
            "output_version": "3.0.0",
        },
        "current_file_stats": current_stats,
        "comparison_summary": {
            "total_changes": len(all_changes),
            "types_modified": len(modified_types),
            "types_added": len(added_types),
            "types_removed": len(removed_types),
        },
        "all_changes": all_changes,
    }

    with open(output_path, "w") as f:
        json.dump(output_result, f, indent=2)


if __name__ == "__main__":
    main()
