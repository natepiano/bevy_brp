//! Primitive and type example generation for format discovery
//!
//! This module provides consolidated functions for generating example values
//! for primitive types and complex type structures.

use serde_json::{Value, json};

use super::error::{DiscoveryError, DiscoveryResult};

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
        "alloc::string::String" | "std::string::String" | "String" => json!("example_string"),
        "&str" | "str" => json!("example_str"),
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
            json!(format!(
                "example_{}",
                type_name.split("::").last().unwrap_or(type_name)
            ))
        }
    })
}
