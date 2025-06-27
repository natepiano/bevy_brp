//! Public API and request handling for format discovery
//!
//! This module provides the public API functions and handles BRP requests
//! for format discovery operations.

use bevy::prelude::*;
use bevy::remote::{BrpError, BrpResult, error_codes};
use serde_json::Value;

use super::core::{
    create_discovery_response, discover_multiple_formats, discover_multiple_formats_with_debug,
    get_common_component_types,
};
use super::error::DebugContext;
use crate::format::FormatInfo;

/// Discover format information for a single component type (public API)
///
/// Returns `None` if the type is not found or cannot be processed.
/// For detailed error information, use `discover_component_format_with_context`.
pub fn discover_component_format_simple(world: &World, type_name: &str) -> Option<FormatInfo> {
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

/// Handler for format discovery BRP requests
///
/// Processes incoming BRP requests for component format discovery and returns
/// formatted responses with format information, errors, and debug data.
pub fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Parse parameters - types parameter is required
    let type_names = parse_types_parameter(params)?;

    // Check if debug mode is enabled
    let mut debug_info = DebugContext::new();
    let include_debug = crate::debug_mode::is_debug_enabled();

    debug_info.push(format!("Processing request for {} types", type_names.len()));
    debug_info.push(format!("Debug mode enabled: {include_debug}"));

    // Discover formats for the requested types
    let discovery_result = if include_debug {
        discover_multiple_formats_with_debug(world, &type_names, &mut debug_info)
    } else {
        discover_multiple_formats(world, &type_names)
    };

    // Create comprehensive response
    let response = create_discovery_response(
        &discovery_result,
        &type_names,
        if include_debug {
            Some(&debug_info)
        } else {
            None
        },
    );

    debug_info.push("Request processing complete".to_string());

    // Return the discovered formats
    Ok(response)
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

/// Get common component types (convenience function for API users)
#[must_use]
pub fn get_common_component_types_public() -> Vec<String> {
    get_common_component_types()
}
