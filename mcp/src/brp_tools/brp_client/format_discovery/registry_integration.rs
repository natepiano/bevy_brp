//! Integration with Bevy's type registry for Level 1 format discovery
//!
//! Provides fast registry and serialization trait checks to quickly identify
//! if types are BRP-compatible. Called by recovery engine Level 1 for early
//! bailout on unsupported types and educational responses.

use serde_json::{Value, json};
use tracing::debug;

use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::{self, Port, ResponseStatus};
use crate::tool::BrpMethod;

/// Find type in registry response (handles various response formats)
fn find_type_in_registry_response(type_name: &str, response_data: &Value) -> Option<Value> {
    debug!("Registry Integration: Searching for '{type_name}' in registry response");

    // Try different possible response formats:

    // Format 1: Direct object with type name as key
    if let Some(obj) = response_data.as_object() {
        if let Some(type_data) = obj.get(type_name) {
            debug!("Registry Integration: Found '{type_name}' as direct key");
            return Some(type_data.clone());
        }
    }

    // Format 2: Array of type objects with typePath field
    if let Some(arr) = response_data.as_array() {
        for item in arr {
            if let Some(item_type_path) = item.get("typePath").and_then(Value::as_str) {
                if item_type_path == type_name {
                    debug!("Registry Integration: Found '{type_name}' in array by typePath");
                    return Some(item.clone());
                }
            }
            // Also check shortPath for convenience
            if let Some(item_short_path) = item.get("shortPath").and_then(Value::as_str) {
                if item_short_path == type_name {
                    debug!("Registry Integration: Found '{type_name}' in array by shortPath");
                    return Some(item.clone());
                }
            }
        }
    }

    // Format 3: Single type object (if we requested only one type)
    if let Some(item_type_path) = response_data.get("typePath").and_then(Value::as_str) {
        if item_type_path == type_name {
            debug!("Registry Integration: Found '{type_name}' as single object");
            return Some(response_data.clone());
        }
    }

    // Format 4: Nested under specific keys
    for key in ["types", "schemas", "data"] {
        if let Some(nested) = response_data.get(key) {
            if let Some(result) = find_type_in_registry_response(type_name, nested) {
                return Some(result);
            }
        }
    }

    debug!("Registry Integration: Type '{type_name}' not found in any expected format");
    None
}

/// Batch check multiple types in a single registry call
pub async fn check_multiple_types_registry_status(
    type_names: &[String],
    port: Port,
) -> Vec<(String, Option<UnifiedTypeInfo>)> {
    debug!(
        "Registry Integration: Batch checking {} types",
        type_names.len()
    );

    // Extract unique crate names from type paths for filtering
    let mut crate_names: Vec<String> = type_names
        .iter()
        .filter_map(|type_name| {
            type_name
                .split("::")
                .next()
                .map(std::string::ToString::to_string)
        })
        .collect();
    crate_names.sort_unstable();
    crate_names.dedup();

    // Call registry_schema with crate names
    let params = json!({
        "with_crates": crate_names
    });

    debug!("Registry Integration: Batch call with params: {params}");

    let client = brp_tools::BrpClient::new(BrpMethod::BevyRegistrySchema, port, Some(params));
    match client.execute_raw().await {
        Ok(ResponseStatus::Success(Some(response_data))) => {
            debug!("Registry Integration: Received successful batch response");

            // Process each type in the response
            let mut results = Vec::new();
            for type_name in type_names {
                if let Some(schema_data) = find_type_in_registry_response(type_name, &response_data)
                {
                    let type_info = UnifiedTypeInfo::from_registry_schema(type_name, &schema_data);
                    results.push((type_name.clone(), Some(type_info)));
                } else {
                    debug!("Registry Integration: Type '{type_name}' not found in batch response");
                    results.push((type_name.clone(), None));
                }
            }
            results
        }
        Ok(ResponseStatus::Success(None) | ResponseStatus::Error(_)) | Err(_) => {
            debug!("Registry Integration: Batch registry check failed");
            type_names.iter().map(|name| (name.clone(), None)).collect()
        }
    }
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

        let result = find_type_in_registry_response(
            "bevy_transform::components::transform::Transform",
            &response,
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

        let result = find_type_in_registry_response("bevy_core::name::Name", &response);

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

        let result = find_type_in_registry_response(
            "bevy_transform::components::transform::Transform",
            &response,
        );

        assert!(result.is_none());
    }
}
