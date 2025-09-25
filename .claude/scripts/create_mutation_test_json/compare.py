#!/usr/bin/env python3
"""
Comprehensive comparison tool for mutation test JSON files.
Outputs ALL changes with complete details to a JSON file for review.
"""

import json
import sys
import os
import re
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from collections import defaultdict
from datetime import datetime

def get_output_path() -> Path:
    """Get the output path using TMPDIR environment variable."""
    tmpdir = os.environ.get('TMPDIR', '/tmp')
    return Path(tmpdir) / 'mutation_comparison_full.json'

def load_files(baseline_path: str, current_path: str) -> Tuple[Dict, Dict]:
    """Load baseline and current JSON files."""
    with open(baseline_path) as f:
        baseline_data = json.load(f)

    with open(current_path) as f:
        current_data = json.load(f)

    # Extract type guides
    baseline = baseline_data.get('type_guide', baseline_data)
    current = current_data.get('type_guide', current_data)

    return baseline, current

def load_expected_changes() -> List[Dict[str, Any]]:
    """Load expected changes configuration."""
    # Script is now in .claude/scripts/create_mutation_test_json/, so go up 3 levels
    config_path = Path(__file__).parent.parent.parent / "config" / "create_mutation_test_json_expected_changes.json"
    if not config_path.exists():
        return []

    with open(config_path) as f:
        data = json.load(f)
        # Filter out the example entry (id: 0)
        return [c for c in data.get("expected_changes", []) if c.get("id", 0) != 0]

def describe_value(val: Any) -> str:
    """Create a concise description of a value."""
    if val is None:
        return "null"
    elif isinstance(val, bool):
        return str(val).lower()
    elif isinstance(val, (int, float)):
        return str(val)
    elif isinstance(val, str):
        if len(val) > 50:
            return f'"{val[:47]}..."'
        return f'"{val}"'
    elif isinstance(val, list):
        return f"array[{len(val)}]"
    elif isinstance(val, dict):
        keys = list(val.keys())[:3]
        if len(keys) < len(val):
            keys.append("...")
        return f"object{{{','.join(keys)}}}"
    else:
        return str(type(val).__name__)

def deep_compare_values(path: str, baseline_val: Any, current_val: Any) -> List[Dict[str, Any]]:
    """Recursively compare values and return all differences."""
    differences = []

    # Check if both are None
    if baseline_val is None and current_val is None:
        return []

    # Check if one is None
    if baseline_val is None:
        differences.append({
            "path": path,
            "change_type": "added",
            "baseline": None,
            "current": current_val,
            "description": f"Added: {describe_value(current_val)}"
        })
        return differences

    if current_val is None:
        differences.append({
            "path": path,
            "change_type": "removed",
            "baseline": baseline_val,
            "current": None,
            "description": f"Removed: {describe_value(baseline_val)}"
        })
        return differences

    # Check if types differ
    if type(baseline_val) != type(current_val):
        differences.append({
            "path": path,
            "change_type": "type_changed",
            "baseline": baseline_val,
            "current": current_val,
            "description": f"Type changed: {type(baseline_val).__name__} → {type(current_val).__name__}"
        })
        return differences

    # Compare based on type
    if isinstance(baseline_val, dict):
        all_keys = set(baseline_val.keys()) | set(current_val.keys())
        for key in sorted(all_keys):
            new_path = f"{path}.{key}" if path else key
            base_item = baseline_val.get(key)
            curr_item = current_val.get(key)
            differences.extend(deep_compare_values(new_path, base_item, curr_item))

    elif isinstance(baseline_val, list):
        # For lists, compare by index
        max_len = max(len(baseline_val), len(current_val))
        for i in range(max_len):
            new_path = f"{path}[{i}]"
            base_item = baseline_val[i] if i < len(baseline_val) else None
            curr_item = current_val[i] if i < len(current_val) else None

            if base_item is None and curr_item is not None:
                differences.append({
                    "path": new_path,
                    "change_type": "added",
                    "baseline": None,
                    "current": curr_item,
                    "description": f"Added element at index {i}"
                })
            elif base_item is not None and curr_item is None:
                differences.append({
                    "path": new_path,
                    "change_type": "removed",
                    "baseline": base_item,
                    "current": None,
                    "description": f"Removed element at index {i}"
                })
            else:
                differences.extend(deep_compare_values(new_path, base_item, curr_item))

    elif baseline_val != current_val:
        # Primitive values that differ
        differences.append({
            "path": path,
            "change_type": "value_changed",
            "baseline": baseline_val,
            "current": current_val,
            "description": f"Value changed: {describe_value(baseline_val)} → {describe_value(current_val)}"
        })

    return differences

def compare_types(baseline: Dict, current: Dict) -> Dict[str, Any]:
    """Compare all types and collect ALL differences."""
    all_changes = []
    type_stats = {
        "baseline_only": [],
        "current_only": [],
        "modified": []
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

        type_differences = deep_compare_values("", baseline_entry, current_entry)

        if type_differences:
            type_stats["modified"].append(type_name)
            for diff in type_differences:
                diff["type_name"] = type_name
                all_changes.append(diff)

    return {
        "all_changes": all_changes,
        "type_stats": type_stats
    }

def auto_detect_patterns(changes: List[Dict[str, Any]]) -> Dict[str, List[Dict[str, Any]]]:
    """Auto-detect patterns in the changes."""
    patterns = defaultdict(list)

    for change in changes:
        path = change["path"]
        change_type = change["change_type"]

        # Detect specific patterns based on path and change characteristics
        if "enum_variant_path" in path and "instructions" in path:
            # Check for specific instruction changes
            if change_type == "value_changed":
                baseline_str = str(change["baseline"])
                current_str = str(change["current"])

                if ">>::Some" in baseline_str or ">>::None" in baseline_str:
                    pattern_key = "malformed_option_variant_names"
                elif ">::" in baseline_str and "Handle<" in current_str:
                    pattern_key = "incomplete_handle_type_names"
                elif ">::" in baseline_str and "AssetId<" in current_str:
                    pattern_key = "incomplete_assetid_type_names"
                else:
                    pattern_key = "other_instruction_changes"
                patterns[pattern_key].append(change)

        elif ".examples" in path:
            if change_type == "added":
                patterns["examples_field_added"].append(change)
            elif change_type == "removed":
                patterns["examples_field_removed"].append(change)
            else:
                patterns["examples_field_changed"].append(change)

        elif ".example" in path:
            if change_type == "added":
                patterns["example_field_added"].append(change)
            elif change_type == "removed":
                patterns["example_field_removed"].append(change)
            elif change_type == "value_changed":
                patterns["example_value_changed"].append(change)

        elif "applicable_variants" in path:
            patterns["applicable_variants_changed"].append(change)

        elif "variant_example" in path:
            patterns["variant_example_changed"].append(change)

        elif change_type == "type_changed":
            patterns["type_changes"].append(change)

        elif change_type == "value_changed":
            patterns["other_value_changes"].append(change)

        elif change_type == "added":
            patterns["fields_added"].append(change)

        elif change_type == "removed":
            patterns["fields_removed"].append(change)

        else:
            patterns["uncategorized"].append(change)

    return dict(patterns)

def match_expected_changes(patterns: Dict[str, List], expected_changes: List[Dict]) -> Tuple[Dict, Dict]:
    """Match detected patterns against expected changes."""
    matched = {}
    unmatched = {}

    # For now, do simple pattern matching
    # This can be enhanced to use the value_pattern matching from expected_changes
    for pattern_name, changes in patterns.items():
        # Check if this pattern matches any expected change
        matched_expected = None

        for expected in expected_changes:
            # Simple matching logic - can be enhanced
            if pattern_name == "malformed_option_variant_names" and expected.get("id") == 1:
                matched_expected = expected
                break
            elif pattern_name in ["incomplete_handle_type_names", "incomplete_assetid_type_names"] and expected.get("id") == 2:
                matched_expected = expected
                break
            elif pattern_name == "examples_field_added" and expected.get("id") == 3:
                matched_expected = expected
                break
            elif pattern_name == "examples_field_removed" and expected.get("id") == 4:
                matched_expected = expected
                break

        if matched_expected:
            matched[pattern_name] = {
                "expected_id": matched_expected["id"],
                "expected_name": matched_expected["name"],
                "count": len(changes),
                "changes": changes
            }
        else:
            unmatched[pattern_name] = {
                "count": len(changes),
                "changes": changes
            }

    return matched, unmatched

def generate_summary(comparison_result: Dict, matched: Dict, unmatched: Dict) -> Dict[str, Any]:
    """Generate a summary of the comparison."""
    all_changes = comparison_result["all_changes"]
    type_stats = comparison_result["type_stats"]

    summary = {
        "total_changes": len(all_changes),
        "types_modified": len(type_stats["modified"]),
        "types_added": len(type_stats["current_only"]),
        "types_removed": len(type_stats["baseline_only"]),
        "expected_patterns": len(matched),
        "unexpected_patterns": len(unmatched),
        "expected_changes_count": sum(p["count"] for p in matched.values()),
        "unexpected_changes_count": sum(p["count"] for p in unmatched.values())
    }

    return summary

def save_full_comparison(output_path: Path, summary: Dict, matched: Dict, unmatched: Dict,
                         type_stats: Dict, all_changes: List) -> None:
    """Save the full comparison results to JSON file."""
    full_result = {
        "metadata": {
            "generated_at": datetime.now().isoformat(),
            "output_version": "1.0.0"
        },
        "summary": summary,
        "expected_patterns": matched,
        "unexpected_patterns": unmatched,
        "type_statistics": type_stats,
        "all_changes": all_changes
    }

    with open(output_path, 'w') as f:
        json.dump(full_result, f, indent=2)

def print_summary(summary: Dict, matched: Dict, unmatched: Dict, output_path: Path) -> None:
    """Print a summary to stdout."""
    print("🔍 MUTATION TEST COMPARISON COMPLETE")
    print("=" * 60)

    print(f"\n📊 SUMMARY:")
    print(f"   Total changes: {summary['total_changes']}")
    print(f"   Types modified: {summary['types_modified']}")
    print(f"   Types added: {summary['types_added']}")
    print(f"   Types removed: {summary['types_removed']}")

    if matched:
        print(f"\n✅ EXPECTED PATTERNS: {len(matched)}")
        for pattern_name, data in matched.items():
            print(f"   - {data['expected_name']}: {data['count']} changes")

    if unmatched:
        print(f"\n⚠️  UNEXPECTED PATTERNS: {len(unmatched)}")
        for pattern_name, data in unmatched.items():
            print(f"   - {pattern_name}: {data['count']} changes")

    print(f"\n📁 Full details saved to: {output_path}")
    print(f"   Use 'read_comparison_detail.py' to explore the changes")

    # If everything is expected, suggest promotion
    if not unmatched or summary['unexpected_changes_count'] == 0:
        print("\n✅ All changes match expected patterns!")
        print("   Consider: promote")
    else:
        print(f"\n⚠️  {summary['unexpected_changes_count']} unexpected changes need review")
        print("   Use: comparison_review")

def main():
    if len(sys.argv) != 3:
        print("Usage: create_mutation_test_json_compare.py <baseline.json> <current.json>")
        sys.exit(1)

    baseline_path = sys.argv[1]
    current_path = sys.argv[2]

    # Load files
    baseline, current = load_files(baseline_path, current_path)

    # Load expected changes
    expected_changes = load_expected_changes()

    # Compare types and get ALL changes
    comparison_result = compare_types(baseline, current)

    # Auto-detect patterns
    patterns = auto_detect_patterns(comparison_result["all_changes"])

    # Match against expected changes
    matched, unmatched = match_expected_changes(patterns, expected_changes)

    # Generate summary
    summary = generate_summary(comparison_result, matched, unmatched)

    # Save full results
    output_path = get_output_path()
    save_full_comparison(output_path, summary, matched, unmatched,
                        comparison_result["type_stats"], comparison_result["all_changes"])

    # Print summary
    print_summary(summary, matched, unmatched, output_path)

if __name__ == "__main__":
    main()