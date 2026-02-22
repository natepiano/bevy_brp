#!/bin/bash
# Unified query result validation script
# Usage: query_validate.sh <command> <result_file> [args...]
#
# Commands:
#   has_entity <entity_id>                    - Check entity exists
#   has_component <entity_id> <component>     - Check entity has component
#   no_component <entity_id> <component>      - Check entity lacks component
#   count_entities                             - Print entity count
#   has_camera_excluded                        - Check no Camera entities in results
#   validate_all_query                         - Verify "all" query returns entities with multiple components
#   validate_name_filter                       - Verify all entities have Name component and multiple components

set -euo pipefail

command="$1"
result_file="$2"
shift 2

case "${command}" in
    has_entity)
        entity_id="$1"
        if jq -e ".[] | select(.entity == ${entity_id})" "${result_file}" > /dev/null 2>&1; then
            echo "✅ Entity ${entity_id} found"
            exit 0
        else
            echo "❌ Entity ${entity_id} not found"
            exit 1
        fi
        ;;

    has_component)
        entity_id="$1"
        component="$2"
        if jq -e ".[] | select(.entity == ${entity_id}) | .components.\"${component}\"" "${result_file}" > /dev/null 2>&1; then
            echo "✅ Entity ${entity_id} has ${component}"
            exit 0
        else
            echo "❌ Entity ${entity_id} missing ${component}"
            exit 1
        fi
        ;;

    no_component)
        entity_id="$1"
        component="$2"
        if jq -e ".[] | select(.entity == ${entity_id}) | .components.\"${component}\"" "${result_file}" > /dev/null 2>&1; then
            echo "❌ Entity ${entity_id} unexpectedly has ${component}"
            exit 1
        else
            echo "✅ Entity ${entity_id} correctly lacks ${component}"
            exit 0
        fi
        ;;

    count_entities)
        count=$(jq 'length' "${result_file}")
        echo "${count}"
        exit 0
        ;;

    has_camera_excluded)
        # Check if any entity has Camera component
        if jq -e '.[] | select(.components."bevy_camera::camera::Camera")' "${result_file}" > /dev/null 2>&1; then
            echo "❌ Found Camera entity (should be excluded)"
            exit 1
        else
            echo "✅ Camera entities correctly excluded"
            exit 0
        fi
        ;;

    validate_all_query)
        # Check that results contain entities with multiple components
        entity_count=$(jq 'length' "${result_file}")

        # Check at least one entity has multiple component types
        multi_component=$(jq '[.[] | select((.components | length) > 1)] | length' "${result_file}")

        if [ "${multi_component}" -gt 0 ]; then
            echo "✅ Query returned ${entity_count} entities, ${multi_component} with multiple components"
            exit 0
        else
            echo "❌ No entities with multiple components found (expected from 'all' query)"
            exit 1
        fi
        ;;

    validate_name_filter)
        # Check all entities have Name component
        entity_count=$(jq 'length' "${result_file}")
        missing_name=$(jq '[.[] | select(.components."bevy_ecs::name::Name" == null)] | length' "${result_file}")

        if [ "${missing_name}" -gt 0 ]; then
            echo "❌ Found ${missing_name} entities without Name component"
            exit 1
        fi

        # Check entities have multiple components (verifying "all" option works)
        multi_component=$(jq '[.[] | select((.components | length) > 1)] | length' "${result_file}")

        if [ "${multi_component}" -gt 0 ]; then
            echo "✅ All ${entity_count} entities have Name, ${multi_component} with multiple components"
            exit 0
        else
            echo "❌ No entities with multiple components found"
            exit 1
        fi
        ;;

    *)
        echo "Unknown command: ${command}"
        echo "Available commands: has_entity, has_component, no_component, count_entities, has_camera_excluded, validate_all_query, validate_name_filter"
        exit 1
        ;;
esac
