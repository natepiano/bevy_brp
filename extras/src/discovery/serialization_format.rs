//! Serialization format generation logic for BRP operations
//!
//! This module consolidates all serialization format generation functions, eliminating
//! the duplication between debug and error-handling variants.

use bevy::prelude::*;
use bevy::reflect::{EnumInfo, StructInfo, TupleStructInfo, TypeInfo, VariantInfo};
use serde_json::{Value, json};

use super::error::{DebugContext, DiscoveryError, DiscoveryResult};
use super::examples::{
    generate_default_example_for_type, generate_primitive_example, is_primitive_type,
};
use super::types::{
    SerializationFormat, TypeCategory, analyze_type_info, cast_type_info, extract_enum_variants,
    extract_struct_fields, extract_tuple_struct_fields,
};

/// Generate serialization format for any type based on its `TypeInfo`
pub fn generate_serialization_format(
    type_info: &TypeInfo,
    type_name: &str,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<SerializationFormat> {
    debug_context.push(format!("Generating serialization format for: {type_name}"));

    match analyze_type_info(type_info) {
        TypeCategory::Struct => {
            let struct_info = cast_type_info(type_info, TypeInfo::as_struct, "StructInfo")?;
            generate_serialization_format_for_struct(struct_info, debug_context)
        }
        TypeCategory::TupleStruct => {
            let tuple_struct_info =
                cast_type_info(type_info, TypeInfo::as_tuple_struct, "TupleStructInfo")?;
            generate_serialization_format_for_tuple_struct(tuple_struct_info, debug_context)
        }
        TypeCategory::Enum => {
            let enum_info = cast_type_info(type_info, TypeInfo::as_enum, "EnumInfo")?;
            generate_serialization_format_for_enum(enum_info, debug_context)
        }
        TypeCategory::Opaque => {
            generate_serialization_format_for_primitive(type_name, debug_context)
        }
        _ => Err(DiscoveryError::type_not_supported_for(
            type_name,
            "Serialization format generation",
        )),
    }
}

/// Generate serialization format for struct types
#[allow(clippy::unnecessary_wraps)] // Used in Result context for consistency
pub fn generate_serialization_format_for_struct(
    struct_info: &StructInfo,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<SerializationFormat> {
    debug_context.push("Processing struct type for serialization format".to_string());

    let fields = extract_struct_fields(struct_info);
    let mut serialization_example = serde_json::Map::new();

    for (field_name, field_type) in fields {
        debug_context.push(format!(
            "Processing struct field: {field_name}: {field_type}"
        ));

        let field_example = if is_primitive_type(&field_type) {
            generate_primitive_example(&field_type).unwrap_or_else(|_| {
                debug_context.push(format!(
                    "Failed to generate primitive example for {field_type}, using default"
                ));
                generate_default_example_for_type(&field_type)
            })
        } else {
            generate_default_example_for_type(&field_type)
        };

        serialization_example.insert(field_name, field_example);
    }

    let field_count = serialization_example.len();
    Ok(SerializationFormat {
        example:     Value::Object(serialization_example),
        description: format!("Serialization format for struct with {field_count} fields"),
    })
}

/// Generate serialization format for tuple struct types
#[allow(clippy::unnecessary_wraps)] // Used in Result context for consistency
pub fn generate_serialization_format_for_tuple_struct(
    tuple_struct_info: &TupleStructInfo,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<SerializationFormat> {
    debug_context.push("Processing tuple struct type for serialization format".to_string());

    let fields = extract_tuple_struct_fields(tuple_struct_info);
    let mut serialization_example = Vec::new();

    for (index, field_type) in fields {
        debug_context.push(format!(
            "Processing tuple struct field {index}: {field_type}"
        ));

        let field_example = if is_primitive_type(&field_type) {
            generate_primitive_example(&field_type).unwrap_or_else(|_| {
                debug_context.push(format!(
                    "Failed to generate primitive example for {field_type}, using default"
                ));
                generate_default_example_for_type(&field_type)
            })
        } else {
            generate_default_example_for_type(&field_type)
        };

        serialization_example.push(field_example);
    }

    let field_count = serialization_example.len();
    Ok(SerializationFormat {
        example:     Value::Array(serialization_example),
        description: format!("Serialization format for tuple struct with {field_count} fields"),
    })
}

/// Generate serialization format for enum types
pub fn generate_serialization_format_for_enum(
    enum_info: &EnumInfo,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<SerializationFormat> {
    debug_context.push("Processing enum type for serialization format".to_string());

    let variants = extract_enum_variants(enum_info);

    if variants.is_empty() {
        return Err(DiscoveryError::format_generation("Enum has no variants"));
    }

    // Generate examples for all variants
    let mut variant_examples = serde_json::Map::new();

    for (variant_name, variant_info) in variants {
        debug_context.push(format!("Processing enum variant: {variant_name}"));

        let variant_example = match &variant_info {
            VariantInfo::Unit(_) => {
                debug_context.push(format!("Unit variant: {variant_name}"));
                json!(variant_name)
            }
            VariantInfo::Struct(struct_variant) => {
                debug_context.push(format!("Struct variant: {variant_name}"));
                let mut variant_fields = serde_json::Map::new();

                for field in struct_variant.iter() {
                    let field_name = field.name();
                    let field_type = field.type_path();

                    debug_context.push(format!(
                        "Processing variant field: {field_name}: {field_type}"
                    ));

                    let field_example = generate_default_example_for_type(field_type);
                    variant_fields.insert(field_name.to_string(), field_example);
                }

                let mut variant_obj = serde_json::Map::new();
                variant_obj.insert(variant_name.clone(), Value::Object(variant_fields));
                Value::Object(variant_obj)
            }
            VariantInfo::Tuple(tuple_variant) => {
                debug_context.push(format!("Tuple variant: {variant_name}"));
                let mut variant_fields = Vec::new();

                for (index, field) in tuple_variant.iter().enumerate() {
                    let field_type = field.type_path();

                    debug_context.push(format!(
                        "Processing variant tuple field {index}: {field_type}"
                    ));

                    let field_example = generate_default_example_for_type(field_type);
                    variant_fields.push(field_example);
                }

                let mut variant_obj = serde_json::Map::new();
                variant_obj.insert(variant_name.clone(), Value::Array(variant_fields));
                Value::Object(variant_obj)
            }
        };

        variant_examples.insert(format!("variant_{variant_name}"), variant_example);
    }

    let variant_count = variant_examples.len();
    Ok(SerializationFormat {
        example:     Value::Object(variant_examples),
        description: format!(
            "Serialization format for enum with {variant_count} variants (showing all possible variants)"
        ),
    })
}

/// Generate serialization format for primitive types
pub fn generate_serialization_format_for_primitive(
    type_name: &str,
    debug_context: &mut DebugContext,
) -> DiscoveryResult<SerializationFormat> {
    debug_context.push(format!("Processing primitive type: {type_name}"));

    let example = generate_primitive_example(type_name)?;

    Ok(SerializationFormat {
        example,
        description: format!("Serialization format for primitive type: {type_name}"),
    })
}
