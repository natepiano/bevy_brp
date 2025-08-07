//! Hardcoded BRP format knowledge
//!
//! This module contains the static knowledge of how types should be serialized
//! for BRP, which often differs from their reflection-based representation.
//! This knowledge is extracted from the extras plugin's examples.rs.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::{Value, json};

use crate::brp_tools::brp_client::format_discovery::engine::types::BrpTypeName;

/// Static map of hardcoded BRP format knowledge
/// This captures the serialization rules that can't be derived from registry
pub static BRP_FORMAT_KNOWLEDGE: LazyLock<HashMap<BrpTypeName, Value>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // ===== Numeric types =====
    map.insert("i8".into(), json!(-128));
    map.insert("i16".into(), json!(-32768));
    map.insert("i32".into(), json!(-2_147_483_648));
    map.insert("i64".into(), json!(-9_223_372_036_854_775_808_i64));
    map.insert(
        "i128".into(),
        json!("-170141183460469231731687303715884105728"),
    );
    map.insert("u8".into(), json!(255));
    map.insert("u16".into(), json!(65535));
    map.insert("u32".into(), json!(4_294_967_295_u32));
    map.insert("u64".into(), json!(18_446_744_073_709_551_615_u64));
    map.insert(
        "u128".into(),
        json!("340282366920938463463374607431768211455"),
    );
    map.insert("f32".into(), json!(std::f32::consts::PI));
    map.insert("f64".into(), json!(std::f64::consts::PI));

    // ===== Size types =====
    map.insert("isize".into(), json!(-9_223_372_036_854_775_808_i64));
    map.insert("usize".into(), json!(18_446_744_073_709_551_615_u64));

    // ===== Text types =====
    map.insert("alloc::string::String".into(), json!("Hello, World!"));
    map.insert("std::string::String".into(), json!("Hello, World!"));
    map.insert("String".into(), json!("Hello, World!"));
    map.insert("&str".into(), json!("static string"));
    map.insert("str".into(), json!("static string"));
    map.insert("char".into(), json!('A'));

    // ===== Boolean =====
    map.insert("bool".into(), json!(true));

    // ===== Bevy math types (these serialize as arrays, not objects!) =====
    // Vec2
    map.insert("bevy_math::vec2::Vec2".into(), json!([1.0, 2.0]));
    map.insert("glam::Vec2".into(), json!([1.0, 2.0]));
    // Vec3
    map.insert("bevy_math::vec3::Vec3".into(), json!([1.0, 2.0, 3.0]));
    map.insert("bevy_math::vec3a::Vec3A".into(), json!([1.0, 2.0, 3.0]));
    map.insert("glam::Vec3".into(), json!([1.0, 2.0, 3.0]));
    map.insert("glam::Vec3A".into(), json!([1.0, 2.0, 3.0]));

    // Vec4
    map.insert("bevy_math::vec4::Vec4".into(), json!([1.0, 2.0, 3.0, 4.0]));
    map.insert("glam::Vec4".into(), json!([1.0, 2.0, 3.0, 4.0]));

    // Quaternion
    map.insert("bevy_math::quat::Quat".into(), json!([0.0, 0.0, 0.0, 1.0]));
    map.insert("glam::Quat".into(), json!([0.0, 0.0, 0.0, 1.0]));

    // Matrices
    map.insert(
        "bevy_math::mat2::Mat2".into(),
        json!([[1.0, 0.0], [0.0, 1.0]]),
    );
    map.insert("glam::Mat2".into(), json!([[1.0, 0.0], [0.0, 1.0]]));
    map.insert(
        "bevy_math::mat3::Mat3".into(),
        json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
    );
    map.insert(
        "glam::Mat3".into(),
        json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
    );
    map.insert(
        "bevy_math::mat4::Mat4".into(),
        json!([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]),
    );
    map.insert(
        "glam::Mat4".into(),
        json!([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]),
    );

    // ===== Bevy color types =====
    map.insert(
        "bevy_color::srgba::Srgba".into(),
        json!({
            "red": 1.0,
            "green": 0.0,
            "blue": 0.0,
            "alpha": 1.0
        }),
    );
    map.insert(
        "bevy_color::linear_rgba::LinearRgba".into(),
        json!({
            "red": 1.0,
            "green": 0.0,
            "blue": 0.0,
            "alpha": 1.0
        }),
    );
    map.insert(
        "bevy_color::Color".into(),
        json!({
            "Srgba": {
                "red": 1.0,
                "green": 0.0,
                "blue": 0.0,
                "alpha": 1.0
            }
        }),
    );

    // ===== Collections =====
    map.insert("alloc::vec::Vec".into(), json!([]));
    map.insert("std::collections::HashMap".into(), json!({}));
    map.insert("std::collections::BTreeMap".into(), json!({}));

    // ===== Option types =====
    map.insert("core::option::Option".into(), json!(null));

    map
});
