#!/usr/bin/env python3
"""
Check mutation paths for a specific type across all available versions.
Usage: python3 check_type.py <type_name>
"""

import json
import sys
import glob
from pathlib import Path

def check_type(type_name):
    """Check a specific type's mutation paths across all versions."""
    # Find all all_types*.json files in $TMPDIR
    import os
    tmpdir = os.environ.get('TMPDIR', '/var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T')
    pattern = f"{tmpdir}/all_types*.json"
    files = sorted(glob.glob(pattern))
    
    if not files:
        print("No all_types*.json files found in $TMPDIR")
        return
    
    print(f"Checking type: {type_name}")
    print("=" * 60)
    
    found_any = False
    for filepath in files:
        filename = Path(filepath).name
        
        try:
            with open(filepath, 'r') as f:
                types = json.load(f)
            
            # Find the type
            type_data = next((t for t in types if t['type'] == type_name), None)
            
            if type_data:
                found_any = True
                paths = type_data.get('mutation_paths', [])
                spawn = type_data.get('spawn_support', 'not_supported')
                
                print(f"\n📄 {filename}")
                print(f"   Spawn support: {spawn}")
                print(f"   Mutation paths: {len(paths)}")
                if paths:
                    for path in sorted(paths)[:10]:
                        print(f"     {path}")
                    if len(paths) > 10:
                        print(f"     ... and {len(paths) - 10} more")
                else:
                    print("     (none)")
        except Exception as e:
            print(f"\n❌ Error reading {filename}: {e}")
    
    if not found_any:
        print(f"\nType '{type_name}' not found in any file")
        print("\nAvailable types (from latest file):")
        try:
            with open(files[-1], 'r') as f:
                types = json.load(f)
            all_type_names = sorted([t['type'] for t in types])
            # Show types that partially match
            matches = [t for t in all_type_names if type_name.lower() in t.lower()]
            if matches:
                print("Possible matches:")
                for t in matches[:10]:
                    print(f"  - {t}")
        except:
            pass

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 check_type.py <type_name>")
        print("Example: python3 check_type.py 'bevy_transform::components::transform::Transform'")
        sys.exit(1)
    
    check_type(sys.argv[1])
