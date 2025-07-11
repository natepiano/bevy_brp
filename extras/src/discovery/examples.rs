//! Primitive and type example generation for format discovery
//!
//! This module provides consolidated functions for generating example values
//! for primitive types and complex type structures.

use bevy::prelude::*;
use serde_json::{Value, json};

use super::error::{DiscoveryError, DiscoveryResult};

/// Maximum recursion depth to prevent stack overflow
const MAX_RECURSION_DEPTH: usize = 10;

/// Generate example values for primitive types
pub fn generate_primitive_example(type_name: &str) -> DiscoveryResult<Value> {
    let example = match type_name {
        // Numeric types
        "i8" => json!(-128),
        "i16" => json!(-32768),
        "i32" => json!(-2_147_483_648),
        "i64" => json!(-9_223_372_036_854_775_808_i64),
        "i128" => json!("-170141183460469231731687303715884105728"),
        "u8" => json!(255),
        "u16" => json!(65535),
        "u32" => json!(4_294_967_295_u32),
        "u64" => json!(18_446_744_073_709_551_615_u64),
        "u128" => json!("340282366920938463463374607431768211455"),
        "f32" => json!(std::f32::consts::PI),
        "f64" => json!(std::f64::consts::PI),

        // Text types
        "alloc::string::String" | "std::string::String" | "String" => json!("Hello, World!"),
        "&str" | "str" => json!("static string"),
        "char" => json!('A'),

        // Boolean
        "bool" => json!(true),

        // Bevy math types (both bevy_math and glam types)
        "bevy_math::vec2::Vec2" | "glam::Vec2" => json!([1.0, 2.0]),
        "bevy_math::vec3::Vec3" | "bevy_math::vec3a::Vec3A" | "glam::Vec3" | "glam::Vec3A" => {
            json!([1.0, 2.0, 3.0])
        }
        "bevy_math::vec4::Vec4" | "glam::Vec4" => json!([1.0, 2.0, 3.0, 4.0]),
        "bevy_math::quat::Quat" | "glam::Quat" => json!([0.0, 0.0, 0.0, 1.0]),
        "bevy_math::mat2::Mat2" | "glam::Mat2" => json!([[1.0, 0.0], [0.0, 1.0]]),
        "bevy_math::mat3::Mat3" | "glam::Mat3" => {
            json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
        }
        "bevy_math::mat4::Mat4" | "glam::Mat4" => json!([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]),

        // Bevy color types
        "bevy_color::srgba::Srgba" | "bevy_color::linear_rgba::LinearRgba" => json!({
            "red": 1.0,
            "green": 0.0,
            "blue": 0.0,
            "alpha": 1.0
        }),
        "bevy_color::Color" => json!({
            "Srgba": {
                "red": 1.0,
                "green": 0.0,
                "blue": 0.0,
                "alpha": 1.0
            }
        }),

        // Collections
        "alloc::vec::Vec" => json!([]),
        "std::collections::HashMap" | "std::collections::BTreeMap" => json!({}),

        // Option types
        "core::option::Option" => json!(null),

        _ => {
            return Err(DiscoveryError::no_example_for_type(type_name));
        }
    };

    Ok(example)
}

/// Check if a type name represents a primitive type
pub fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "alloc::string::String"
            | "std::string::String"
            | "String"
            | "&str"
            | "str"
    )
}

/// Generate an appropriate default example for any type
pub fn generate_default_example_for_type(type_name: &str) -> Value {
    generate_primitive_example(type_name).unwrap_or_else(|_| {
        if type_name.contains("Option") {
            json!(null)
        } else if type_name.contains("Vec") {
            json!([])
        } else if type_name.contains("HashMap") || type_name.contains("BTreeMap") {
            json!({})
        } else {
            // Return a descriptive object instead of a placeholder string
            json!({
                "type_name": type_name,
                "note": "Complex type - use recursive discovery for accurate format"
            })
        }
    })
}

/// Generate example with recursive type lookup from registry
pub fn generate_recursive_example(
    world: &World,
    type_name: &str,
    visited_types: &mut Vec<String>,
) -> Value {
    generate_recursive_example_with_depth(world, type_name, visited_types, 0)
}

/// Generate example with recursive type lookup from registry with depth limit
pub fn generate_recursive_example_with_depth(
    world: &World,
    type_name: &str,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    // Check depth limit to prevent stack overflow
    if depth >= MAX_RECURSION_DEPTH {
        return json!(format!(
            "max_depth_reached_{}",
            type_name.split("::").last().unwrap_or(type_name)
        ));
    }
    // Check for circular references
    if visited_types.contains(&type_name.to_string()) {
        return json!(format!(
            "circular_reference_{}",
            type_name.split("::").last().unwrap_or(type_name)
        ));
    }

    // Try primitive example first
    if let Ok(example) = generate_primitive_example(type_name) {
        return example;
    }

    // Add to visited types to prevent infinite recursion
    visited_types.push(type_name.to_string());

    // Try to get type info from registry
    let result = {
        let registry = world.resource::<AppTypeRegistry>().read();
        registry.get_with_type_path(type_name).map_or_else(
            || generate_default_example_for_type(type_name),
            |registration| {
                let type_info = registration.type_info();
                generate_example_from_type_info(world, type_info, type_name, visited_types, depth)
            },
        )
    };

    // Remove from visited types
    visited_types.pop();

    result
}

/// Generate example from type info using pattern matching
fn generate_example_from_type_info(
    world: &World,
    type_info: &bevy::reflect::TypeInfo,
    type_name: &str,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    use bevy::reflect::TypeInfo;

    match type_info {
        TypeInfo::Struct(_) => {
            generate_struct_example(world, type_info, type_name, visited_types, depth)
        }
        TypeInfo::TupleStruct(_) => {
            generate_tuple_struct_example(world, type_info, type_name, visited_types, depth)
        }
        TypeInfo::Enum(enum_info) => {
            generate_enum_example(world, enum_info, type_name, visited_types, depth)
        }
        _ => generate_default_example_for_type(type_name),
    }
}

/// Generate example for struct types
fn generate_struct_example(
    world: &World,
    type_info: &bevy::reflect::TypeInfo,
    type_name: &str,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    use bevy::reflect::TypeInfo;

    use super::types::{cast_type_info, extract_struct_fields};

    cast_type_info(type_info, TypeInfo::as_struct, "StructInfo").map_or_else(
        |_| generate_default_example_for_type(type_name),
        |struct_info| {
            let mut example_obj = serde_json::Map::new();
            for (field_name, field_type) in extract_struct_fields(struct_info) {
                let field_example = generate_recursive_example_with_depth(
                    world,
                    &field_type,
                    visited_types,
                    depth + 1,
                );
                example_obj.insert(field_name, field_example);
            }
            json!(example_obj)
        },
    )
}

/// Generate example for tuple struct types
fn generate_tuple_struct_example(
    world: &World,
    type_info: &bevy::reflect::TypeInfo,
    type_name: &str,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    use bevy::reflect::TypeInfo;

    use super::types::{cast_type_info, extract_tuple_struct_fields};

    cast_type_info(type_info, TypeInfo::as_tuple_struct, "TupleStructInfo").map_or_else(
        |_| generate_default_example_for_type(type_name),
        |tuple_info| {
            let mut example_array = Vec::new();
            for (_idx, field_type) in extract_tuple_struct_fields(tuple_info) {
                let field_example = generate_recursive_example_with_depth(
                    world,
                    &field_type,
                    visited_types,
                    depth + 1,
                );
                example_array.push(field_example);
            }
            json!(example_array)
        },
    )
}

/// Generate example for enum types
fn generate_enum_example(
    world: &World,
    enum_info: &bevy::reflect::EnumInfo,
    type_name: &str,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    // For enums, show the first variant as an example
    enum_info.iter().next().map_or_else(
        || generate_default_example_for_type(type_name),
        |variant| generate_variant_example(world, variant, visited_types, depth),
    )
}

/// Generate example for enum variant
fn generate_variant_example(
    world: &World,
    variant: &bevy::reflect::VariantInfo,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    use bevy::reflect::VariantInfo;

    match variant {
        VariantInfo::Unit(_) => json!(variant.name()),
        VariantInfo::Struct(struct_variant) => {
            generate_struct_variant_example(world, variant, struct_variant, visited_types, depth)
        }
        VariantInfo::Tuple(tuple_variant) => {
            generate_tuple_variant_example(world, variant, tuple_variant, visited_types, depth)
        }
    }
}

/// Generate example for struct variant
fn generate_struct_variant_example(
    world: &World,
    variant: &bevy::reflect::VariantInfo,
    struct_variant: &bevy::reflect::StructVariantInfo,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    let mut variant_fields = serde_json::Map::new();
    for field in struct_variant.iter() {
        let field_example = generate_recursive_example_with_depth(
            world,
            field.type_path(),
            visited_types,
            depth + 1,
        );
        variant_fields.insert(field.name().to_string(), field_example);
    }
    let mut variant_obj = serde_json::Map::new();
    variant_obj.insert(variant.name().to_string(), json!(variant_fields));
    json!(variant_obj)
}

/// Generate example for tuple variant
fn generate_tuple_variant_example(
    world: &World,
    variant: &bevy::reflect::VariantInfo,
    tuple_variant: &bevy::reflect::TupleVariantInfo,
    visited_types: &mut Vec<String>,
    depth: usize,
) -> Value {
    let mut variant_fields = Vec::new();
    for field in tuple_variant.iter() {
        let field_example = generate_recursive_example_with_depth(
            world,
            field.type_path(),
            visited_types,
            depth + 1,
        );
        variant_fields.push(field_example);
    }
    let mut variant_obj = serde_json::Map::new();
    variant_obj.insert(variant.name().to_string(), json!(variant_fields));
    json!(variant_obj)
}
