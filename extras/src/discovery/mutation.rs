//! Mutation info generation logic for BRP operations
//!
//! This module consolidates all mutation info generation functions, eliminating
//! the duplication between debug and error-handling variants.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{StructInfo, TupleStructInfo, TypeInfo};
use serde_json::{Value, json};

use super::error::{DebugContext, DiscoveryError, DiscoveryResult};
use super::examples::{
    generate_default_example_for_type, generate_primitive_example, is_primitive_type,
};
use super::types::{
    TypeCategory, analyze_type_info, cast_type_info, extract_struct_fields,
    extract_tuple_struct_fields, is_mutable_type,
};
use crate::format::{FieldInfo, MutationInfo};

/// Helper function to create a `FieldInfo` instance
fn create_field_info(
    path: impl Into<String>,
    value_type: impl Into<String>,
    example: Value,
    description: impl Into<String>,
) -> FieldInfo {
    FieldInfo {
        path: path.into(),
        value_type: value_type.into(),
        example,
        description: description.into(),
    }
}

/// Helper function to convert `Vec<FieldInfo>` to `HashMap<String, FieldInfo>`
fn field_info_vec_to_map(field_paths: Vec<FieldInfo>) -> HashMap<String, FieldInfo> {
    field_paths
        .into_iter()
        .map(|field_info| (field_info.path.clone(), field_info))
        .collect()
}

/// Generate mutation info for any type based on its `TypeInfo`
pub fn generate_mutation_info(
    type_info: &TypeInfo,
    type_name: &str,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<MutationInfo> {
    debug_context.push(format!("Generating mutation info for: {type_name}"));

    if !is_mutable_type(type_info) {
        return Err(DiscoveryError::unsupported_type(format!(
            "Type {type_name} is not mutable (only structs, tuple structs, and tuples support mutation)"
        )));
    }

    match analyze_type_info(type_info) {
        TypeCategory::Struct => {
            let struct_info = cast_type_info(type_info, TypeInfo::as_struct, "StructInfo")?;
            generate_mutation_info_for_struct(struct_info, debug_context)
        }
        TypeCategory::TupleStruct => {
            let tuple_struct_info =
                cast_type_info(type_info, TypeInfo::as_tuple_struct, "TupleStructInfo")?;
            generate_mutation_info_for_tuple_struct(tuple_struct_info, debug_context)
        }
        _ => Err(DiscoveryError::type_not_supported_for(
            type_name,
            "Mutation info generation",
        )),
    }
}

/// Generate mutation info for struct types
pub fn generate_mutation_info_for_struct(
    struct_info: &StructInfo,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<MutationInfo> {
    debug_context.push("Processing struct type for mutation info".to_string());

    let fields = extract_struct_fields(struct_info);
    let mut field_paths = Vec::new();

    for (field_name, field_type) in fields {
        debug_context.push(format!(
            "Processing struct field for mutation: {field_name}: {field_type}"
        ));

        // Generate mutation paths for this field
        let paths = generate_field_mutation_paths(&field_name, &field_type, debug_context)?;
        field_paths.extend(paths);
    }

    // Convert Vec<FieldInfo> to HashMap<String, FieldInfo>
    let fields_map = field_info_vec_to_map(field_paths);

    Ok(MutationInfo {
        fields:      fields_map,
        description: format!(
            "Mutation info for struct with {} fields",
            struct_info.field_len()
        ),
    })
}

/// Generate mutation info for tuple struct types
pub fn generate_mutation_info_for_tuple_struct(
    tuple_struct_info: &TupleStructInfo,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<MutationInfo> {
    debug_context.push("Processing tuple struct type for mutation info".to_string());

    let fields = extract_tuple_struct_fields(tuple_struct_info);
    let mut field_paths = Vec::new();

    for (index, field_type) in fields {
        debug_context.push(format!(
            "Processing tuple struct field for mutation: {index}: {field_type}"
        ));

        // Generate mutation paths for this field
        let field_name = index.to_string();
        let paths = generate_field_mutation_paths(&field_name, &field_type, debug_context)?;
        field_paths.extend(paths);
    }

    // Convert Vec<FieldInfo> to HashMap<String, FieldInfo>
    let fields_map = field_info_vec_to_map(field_paths);

    Ok(MutationInfo {
        fields:      fields_map,
        description: format!(
            "Mutation info for tuple struct with {} fields",
            tuple_struct_info.field_len()
        ),
    })
}

/// Generate mutation paths for a specific field
#[allow(clippy::unnecessary_wraps)] // Used in Result context for consistency
pub fn generate_field_mutation_paths(
    field_name: &str,
    field_type: &str,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<Vec<FieldInfo>> {
    let mut paths = Vec::new();

    // Base path for the field itself
    let base_path = format!(".{field_name}");
    let example_value = if is_primitive_type(field_type) {
        generate_primitive_example(field_type)
            .unwrap_or_else(|_| generate_default_example_for_type(field_type))
    } else {
        generate_default_example_for_type(field_type)
    };

    paths.push(create_field_info(
        base_path.clone(),
        field_type,
        example_value,
        format!("Mutate the entire {field_name} field"),
    ));

    // Generate nested paths for complex types
    // Check for math types first before generic Vec to avoid catching Vec2/Vec3/Vec4
    if is_bevy_math_type(field_type) {
        debug_context.push(format!(
            "Generating math type mutation paths for {field_name}"
        ));
        paths.extend(generate_math_type_mutation_paths(&base_path, field_type));
    } else if is_bevy_transform_type(field_type) {
        debug_context.push(format!(
            "Generating Transform mutation paths for {field_name}"
        ));
        paths.extend(generate_transform_mutation_paths(&base_path));
    } else if field_type.contains("Vec") {
        debug_context.push(format!("Generating Vec mutation paths for {field_name}"));
        paths.extend(generate_vec_mutation_paths(&base_path, field_type));
    } else if field_type.contains("HashMap") || field_type.contains("BTreeMap") {
        debug_context.push(format!("Generating Map mutation paths for {field_name}"));
        paths.extend(generate_map_mutation_paths(&base_path, field_type));
    }

    Ok(paths)
}

/// Generate mutation paths for Vec types
fn generate_vec_mutation_paths(base_path: &str, _field_type: &str) -> Vec<FieldInfo> {
    let mut paths = Vec::new();

    // Array index access
    paths.push(create_field_info(
        format!("{base_path}[0]"),
        "array_element",
        json!("first_element_value"),
        "Mutate the first element of the Vec",
    ));

    paths.push(create_field_info(
        format!("{base_path}[1]"),
        "array_element",
        json!("second_element_value"),
        "Mutate the second element of the Vec",
    ));

    paths
}

/// Generate mutation paths for Map types
fn generate_map_mutation_paths(base_path: &str, _field_type: &str) -> Vec<FieldInfo> {
    let mut paths = Vec::new();

    // Map key access
    paths.push(create_field_info(
        format!("{base_path}[\"key\"]"),
        "map_value",
        json!("value_for_key"),
        "Mutate a value in the map by key",
    ));

    paths
}

/// Generate mutation paths for Bevy math types
fn generate_math_type_mutation_paths(base_path: &str, field_type: &str) -> Vec<FieldInfo> {
    let components = if field_type.contains("Vec2") {
        vec![("x", 1.0), ("y", 2.0)]
    } else if field_type.contains("Vec3") {
        vec![("x", 1.0), ("y", 2.0), ("z", 3.0)]
    } else if field_type.contains("Vec4") || field_type.contains("Quat") {
        vec![("x", 1.0), ("y", 2.0), ("z", 3.0), ("w", 4.0)]
    } else {
        vec![]
    };

    components
        .into_iter()
        .map(|(component, value)| {
            create_field_info(
                format!("{base_path}.{component}"),
                "f32",
                json!(value),
                format!("Mutate the {component} component"),
            )
        })
        .collect()
}

/// Helper to generate Vec3 field mutation paths
fn generate_vec3_field_paths(
    base_path: &str,
    field_name: &str,
    default_value: [f32; 3],
    component_values: Option<[f32; 3]>,
) -> Vec<FieldInfo> {
    let mut paths = vec![create_field_info(
        format!("{base_path}.{field_name}"),
        "bevy_math::vec3::Vec3",
        json!(default_value),
        format!("Mutate the entire {field_name}"),
    )];

    if let Some(values) = component_values {
        let components = [("x", values[0]), ("y", values[1]), ("z", values[2])];
        for (component, value) in components {
            paths.push(create_field_info(
                format!("{base_path}.{field_name}.{component}"),
                "f32",
                json!(value),
                format!("Mutate the {field_name} {component} component"),
            ));
        }
    }

    paths
}

/// Generate mutation paths for Transform types
fn generate_transform_mutation_paths(base_path: &str) -> Vec<FieldInfo> {
    let mut paths = Vec::new();

    // Translation paths
    paths.extend(generate_vec3_field_paths(
        base_path,
        "translation",
        [0.0, 0.0, 0.0],
        Some([10.0, 20.0, 30.0]),
    ));

    // Rotation paths
    paths.push(create_field_info(
        format!("{base_path}.rotation"),
        "bevy_math::quat::Quat",
        json!([0.0, 0.0, 0.0, 1.0]),
        "Mutate the entire rotation",
    ));

    // Scale paths - only showing x component as per original
    paths.push(create_field_info(
        format!("{base_path}.scale"),
        "bevy_math::vec3::Vec3",
        json!([1.0, 1.0, 1.0]),
        "Mutate the entire scale",
    ));
    paths.push(create_field_info(
        format!("{base_path}.scale.x"),
        "f32",
        json!(2.0),
        "Mutate the scale x component",
    ));

    paths
}

/// Check if a type is a Bevy math type
fn is_bevy_math_type(type_name: &str) -> bool {
    type_name.starts_with("bevy_math::") || type_name.starts_with("glam::")
}

/// Check if a type is a Bevy Transform type
fn is_bevy_transform_type(type_name: &str) -> bool {
    type_name.contains("Transform")
}
