#!/usr/bin/env python3
"""
Deep comparison tool for mutation test JSON files.
Detects and categorizes structural differences between baseline and current files.
"""

import json
import sys
from typing import Any, Dict, List, Tuple, Optional
from dataclasses import dataclass
from enum import Enum

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
    before_sample: Any
    after_sample: Any
    
def describe_structure(val: Any) -> str:
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
        if isinstance(first, dict) and 'variants' in first:
            return "enum_schema"
        elif isinstance(first, list) and first and isinstance(first[0], dict) and 'variants' in first[0]:
            return "enum_schema_array"
        else:
            return f"array[{describe_structure(first)}]"
    elif isinstance(val, dict):
        if 'variants' in val:
            return "enum_schema"
        elif all(k in val for k in ['x', 'y', 'z']):
            return "vec3_object"
        elif all(k in val for k in ['x', 'y', 'z', 'w']):
            return "quat_object"
        else:
            return "object"
    return "unknown"

def detect_pattern(before: Any, after: Any, path: str) -> ChangePattern:
    """Detect what kind of change pattern this is"""
    before_struct = describe_structure(before)
    after_struct = describe_structure(after)
    
    # Enum representation change
    if before_struct == "string" and "enum_schema" in after_struct:
        return ChangePattern.ENUM_REPRESENTATION
    
    # Vector format change
    if (before_struct in ["vec3_object", "quat_object"] and 
        after_struct.startswith("array")):
        return ChangePattern.VEC_FORMAT
    
    # Type change
    if before_struct != after_struct:
        return ChangePattern.TYPE_CHANGE
    
    # Value change (same structure)
    if before != after:
        return ChangePattern.VALUE_CHANGE
    
    return ChangePattern.UNKNOWN

def find_differences(
    baseline_type: Dict,
    current_type: Dict,
    type_name: str
) -> List[Difference]:
    """Find all differences in a single type"""
    differences = []
    
    def recurse(b_val: Any, c_val: Any, path: str):
        if type(b_val) != type(c_val):
            # Structural difference
            pattern = detect_pattern(b_val, c_val, path)
            # Always capture the actual values for comparison
            differences.append(Difference(
                type_name=type_name,
                path=path,
                pattern=pattern,
                before_structure=describe_structure(b_val),
                after_structure=describe_structure(c_val),
                before_sample=b_val,
                after_sample=c_val
            ))
        elif isinstance(b_val, dict):
            all_keys = set(b_val.keys()) | set(c_val.keys())
            for key in all_keys:
                new_path = f"{path}.{key}" if path else key
                if key not in b_val:
                    differences.append(Difference(
                        type_name=type_name,
                        path=new_path,
                        pattern=ChangePattern.FIELD_ADDED,
                        before_structure="missing",
                        after_structure=describe_structure(c_val[key]),
                        before_sample=None,
                        after_sample=c_val[key] if not isinstance(c_val[key], (dict, list)) else "..."
                    ))
                elif key not in c_val:
                    differences.append(Difference(
                        type_name=type_name,
                        path=new_path,
                        pattern=ChangePattern.FIELD_REMOVED,
                        before_structure=describe_structure(b_val[key]),
                        after_structure="missing",
                        before_sample=b_val[key] if not isinstance(b_val[key], (dict, list)) else "...",
                        after_sample=None
                    ))
                else:
                    recurse(b_val[key], c_val[key], new_path)
        elif isinstance(b_val, list):
            for i in range(min(len(b_val), len(c_val))):
                recurse(b_val[i], c_val[i], f"{path}[{i}]")
            if len(b_val) != len(c_val):
                pattern = detect_pattern(b_val, c_val, path)
                differences.append(Difference(
                    type_name=type_name,
                    path=f"{path}.length",
                    pattern=pattern,
                    before_structure=f"array[{len(b_val)}]",
                    after_structure=f"array[{len(c_val)}]",
                    before_sample=len(b_val),
                    after_sample=len(c_val)
                ))
        elif b_val != c_val:
            # Simple value difference
            pattern = detect_pattern(b_val, c_val, path)
            differences.append(Difference(
                type_name=type_name,
                path=path,
                pattern=pattern,
                before_structure=describe_structure(b_val),
                after_structure=describe_structure(c_val),
                before_sample=b_val,
                after_sample=c_val
            ))
    
    recurse(baseline_type, current_type, "")
    return differences

def main(baseline_file: str, current_file: str):
    """Main comparison logic"""
    
    # Load files
    with open(baseline_file) as f:
        baseline = json.load(f)
    with open(current_file) as f:
        current = json.load(f)
    
    # Extract type_guide arrays
    baseline_tg = baseline.get('type_guide', baseline)
    current_tg = current.get('type_guide', current)
    
    # Create lookups
    baseline_dict = {t['type_name']: t for t in baseline_tg}
    current_dict = {t['type_name']: t for t in current_tg}
    
    # Find all differences
    all_differences = []
    for type_name in baseline_dict:
        if type_name in current_dict:
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
    by_pattern = {}
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
        pattern_label = "IDENTIFIED PATTERN" if pattern != ChangePattern.UNKNOWN else "UNRECOGNIZED PATTERN"
        print(f"📌 {pattern_label}: {pattern.value.replace('_', ' ').upper()}")
        print("-" * 40)
        
        affected_types = list(set(d.type_name for d in diffs))
        print(f"Types affected: {len(affected_types)}")
        print(f"Total changes: {len(diffs)}")
        print()
        
        # Show up to 3 examples with actual data
        for i, diff in enumerate(diffs[:3]):
            print(f"Example {i+1}:")
            print(f"  Type: {diff.type_name}")
            print(f"  Path: {diff.path}")
            print(f"  Structure change: {diff.before_structure} → {diff.after_structure}")
            
            # Show actual values
            if isinstance(diff.before_sample, (str, int, float, bool, type(None))):
                print(f"  Before value: {json.dumps(diff.before_sample)}")
            else:
                before_str = json.dumps(diff.before_sample, indent=2)
                if len(before_str) > 300:
                    before_str = before_str[:300] + "..."
                print(f"  Before value:\n    {before_str.replace(chr(10), chr(10) + '    ')}")
                
            if isinstance(diff.after_sample, (str, int, float, bool, type(None))):
                print(f"  After value: {json.dumps(diff.after_sample)}")
            else:
                after_str = json.dumps(diff.after_sample, indent=2)
                if len(after_str) > 300:
                    after_str = after_str[:300] + "..."
                print(f"  After value:\n    {after_str.replace(chr(10), chr(10) + '    ')}")
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
            print(f"    Pattern: UNKNOWN - needs investigation")
        
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
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <baseline_file> <current_file>")
        sys.exit(1)
    
    sys.exit(main(sys.argv[1], sys.argv[2]))