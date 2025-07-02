//! Integration with Bevy's type registry for Level 1 format discovery
//!
//! This module implements Level 1 of the recovery engine: fast registry and
//! serialization checks that can quickly identify if types are suitable for
//! BRP operations without expensive discovery calls.
//!
//! # Key Functions
//!
//! - **`check_type_registry_status()`**: Main entry point for registry checks
//! - **`fetch_registry_schema()`**: Get type schema from `bevy/registry_schema`
//! - **Schema Processing**: Convert registry data → `UnifiedTypeInfo`
//!
//! # Recovery Integration
//!
//! This module is called by `recovery_engine.rs` Level 1 and provides:
//! - Fast bailout for types not in registry
//! - Early educational responses for missing Serialize/Deserialize traits
//! - Basic type information for downstream levels

use serde_json::{Value, json};

use super::adapters::from_registry_schema;
use super::flow_types::CorrectionResult;
use super::unified_types::{
    DiscoverySource, RegistryStatus, SerializationSupport, UnifiedTypeInfo,
};
use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};

/// Check the registry status of a type for Level 1 recovery
///
/// This is the main entry point for Level 1 registry checks. It determines
/// if a type is registered in Bevy's type registry and has the required
/// serialization traits for BRP operations.
///
/// # Arguments
/// * `type_name` - Fully-qualified type name to check
/// * `port` - BRP port for registry queries
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `Ok(Some(UnifiedTypeInfo))` - Type found with registry information
/// * `Ok(None)` - Type not found or missing required traits
/// * `Err(String)` - Registry query failed
#[allow(dead_code)]
pub async fn check_type_registry_status(
    type_name: &str,
    port: Option<u16>,
    debug_info: &mut Vec<String>,
) -> Result<Option<UnifiedTypeInfo>, String> {
    debug_info.push(format!(
        "Registry Integration: Checking registry status for type '{type_name}'"
    ));

    // First, try to fetch the registry schema for this type
    match fetch_registry_schema(type_name, port, debug_info).await {
        Ok(Some(schema_data)) => {
            debug_info.push(format!(
                "Registry Integration: Found registry schema for '{type_name}'"
            ));

            // Convert the registry schema to UnifiedTypeInfo
            let type_info = from_registry_schema(type_name, &schema_data);

            // Check if the type has the required serialization traits
            if type_info.serialization.brp_compatible {
                debug_info.push(format!(
                    "Registry Integration: Type '{type_name}' is BRP compatible"
                ));
            } else {
                debug_info.push(format!(
                    "Registry Integration: Type '{type_name}' lacks required serialization traits"
                ));
            }
            Ok(Some(type_info))
        }
        Ok(None) => {
            debug_info.push(format!(
                "Registry Integration: Type '{type_name}' not found in registry"
            ));
            Ok(None)
        }
        Err(e) => {
            debug_info.push(format!("Registry Integration: Registry query failed: {e}"));
            Err(e)
        }
    }
}

/// Fetch registry schema data for a specific type
///
/// This function calls the `bevy/registry_schema` method to get type information
/// directly from Bevy's type registry.
///
/// # Arguments
/// * `type_name` - Type name to look up
/// * `port` - BRP port for the query
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `Ok(Some(Value))` - Schema data found
/// * `Ok(None)` - Type not found in registry
/// * `Err(String)` - Query failed
#[allow(dead_code)]
pub async fn fetch_registry_schema(
    type_name: &str,
    port: Option<u16>,
    debug_info: &mut Vec<String>,
) -> Result<Option<Value>, String> {
    debug_info.push(format!(
        "Registry Integration: Fetching schema for '{type_name}' via bevy/registry_schema"
    ));

    // Build parameters for registry schema query
    let params = json!({
        "with_types": [type_name]
    });

    debug_info.push(format!(
        "Registry Integration: Calling bevy/registry_schema with params: {params}"
    ));

    match execute_brp_method("bevy/registry_schema", Some(params), port).await {
        Ok(BrpResult::Success(Some(response_data))) => {
            debug_info.push(
                "Registry Integration: Received successful response from bevy/registry_schema"
                    .to_string(),
            );

            // Process the response to find our specific type
            find_type_in_registry_response(type_name, &response_data, debug_info).map_or_else(
                || {
                    debug_info.push(format!(
                        "Registry Integration: Type '{type_name}' not found in registry response"
                    ));
                    Ok(None)
                },
                |schema_data| Ok(Some(schema_data)),
            )
        }
        Ok(BrpResult::Success(None)) => {
            debug_info.push("Registry Integration: Received empty success response".to_string());
            Ok(None)
        }
        Ok(BrpResult::Error(error)) => {
            debug_info.push(format!(
                "Registry Integration: BRP error: {} - {}",
                error.code, error.message
            ));
            Err(format!("BRP error {}: {}", error.code, error.message))
        }
        Err(e) => {
            debug_info.push(format!(
                "Registry Integration: Network/connection error: {e}"
            ));
            Err(format!("Connection error: {e}"))
        }
    }
}

/// Find a specific type in the registry schema response
///
/// The registry schema response format may vary, so this function handles
/// different possible response structures to locate the type data.
///
/// # Arguments
/// * `type_name` - Type name to find
/// * `response_data` - Raw response from `bevy/registry_schema`
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `Some(Value)` - Schema data for the type if found
/// * `None` - Type not found in response
fn find_type_in_registry_response(
    type_name: &str,
    response_data: &Value,
    debug_info: &mut Vec<String>,
) -> Option<Value> {
    debug_info.push(format!(
        "Registry Integration: Searching for '{type_name}' in registry response"
    ));

    // Try different possible response formats:

    // Format 1: Direct object with type name as key
    if let Some(obj) = response_data.as_object() {
        if let Some(type_data) = obj.get(type_name) {
            debug_info.push(format!(
                "Registry Integration: Found '{type_name}' as direct key"
            ));
            return Some(type_data.clone());
        }
    }

    // Format 2: Array of type objects with typePath field
    if let Some(arr) = response_data.as_array() {
        for item in arr {
            if let Some(item_type_path) = item.get("typePath").and_then(Value::as_str) {
                if item_type_path == type_name {
                    debug_info.push(format!(
                        "Registry Integration: Found '{type_name}' in array by typePath"
                    ));
                    return Some(item.clone());
                }
            }
            // Also check shortPath for convenience
            if let Some(item_short_path) = item.get("shortPath").and_then(Value::as_str) {
                if item_short_path == type_name {
                    debug_info.push(format!(
                        "Registry Integration: Found '{type_name}' in array by shortPath"
                    ));
                    return Some(item.clone());
                }
            }
        }
    }

    // Format 3: Single type object (if we requested only one type)
    if let Some(item_type_path) = response_data.get("typePath").and_then(Value::as_str) {
        if item_type_path == type_name {
            debug_info.push(format!(
                "Registry Integration: Found '{type_name}' as single object"
            ));
            return Some(response_data.clone());
        }
    }

    // Format 4: Nested under specific keys
    for key in ["types", "schemas", "data"] {
        if let Some(nested) = response_data.get(key) {
            if let Some(result) = find_type_in_registry_response(type_name, nested, debug_info) {
                return Some(result);
            }
        }
    }

    debug_info.push(format!(
        "Registry Integration: Type '{type_name}' not found in any expected format"
    ));
    None
}

/// Create an educational correction result for registry issues
///
/// This function creates educational responses when types are found in the registry
/// but have issues (like missing serialization traits) that prevent BRP operations.
///
/// # Arguments
/// * `type_info` - Type information with identified issues
/// * `issue_message` - Description of the specific issue
///
/// # Returns
/// * `CorrectionResult::MetadataOnly` with educational information
#[allow(dead_code)]
pub const fn create_educational_correction(
    type_info: UnifiedTypeInfo,
    issue_message: String,
) -> CorrectionResult {
    CorrectionResult::MetadataOnly {
        type_info,
        reason: issue_message,
    }
}

/// Batch check registry status for multiple types
///
/// This function efficiently checks multiple types in a single registry call,
/// which is more efficient than individual checks.
///
/// # Arguments
/// * `type_names` - List of type names to check
/// * `port` - BRP port for registry queries
/// * `debug_info` - Debug information collector
///
/// # Returns
/// * `Vec<(String, Option<UnifiedTypeInfo>)>` with results for each type
pub async fn check_multiple_types_registry_status(
    type_names: &[String],
    port: Option<u16>,
    debug_info: &mut Vec<String>,
) -> Vec<(String, Option<UnifiedTypeInfo>)> {
    debug_info.push(format!(
        "Registry Integration: Batch checking {} types",
        type_names.len()
    ));

    // Call registry_schema with all types at once
    let params = json!({
        "with_types": type_names
    });

    debug_info.push(format!(
        "Registry Integration: Batch call with params: {params}"
    ));

    match execute_brp_method("bevy/registry_schema", Some(params), port).await {
        Ok(BrpResult::Success(Some(response_data))) => {
            debug_info.push("Registry Integration: Received successful batch response".to_string());

            // Process each type in the response
            let mut results = Vec::new();
            for type_name in type_names {
                if let Some(schema_data) =
                    find_type_in_registry_response(type_name, &response_data, debug_info)
                {
                    let type_info = from_registry_schema(type_name, &schema_data);
                    results.push((type_name.clone(), Some(type_info)));
                } else {
                    debug_info.push(format!(
                        "Registry Integration: Type '{type_name}' not found in batch response"
                    ));
                    results.push((type_name.clone(), None));
                }
            }
            results
        }
        Ok(BrpResult::Success(None) | BrpResult::Error(_)) | Err(_) => {
            debug_info.push("Registry Integration: Batch registry check failed".to_string());
            type_names.iter().map(|name| (name.clone(), None)).collect()
        }
    }
}

/// Create a minimal `UnifiedTypeInfo` for types not in registry
///
/// This helper creates a basic type info structure for types that aren't
/// found in the registry, useful for educational responses.
///
/// # Arguments
/// * `type_name` - The type name
///
/// # Returns
/// * `UnifiedTypeInfo` with minimal information
#[allow(dead_code)]
pub fn create_unregistered_type_info(type_name: &str) -> UnifiedTypeInfo {
    UnifiedTypeInfo {
        type_name:            type_name.to_string(),
        registry_status:      RegistryStatus::not_in_registry(),
        serialization:        SerializationSupport::no_support(),
        format_info:          super::unified_types::FormatInfo::empty(),
        supported_operations: Vec::new(),
        type_category:        "Unknown".to_string(),
        child_types:          std::collections::HashMap::new(),
        enum_info:            None,
        discovery_source:     DiscoverySource::TypeRegistry,
    }
}

/// Extract type name from common type formats
///
/// This helper function normalizes different type name formats that might
/// be encountered in registry queries.
///
/// # Arguments
/// * `type_name` - Raw type name input
///
/// # Returns
/// * Normalized type name for registry lookup
#[allow(dead_code)]
pub fn normalize_type_name(type_name: &str) -> String {
    // Handle common type name variations
    type_name.trim().replace(' ', "") // Remove spaces
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_find_type_in_registry_response_direct_key() {
        let response = json!({
            "bevy_transform::components::transform::Transform": {
                "typePath": "bevy_transform::components::transform::Transform",
                "reflectTypes": ["Component", "Serialize", "Deserialize"]
            }
        });

        let mut debug_info = Vec::new();
        let result = find_type_in_registry_response(
            "bevy_transform::components::transform::Transform",
            &response,
            &mut debug_info,
        );

        assert!(result.is_some());
        let result = result.expect("Expected to find type in registry response");
        let type_path = result
            .get("typePath")
            .and_then(|v| v.as_str())
            .expect("Expected typePath to be a string");
        assert_eq!(
            type_path,
            "bevy_transform::components::transform::Transform"
        );
    }

    #[test]
    fn test_find_type_in_registry_response_array_format() {
        let response = json!([
            {
                "typePath": "bevy_transform::components::transform::Transform",
                "shortPath": "Transform",
                "reflectTypes": ["Component", "Serialize", "Deserialize"]
            },
            {
                "typePath": "bevy_core::name::Name",
                "shortPath": "Name",
                "reflectTypes": ["Component"]
            }
        ]);

        let mut debug_info = Vec::new();
        let result =
            find_type_in_registry_response("bevy_core::name::Name", &response, &mut debug_info);

        assert!(result.is_some());
        let result = result.expect("Expected to find type in registry response");
        let type_path = result
            .get("typePath")
            .and_then(|v| v.as_str())
            .expect("Expected typePath to be a string");
        assert_eq!(type_path, "bevy_core::name::Name");
    }

    #[test]
    fn test_find_type_in_registry_response_not_found() {
        let response = json!({
            "other_type": {
                "typePath": "other::Type"
            }
        });

        let mut debug_info = Vec::new();
        let result = find_type_in_registry_response(
            "bevy_transform::components::transform::Transform",
            &response,
            &mut debug_info,
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_create_unregistered_type_info() {
        let type_info = create_unregistered_type_info("some::Type");

        assert_eq!(type_info.type_name, "some::Type");
        assert!(!type_info.registry_status.in_registry);
        assert!(!type_info.serialization.brp_compatible);
        assert_eq!(type_info.discovery_source, DiscoverySource::TypeRegistry);
    }

    #[test]
    fn test_normalize_type_name() {
        assert_eq!(normalize_type_name("  my::Type  "), "my::Type");
        assert_eq!(normalize_type_name("my :: Type"), "my::Type");
        assert_eq!(normalize_type_name("NormalType"), "NormalType");
    }

    #[test]
    fn test_create_educational_correction() {
        let type_info = create_unregistered_type_info("test::Type");
        let correction =
            create_educational_correction(type_info, "Type lacks Serialize trait".to_string());

        match correction {
            CorrectionResult::MetadataOnly { type_info, reason } => {
                assert_eq!(type_info.type_name, "test::Type");
                assert_eq!(reason, "Type lacks Serialize trait");
            }
            CorrectionResult::Applied { .. } => {
                unreachable!("Expected MetadataOnly correction result")
            }
        }
    }
}
