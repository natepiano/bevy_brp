#!/usr/bin/env python3
"""
Deep comparison tool for mutation test JSON files.
Detects and categorizes structural differences between baseline and current files.
"""
# pylint: disable=too-many-locals,too-many-branches,too-many-statements,too-many-nested-blocks
# pylint: disable=too-many-lines,too-complex,too-many-arguments,line-too-long

import json
import sys
from typing import Any, TypedDict, cast
from dataclasses import dataclass
from enum import Enum
from collections import defaultdict


# Type definitions for the JSON structure
JsonValue = str | int | float | bool | None | dict[str, "JsonValue"] | list["JsonValue"]

# Root JSON file structures
class RootJsonFile(TypedDict, total=False):
    """Root structure of mutation test JSON files"""
    type_guide: dict[str, "TypeData"] | list["TypeData"]
    result: "ResultWrapper"

class ResultWrapper(TypedDict, total=False):
    """Wrapper for nested result structure"""
    type_guide: dict[str, "TypeData"] | list["TypeData"]

class ExcludedType(TypedDict):
    """Structure for excluded types JSON"""
    type_name: str

class ExclusionFile(TypedDict):
    """Structure for the exclusion file"""
    excluded_types: list[ExcludedType]
class PathInfo(TypedDict):
    mutation_status: str
    path_kind: str
    type: str
    type_kind: str


class MutationPathData(TypedDict):
    description: str
    example: Any  # pyright: ignore[reportExplicitAny] - arbitrary JSON value
    path_info: PathInfo


class TypeData(TypedDict, total=False):
    agent_guidance: str | None
    batch_number: int | None
    fail_reason: str | None
    has_deserialize: bool
    has_serialize: bool
    in_registry: bool
    mutation_paths: dict[str, MutationPathData]
    schema_info: dict[str, Any] | None  # pyright: ignore[reportExplicitAny] - JSON schema
    spawn_format: Any | None  # pyright: ignore[reportExplicitAny] - arbitrary JSON structure
    supported_operations: list[str]
    test_status: str | None
    type: str
    type_name: str


class TypeGuideData(TypedDict):
    discovered_count: int
    requested_types: list[str]
    summary: dict[str, Any]  # pyright: ignore[reportExplicitAny] - summary data
    type_guide: dict[str, TypeData]


class ChangePattern(Enum):
    """Known patterns of changes we can identify"""

    ENUM_REPRESENTATION = "enum_representation"  # string → enum schema
    VEC_FORMAT = "vec_format"  # object {x,y,z} → array [x,y,z]
    VALUE_CHANGE = "value_change"  # same structure, different value
    FIELD_ADDED = "field_added"
    FIELD_REMOVED = "field_removed"
    TYPE_CHANGE = "type_change"  # different type (string → number, etc)
    UNKNOWN = "unknown"


@dataclass
class Difference:
    """Represents a single difference found"""

    type_name: str
    path: str
    pattern: ChangePattern
    before_structure: str
    after_structure: str
    before_sample: JsonValue
    after_sample: JsonValue


def describe_structure(val: JsonValue) -> str:
    """Describe the structure/type of a value"""
    if val is None:
        return "null"
    elif isinstance(val, bool):
        return "bool"
    elif isinstance(val, (int, float)):
        return "number"
    elif isinstance(val, str):
        return "string"
    elif isinstance(val, list):
        if not val:
            return "empty_array"
        first = val[0]
        if isinstance(first, dict) and "variants" in first:
            return "enum_schema"
        elif (
            isinstance(first, list)
            and first
            and isinstance(first[0], dict)
            and "variants" in first[0]
        ):
            return "enum_schema_array"
        else:
            return f"array[{describe_structure(first)}]"
    elif isinstance(val, dict):  # pyright: ignore[reportUnnecessaryIsInstance]
        if "variants" in val:
            return "enum_schema"
        elif all(k in val for k in ["x", "y", "z"]):
            return "vec3_object"
        elif all(k in val for k in ["x", "y", "z", "w"]):
            return "quat_object"
        else:
            return "object"


def detect_pattern(
    before: JsonValue, after: JsonValue, _path: str
) -> ChangePattern:
    """Detect what kind of change pattern this is"""
    before_struct = describe_structure(before)
    after_struct = describe_structure(after)

    # Field removal: before had value, after is null (likely missing field)
    if before is not None and after is None and before_struct != "null":
        return ChangePattern.FIELD_REMOVED

    # Field addition: before was null, after has value
    if before is None and after is not None and after_struct != "null":
        return ChangePattern.FIELD_ADDED

    # Enum representation change
    if before_struct == "string" and "enum_schema" in after_struct:
        return ChangePattern.ENUM_REPRESENTATION

    # Vector format change
    if before_struct in [
        "vec3_object",
        "quat_object",
    ] and after_struct.startswith("array"):
        return ChangePattern.VEC_FORMAT

    # Type change
    if before_struct != after_struct:
        return ChangePattern.TYPE_CHANGE

    # Value change (same structure)
    if before != after:
        return ChangePattern.VALUE_CHANGE

    return ChangePattern.UNKNOWN


def find_differences(
    baseline_type: TypeData, current_type: TypeData, type_name: str
) -> list[Difference]:
    """Find all differences in a single type"""
    differences: list[Difference] = []

    # Define which fields at each level should have their contents compared as values, not structure
    VALUE_FIELDS = {
        "example",
        "examples",
        "spawn_format",
        "path_info",
        "schema_info",
    }

    def should_compare_as_value(_path: str, key: str) -> bool:
        """Check if this key's value should be compared as a whole value rather than recursively"""
        # These fields contain data values, not structural schema
        return key in VALUE_FIELDS

    def recurse(b_val: JsonValue, c_val: JsonValue, path: str) -> None:
        # CRITICAL FIX: Don't flag identical values as changes
        if b_val == c_val:
            return

        if type(b_val) is not type(c_val):  # type: ignore[comparison-overlap]
            # Structural difference
            pattern = detect_pattern(b_val, c_val, path)
            # Always capture the actual values for comparison
            differences.append(
                Difference(
                    type_name=type_name,
                    path=path,
                    pattern=pattern,
                    before_structure=describe_structure(b_val),
                    after_structure=describe_structure(c_val),
                    before_sample=b_val,
                    after_sample=c_val,
                )
            )
        elif isinstance(b_val, dict) and isinstance(c_val, dict):
            all_keys = set(b_val.keys()) | set(c_val.keys())
            for key in all_keys:
                new_path = f"{path}.{key}" if path else str(key)

                if key not in b_val:
                    differences.append(
                        Difference(
                            type_name=type_name,
                            path=new_path,  # type: ignore[arg-type]
                            pattern=ChangePattern.FIELD_ADDED,
                            before_structure="missing",
                            after_structure=describe_structure(c_val[key]),
                            before_sample=None,
                            after_sample=(
                                c_val[key]
                                if not isinstance(c_val[key], (dict, list))
                                else "..."
                            ),
                        )
                    )
                elif key not in c_val:
                    differences.append(
                        Difference(
                            type_name=type_name,
                            path=new_path,  # type: ignore[arg-type]
                            pattern=ChangePattern.FIELD_REMOVED,
                            before_structure=describe_structure(b_val[key]),
                            after_structure="missing",
                            before_sample=(
                                b_val[key]
                                if not isinstance(b_val[key], (dict, list))
                                else "..."
                            ),
                            after_sample=None,
                        )
                    )
                else:
                    # Check if this field should be compared as a whole value
                    if should_compare_as_value(path, str(key)):
                        # Compare the entire value, don't recurse into it
                        if b_val[key] != c_val[key]:
                            pattern = detect_pattern(
                                b_val[key], c_val[key], new_path
                            )
                            differences.append(
                                Difference(
                                    type_name=type_name,
                                    path=new_path,  # type: ignore[arg-type]
                                    pattern=pattern,
                                    before_structure=describe_structure(
                                        b_val[key]
                                    ),
                                    after_structure=describe_structure(
                                        c_val[key]
                                    ),
                                    before_sample=b_val[key],
                                    after_sample=c_val[key],
                                )
                            )
                    else:
                        # Recurse into structural fields
                        recurse(b_val[key], c_val[key], new_path)  # type: ignore[arg-type]
        elif isinstance(b_val, list) and isinstance(c_val, list):
            for i in range(min(len(b_val), len(c_val))):
                recurse(b_val[i], c_val[i], f"{path}[{i}]")
            if len(b_val) != len(c_val):
                pattern = detect_pattern(b_val, c_val, path)
                differences.append(
                    Difference(
                        type_name=type_name,
                        path=f"{path}.length",
                        pattern=pattern,
                        before_structure=f"array[{len(b_val)}]",
                        after_structure=f"array[{len(c_val)}]",
                        before_sample=len(b_val),
                        after_sample=len(c_val),
                    )
                )
        elif b_val != c_val:
            # Simple value difference
            pattern = detect_pattern(b_val, c_val, path)
            differences.append(
                Difference(
                    type_name=type_name,
                    path=path,
                    pattern=pattern,
                    before_structure=describe_structure(b_val),
                    after_structure=describe_structure(c_val),
                    before_sample=b_val,
                    after_sample=c_val,
                )
            )

    recurse(cast(JsonValue, cast(object, baseline_type)), cast(JsonValue, cast(object, current_type)), "")
    return differences


def extract_type_guide(data: RootJsonFile) -> list[TypeData]:
    """Extract type_guide array from either format"""
    if "type_guide" in data:
        type_guide = data["type_guide"]
        # Handle both object format (keys are type names) and array format
        if isinstance(type_guide, dict):
            # Convert object format to array format, adding type_name field
            result: list[TypeData] = []
            for type_name, guide in type_guide.items():
                type_entry = dict(guide)  # Create mutable copy
                type_entry["type_name"] = type_name
                result.append(cast(TypeData, cast(object, type_entry)))
            return result
        else:
            return type_guide
    elif "result" in data and "type_guide" in data["result"]:
        type_guide_nested = data["result"]["type_guide"]
        # Handle both object format (keys are type names) and array format
        if isinstance(type_guide_nested, dict):
            # Convert object format to array format, adding type_name field
            result_nested: list[TypeData] = []
            for type_name, guide in type_guide_nested.items():
                type_entry = dict(guide)  # Create mutable copy
                type_entry["type_name"] = type_name
                result_nested.append(cast(TypeData, cast(object, type_entry)))
            return result_nested
        else:
            return type_guide_nested
    else:
        # If data is a dict with type names as keys, return the values
        return []


def calculate_metadata(type_guide: list[TypeData]) -> dict[str, int]:
    """Calculate metadata statistics for a type guide"""
    total_types = len(type_guide)

    spawn_supported = len(
        [t for t in type_guide if "spawn_format" in t]
    )

    with_mutations = len(
        [
            t
            for t in type_guide
            if t.get("mutation_paths")
            and t.get("mutation_paths") != {}
        ]
    )

    total_paths = sum(
        [
            (
                len(t.get("mutation_paths", {}).keys())
                if isinstance(t.get("mutation_paths"), dict)
                else 0
            )
            for t in type_guide
        ]
    )

    return {
        "total_types": total_types,
        "spawn_supported": spawn_supported,
        "with_mutations": with_mutations,
        "total_paths": total_paths,
    }


def get_excluded_types() -> list[str]:
    """Get list of excluded types from the exclusion file"""
    exclusion_file = ".claude/scripts/mutation_test_excluded_types.json"
    excluded = []

    try:
        with open(exclusion_file, "r") as f:
            data = cast(ExclusionFile, json.load(f))
            excluded = [
                item["type_name"] for item in data.get("excluded_types", [])
            ]
    except (FileNotFoundError, json.JSONDecodeError):
        # Fall back to old text file format if JSON doesn't exist or is invalid
        old_file = ".claude/scripts/mutation_test_excluded_types.txt"
        try:
            with open(old_file, "r") as f:
                for line in f:
                    line = line.strip()
                    # Skip comments and empty lines
                    if line and not line.startswith("#"):
                        excluded.append(line)
        except FileNotFoundError:
            # If neither file exists, return empty list
            pass

    return excluded


def main(baseline_file: str, current_file: str) -> int:
    """Main comparison logic.
    """

    print("🔍 STRUCTURED MUTATION TEST COMPARISON (Full Schema)")
    print("=" * 60)
    print()

    # Load files
    try:
        with open(baseline_file) as f:
            baseline = cast(RootJsonFile, json.load(f))
    except FileNotFoundError:
        print(f"❌ Baseline file not found: {baseline_file}")
        return 1
    except json.JSONDecodeError:
        print(f"❌ Invalid JSON in baseline file: {baseline_file}")
        return 1

    try:
        with open(current_file) as f:
            current = cast(RootJsonFile, json.load(f))
    except FileNotFoundError:
        print(f"❌ Current file not found: {current_file}")
        return 1
    except json.JSONDecodeError:
        print(f"❌ Invalid JSON in current file: {current_file}")
        return 1

    # Binary identity check
    print("📊 IDENTITY CHECK")
    with open(baseline_file, "rb") as f1, open(current_file, "rb") as f2:
        if f1.read() == f2.read():
            print("✅ FILES ARE IDENTICAL")
            print(
                "   └─ Baseline and current files are byte-for-byte identical"
            )
            print()

            # Show current stats even for identical files
            current_tg = extract_type_guide(current)
            current_meta = calculate_metadata(current_tg)

            # Get excluded types
            excluded_types = get_excluded_types()

            print("📈 CURRENT FILE STATISTICS")
            print(f"   Total Types: {current_meta['total_types']}")
            print(f"   Spawn-Supported: {current_meta['spawn_supported']}")
            print(f"   Types with Mutations: {current_meta['with_mutations']}")
            print(f"   Total Mutation Paths: {current_meta['total_paths']}")
            print(
                f"   Excluded Types: {', '.join(excluded_types) if excluded_types else 'None'}"
            )
            print()
            print("📋 SUMMARY")
            print("   └─ No changes detected - safe for promotion")
            return 0

    print("⚠️  FILES DIFFER - ANALYZING CHANGES")
    print("   └─ Found differences requiring review")
    print()

    # Extract type_guide arrays
    baseline_tg = extract_type_guide(baseline)
    current_tg = extract_type_guide(current)

    # Metadata comparison
    baseline_meta = calculate_metadata(baseline_tg)
    current_meta = calculate_metadata(current_tg)

    # Get excluded types
    excluded_types = get_excluded_types()

    print("📈 METADATA COMPARISON")
    for key in [
        "total_types",
        "spawn_supported",
        "with_mutations",
        "total_paths",
    ]:
        baseline_val = baseline_meta[key]
        current_val = current_meta[key]
        label = (
            key.replace("_", " ").title().replace("Total ", "Total Mutation ")
        )

        if baseline_val == current_val:
            print(f"   {label}: {baseline_val} → {current_val} (no change)")
        else:
            diff = current_val - baseline_val
            print(
                f"   {label}: {baseline_val} → {current_val} ({current_val} - {baseline_val} = {diff:+d})"
            )

    print(
        f"   Excluded Types: {', '.join(excluded_types) if excluded_types else 'None'}"
    )
    print()

    # Type-level changes analysis
    print("🔍 TYPE-LEVEL CHANGES")

    baseline_types = set(t.get("type_name", "") for t in baseline_tg if t.get("type_name"))
    current_types = set(t.get("type_name", "") for t in current_tg if t.get("type_name"))

    new_types = current_types - baseline_types
    removed_types = baseline_types - current_types
    common_types = baseline_types & current_types

    # Create lookups
    baseline_dict = {t.get("type_name", f"type_{i}"): t for i, t in enumerate(baseline_tg) if t.get("type_name")}
    current_dict = {t.get("type_name", f"type_{i}"): t for i, t in enumerate(current_tg) if t.get("type_name")}

    # Check for changes in common types
    modified_types: list[str] = []
    for type_name in common_types:
        if baseline_dict[type_name] != current_dict[type_name]:
            modified_types.append(type_name)

    print(f"   ├─ Modified Types: {len(modified_types)}")
    if modified_types:
        for type_name in modified_types[:5]:
            print(f"   │  ├─ {type_name}: mutation paths changed")
        if len(modified_types) > 5:
            print(f"   │  └─ ... and {len(modified_types) - 5} more")

    print(f"   ├─ New Types: {len(new_types)}")
    if new_types and len(new_types) <= 5:
        for type_name in sorted(new_types):
            print(f"   │  ├─ {type_name}")
    elif len(new_types) > 5:
        for type_name in sorted(list(new_types)[:5]):
            print(f"   │  ├─ {type_name}")
        print(f"   │  └─ ... and {len(new_types) - 5} more")

    print(f"   └─ Removed Types: {len(removed_types)}")
    if removed_types and len(removed_types) <= 5:
        for type_name in sorted(removed_types):
            print(f"       ├─ {type_name}")
    elif len(removed_types) > 5:
        for type_name in sorted(list(removed_types)[:5]):
            print(f"       ├─ {type_name}")
        print(f"       └─ ... and {len(removed_types) - 5} more")
    print()

    # Find all structural differences in modified types
    all_differences: list[Difference] = []
    for type_name in modified_types:
        diffs = find_differences(
            baseline_dict[type_name],
            current_dict[type_name],
            type_name
        )
        all_differences.extend(diffs)

    if not all_differences:
        print("✅ NO STRUCTURAL DIFFERENCES FOUND")
        return 0

    # Categorize differences
    by_pattern: dict[ChangePattern, list[Difference]] = {}
    for diff in all_differences:
        if diff.pattern not in by_pattern:
            by_pattern[diff.pattern] = []
        by_pattern[diff.pattern].append(diff)

    # Report findings
    print("🔍 STRUCTURAL CHANGES DETECTED")
    print("=" * 60)
    print()

    # Show actual differences with before/after samples
    for pattern, diffs in by_pattern.items():
        pattern_label = (
            "IDENTIFIED PATTERN"
            if pattern != ChangePattern.UNKNOWN
            else "UNRECOGNIZED PATTERN"
        )
        print(f"📌 {pattern_label}: {pattern.value.replace('_', ' ').upper()}")
        print("-" * 40)

        affected_types = list(set(d.type_name for d in diffs))
        print(f"Types affected: {len(affected_types)}")
        print(f"Total changes: {len(diffs)}")

        # Special handling for field removals/additions - show which fields changed
        if pattern == ChangePattern.FIELD_REMOVED:
            # Group by field name to show what's being removed
            field_changes_removed: dict[str, list[Difference]] = {}
            for diff in diffs:
                field_name = diff.path.split(".")[
                    -1
                ]  # Get the last part of the path as field name
                if field_name not in field_changes_removed:
                    field_changes_removed[field_name] = []
                field_changes_removed[field_name].append(diff)

            print()
            print("Fields removed breakdown:")
            for field_name, field_diffs in field_changes_removed.items():
                affected_types_for_field = len(
                    set(d.type_name for d in field_diffs)
                )
                print(
                    f"  • '{field_name}' field: {len(field_diffs)} removal(s) across {affected_types_for_field} type(s)"
                )

        elif pattern == ChangePattern.FIELD_ADDED:
            # Group by field name to show what's being added
            field_changes_added: dict[str, list[Difference]] = {}
            for diff in diffs:
                field_name = diff.path.split(".")[
                    -1
                ]  # Get the last part of the path as field name
                if field_name not in field_changes_added:
                    field_changes_added[field_name] = []
                field_changes_added[field_name].append(diff)

            print()
            print("Fields added breakdown:")
            for field_name, field_diffs in field_changes_added.items():
                affected_types_for_field = len(
                    set(d.type_name for d in field_diffs)
                )
                print(
                    f"  • '{field_name}' field: {len(field_diffs)} addition(s) across {affected_types_for_field} type(s)"
                )

        print()

        # Show up to 3 examples with actual data
        for i, diff in enumerate(diffs[:3]):
            print(f"Example {i+1}:")
            print(f"  Type: {diff.type_name}")
            print(f"  Path: {diff.path}")
            print(
                f"  Structure change: {diff.before_structure} → {diff.after_structure}"
            )

            # Show actual values
            if isinstance(
                diff.before_sample, (str, int, float, bool, type(None))
            ):
                print(f"  Before value: {json.dumps(diff.before_sample)}")
            else:
                before_str = json.dumps(diff.before_sample, indent=2)
                if len(before_str) > 300:
                    before_str = before_str[:300] + "..."
                print(
                    f"  Before value:\n    {before_str.replace(chr(10), chr(10) + '    ')}"
                )

            if isinstance(
                diff.after_sample, (str, int, float, bool, type(None))
            ):
                print(f"  After value: {json.dumps(diff.after_sample)}")
            else:
                after_str = json.dumps(diff.after_sample, indent=2)
                if len(after_str) > 300:
                    after_str = after_str[:300] + "..."
                print(
                    f"  After value:\n    {after_str.replace(chr(10), chr(10) + '    ')}"
                )
            print()

        if len(diffs) > 3:
            print(f"... and {len(diffs)-3} more changes with this pattern")
        print()

    # Unknown patterns
    if ChangePattern.UNKNOWN in by_pattern:
        unknown_diffs = by_pattern[ChangePattern.UNKNOWN]
        print(f"\n⚠️  UNKNOWN PATTERNS ({len(unknown_diffs)} change(s)):")

        for diff in unknown_diffs[:5]:
            print(f"\n  • {diff.type_name}")
            print(f"    Path: {diff.path}")
            print(f"    Before: {diff.before_structure}")
            if diff.before_sample != "...":
                print(f"      Sample: {json.dumps(diff.before_sample)[:60]}")
            print(f"    After: {diff.after_structure}")
            if diff.after_sample != "...":
                print(f"      Sample: {json.dumps(diff.after_sample)[:60]}")
            print("    Pattern: UNKNOWN - needs investigation")

        if len(unknown_diffs) > 5:
            print(f"\n  ... and {len(unknown_diffs)-5} more unknown changes")

    # Summary
    print("\n" + "=" * 60)
    print("📊 SUMMARY:")
    total_types_affected = len(set(d.type_name for d in all_differences))
    print(f"  Total types affected: {total_types_affected}")

    for pattern, diffs in by_pattern.items():
        affected = len(set(d.type_name for d in diffs))
        print(f"  {pattern.value}: {affected} type(s), {len(diffs)} change(s)")

    # Action guidance
    print("\n📋 DETECTED CHANGES:")
    if ChangePattern.UNKNOWN in by_pattern:
        print("  ⚠️  Contains unrecognized structural patterns")

    for pattern in by_pattern:
        if pattern == ChangePattern.ENUM_REPRESENTATION:
            print("  • Enum representation changes (values in collections)")
        elif pattern == ChangePattern.VEC_FORMAT:
            print("  • Vector/quaternion format changes")
        elif pattern == ChangePattern.VALUE_CHANGE:
            print("  • Value changes (same structure, different values)")
        elif pattern == ChangePattern.TYPE_CHANGE:
            print("  • Type changes (different data types)")
        elif pattern == ChangePattern.FIELD_ADDED:
            print("  • New fields added")
        elif pattern == ChangePattern.FIELD_REMOVED:
            print("  • Fields removed")

    print("\n  Actions: investigate | promote | skip")

    return 0


if __name__ == "__main__":
    if len(sys.argv) not in [3, 4]:
        print(
            f"Usage: {sys.argv[0]} <baseline_file> <current_file> [--detailed]"
        )
        sys.exit(1)

    baseline_file = sys.argv[1]
    current_file = sys.argv[2]

    # Check for --detailed flag
    if len(sys.argv) == 4 and sys.argv[3] == "--detailed":
        # Generate detailed JSON output
        import os
        from collections import defaultdict

        detailed_output_file = os.path.join(
            os.environ.get("TMPDIR", "/tmp"),
            "mutation_comparison_details.json",
        )

        # Load files
        with open(baseline_file) as f:
            baseline = cast(RootJsonFile, json.load(f))
        with open(current_file) as f:
            current = cast(RootJsonFile, json.load(f))

        # Extract type guides
        baseline_tg = extract_type_guide(baseline)
        current_tg = extract_type_guide(current)

        # Create lookups
        baseline_dict: dict[str, TypeData] = {
            t.get("type_name", f"type_{i}"): t
            for i, t in enumerate(baseline_tg)
        }
        current_dict: dict[str, TypeData] = {
            t.get("type_name", f"type_{i}"): t
            for i, t in enumerate(current_tg)
        }

        # Find changes focusing on the unexpected patterns
        all_type_names = set(baseline_dict.keys()) | set(current_dict.keys())
        detailed_changes: list[dict[str, str]] = []

        for type_name in all_type_names:
            b_type_data = baseline_dict.get(type_name)
            c_type_data = current_dict.get(type_name)

            # Skip if neither type exists
            if not b_type_data and not c_type_data:
                continue

            # Use empty TypeData for missing types
            b_type: TypeData = b_type_data or cast(TypeData, cast(object, {}))
            c_type: TypeData = c_type_data or cast(TypeData, cast(object, {}))

            # Check mutation_paths for removed/added example fields
            b_mutations: dict[str, MutationPathData] = b_type.get("mutation_paths", {})
            c_mutations: dict[str, MutationPathData] = c_type.get("mutation_paths", {})

            for path, b_data in b_mutations.items():
                c_data = c_mutations.get(path, cast(MutationPathData, cast(object, {})))
                if "examples" in b_data and "examples" not in c_data:
                    detailed_changes.append(
                        {
                            "pattern": "FIELD_REMOVED examples",
                            "type": type_name,
                            "mutation_path": path,
                        }
                    )
                if "example" in b_data and "example" not in c_data:
                    detailed_changes.append(
                        {
                            "pattern": "FIELD_REMOVED example",
                            "type": type_name,
                            "mutation_path": path,
                        }
                    )

            for path, c_data in c_mutations.items():
                b_data = b_mutations.get(path, cast(MutationPathData, cast(object, {})))
                if "example" in c_data and "example" not in b_data:
                    detailed_changes.append(
                        {
                            "pattern": "FIELD_ADDED example",
                            "type": type_name,
                            "mutation_path": path,
                        }
                    )
                if "examples" in c_data and "examples" not in b_data:
                    detailed_changes.append(
                        {
                            "pattern": "FIELD_ADDED examples",
                            "type": type_name,
                            "mutation_path": path,
                        }
                    )

            # Check spawn_format changes
            if "spawn_format" not in b_type and "spawn_format" in c_type:
                detailed_changes.append(
                    {
                        "pattern": "FIELD_ADDED spawn_format",
                        "type": type_name,
                        "mutation_path": "",
                    }
                )

        # Group by pattern
        patterns: defaultdict[str, list[dict[str, str]]] = defaultdict(list)
        for change in detailed_changes:
            patterns[change["pattern"]].append(change)

        # Create output
        output: dict[str, dict[str, dict[str, int | list[dict[str, str]]]]] = {"unexpected_changes": {}}
        for pattern, changes in patterns.items():
            output["unexpected_changes"][pattern] = {
                "count": len(changes),
                "types_affected": len(set(c["type"] for c in changes)),
                "examples": changes[:10],  # First 10 examples
            }

        # Write output
        with open(detailed_output_file, "w") as f:
            json.dump(output, f, indent=2)

        print(
            f"✅ Generated detailed comparison data to {detailed_output_file}"
        )
        sys.exit(0)
    else:
        # Run normal comparison
        sys.exit(main(baseline_file, current_file))
