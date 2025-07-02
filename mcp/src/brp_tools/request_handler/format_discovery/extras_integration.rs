//! Integration with `bevy_brp_extras` for direct format discovery
//!
//! This module implements Level 2 of the recovery engine: direct discovery
//! via the `bevy_brp_extras/discover_format` method. This provides the most
//! authoritative type information available by querying the running Bevy app directly.
//!
//! # Key Functions
//!
//! - **`discover_type_format()`**: Main entry point for direct type discovery
//! - **`check_brp_extras_availability()`**: Verify `bevy_brp_extras` is available
//! - **Response Processing**: Convert `TypeDiscoveryResponse` → `UnifiedTypeInfo`
//!
//! # Recovery Integration
//!
//! This module is called by `recovery_engine.rs` Level 2 and provides:
//! - Complete type schemas with examples and mutation paths
//! - Real-world format data from the running Bevy application
//! - Authoritative answers to "how should I format this type?"

use serde_json::{Value, json};

use super::adapters::from_type_discovery_response_json;
use super::flow_types::CorrectionResult;
use super::unified_types::{CorrectionInfo, CorrectionMethod, UnifiedTypeInfo};
use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};

/// Discover format information for a type using `bevy_brp_extras`
///
/// This is the main entry point for Level 2 direct discovery. It calls
/// the running Bevy app's `bevy_brp_extras/discover_format` method to get
/// authoritative type information.
///
/// # Arguments
/// * `type_name` - Fully-qualified type name to discover
/// * `port` - BRP port for the discovery call
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `Result<Option<UnifiedTypeInfo>, String>` with discovered type info or error
pub async fn discover_type_format(
    type_name: &str,
    port: Option<u16>,
    debug_info: &mut Vec<String>,
) -> Result<Option<UnifiedTypeInfo>, String> {
    debug_info.push(format!(
        "Extras Integration: Starting discovery for type '{type_name}'"
    ));

    // Check if bevy_brp_extras is available
    if !check_brp_extras_availability(port, debug_info).await {
        debug_info.push("Extras Integration: bevy_brp_extras not available".to_string());
        return Ok(None);
    }

    // Call bevy_brp_extras/discover_format
    let params = json!({
        "types": [type_name]
    });

    debug_info.push(format!(
        "Extras Integration: Calling bevy_brp_extras/discover_format with params: {params}"
    ));

    match execute_brp_method("bevy_brp_extras/discover_format", Some(params), port).await {
        Ok(BrpResult::Success(Some(response_data))) => {
            debug_info.push(
                "Extras Integration: Received successful response from bevy_brp_extras".to_string(),
            );

            // Process the response to extract type information
            process_discovery_response(type_name, &response_data, debug_info)
        }
        Ok(BrpResult::Success(None)) => {
            debug_info.push("Extras Integration: Received empty success response".to_string());
            Ok(None)
        }
        Ok(BrpResult::Error(error)) => {
            debug_info.push(format!(
                "Extras Integration: BRP error: {} - {}",
                error.code, error.message
            ));
            Err(format!("BRP error {}: {}", error.code, error.message))
        }
        Err(e) => {
            debug_info.push(format!("Extras Integration: Network/connection error: {e}"));
            Err(format!("Connection error: {e}"))
        }
    }
}

/// Check if `bevy_brp_extras` is available on the target Bevy app
///
/// This function verifies that the connected Bevy app has `bevy_brp_extras`
/// installed and can respond to format discovery requests.
///
/// # Arguments
/// * `port` - BRP port to check
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `bool` indicating if `bevy_brp_extras` is available
pub async fn check_brp_extras_availability(
    port: Option<u16>,
    debug_info: &mut Vec<String>,
) -> bool {
    debug_info.push(
        "Extras Integration: Checking bevy_brp_extras availability via rpc.discover".to_string(),
    );

    // Use rpc.discover to list available methods
    match execute_brp_method("rpc.discover", None, port).await {
        Ok(BrpResult::Success(Some(response))) => {
            // Check if the response contains bevy_brp_extras methods
            if let Some(methods) = response.get("methods").and_then(|m| m.as_object()) {
                let has_discover_format = methods.contains_key("bevy_brp_extras/discover_format");

                debug_info.push(format!(
                    "Extras Integration: rpc.discover returned {} methods, bevy_brp_extras/discover_format present: {}",
                    methods.len(),
                    has_discover_format
                ));

                has_discover_format
            } else {
                debug_info.push(
                    "Extras Integration: rpc.discover response missing methods field".to_string(),
                );
                false
            }
        }
        Ok(BrpResult::Success(None)) => {
            debug_info.push("Extras Integration: rpc.discover returned empty response".to_string());
            false
        }
        Ok(BrpResult::Error(error)) => {
            debug_info.push(format!(
                "Extras Integration: rpc.discover error: {} - {}",
                error.code, error.message
            ));
            false
        }
        Err(e) => {
            debug_info.push(format!(
                "Extras Integration: Failed to call rpc.discover: {e}"
            ));
            false
        }
    }
}

/// Process the discovery response from `bevy_brp_extras`
///
/// This function takes the raw JSON response from `bevy_brp_extras/discover_format`
/// and converts it to a `UnifiedTypeInfo` using the schema adapters.
///
/// # Arguments
/// * `type_name` - The type that was discovered
/// * `response_data` - Raw JSON response from `bevy_brp_extras`
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `Result<Option<UnifiedTypeInfo>, String>` with processed type info
fn process_discovery_response(
    type_name: &str,
    response_data: &Value,
    debug_info: &mut Vec<String>,
) -> Result<Option<UnifiedTypeInfo>, String> {
    debug_info.push(format!(
        "Extras Integration: Processing discovery response for '{type_name}'"
    ));

    // The response should contain type information, possibly as an array or object
    // We need to find the entry for our specific type

    if let Some(type_data) = find_type_in_response(type_name, response_data) {
        debug_info.push(format!(
            "Extras Integration: Found type data for '{type_name}'"
        ));

        // Use the schema adapter to convert TypeDiscoveryResponse → UnifiedTypeInfo
        if let Some(unified_info) = from_type_discovery_response_json(type_data) {
            debug_info.push(format!(
                "Extras Integration: Successfully converted to UnifiedTypeInfo with {} mutation paths, {} examples",
                unified_info.format_info.mutation_paths.len(),
                unified_info.format_info.examples.len()
            ));
            Ok(Some(unified_info))
        } else {
            debug_info.push(
                "Extras Integration: Failed to convert response to UnifiedTypeInfo".to_string(),
            );
            Err("Failed to parse type discovery response".to_string())
        }
    } else {
        debug_info.push(format!(
            "Extras Integration: Type '{type_name}' not found in discovery response"
        ));
        Ok(None)
    }
}

/// Find type data in the discovery response
///
/// The `bevy_brp_extras` response format may vary, so this function handles
/// different possible response structures to locate the type data.
///
/// # Arguments
/// * `type_name` - Type name to find
/// * `response_data` - Raw response from `bevy_brp_extras`
///
/// # Returns
/// * `Option<&Value>` pointing to the type data if found
fn find_type_in_response<'a>(type_name: &str, response_data: &'a Value) -> Option<&'a Value> {
    // Try different possible response formats:

    // Format 1: Direct object with type name as key
    if let Some(obj) = response_data.as_object() {
        if let Some(type_data) = obj.get(type_name) {
            return Some(type_data);
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

/// Create a correction result from discovered type information
///
/// This function converts a `UnifiedTypeInfo` into a `CorrectionResult` that
/// can be used by the recovery engine to fix format errors.
///
/// # Arguments
/// * `type_info` - Discovered type information
/// * `original_value` - The original incorrect value (if available)
///
/// # Returns
/// * `CorrectionResult` with correction information
pub fn create_correction_from_discovery(
    type_info: UnifiedTypeInfo,
    original_value: Option<Value>,
) -> CorrectionResult {
    // If we have an example for the spawn operation, use it as the corrected format
    if let Some(spawn_example) = type_info.get_example("spawn") {
        let correction_info = CorrectionInfo {
            type_name:         type_info.type_name.clone(),
            original_value:    original_value.unwrap_or(json!(null)),
            corrected_value:   spawn_example.clone(),
            hint:              format!(
                "Use this format for type '{}' (discovered via bevy_brp_extras)",
                type_info.type_name
            ),
            type_info:         Some(type_info),
            correction_method: CorrectionMethod::DirectReplacement,
        };

        CorrectionResult::Applied { correction_info }
    } else {
        // No direct correction available, but we have useful metadata
        CorrectionResult::MetadataOnly {
            type_info,
            reason: "Type discovered but no format example available for correction".to_string(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::brp_tools::request_handler::format_discovery::DiscoverySource;

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
            CorrectionResult::Applied { correction_info } => {
                assert_eq!(correction_info.original_value, original);
                assert!(correction_info.corrected_value.get("translation").is_some());
                assert_eq!(
                    correction_info.correction_method,
                    CorrectionMethod::DirectReplacement
                );
            }
            CorrectionResult::MetadataOnly { .. } => {
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
            CorrectionResult::MetadataOnly { type_info, reason } => {
                assert_eq!(
                    type_info.type_name,
                    "bevy_transform::components::transform::Transform"
                );
                assert!(reason.contains("no format example"));
            }
            CorrectionResult::Applied { .. } => {
                unreachable!("Expected MetadataOnly correction result")
            }
        }
    }
}
