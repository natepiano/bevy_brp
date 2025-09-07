#!/usr/bin/env python3
"""
Compare mutation paths between two all_types.json files.
Usage: python3 compare_mutations.py <old_file> <new_file>
"""

import json
import sys
from pathlib import Path

def load_json(filepath):
    """Load JSON file and return parsed data."""
    with open(filepath, 'r') as f:
        return json.load(f)

def compare_mutations(old_file, new_file):
    """Compare mutation paths between two versions."""
    # Load files
    old_types = load_json(old_file)
    new_types = load_json(new_file)
    
    # Create lookup dictionaries
    old_dict = {t['type']: t for t in old_types}
    new_dict = {t['type']: t for t in new_types}
    
    # Track all changes
    added_types = set(new_dict.keys()) - set(old_dict.keys())
    removed_types = set(old_dict.keys()) - set(new_dict.keys())
    common_types = set(old_dict.keys()) & set(new_dict.keys())
    
    # Find mutation path differences
    path_differences = []
    spawn_changes = []
    
    for type_name in sorted(common_types):
        old_paths = set(old_dict[type_name].get('mutation_paths', []))
        new_paths = set(new_dict[type_name].get('mutation_paths', []))
        
        if old_paths != new_paths:
            path_differences.append({
                'type': type_name,
                'added': sorted(list(new_paths - old_paths)),
                'removed': sorted(list(old_paths - new_paths)),
                'old_count': len(old_paths),
                'new_count': len(new_paths)
            })
        
        # Check spawn support changes
        old_spawn = old_dict[type_name].get('spawn_support', 'not_supported')
        new_spawn = new_dict[type_name].get('spawn_support', 'not_supported')
        if old_spawn != new_spawn:
            spawn_changes.append({
                'type': type_name,
                'old': old_spawn,
                'new': new_spawn
            })
    
    # Print summary
    print("=" * 60)
    print("MUTATION PATH COMPARISON REPORT")
    print("=" * 60)
    print(f"\nFiles compared:")
    print(f"  Old: {Path(old_file).name} ({len(old_types)} types)")
    print(f"  New: {Path(new_file).name} ({len(new_types)} types)")
    
    # Type changes
    if added_types or removed_types:
        print(f"\n📦 TYPE CHANGES:")
        if added_types:
            print(f"  Added types: {len(added_types)}")
            for t in sorted(added_types)[:5]:
                print(f"    + {t}")
            if len(added_types) > 5:
                print(f"    ... and {len(added_types) - 5} more")
        if removed_types:
            print(f"  Removed types: {len(removed_types)}")
            for t in sorted(removed_types)[:5]:
                print(f"    - {t}")
            if len(removed_types) > 5:
                print(f"    ... and {len(removed_types) - 5} more")
    
    # Mutation path changes
    if not path_differences:
        print(f"\n✅ NO MUTATION PATH DIFFERENCES")
        print(f"   All {len(common_types)} common types have identical mutation paths")
    else:
        print(f"\n⚠️  MUTATION PATH DIFFERENCES: {len(path_differences)} types")
        
        # Categorize changes
        only_added = [d for d in path_differences if d['added'] and not d['removed']]
        only_removed = [d for d in path_differences if d['removed'] and not d['added']]
        modified = [d for d in path_differences if d['added'] and d['removed']]
        
        if only_added:
            print(f"\n  📈 Types with added paths only: {len(only_added)}")
            for diff in only_added[:3]:
                print(f"     {diff['type']}: +{len(diff['added'])} paths")
        
        if only_removed:
            print(f"\n  📉 Types with removed paths only: {len(only_removed)}")
            for diff in only_removed[:3]:
                print(f"     {diff['type']}: -{len(diff['removed'])} paths")
        
        if modified:
            print(f"\n  🔄 Types with modified paths: {len(modified)}")
            for diff in modified[:3]:
                print(f"     {diff['type']}: +{len(diff['added'])} / -{len(diff['removed'])} paths")
        
        # Show details for first few
        print(f"\n  Detailed changes (first 5):")
        for diff in path_differences[:5]:
            print(f"\n  {diff['type']} ({diff['old_count']} → {diff['new_count']} paths):")
            if diff['added']:
                print(f"    Added: {diff['added'][:3]}")
                if len(diff['added']) > 3:
                    print(f"           ... and {len(diff['added']) - 3} more")
            if diff['removed']:
                print(f"    Removed: {diff['removed'][:3]}")
                if len(diff['removed']) > 3:
                    print(f"             ... and {len(diff['removed']) - 3} more")
    
    # Spawn support changes
    if spawn_changes:
        print(f"\n🔄 SPAWN SUPPORT CHANGES: {len(spawn_changes)} types")
        for change in spawn_changes[:5]:
            print(f"   {change['type']}: {change['old']} → {change['new']}")
    
    # Statistics
    print(f"\n📊 STATISTICS:")
    old_with_mutations = sum(1 for t in old_types if t.get('mutation_paths'))
    new_with_mutations = sum(1 for t in new_types if t.get('mutation_paths'))
    print(f"   Types with mutations: {old_with_mutations} → {new_with_mutations}")
    
    old_spawn = sum(1 for t in old_types if t.get('spawn_support') == 'supported')
    new_spawn = sum(1 for t in new_types if t.get('spawn_support') == 'supported')
    print(f"   Spawn-supported types: {old_spawn} → {new_spawn}")
    
    return len(path_differences) == 0

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 compare_mutations.py <old_file> <new_file>")
        sys.exit(1)
    
    old_file = sys.argv[1]
    new_file = sys.argv[2]
    
    if not Path(old_file).exists():
        print(f"Error: {old_file} does not exist")
        sys.exit(1)
    
    if not Path(new_file).exists():
        print(f"Error: {new_file} does not exist")
        sys.exit(1)
    
    identical = compare_mutations(old_file, new_file)
    sys.exit(0 if identical else 1)
