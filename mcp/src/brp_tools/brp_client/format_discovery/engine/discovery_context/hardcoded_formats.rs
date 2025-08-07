//! Hardcoded BRP format knowledge
//!
//! This module contains the static knowledge of how types should be serialized
//! for BRP, which often differs from their reflection-based representation.
//! This knowledge is extracted from the extras plugin's examples.rs.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::{Value, json};

use crate::brp_tools::brp_client::format_discovery::engine::discovery_context::types::BrpFormatKnowledge;
use crate::brp_tools::brp_client::format_discovery::engine::types::BrpTypeName;

/// Static map of hardcoded BRP format knowledge
/// This captures the serialization rules that can't be derived from registry
pub static BRP_FORMAT_KNOWLEDGE: LazyLock<HashMap<BrpTypeName, BrpFormatKnowledge>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // ===== Numeric types =====
        map.insert(
            "i8".into(),
            BrpFormatKnowledge {
                example_value:  json!(-128),
                subfield_paths: None,
            },
        );
        map.insert(
            "i16".into(),
            BrpFormatKnowledge {
                example_value:  json!(-32768),
                subfield_paths: None,
            },
        );
        map.insert(
            "i32".into(),
            BrpFormatKnowledge {
                example_value:  json!(-2_147_483_648),
                subfield_paths: None,
            },
        );
        map.insert(
            "i64".into(),
            BrpFormatKnowledge {
                example_value:  json!(-9_223_372_036_854_775_808_i64),
                subfield_paths: None,
            },
        );
        map.insert(
            "i128".into(),
            BrpFormatKnowledge {
                example_value:  json!("-170141183460469231731687303715884105728"),
                subfield_paths: None,
            },
        );
        map.insert(
            "u8".into(),
            BrpFormatKnowledge {
                example_value:  json!(255),
                subfield_paths: None,
            },
        );
        map.insert(
            "u16".into(),
            BrpFormatKnowledge {
                example_value:  json!(65535),
                subfield_paths: None,
            },
        );
        map.insert(
            "u32".into(),
            BrpFormatKnowledge {
                example_value:  json!(4_294_967_295_u32),
                subfield_paths: None,
            },
        );
        map.insert(
            "u64".into(),
            BrpFormatKnowledge {
                example_value:  json!(18_446_744_073_709_551_615_u64),
                subfield_paths: None,
            },
        );
        map.insert(
            "u128".into(),
            BrpFormatKnowledge {
                example_value:  json!("340282366920938463463374607431768211455"),
                subfield_paths: None,
            },
        );
        map.insert(
            "f32".into(),
            BrpFormatKnowledge {
                example_value:  json!(std::f32::consts::PI),
                subfield_paths: None,
            },
        );
        map.insert(
            "f64".into(),
            BrpFormatKnowledge {
                example_value:  json!(std::f64::consts::PI),
                subfield_paths: None,
            },
        );

        // ===== Size types =====
        map.insert(
            "isize".into(),
            BrpFormatKnowledge {
                example_value:  json!(-9_223_372_036_854_775_808_i64),
                subfield_paths: None,
            },
        );
        map.insert(
            "usize".into(),
            BrpFormatKnowledge {
                example_value:  json!(18_446_744_073_709_551_615_u64),
                subfield_paths: None,
            },
        );

        // ===== Text types =====
        map.insert(
            "alloc::string::String".into(),
            BrpFormatKnowledge {
                example_value:  json!("Hello, World!"),
                subfield_paths: None,
            },
        );
        map.insert(
            "std::string::String".into(),
            BrpFormatKnowledge {
                example_value:  json!("Hello, World!"),
                subfield_paths: None,
            },
        );
        map.insert(
            "String".into(),
            BrpFormatKnowledge {
                example_value:  json!("Hello, World!"),
                subfield_paths: None,
            },
        );
        map.insert(
            "&str".into(),
            BrpFormatKnowledge {
                example_value:  json!("static string"),
                subfield_paths: None,
            },
        );
        map.insert(
            "str".into(),
            BrpFormatKnowledge {
                example_value:  json!("static string"),
                subfield_paths: None,
            },
        );
        map.insert(
            "char".into(),
            BrpFormatKnowledge {
                example_value:  json!('A'),
                subfield_paths: None,
            },
        );

        // ===== Boolean =====
        map.insert(
            "bool".into(),
            BrpFormatKnowledge {
                example_value:  json!(true),
                subfield_paths: None,
            },
        );

        // ===== Bevy math types (these serialize as arrays, not objects!) =====
        // Vec2
        map.insert(
            "bevy_math::vec2::Vec2".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0]),
                subfield_paths: Some(vec![("x", json!(1.0)), ("y", json!(2.0))]),
            },
        );
        map.insert(
            "glam::Vec2".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0]),
                subfield_paths: Some(vec![("x", json!(1.0)), ("y", json!(2.0))]),
            },
        );

        // Vec3
        map.insert(
            "bevy_math::vec3::Vec3".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0, 3.0]),
                subfield_paths: Some(vec![
                    ("x", json!(1.0)),
                    ("y", json!(2.0)),
                    ("z", json!(3.0)),
                ]),
            },
        );
        map.insert(
            "bevy_math::vec3a::Vec3A".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0, 3.0]),
                subfield_paths: Some(vec![
                    ("x", json!(1.0)),
                    ("y", json!(2.0)),
                    ("z", json!(3.0)),
                ]),
            },
        );
        map.insert(
            "glam::Vec3".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0, 3.0]),
                subfield_paths: Some(vec![
                    ("x", json!(1.0)),
                    ("y", json!(2.0)),
                    ("z", json!(3.0)),
                ]),
            },
        );
        map.insert(
            "glam::Vec3A".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0, 3.0]),
                subfield_paths: Some(vec![
                    ("x", json!(1.0)),
                    ("y", json!(2.0)),
                    ("z", json!(3.0)),
                ]),
            },
        );

        // Vec4
        map.insert(
            "bevy_math::vec4::Vec4".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0, 3.0, 4.0]),
                subfield_paths: Some(vec![
                    ("x", json!(1.0)),
                    ("y", json!(2.0)),
                    ("z", json!(3.0)),
                    ("w", json!(4.0)),
                ]),
            },
        );
        map.insert(
            "glam::Vec4".into(),
            BrpFormatKnowledge {
                example_value:  json!([1.0, 2.0, 3.0, 4.0]),
                subfield_paths: Some(vec![
                    ("x", json!(1.0)),
                    ("y", json!(2.0)),
                    ("z", json!(3.0)),
                    ("w", json!(4.0)),
                ]),
            },
        );

        // Quaternion
        map.insert(
            "bevy_math::quat::Quat".into(),
            BrpFormatKnowledge {
                example_value:  json!([0.0, 0.0, 0.0, 1.0]),
                subfield_paths: Some(vec![
                    ("x", json!(0.0)),
                    ("y", json!(0.0)),
                    ("z", json!(0.0)),
                    ("w", json!(1.0)),
                ]),
            },
        );
        map.insert(
            "glam::Quat".into(),
            BrpFormatKnowledge {
                example_value:  json!([0.0, 0.0, 0.0, 1.0]),
                subfield_paths: Some(vec![
                    ("x", json!(0.0)),
                    ("y", json!(0.0)),
                    ("z", json!(0.0)),
                    ("w", json!(1.0)),
                ]),
            },
        );

        // Matrices
        map.insert(
            "bevy_math::mat2::Mat2".into(),
            BrpFormatKnowledge {
                example_value:  json!([[1.0, 0.0], [0.0, 1.0]]),
                subfield_paths: None, // Matrices don't have simple component access
            },
        );
        map.insert(
            "glam::Mat2".into(),
            BrpFormatKnowledge {
                example_value:  json!([[1.0, 0.0], [0.0, 1.0]]),
                subfield_paths: None,
            },
        );
        map.insert(
            "bevy_math::mat3::Mat3".into(),
            BrpFormatKnowledge {
                example_value:  json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                subfield_paths: None,
            },
        );
        map.insert(
            "glam::Mat3".into(),
            BrpFormatKnowledge {
                example_value:  json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                subfield_paths: None,
            },
        );
        map.insert(
            "bevy_math::mat4::Mat4".into(),
            BrpFormatKnowledge {
                example_value:  json!([
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0]
                ]),
                subfield_paths: None,
            },
        );
        map.insert(
            "glam::Mat4".into(),
            BrpFormatKnowledge {
                example_value:  json!([
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0]
                ]),
                subfield_paths: None,
            },
        );

        // ===== Bevy color types =====
        map.insert(
            "bevy_color::srgba::Srgba".into(),
            BrpFormatKnowledge {
                example_value:  json!({
                    "red": 1.0,
                    "green": 0.0,
                    "blue": 0.0,
                    "alpha": 1.0
                }),
                subfield_paths: None, // Colors use named fields, not component access
            },
        );
        map.insert(
            "bevy_color::linear_rgba::LinearRgba".into(),
            BrpFormatKnowledge {
                example_value:  json!({
                    "red": 1.0,
                    "green": 0.0,
                    "blue": 0.0,
                    "alpha": 1.0
                }),
                subfield_paths: None,
            },
        );
        map.insert(
            "bevy_color::Color".into(),
            BrpFormatKnowledge {
                example_value:  json!({
                    "Srgba": {
                        "red": 1.0,
                        "green": 0.0,
                        "blue": 0.0,
                        "alpha": 1.0
                    }
                }),
                subfield_paths: None,
            },
        );

        // ===== Collections =====
        map.insert(
            "alloc::vec::Vec".into(),
            BrpFormatKnowledge {
                example_value:  json!([]),
                subfield_paths: None, // Collections have index access, not component access
            },
        );
        map.insert(
            "std::collections::HashMap".into(),
            BrpFormatKnowledge {
                example_value:  json!({}),
                subfield_paths: None,
            },
        );
        map.insert(
            "std::collections::BTreeMap".into(),
            BrpFormatKnowledge {
                example_value:  json!({}),
                subfield_paths: None,
            },
        );

        // ===== Option types =====
        map.insert(
            "core::option::Option".into(),
            BrpFormatKnowledge {
                example_value:  json!(null),
                subfield_paths: None,
            },
        );

        map
    });
