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

    # Define which fields at each level should have their contents compared as values, not structure
    VALUE_FIELDS = {'example', 'examples', 'spawn_format', 'path_info', 'schema_info'}

    def should_compare_as_value(path: str, key: str) -> bool:
        """Check if this key's value should be compared as a whole value rather than recursively"""
        # These fields contain data values, not structural schema
        return key in VALUE_FIELDS

    def recurse(b_val: Any, c_val: Any, path: str):
        # CRITICAL FIX: Don't flag identical values as changes
        if b_val == c_val:
            return

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
                    # Check if this field should be compared as a whole value
                    if should_compare_as_value(path, key):
                        # Compare the entire value, don't recurse into it
                        if b_val[key] != c_val[key]:
                            pattern = detect_pattern(b_val[key], c_val[key], new_path)
                            differences.append(Difference(
                                type_name=type_name,
                                path=new_path,
                                pattern=pattern,
                                before_structure=describe_structure(b_val[key]),
                                after_structure=describe_structure(c_val[key]),
                                before_sample=b_val[key],
                                after_sample=c_val[key]
                            ))
                    else:
                        # Recurse into structural fields
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

def extract_type_guide(data: Dict) -> List[Dict]:
    """Extract type_guide array from either format"""
    if 'type_guide' in data:
        type_guide = data['type_guide']
        # Handle both object format (keys are type names) and array format
        if isinstance(type_guide, dict):
            # Convert object format to array format, adding type_name field
            return [
                {**guide, 'type_name': type_name}
                for type_name, guide in type_guide.items()
            ]
        else:
            return type_guide
    elif 'result' in data and 'type_guide' in data['result']:
        type_guide = data['result']['type_guide']
        # Handle both object format (keys are type names) and array format
        if isinstance(type_guide, dict):
            # Convert object format to array format, adding type_name field
            return [
                {**guide, 'type_name': type_name}
                for type_name, guide in type_guide.items()
            ]
        else:
            return type_guide
    else:
        # If data is a dict with type names as keys, return the values
        if isinstance(data, dict):
            return list(data.values())
        return data

def calculate_metadata(type_guide: List[Dict]) -> Dict[str, int]:
    """Calculate metadata statistics for a type guide"""
    total_types = len(type_guide)
    
    spawn_supported = len([t for t in type_guide if isinstance(t, dict) and 'spawn_format' in t])

    with_mutations = len([
        t for t in type_guide
        if isinstance(t, dict) and t.get('mutation_paths') and t['mutation_paths'] != {} and t['mutation_paths'] != []
    ])
    
    total_paths = sum([
        len(t['mutation_paths'].keys()) if isinstance(t, dict) and isinstance(t.get('mutation_paths'), dict) else 0
        for t in type_guide
    ])
    
    return {
        'total_types': total_types,
        'spawn_supported': spawn_supported, 
        'with_mutations': with_mutations,
        'total_paths': total_paths
    }

def get_excluded_types() -> List[str]:
    """Get list of excluded types from the exclusion file"""
    exclusion_file = ".claude/scripts/mutation_test_excluded_types.json"
    excluded = []

    try:
        with open(exclusion_file, 'r') as f:
            import json
            data = json.load(f)
            excluded = [item['type_name'] for item in data.get('excluded_types', [])]
    except (FileNotFoundError, json.JSONDecodeError):
        # Fall back to old text file format if JSON doesn't exist or is invalid
        old_file = ".claude/scripts/mutation_test_excluded_types.txt"
        try:
            with open(old_file, 'r') as f:
                for line in f:
                    line = line.strip()
                    # Skip comments and empty lines
                    if line and not line.startswith('#'):
                        excluded.append(line)
        except FileNotFoundError:
            # If neither file exists, return empty list
            pass

    return excluded

def main(baseline_file: str, current_file: str):
    """Main comparison logic"""

    print("🔍 STRUCTURED MUTATION TEST COMPARISON (Full Schema)")
    print("=" * 60)
    print()

    # Load files
    try:
        with open(baseline_file) as f:
            baseline = json.load(f)
    except FileNotFoundError:
        print(f"❌ Baseline file not found: {baseline_file}")
        return 1
    except json.JSONDecodeError:
        print(f"❌ Invalid JSON in baseline file: {baseline_file}")
        return 1

    try:
        with open(current_file) as f:
            current = json.load(f)
    except FileNotFoundError:
        print(f"❌ Current file not found: {current_file}")
        return 1
    except json.JSONDecodeError:
        print(f"❌ Invalid JSON in current file: {current_file}")
        return 1
    
    # Binary identity check
    print("📊 IDENTITY CHECK")
    with open(baseline_file, 'rb') as f1, open(current_file, 'rb') as f2:
        if f1.read() == f2.read():
            print("✅ FILES ARE IDENTICAL")
            print("   └─ Baseline and current files are byte-for-byte identical")
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
            print(f"   Excluded Types: {', '.join(excluded_types) if excluded_types else 'None'}")
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
    for key in ['total_types', 'spawn_supported', 'with_mutations', 'total_paths']:
        baseline_val = baseline_meta[key]
        current_val = current_meta[key]
        label = key.replace('_', ' ').title().replace('Total ', 'Total Mutation ')

        if baseline_val == current_val:
            print(f"   {label}: {baseline_val} → {current_val} (no change)")
        else:
            diff = current_val - baseline_val
            print(f"   {label}: {baseline_val} → {current_val} ({current_val} - {baseline_val} = {diff:+d})")

    print(f"   Excluded Types: {', '.join(excluded_types) if excluded_types else 'None'}")
    print()
    
    # Type-level changes analysis
    print("🔍 TYPE-LEVEL CHANGES")
    
    baseline_types = set(t['type_name'] for t in baseline_tg)
    current_types = set(t['type_name'] for t in current_tg)
    
    new_types = current_types - baseline_types
    removed_types = baseline_types - current_types
    common_types = baseline_types & current_types
    
    # Create lookups
    baseline_dict = {t['type_name']: t for t in baseline_tg}
    current_dict = {t['type_name']: t for t in current_tg}
    
    # Check for changes in common types
    modified_types = []
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
    all_differences = []
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
        
        # Special handling for field removals/additions - show which fields changed
        if pattern == ChangePattern.FIELD_REMOVED:
            # Group by field name to show what's being removed
            field_changes = {}
            for diff in diffs:
                field_name = diff.path.split('.')[-1]  # Get the last part of the path as field name
                if field_name not in field_changes:
                    field_changes[field_name] = []
                field_changes[field_name].append(diff)
            
            print()
            print(f"Fields removed breakdown:")
            for field_name, field_diffs in field_changes.items():
                affected_types_for_field = len(set(d.type_name for d in field_diffs))
                print(f"  • '{field_name}' field: {len(field_diffs)} removal(s) across {affected_types_for_field} type(s)")
                
        elif pattern == ChangePattern.FIELD_ADDED:
            # Group by field name to show what's being added
            field_changes = {}
            for diff in diffs:
                field_name = diff.path.split('.')[-1]  # Get the last part of the path as field name
                if field_name not in field_changes:
                    field_changes[field_name] = []
                field_changes[field_name].append(diff)
            
            print()
            print(f"Fields added breakdown:")
            for field_name, field_diffs in field_changes.items():
                affected_types_for_field = len(set(d.type_name for d in field_diffs))
                print(f"  • '{field_name}' field: {len(field_diffs)} addition(s) across {affected_types_for_field} type(s)")
        
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
    if len(sys.argv) not in [3, 4]:
        print(f"Usage: {sys.argv[0]} <baseline_file> <current_file> [--detailed]")
        sys.exit(1)

    baseline_file = sys.argv[1]
    current_file = sys.argv[2]

    # Check for --detailed flag
    if len(sys.argv) == 4 and sys.argv[3] == "--detailed":
        # Generate detailed JSON output
        import os
        from collections import defaultdict

        detailed_output_file = os.path.join(os.environ.get('TMPDIR', '/tmp'), 'mutation_comparison_details.json')

        # Load files
        with open(baseline_file) as f:
            baseline = json.load(f)
        with open(current_file) as f:
            current = json.load(f)

        # Extract type guides
        baseline_tg = extract_type_guide(baseline)
        current_tg = extract_type_guide(current)

        # Create lookups
        baseline_dict = {t.get('type_name', f'type_{i}'): t for i, t in enumerate(baseline_tg)}
        current_dict = {t.get('type_name', f'type_{i}'): t for i, t in enumerate(current_tg)}

        # Find changes focusing on the unexpected patterns
        all_type_names = set(baseline_dict.keys()) | set(current_dict.keys())
        detailed_changes = []

        for type_name in all_type_names:
            b_type = baseline_dict.get(type_name, {})
            c_type = current_dict.get(type_name, {})

            # Check mutation_paths for removed/added example fields
            b_mutations = b_type.get('mutation_paths', {})
            c_mutations = c_type.get('mutation_paths', {})

            for path, b_data in b_mutations.items():
                c_data = c_mutations.get(path, {})
                if 'examples' in b_data and 'examples' not in c_data:
                    detailed_changes.append({
                        'pattern': 'FIELD_REMOVED examples',
                        'type': type_name,
                        'mutation_path': path
                    })
                if 'example' in b_data and 'example' not in c_data:
                    detailed_changes.append({
                        'pattern': 'FIELD_REMOVED example',
                        'type': type_name,
                        'mutation_path': path
                    })

            for path, c_data in c_mutations.items():
                b_data = b_mutations.get(path, {})
                if 'example' in c_data and 'example' not in b_data:
                    detailed_changes.append({
                        'pattern': 'FIELD_ADDED example',
                        'type': type_name,
                        'mutation_path': path
                    })
                if 'examples' in c_data and 'examples' not in b_data:
                    detailed_changes.append({
                        'pattern': 'FIELD_ADDED examples',
                        'type': type_name,
                        'mutation_path': path
                    })

            # Check spawn_format changes
            if 'spawn_format' not in b_type and 'spawn_format' in c_type:
                detailed_changes.append({
                    'pattern': 'FIELD_ADDED spawn_format',
                    'type': type_name,
                    'mutation_path': ''
                })

        # Group by pattern
        patterns = defaultdict(list)
        for change in detailed_changes:
            patterns[change['pattern']].append(change)

        # Create output
        output = {'unexpected_changes': {}}
        for pattern, changes in patterns.items():
            output['unexpected_changes'][pattern] = {
                'count': len(changes),
                'types_affected': len(set(c['type'] for c in changes)),
                'examples': changes[:10]  # First 10 examples
            }

        # Write output
        with open(detailed_output_file, 'w') as f:
            json.dump(output, f, indent=2)

        print(f"✅ Generated detailed comparison data to {detailed_output_file}")
        sys.exit(0)
    else:
        # Run normal comparison
        sys.exit(main(baseline_file, current_file))