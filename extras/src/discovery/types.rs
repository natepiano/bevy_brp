//! `TypeInfo` processing and analysis for format discovery
//!
//! This module provides consolidated functions for processing Bevy's `TypeInfo`
//! and analyzing type structures to eliminate pattern matching duplication.

use bevy::prelude::*;
use bevy::reflect::{EnumInfo, StructInfo, TupleStructInfo, TypeInfo, TypeInfoError, VariantInfo};

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
