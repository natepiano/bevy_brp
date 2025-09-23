#!/usr/bin/env python3
"""
Check mutation paths for a specific type across all available versions.
Usage: python3 check_type.py <type_name>
"""

import json
import sys
import glob
from pathlib import Path
from typing import TypedDict


class PathInfo(TypedDict, total=False):
    mutation_status: str


class MutationPath(TypedDict):
    path_info: PathInfo


class TypeData(TypedDict, total=False):
    mutation_paths: dict[str, MutationPath]
    supported_operations: list[str]


class TypeGuide(TypedDict):
    type_guide: dict[str, TypeData]


def check_type(type_name: str) -> None:
    """Check a specific type's mutation paths across all versions."""
    # Find all all_types*.json files in .claude/transient
    pattern = ".claude/transient/all_types*.json"
    files = sorted(glob.glob(pattern))
    
    if not files:
        print("No all_types*.json files found in .claude/transient")
        return
    
    print(f"Checking type: {type_name}")
    print("=" * 60)
    
    found_any = False
    for filepath in files:
        filename = Path(filepath).name
        
        try:
            with open(filepath, 'r') as f:
                file_data: TypeGuide = json.load(f)  # pyright: ignore[reportAny]

            # Handle JSON structure with type_guide dict
            type_guide = file_data.get('type_guide', {})

            # Find the type in dict format
            type_data = type_guide.get(type_name)
            
            if type_data:
                found_any = True
                mutation_paths = type_data.get('mutation_paths', {})
                supported_ops = type_data.get('supported_operations', [])
                
                # Check for spawn/insert support
                spawn_supported = any(op in ['Spawn', 'Insert'] for op in supported_ops)
                spawn_status = "supported" if spawn_supported else "not_supported"
                
                print(f"\n📄 {filename}")
                print(f"   Spawn support: {spawn_status}")
                print(f"   Supported operations: {', '.join(supported_ops)}")
                print(f"   Mutation paths: {len(mutation_paths)}")
                
                if mutation_paths:
                    # Show mutation paths with their status
                    sorted_paths = sorted(mutation_paths.keys())
                    for path in sorted_paths[:10]:
                        path_info = mutation_paths[path]
                        # Get mutation_status from path_info (standard structure)
                        status = path_info.get('path_info', {}).get('mutation_status', 'unknown')
                        print(f"     {path} ({status})")
                    if len(mutation_paths) > 10:
                        print(f"     ... and {len(mutation_paths) - 10} more")
                else:
                    print("     (none)")
        except Exception as e:
            print(f"\n❌ Error reading {filename}: {e}")
    
    if not found_any:
        print(f"\nType '{type_name}' not found in any file")
        print("\nAvailable types (from latest file):")
        try:
            with open(files[-1], 'r') as f:
                last_file_data: TypeGuide = json.load(f)  # pyright: ignore[reportAny]
            type_guide = last_file_data.get('type_guide', {})
            all_type_names = sorted(type_guide.keys())
            # Show types that partially match
            matches = [t for t in all_type_names if type_name.lower() in t.lower()]
            if matches:
                print("Possible matches:")
                for t in matches[:10]:
                    print(f"  - {t}")
        except Exception:
            pass

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 check_type.py <type_name>")
        print("Example: python3 check_type.py 'bevy_transform::components::transform::Transform'")
        sys.exit(1)
    
    check_type(sys.argv[1])
