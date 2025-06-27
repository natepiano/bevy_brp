//! Main discovery orchestration with unified error handling
//!
//! This module provides the core discovery functions that tie together
//! all the specialized modules with unified error handling.

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::error::{DebugContext, DiscoveryResult};
use super::mutation::generate_mutation_info;
use super::registry::get_type_info_from_registry;
use super::spawn::generate_spawn_format;
use super::types::is_mutable_type;
use crate::format::FormatInfo;

/// Result of discovering multiple component formats
#[derive(Debug, Clone)]
pub struct MultiDiscoveryResult {
    pub formats: HashMap<String, FormatInfo>,
    pub errors:  HashMap<String, serde_json::Map<String, Value>>,
}

/// Discover format information for a single component type with unified error handling
pub fn discover_component_format(
    world: &World,
    type_name: &str,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<FormatInfo> {
    debug_context.push(format!("Discovering format for type: {type_name}"));

    // Get type info from registry
    let type_info = get_type_info_from_registry(world, type_name, debug_context.as_mut_vec())?;

    // Generate spawn format
    debug_context.push("Generating spawn format".to_string());
    let spawn_info = generate_spawn_format(&type_info, type_name, debug_context)?;

    // Generate mutation info (if supported)
    debug_context.push("Generating mutation info".to_string());
    let mutation_info = if is_mutable_type(&type_info) {
        generate_mutation_info(&type_info, type_name, debug_context)?
    } else {
        debug_context.push("Type is not mutable, creating empty mutation info".to_string());
        crate::format::MutationInfo {
            fields:      HashMap::new(),
            description: format!("Type {type_name} does not support mutation"),
        }
    };

    let format_info = FormatInfo {
        type_name: type_name.to_string(),
        spawn_format: spawn_info,
        mutation_info,
    };

    debug_context.push("Successfully generated format info".to_string());
    Ok(format_info)
}

/// Discover format information for multiple component types
pub fn discover_multiple_formats(world: &World, type_names: &[String]) -> MultiDiscoveryResult {
    let mut debug_context = DebugContext::new();
    discover_multiple_formats_with_debug(world, type_names, &mut debug_context)
}

/// Discover format information for multiple component types with debug information
pub fn discover_multiple_formats_with_debug(
    world: &World,
    type_names: &[String],
    debug_context: &mut DebugContext,
) -> MultiDiscoveryResult {
    debug_context.push(format!(
        "Discovering formats for {} types",
        type_names.len()
    ));

    let mut formats = HashMap::new();
    let mut errors = HashMap::new();

    for type_name in type_names {
        debug_context.push(format!("Processing type: {type_name}"));

        let mut type_debug_context = DebugContext::new();
        match discover_component_format(world, type_name, &mut type_debug_context) {
            Ok(format_info) => {
                debug_context.push(format!("Successfully discovered format for: {type_name}"));
                // Include debug info from the type-specific discovery
                debug_context.messages.extend(type_debug_context.messages);
                formats.insert(type_name.clone(), format_info);
            }
            Err(error) => {
                debug_context.push(format!("Failed to discover format for: {type_name}"));
                // Include debug info from the failed discovery
                debug_context.messages.extend(type_debug_context.messages);
                errors.insert(type_name.clone(), error.to_json_error());
            }
        }
    }

    debug_context.push(format!(
        "Discovery complete: {} successful, {} errors",
        formats.len(),
        errors.len()
    ));

    MultiDiscoveryResult { formats, errors }
}

/// Get a list of common component types that are typically available
pub fn get_common_component_types() -> Vec<String> {
    vec![
        "bevy_transform::components::transform::Transform".to_string(),
        "bevy_core::name::Name".to_string(),
        "bevy_render::color::LinearRgba".to_string(),
        "bevy_sprite::sprite::Sprite".to_string(),
        "bevy_render::camera::camera::Camera".to_string(),
    ]
}

/// Create a comprehensive discovery response with all metadata
pub fn create_discovery_response(
    discovery_result: &MultiDiscoveryResult,
    requested_types: &[String],
    debug_context: Option<&DebugContext>,
) -> Value {
    let mut response = json!({
        "success": true,
        "formats": discovery_result.formats,
        "requested_types": requested_types,
        "discovered_count": discovery_result.formats.len()
    });

    // Add errors if any types were undiscoverable
    if !discovery_result.errors.is_empty() {
        response["errors"] = json!(discovery_result.errors);
        response["error_count"] = json!(discovery_result.errors.len());
    }

    // Add debug info if provided
    if let Some(debug_ctx) = debug_context {
        if !debug_ctx.messages.is_empty() {
            response["debug_info"] = json!(debug_ctx.messages);
        }
    }

    // Add summary information
    response["summary"] = json!({
        "total_requested": requested_types.len(),
        "successful_discoveries": discovery_result.formats.len(),
        "failed_discoveries": discovery_result.errors.len(),
        "success_rate": if requested_types.is_empty() {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                discovery_result.formats.len() as f64 / requested_types.len() as f64
            }
        }
    });

    response
}
