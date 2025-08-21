//! Schema processing domain type for the V2 path
//!
//! This module provides a proper domain type `SchemaProcessor` that replaces
//! utility functions with methods for processing type schemas in the new V2 path.

use std::collections::HashMap;
use std::str::FromStr;

use serde_json::{Value, json};

use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::types::{BrpTypeName, MutationPath, SchemaField, TypeKind};
use crate::string_traits::JsonFieldAccess;

/// Domain type for schema processing in the new V2 path
///
/// This replaces utility functions with proper methods for better organization
/// and type safety in the V2 implementation.
pub struct SchemaProcessor<'a> {
    type_schema: &'a Value,
    registry:    &'a HashMap<BrpTypeName, Value>,
}

impl<'a> SchemaProcessor<'a> {
    /// Create a new schema processor for a type
    pub const fn new(type_schema: &'a Value, registry: &'a HashMap<BrpTypeName, Value>) -> Self {
        Self {
            type_schema,
            registry,
        }
    }

    /// Build mutation paths as a method
    pub fn build_mutation_paths(&self) -> Vec<MutationPath> {
        let mut mutation_paths = Vec::new();

        let Some(properties) = self
            .type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            return mutation_paths;
        };

        for (field_name, field_info) in properties {
            let paths = self.build_field_mutation_paths(field_name, field_info);
            mutation_paths.extend(paths);
        }

        mutation_paths
    }

    /// Build mutation paths for a single field
    fn build_field_mutation_paths(
        &self,
        field_name: &str,
        field_info: &Value,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        // Extract field type
        let field_type = Self::extract_field_type(field_info);

        let Some(ft) = field_type else {
            // No type info, add null mutation path
            paths.push(MutationPath {
                path:          format!(".{field_name}"),
                example:       json!(null),
                enum_variants: None,
                type_name:     None,
            });
            return paths;
        };

        // Check if this is an Option type
        let is_option = ft.starts_with("core::option::Option<");

        // Get example value and enum variants
        let (example_value, enum_variants) = self.get_field_example_and_variants(&ft, is_option);

        // Check for hardcoded knowledge
        if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&BrpTypeName::from(ft.as_str())) {
            paths.extend(Self::build_hardcoded_paths(
                field_name,
                &ft,
                hardcoded,
                is_option,
                enum_variants,
            ));
        } else {
            paths.push(Self::build_standard_path(
                field_name,
                &ft,
                example_value,
                enum_variants,
                is_option,
            ));
        }

        paths
    }

    /// Extract field type from field info
    fn extract_field_type(field_info: &Value) -> Option<String> {
        field_info
            .get_field(SchemaField::Type)
            .and_then(|t| t.get_field(SchemaField::Ref))
            .and_then(Value::as_str)
            .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"))
            .map(String::from)
    }

    /// Get example value and enum variants for a field
    fn get_field_example_and_variants(
        &self,
        field_type: &str,
        is_option: bool,
    ) -> (Value, Option<Vec<String>>) {
        if is_option {
            self.get_option_example_and_variants(field_type)
        } else {
            self.get_regular_example_and_variants(field_type)
        }
    }

    /// Get example and variants for Option types
    fn get_option_example_and_variants(&self, field_type: &str) -> (Value, Option<Vec<String>>) {
        // Extract inner type
        let inner_type = field_type
            .strip_prefix("core::option::Option<")
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or("");

        // Get example for inner type
        let inner_example = BRP_FORMAT_KNOWLEDGE
            .get(&inner_type.into())
            .map(|h| h.example_value.clone())
            .unwrap_or(json!(null));

        // Get variants from registry
        let variants = self
            .registry
            .get(&BrpTypeName::from(field_type))
            .and_then(|schema| {
                if Self::get_type_kind(schema) == Some(TypeKind::Enum) {
                    Self::extract_enum_variants(schema)
                } else {
                    None
                }
            });

        (inner_example, variants)
    }

    /// Get example and variants for regular types
    fn get_regular_example_and_variants(&self, field_type: &str) -> (Value, Option<Vec<String>>) {
        self.registry
            .get(&BrpTypeName::from(field_type))
            .map_or((json!(null), None), |schema| {
                if Self::get_type_kind(schema) == Some(TypeKind::Enum) {
                    let variants = Self::extract_enum_variants(schema);
                    let example = Self::build_enum_example(schema);
                    (example, variants)
                } else {
                    (json!(null), None)
                }
            })
    }

    /// Build paths for types with hardcoded knowledge
    fn build_hardcoded_paths(
        field_name: &str,
        field_type: &str,
        hardcoded: &super::types::BrpFormatKnowledge,
        is_option: bool,
        enum_variants: Option<Vec<String>>,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        // Build main path with appropriate example format
        let final_example = if is_option {
            json!({
                "some": hardcoded.example_value.clone(),
                "none": null
            })
        } else {
            hardcoded.example_value.clone()
        };

        paths.push(MutationPath {
            path: format!(".{field_name}"),
            example: final_example,
            enum_variants,
            type_name: Some(field_type.to_string()),
        });

        // Add component paths if available (e.g., .x, .y, .z for Vec3)
        if let Some(component_paths) = &hardcoded.subfield_paths {
            for (component, example_value) in component_paths {
                paths.push(MutationPath {
                    path:          format!(".{field_name}.{component}"),
                    example:       example_value.clone(),
                    enum_variants: None,
                    type_name:     None,
                });
            }
        }

        paths
    }

    /// Build standard mutation path
    fn build_standard_path(
        field_name: &str,
        field_type: &str,
        example_value: Value,
        enum_variants: Option<Vec<String>>,
        is_option: bool,
    ) -> MutationPath {
        let final_example = if is_option {
            json!({
                "some": example_value,
                "none": null
            })
        } else {
            example_value
        };

        MutationPath {
            path: format!(".{field_name}"),
            example: final_example,
            enum_variants,
            type_name: Some(field_type.to_string()),
        }
    }

    /// Extract enum variants from a type schema
    fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
        type_schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)
            .map(|one_of| {
                one_of
                    .iter()
                    .filter_map(|v| v.get_field(SchemaField::ShortPath).and_then(Value::as_str))
                    .map(std::string::ToString::to_string)
                    .collect()
            })
    }

    /// Get the type kind from a schema
    fn get_type_kind(schema: &Value) -> Option<TypeKind> {
        schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| TypeKind::from_str(s).ok())
    }

    /// Build example value for an enum type
    fn build_enum_example(schema: &Value) -> Value {
        if let Some(one_of) = schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)
        {
            if let Some(first_variant) = one_of.first() {
                if let Some(variant_name) = first_variant
                    .get_field(SchemaField::ShortPath)
                    .and_then(Value::as_str)
                {
                    // Check variant type to build appropriate example
                    if let Some(prefix_items) = first_variant
                        .get_field(SchemaField::PrefixItems)
                        .and_then(Value::as_array)
                    {
                        // Tuple variant
                        if let Some(first_item) = prefix_items.first() {
                            if let Some(type_ref) = first_item
                                .get_field(SchemaField::Type)
                                .and_then(|t| t.get_field(SchemaField::Ref))
                                .and_then(Value::as_str)
                            {
                                let inner_type =
                                    type_ref.strip_prefix("#/$defs/").unwrap_or(type_ref);

                                let inner_value = if inner_type.contains("Srgba") {
                                    json!({
                                        "red": 1.0,
                                        "green": 0.0,
                                        "blue": 0.0,
                                        "alpha": 1.0
                                    })
                                } else {
                                    json!({})
                                };

                                return json!({
                                    variant_name: [inner_value]
                                });
                            }
                        }
                        return json!({ variant_name: [] });
                    } else if first_variant.get_field(SchemaField::Properties).is_some() {
                        // Struct variant
                        return json!({ variant_name: {} });
                    }
                    // Unit variant
                    return json!(variant_name);
                }
            }
        }
        json!(null)
    }
}
