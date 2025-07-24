//! Public API and request handling for format discovery
//!
//! This module provides the public API functions and handles BRP requests
//! for format discovery operations.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::remote::{BrpError, BrpResult, error_codes};
use serde_json::{Value, json};

use super::core::{
    discover_multiple_formats, discover_type_as_response, get_common_component_types,
};
use super::error::DebugContext;
use super::types::DiscoveryInfo;

/// Discover format information for a single component type (public API)
///
/// Returns `None` if the type is not found or cannot be processed.
/// For detailed error information, use `discover_component_format_with_context`.
pub fn discover_component_format_simple(world: &World, type_name: &str) -> Option<DiscoveryInfo> {
    let mut debug_context = DebugContext::new();
    super::core::discover_component_format(world, type_name, &mut debug_context).ok()
}

/// Discover format information for multiple component types (public API)
///
/// This is the main entry point for batch format discovery operations.
pub fn discover_multiple_formats_public(
    world: &World,
    type_names: &[String],
) -> super::core::MultiDiscoveryResult {
    discover_multiple_formats(world, type_names)
}

/// Create a BRP error for invalid parameters
fn invalid_params_error(message: &str) -> BrpError {
    BrpError {
        code:    error_codes::INVALID_PARAMS,
        message: message.to_string(),
        data:    None,
    }
}

/// Extract type names from a JSON value
fn extract_type_names(value: &Value) -> Result<Vec<String>, BrpError> {
    match value {
        Value::Array(arr) => Ok(arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .collect()),
        Value::String(s) => Ok(vec![s.clone()]),
        _ => Err(invalid_params_error(
            "Parameter 'types' must be a string or array of strings",
        )),
    }
}

/// Parse the types parameter from BRP request parameters
fn parse_types_parameter(params: Option<Value>) -> Result<Vec<String>, BrpError> {
    const MISSING_TYPES_MSG: &str = "Missing required 'types' parameter. Specify component types to get format information for.";

    let params = params.ok_or_else(|| invalid_params_error(MISSING_TYPES_MSG))?;
    let types = params
        .get("types")
        .ok_or_else(|| invalid_params_error(MISSING_TYPES_MSG))?;
    let type_names = extract_type_names(types)?;

    if type_names.is_empty() {
        return Err(invalid_params_error(
            "At least one type must be specified in the 'types' parameter",
        ));
    }

    Ok(type_names)
}

/// Parse the debug parameter from BRP request parameters
fn parse_debug_parameter(params: Option<&Value>) -> bool {
    params
        .and_then(|p| p.get("enable_debug_info"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Get common component types (convenience function for API users)
#[must_use]
pub fn get_common_component_types_public() -> Vec<String> {
    get_common_component_types()
}

/// Handler for factual format discovery BRP requests using `TypeDiscoveryResponse`
///
/// This handler returns factual information about types instead of placeholder examples
pub fn factual_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Parse parameters - types parameter is required
    let type_names = parse_types_parameter(params.clone())?;

    // Parse debug parameter (defaults to false)
    let include_debug = parse_debug_parameter(params.as_ref());

    let mut debug_info = DebugContext::new();
    debug_info.push(format!(
        "Processing factual request for {} types",
        type_names.len()
    ));
    debug_info.push(format!("Debug mode enabled: {include_debug}"));

    // Discover formats for the requested types using new response format
    let mut responses = HashMap::new();
    let mut successful_discoveries = 0;
    let mut failed_discoveries = 0;

    for type_name in &type_names {
        debug_info.push(format!("Processing type: {type_name}"));

        let mut type_debug_context = DebugContext::new();
        let type_response = discover_type_as_response(world, type_name, &mut type_debug_context);

        // Count successes and failures based on in_registry field
        if type_response.in_registry {
            successful_discoveries += 1;
            debug_info.push(format!("Successfully discovered type: {type_name}"));
        } else {
            failed_discoveries += 1;
            debug_info.push(format!(
                "Failed to discover type: {type_name} - not in registry"
            ));
        }

        if include_debug {
            debug_info.messages.extend(type_debug_context.messages);
        }
        responses.insert(type_name.clone(), type_response);
    }

    // Convert responses to JSON values, creating minimal responses for errors
    let mut type_info_json = serde_json::Map::new();
    for (type_name, type_response) in responses {
        let json_value = if let Some(ref error_msg) = type_response.error {
            // Minimal response for errors
            json!({
                "type_name": type_response.type_name,
                "in_registry": type_response.in_registry,
                "error": error_msg
            })
        } else {
            // Full response for successful discoveries
            serde_json::to_value(&type_response).unwrap_or_else(|_| json!({}))
        };
        type_info_json.insert(type_name, json_value);
    }

    // Create comprehensive response
    let mut response = json!({
        "success": true,
        "type_info": type_info_json,
        "requested_types": type_names,
        "discovered_count": successful_discoveries  // Only count successful discoveries
    });

    // Add debug info if enabled
    if include_debug && !debug_info.messages.is_empty() {
        response["debug_info"] = json!(debug_info.messages);
    }

    // Add summary information with correct counts
    response["summary"] = json!({
        "total_requested": type_names.len(),
        "successful_discoveries": successful_discoveries,
        "failed_discoveries": failed_discoveries,
    });

    debug_info.push("Request processing complete".to_string());

    Ok(response)
}
