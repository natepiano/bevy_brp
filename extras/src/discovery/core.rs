//! Main discovery orchestration with unified error handling
//!
//! This module provides the core discovery functions that tie together
//! all the specialized modules with unified error handling.

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::Value;

use super::error::{DebugContext, DiscoveryResult};
use super::mutation::generate_mutation_info;
use super::registry::get_type_info_from_registry;
use super::spawn::generate_spawn_format;
use super::types::{
    TypeDiscoveryResponse, analyze_type_info, check_serialization_traits, is_mutable_type,
};
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

/// Discover type information as a factual response
pub fn discover_type_as_response(
    world: &World,
    type_name: &str,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<TypeDiscoveryResponse> {
    debug_context.push(format!("Discovering type response for: {type_name}"));

    // Try to get type info from registry
    let type_info_result =
        get_type_info_from_registry(world, type_name, debug_context.as_mut_vec());

    let (in_registry, type_info_opt, has_serialize, has_deserialize) = match type_info_result {
        Ok(type_info) => {
            // Check for Serialize/Deserialize traits using helper function
            let (has_serialize, has_deserialize) = {
                let registry = world.resource::<AppTypeRegistry>().read();
                let registration = registry.get_with_type_path(type_name);
                registration
                    .map(check_serialization_traits)
                    .unwrap_or((false, false))
            };
            (true, Some(type_info), has_serialize, has_deserialize)
        }
        Err(_) => (false, None, false, false),
    };

    // Determine supported operations
    let mut supported_operations = Vec::new();
    if in_registry {
        supported_operations.push("query".to_string());
        supported_operations.push("get".to_string());
        if has_serialize && has_deserialize {
            supported_operations.push("spawn".to_string());
            supported_operations.push("insert".to_string());
        }
        if let Some(ref type_info) = type_info_opt {
            if is_mutable_type(type_info) {
                supported_operations.push("mutate".to_string());
            }
        }
    }

    // Get mutation paths if supported
    let mutation_paths = if let Some(ref type_info) = type_info_opt {
        if is_mutable_type(type_info) {
            match generate_mutation_info(type_info, type_name, debug_context) {
                Ok(mutation_info) => mutation_info
                    .fields
                    .into_iter()
                    .map(|(path, field)| (path, field.description))
                    .collect(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // Generate example values using recursive generation
    let mut example_values = HashMap::new();
    if type_info_opt.is_some() && has_serialize && has_deserialize {
        // Use recursive example generation
        let mut visited_types = Vec::new();
        let example =
            super::examples::generate_recursive_example(world, type_name, &mut visited_types);
        example_values.insert("spawn".to_string(), example);
        debug_context.push("Generated recursive example for spawn".to_string());
    }

    // Determine type category
    let type_category = if let Some(ref type_info) = type_info_opt {
        format!("{:?}", analyze_type_info(type_info))
    } else {
        "Unknown".to_string()
    };

    // Extract child types for complex types
    let child_types = if let Some(ref type_info) = type_info_opt {
        use bevy::reflect::TypeInfo;

        use super::types::{cast_type_info, extract_struct_fields, extract_tuple_struct_fields};

        match type_info {
            TypeInfo::Struct(_) => {
                if let Ok(struct_info) =
                    cast_type_info(type_info, TypeInfo::as_struct, "StructInfo")
                {
                    extract_struct_fields(struct_info)
                        .into_iter()
                        .map(|(name, type_path)| (name, type_path))
                        .collect()
                } else {
                    HashMap::new()
                }
            }
            TypeInfo::TupleStruct(_) => {
                if let Ok(tuple_info) =
                    cast_type_info(type_info, TypeInfo::as_tuple_struct, "TupleStructInfo")
                {
                    extract_tuple_struct_fields(tuple_info)
                        .into_iter()
                        .map(|(idx, type_path)| (format!(".{idx}"), type_path))
                        .collect()
                } else {
                    HashMap::new()
                }
            }
            _ => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    Ok(TypeDiscoveryResponse {
        type_name: type_name.to_string(),
        in_registry,
        has_serialize,
        has_deserialize,
        supported_operations,
        mutation_paths,
        example_values,
        type_category,
        child_types,
    })
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
