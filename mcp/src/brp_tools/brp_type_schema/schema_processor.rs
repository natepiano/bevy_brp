//! Schema processing domain type for the V2 path
//!
//! This module provides a proper domain type `SchemaProcessor` that replaces
//! utility functions with methods for processing type schemas in the new V2 path.

use std::collections::HashSet;

use serde_json::{Map, Value, json};

use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::types::{BrpTypeName, MutationPath, SchemaField};
use crate::brp_tools::Port;
use crate::string_traits::JsonFieldAccess;

/// Domain type for schema processing in the new V2 path
///
/// This replaces utility functions with proper methods for better organization
/// and type safety in the V2 implementation.
pub struct SchemaProcessor<'a> {
    type_schema: &'a Value,
    type_name:   &'a str,
    port:        Port,
}

impl<'a> SchemaProcessor<'a> {
    /// Create a new schema processor for a type
    pub const fn new(type_schema: &'a Value, type_name: &'a str, port: Port) -> Self {
        Self {
            type_schema,
            type_name,
            port,
        }
    }

    /// Build spawn format using proper type methods
    pub fn build_spawn_format(&self) -> Map<String, Value> {
        let mut spawn_format = Map::new();

        if let Some(properties) = self
            .type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        {
            for (field_name, field_info) in properties {
                // Extract field type
                let field_type = field_info
                    .get_field(SchemaField::Type)
                    .and_then(|t| t.get_field(SchemaField::Ref))
                    .and_then(Value::as_str)
                    .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"));

                if let Some(ft) = field_type {
                    // Check if we have hardcoded knowledge for this type
                    if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&ft.into()) {
                        // Use the hardcoded example value
                        spawn_format.insert(field_name.clone(), hardcoded.example_value.clone());
                    } else {
                        // No hardcoded knowledge, use null as placeholder
                        spawn_format.insert(field_name.clone(), json!(null));
                    }
                } else {
                    // No type info, use null
                    spawn_format.insert(field_name.clone(), json!(null));
                }
            }
        }

        spawn_format
    }

    /// Build mutation paths as a method
    pub fn build_mutation_paths(&self) -> Vec<MutationPath> {
        let mut mutation_paths = Vec::new();

        if let Some(properties) = self
            .type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        {
            for (field_name, field_info) in properties {
                let field_type = field_info
                    .get_field(SchemaField::Type)
                    .and_then(|t| t.get_field(SchemaField::Ref))
                    .and_then(Value::as_str)
                    .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"));

                if let Some(ft) = field_type {
                    // Check if we have hardcoded knowledge for this type
                    if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&ft.into()) {
                        // Add the main field with the hardcoded example value
                        mutation_paths.push(MutationPath {
                            path:          format!(".{field_name}"),
                            example_value: hardcoded.example_value.clone(),
                            enum_variants: None,
                            type_name:     Some(ft.to_string()),
                        });

                        // Add component mutation paths if available (e.g., .x, .y, .z for Vec3)
                        if let Some(component_paths) = &hardcoded.subfield_paths {
                            for (component, example_value) in component_paths {
                                let component_path = format!(".{field_name}.{component}");
                                mutation_paths.push(MutationPath {
                                    path:          component_path,
                                    example_value: example_value.clone(),
                                    enum_variants: None,
                                    type_name:     None,
                                });
                            }
                        }
                    } else {
                        // No hardcoded knowledge, use null as before
                        mutation_paths.push(MutationPath {
                            path:          format!(".{field_name}"),
                            example_value: json!(null),
                            enum_variants: None,
                            type_name:     Some(ft.to_string()),
                        });
                    }
                } else {
                    // No type info, use null
                    mutation_paths.push(MutationPath {
                        path:          format!(".{field_name}"),
                        example_value: json!(null),
                        enum_variants: None,
                        type_name:     None,
                    });
                }
            }
        }

        mutation_paths
    }

    /// Correct math types using `set_field`
    pub const fn correct_math_type_schema(&mut self, schema: &mut Value) {
        // This method will be implemented when the infrastructure is used
        // For now, it's a placeholder for the V2 architecture
        let _ = (self, schema);
    }

    /// Extract dependencies from schema
    pub fn extract_dependencies(&self) -> HashSet<BrpTypeName> {
        let mut dependencies = HashSet::new();

        if let Some(properties) = self
            .type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        {
            for (_field_name, field_info) in properties {
                if let Some(field_type) = field_info
                    .get_field(SchemaField::Type)
                    .and_then(|t| t.get_field(SchemaField::Ref))
                    .and_then(Value::as_str)
                    .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"))
                {
                    // Only add non-primitive dependencies
                    if !Self::is_primitive_type(field_type) {
                        dependencies.insert(BrpTypeName::from(field_type));
                    }
                }
            }
        }

        dependencies
    }

    /// Check if a type is primitive and should not be processed as dependency
    fn is_primitive_type(type_name: &str) -> bool {
        type_name.starts_with("core::")
            || type_name.starts_with("alloc::")
            || type_name.starts_with("std::")
    }
}
