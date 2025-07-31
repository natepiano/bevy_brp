//! Integration with `bevy_brp_extras` for direct format discovery
//!
//! Level 2 recovery: queries running Bevy app via `bevy_brp_extras/discover_format`
//! for authoritative type schemas, examples, and mutation paths.

use serde_json::{Value, json};
use tracing::debug;

use super::super::format_correction_fields::FormatCorrectionField;
use super::adapters;
use super::flow_types::CorrectionResult;
use super::unified_types::{CorrectionInfo, CorrectionMethod, UnifiedTypeInfo};
use crate::brp_tools::{self, BrpClientResult, Port};
use crate::tool::BrpMethod;

/// Discover type format via `bevy_brp_extras/discover_format`
pub async fn discover_type_format(
    type_name: &str,
    port: Port,
) -> Result<Option<UnifiedTypeInfo>, String> {
    debug!("Extras Integration: Starting discovery for type '{type_name}'");

    // Call brp_extras/discover_format directly
    let params = json!({
        "types": [type_name]
    });

    debug!(
        "Extras Integration: Calling brp_extras/discover_format on port {port} with params: {params}"
    );

    let client = brp_tools::BrpClient::new(BrpMethod::BrpExtrasDiscoverFormat, port, Some(params));
    match client.execute_direct().await {
        Ok(BrpClientResult::Success(Some(response_data))) => {
            debug!("Extras Integration: Received successful response from brp_extras");

            // Process the response to extract type information
            process_discovery_response(type_name, &response_data)
        }
        Ok(BrpClientResult::Success(None)) => {
            debug!("Extras Integration: Received empty success response");
            Ok(None)
        }
        Ok(BrpClientResult::Error(error)) => {
            debug!(
                "Extras Integration: brp_extras/discover_format failed: {} - {}",
                error.code, error.message
            );
            Ok(None) // Return None instead of Err - this just means brp_extras is not available
        }
        Err(e) => {
            debug!("Extras Integration: Connection error calling brp_extras/discover_format: {e}");
            Ok(None) // Return None instead of Err - this just means brp_extras is not available
        }
    }
}

/// Convert discovery response to `UnifiedTypeInfo`
fn process_discovery_response(
    type_name: &str,
    response_data: &Value,
) -> Result<Option<UnifiedTypeInfo>, String> {
    debug!("Extras Integration: Processing discovery response for '{type_name}'");
    debug!(
        "Extras Integration: Full response data: {}",
        serde_json::to_string_pretty(response_data)
            .unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // The response should contain type information, possibly as an array or object
    // We need to find the entry for our specific type

    find_type_in_response(type_name, response_data).map_or_else(|| {
        debug!("Extras Integration: Type '{type_name}' not found in discovery response");
        Ok(None)
    }, |type_data| {
        debug!("Extras Integration: Found type data for '{type_name}'");

        // Use the schema adapter to convert TypeDiscoveryResponse → UnifiedTypeInfo
        if let Some(unified_info) = adapters::from_type_discovery_response_json(type_data) {
            debug!(
                "Extras Integration: Successfully converted to UnifiedTypeInfo with {} mutation paths, {} examples",
                unified_info.format_info.mutation_paths.len(),
                unified_info.format_info.examples.len()
            );
            Ok(Some(unified_info))
        } else {
            debug!("Extras Integration: Failed to convert response to UnifiedTypeInfo");
            Err("Failed to parse type discovery response".to_string())
        }
    })
}

/// Find type in response (handles various response formats)
fn find_type_in_response<'a>(type_name: &str, response_data: &'a Value) -> Option<&'a Value> {
    debug!("Extras Integration: find_type_in_response looking for '{type_name}'");

    // Try different possible response formats:

    // Format 1: Direct object with type name as key
    if let Some(obj) = response_data.as_object() {
        debug!(
            "Extras Integration: Trying Format 1 - direct object keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        if let Some(type_data) = obj.get(type_name) {
            debug!("Extras Integration: Found type data in Format 1");
            return Some(type_data);
        }

        // Format 1b: Check if there's a type_info field
        if let Some(type_info) = obj.get("type_info").and_then(Value::as_object) {
            debug!(
                "Extras Integration: Found type_info field, checking keys: {:?}",
                type_info.keys().collect::<Vec<_>>()
            );
            if let Some(type_data) = type_info.get(type_name) {
                debug!("Extras Integration: Found type data in type_info field");
                return Some(type_data);
            }
        }
    }

    // Format 2: Array of type objects with type_name field
    if let Some(arr) = response_data.as_array() {
        for item in arr {
            if let Some(item_type_name) = item.get("type_name").and_then(Value::as_str) {
                if item_type_name == type_name {
                    return Some(item);
                }
            }
        }
    }

    // Format 3: Single type object (if we requested only one type)
    if let Some(item_type_name) = response_data.get("type_name").and_then(Value::as_str) {
        if item_type_name == type_name {
            return Some(response_data);
        }
    }

    // Format 4: Nested under "types" key
    if let Some(types) = response_data.get("types") {
        return find_type_in_response(type_name, types);
    }

    None
}

/// Create correction from discovered type info
pub fn create_correction_from_discovery(
    mut type_info: UnifiedTypeInfo,
    original_value: Option<Value>,
) -> CorrectionResult {
    // Ensure examples are generated
    type_info.ensure_examples();

    // Check if this is an enum with variants - create enum-specific correction
    if let Some(enum_info) = &type_info.enum_info {
        let variant_names: Vec<String> =
            enum_info.variants.iter().map(|v| v.name.clone()).collect();

        let corrected_format = json!({
            FormatCorrectionField::Hint.as_ref(): "Use empty path with variant name as value",
            FormatCorrectionField::ValidValues.as_ref(): variant_names,
            FormatCorrectionField::Examples.as_ref(): variant_names.iter().take(2).map(|variant| json!({
                FormatCorrectionField::Path.as_ref(): "",
                FormatCorrectionField::Value.as_ref(): variant
            })).collect::<Vec<_>>()
        });

        let correction_info = CorrectionInfo {
            type_name:         type_info.type_name.clone(),
            original_value:    original_value.unwrap_or(json!(null)),
            corrected_value:   corrected_format.clone(),
            corrected_format:  Some(corrected_format),
            hint:              format!(
                "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}",
                type_info
                    .type_name
                    .split("::")
                    .last()
                    .unwrap_or(&type_info.type_name),
                variant_names.join(", ")
            ),
            target_type:       type_info.type_name.clone(),
            type_info:         Some(type_info),
            correction_method: CorrectionMethod::DirectReplacement,
        };

        return CorrectionResult::Corrected { correction_info };
    }

    // Check if we can actually transform the original input
    if let Some(original_value) = original_value {
        debug!(
            "Extras Integration: Attempting to transform original value: {}",
            serde_json::to_string(&original_value).unwrap_or_else(|_| "invalid json".to_string())
        );
        if let Some(transformed_value) = type_info.transform_value(&original_value) {
            debug!(
                "Extras Integration: Successfully transformed value to: {}",
                serde_json::to_string(&transformed_value)
                    .unwrap_or_else(|_| "invalid json".to_string())
            );
            // We can transform the input - return Corrected with actual transformation
            let correction_info = CorrectionInfo {
                type_name:         type_info.type_name.clone(),
                original_value:    original_value.clone(),
                corrected_value:   transformed_value,
                hint:              format!(
                    "Transformed {} format for type '{}' (discovered via bevy_brp_extras)",
                    if original_value.is_object() {
                        "object"
                    } else {
                        "value"
                    },
                    type_info.type_name
                ),
                target_type:       type_info.type_name.clone(),
                corrected_format:  None,
                type_info:         Some(type_info),
                correction_method: CorrectionMethod::ObjectToArray,
            };

            return CorrectionResult::Corrected { correction_info };
        }
        debug!("Extras Integration: transform_value() returned None - cannot transform input");
    } else {
        debug!("Extras Integration: No original value provided for transformation");
    }

    // Cannot transform input - provide guidance with examples
    let reason = if let Some(spawn_example) = type_info.get_example("spawn") {
        format!(
            "Cannot transform input for type '{}'. Use this format: {}",
            type_info.type_name,
            serde_json::to_string(&spawn_example).unwrap_or_else(|_| "correct format".to_string())
        )
    } else {
        format!(
            "Cannot transform input for type '{}'. Type discovered but no format example available.",
            type_info.type_name
        )
    };

    CorrectionResult::CannotCorrect { type_info, reason }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::brp_tools::format_discovery::unified_types::{DiscoverySource, TypeCategory};

    #[test]
    fn test_find_type_in_response_direct_object() {
        let response = json!({
            "bevy_transform::components::transform::Transform": {
                "type_name": "bevy_transform::components::transform::Transform",
                "in_registry": true,
                "has_serialize": true
            }
        });

        let result = find_type_in_response(
            "bevy_transform::components::transform::Transform",
            &response,
        );
        assert!(result.is_some());
        let result_data = result.expect("Expected to find type in response");
        let type_name = result_data
            .get("type_name")
            .expect("Expected type_name field");
        assert_eq!(
            type_name
                .as_str()
                .expect("Expected type_name to be a string"),
            "bevy_transform::components::transform::Transform"
        );
    }

    #[test]
    fn test_find_type_in_response_array() {
        let response = json!([
            {
                "type_name": "bevy_transform::components::transform::Transform",
                "in_registry": true,
                "has_serialize": true
            },
            {
                "type_name": "bevy_core::name::Name",
                "in_registry": true,
                "has_serialize": false
            }
        ]);

        let result = find_type_in_response("bevy_core::name::Name", &response);
        assert!(result.is_some());
        let result_data = result.expect("Expected to find type in response");
        let type_name = result_data
            .get("type_name")
            .expect("Expected type_name field");
        assert_eq!(
            type_name
                .as_str()
                .expect("Expected type_name to be a string"),
            "bevy_core::name::Name"
        );
    }

    #[test]
    fn test_find_type_in_response_single_object() {
        let response = json!({
            "type_name": "bevy_transform::components::transform::Transform",
            "in_registry": true,
            "has_serialize": true
        });

        let result = find_type_in_response(
            "bevy_transform::components::transform::Transform",
            &response,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_find_type_in_response_not_found() {
        let response = json!({
            "other_type": {
                "type_name": "other_type",
                "in_registry": true
            }
        });

        let result = find_type_in_response(
            "bevy_transform::components::transform::Transform",
            &response,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_create_correction_from_discovery_with_example() {
        let mut type_info = UnifiedTypeInfo::new(
            "bevy_transform::components::transform::Transform".to_string(),
            DiscoverySource::DirectDiscovery,
        );
        type_info.type_category = TypeCategory::Struct;
        type_info.format_info.examples.insert(
            "spawn".to_string(),
            json!({
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }),
        );

        let original = json!({"translation": {"x": 0.0, "y": 0.0, "z": 0.0}});
        let result = create_correction_from_discovery(type_info, Some(original.clone()));

        match result {
            CorrectionResult::Corrected { correction_info } => {
                assert_eq!(correction_info.original_value, original);
                assert!(correction_info.corrected_value.get("translation").is_some());
                assert_eq!(
                    correction_info.correction_method,
                    CorrectionMethod::ObjectToArray
                );
            }
            CorrectionResult::CannotCorrect { .. } => {
                unreachable!("Expected Applied correction result")
            }
        }
    }

    #[test]
    fn test_create_correction_from_discovery_metadata_only() {
        let type_info = UnifiedTypeInfo::new(
            "bevy_transform::components::transform::Transform".to_string(),
            DiscoverySource::DirectDiscovery,
        );

        let result = create_correction_from_discovery(type_info, None);

        match result {
            CorrectionResult::CannotCorrect { type_info, reason } => {
                assert_eq!(
                    type_info.type_name,
                    "bevy_transform::components::transform::Transform"
                );
                assert!(reason.contains("no format example"));
            }
            CorrectionResult::Corrected { .. } => {
                unreachable!("Expected MetadataOnly correction result")
            }
        }
    }
}
