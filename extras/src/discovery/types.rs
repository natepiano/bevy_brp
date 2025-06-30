//! `TypeInfo` processing and analysis for format discovery
//!
//! This module provides consolidated functions for processing Bevy's `TypeInfo`
//! and analyzing type structures to eliminate pattern matching duplication.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{EnumInfo, StructInfo, TupleStructInfo, TypeInfo, TypeInfoError, VariantInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{DiscoveryError, DiscoveryResult};

/// Analyze a `TypeInfo` and determine its category
#[derive(Debug, Clone)]
pub enum TypeCategory {
    Struct,
    TupleStruct,
    Tuple,
    Array,
    List,
    Map,
    Set,
    Enum,
    Opaque,
}

/// Analyze `TypeInfo` and categorize it
pub const fn analyze_type_info(type_info: &TypeInfo) -> TypeCategory {
    match type_info {
        TypeInfo::Struct(_) => TypeCategory::Struct,
        TypeInfo::TupleStruct(_) => TypeCategory::TupleStruct,
        TypeInfo::Tuple(_) => TypeCategory::Tuple,
        TypeInfo::Array(_) => TypeCategory::Array,
        TypeInfo::List(_) => TypeCategory::List,
        TypeInfo::Map(_) => TypeCategory::Map,
        TypeInfo::Set(_) => TypeCategory::Set,
        TypeInfo::Enum(_) => TypeCategory::Enum,
        TypeInfo::Opaque(_) => TypeCategory::Opaque,
    }
}

/// Extract field information from struct `TypeInfo`
pub fn extract_struct_fields(struct_info: &StructInfo) -> Vec<(String, String)> {
    struct_info
        .iter()
        .map(|field| (field.name().to_string(), field.type_path().to_string()))
        .collect()
}

/// Extract field information from tuple struct `TypeInfo`
pub fn extract_tuple_struct_fields(tuple_struct_info: &TupleStructInfo) -> Vec<(usize, String)> {
    tuple_struct_info
        .iter()
        .enumerate()
        .map(|(index, field)| (index, field.type_path().to_string()))
        .collect()
}

/// Extract variant information from enum `TypeInfo`
pub fn extract_enum_variants(enum_info: &EnumInfo) -> Vec<(String, VariantInfo)> {
    enum_info
        .iter()
        .map(|variant| (variant.name().to_string(), variant.clone()))
        .collect()
}

/// Check if a type can be used as a mutation target
pub const fn is_mutable_type(type_info: &TypeInfo) -> bool {
    matches!(
        analyze_type_info(type_info),
        TypeCategory::Struct | TypeCategory::TupleStruct | TypeCategory::Tuple
    )
}

/// Helper to cast `TypeInfo` to a specific type with error handling
pub fn cast_type_info<'a, T, F>(
    type_info: &'a TypeInfo,
    cast_fn: F,
    type_name: &str,
) -> DiscoveryResult<&'a T>
where
    F: FnOnce(&'a TypeInfo) -> Result<&'a T, TypeInfoError>,
{
    cast_fn(type_info).map_err(|_| DiscoveryError::type_cast_failed("TypeInfo", type_name))
}

/// Factual type discovery response that provides clear information about types
///
/// This structure provides comprehensive information about Bevy types for BRP operations,
/// replacing placeholder-based responses with factual data.
///
/// # Examples
///
/// ## Transform Component (Full Support)
/// ```json
/// {
///   "type_name": "bevy_transform::components::transform::Transform",
///   "in_registry": true,
///   "has_serialize": true,
///   "has_deserialize": true,
///   "supported_operations": ["query", "get", "spawn", "insert", "mutate"],
///   "mutation_paths": {
///     ".translation": "Vec3 position in world space",
///     ".translation.x": "X coordinate of translation",
///     ".translation.y": "Y coordinate of translation",
///     ".translation.z": "Z coordinate of translation",
///     ".rotation": "Quat rotation as quaternion",
///     ".scale": "Vec3 scale factors"
///   },
///   "example_values": {
///     "spawn": {
///       "translation": [0.0, 0.0, 0.0],
///       "rotation": [0.0, 0.0, 0.0, 1.0],
///       "scale": [1.0, 1.0, 1.0]
///     }
///   },
///   "type_category": "Struct"
/// }
/// ```
///
/// ## `ClearColor` Resource (No Serialize/Deserialize)
/// ```json
/// {
///   "type_name": "bevy_render::camera::clear_color::ClearColor",
///   "in_registry": true,
///   "has_serialize": false,
///   "has_deserialize": false,
///   "supported_operations": ["query", "get", "mutate"],
///   "mutation_paths": {
///     ".0": "The Color value"
///   },
///   "example_values": {},
///   "type_category": "TupleStruct"
/// }
/// ```
///
/// ## Unknown Type (Not in Registry)
/// ```json
/// {
///   "type_name": "unknown::CustomType",
///   "in_registry": false,
///   "has_serialize": false,
///   "has_deserialize": false,
///   "supported_operations": [],
///   "mutation_paths": {},
///   "example_values": {},
///   "type_category": "Unknown"
/// }
/// ```
///
/// ## Color Enum (Complex Type)
/// ```json
/// {
///   "type_name": "bevy_color::color::Color",
///   "in_registry": true,
///   "has_serialize": true,
///   "has_deserialize": true,
///   "supported_operations": ["query", "get", "spawn", "insert"],
///   "mutation_paths": {},
///   "example_values": {
///     "spawn": {
///       "Srgba": {
///         "red": 1.0,
///         "green": 0.0,
///         "blue": 0.0,
///         "alpha": 1.0
///       }
///     }
///   },
///   "type_category": "Enum",
///   "child_types": {
///     "Srgba": "bevy_color::srgba::Srgba",
///     "LinearRgba": "bevy_color::linear_rgba::LinearRgba"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDiscoveryResponse {
    /// The fully-qualified type name
    pub type_name:            String,
    /// Whether the type is in the type registry
    pub in_registry:          bool,
    /// Whether the type has Serialize trait
    pub has_serialize:        bool,
    /// Whether the type has Deserialize trait
    pub has_deserialize:      bool,
    /// List of supported BRP operations for this type
    pub supported_operations: Vec<String>,
    /// Available mutation paths if the type supports mutation
    pub mutation_paths:       HashMap<String, String>,
    /// Real example values for different contexts (spawn, insert, etc.)
    pub example_values:       HashMap<String, Value>,
    /// Type category for clarity
    pub type_category:        String,
    /// Child type information for complex types
    pub child_types:          HashMap<String, String>,
}

/// Check if a type registration has Serialize and Deserialize traits
pub fn check_serialization_traits(registration: &bevy::reflect::TypeRegistration) -> (bool, bool) {
    let has_serialize = registration
        .data::<bevy::reflect::ReflectSerialize>()
        .is_some();

    let has_deserialize = registration
        .data::<bevy::reflect::ReflectDeserialize>()
        .is_some();

    (has_serialize, has_deserialize)
}
