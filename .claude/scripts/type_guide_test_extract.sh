#!/bin/bash

# Type Guide Test Data Extractor
# Usage: ./type_guide_test_extract.sh <file_path> <operation> [type_name] [field_path]
#
# Operations:
#   summary                           - Get overall summary
#   discovered_count                  - Get discovered count
#   type_info <type_name>            - Get basic type info (has_serialize, supported_operations, etc.)
#   mutation_paths <type_name>       - Get mutation paths for a type
#   spawn_example <type_name>         - Get spawn format for a type
#   schema_info <type_name>          - Get schema info for a type
#   validate_field <type_name> <path> - Check if specific field exists

if [ $# -lt 2 ]; then
    echo "Usage: $0 <file_path> <operation> [type_name] [field_path]"
    echo "Operations: summary, discovered_count, type_info, mutation_paths, spawn_example, schema_info, validate_field"
    exit 1
fi

FILE_PATH="$1"
OPERATION="$2"
TYPE_NAME="$3"
FIELD_PATH="$4"

if [ ! -f "$FILE_PATH" ]; then
    echo "Error: File $FILE_PATH not found"
    exit 1
fi

case "$OPERATION" in
    "summary")
        jq '.summary' "$FILE_PATH"
        ;;
    "discovered_count")
        jq '.discovered_count' "$FILE_PATH"
        ;;
    "type_info")
        if [ -z "$TYPE_NAME" ]; then
            echo "Error: type_name required for type_info operation"
            exit 1
        fi
        jq --arg type "$TYPE_NAME" '.type_guide[$type] | {type_name, in_registry, supported_operations}' "$FILE_PATH"
        ;;
    "mutation_paths")
        if [ -z "$TYPE_NAME" ]; then
            echo "Error: type_name required for mutation_paths operation"
            exit 1
        fi
        jq --arg type "$TYPE_NAME" '.type_guide[$type].mutation_paths' "$FILE_PATH"
        ;;
    "spawn_example")
        if [ -z "$TYPE_NAME" ]; then
            echo "Error: type_name required for spawn_example operation"
            exit 1
        fi
        jq --arg type "$TYPE_NAME" '.type_guide[$type].spawn_example' "$FILE_PATH"
        ;;
    "schema_info")
        if [ -z "$TYPE_NAME" ]; then
            echo "Error: type_name required for schema_info operation"
            exit 1
        fi
        jq --arg type "$TYPE_NAME" '.type_guide[$type].schema_info' "$FILE_PATH"
        ;;
    "validate_field")
        if [ -z "$TYPE_NAME" ] || [ -z "$FIELD_PATH" ]; then
            echo "Error: type_name and field_path required for validate_field operation"
            exit 1
        fi
        # Check if the field path exists in mutation_paths array
        jq --arg type "$TYPE_NAME" --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | any(.path == $path)' "$FILE_PATH"
        ;;
    *)
        echo "Error: Unknown operation $OPERATION"
        echo "Valid operations: summary, discovered_count, type_info, mutation_paths, spawn_example, schema_info, validate_field"
        exit 1
        ;;
esac
