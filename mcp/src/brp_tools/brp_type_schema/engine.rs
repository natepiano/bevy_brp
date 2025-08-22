//! V2 engine for type schema generation
//!
//! This module provides the new parallel implementation of type schema generation
//! that will eventually replace the original engine. It uses the complete registry
//! approach instead of recursive discovery.

use std::collections::HashMap;

use serde_json::Value;

use super::registry_cache::get_full_registry;
use super::result_types::{MutationPathInfo, TypeInfo, TypeSchemaResponse, TypeSchemaSummary};
use super::schema_processor::SchemaProcessor;
use super::type_discovery::{determine_supported_operations, extract_reflect_types};
use super::types::{BrpTypeName, EnumVariantKind, ReflectTrait, SchemaField};
use crate::brp_tools::Port;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

/// V2 engine for type schema generation using complete registry approach
pub struct TypeSchemaEngine {
    registry: HashMap<BrpTypeName, Value>,
}

impl TypeSchemaEngine {
    /// Create a new V2 engine instance
    pub async fn new(port: Port) -> Result<Self> {
        let registry = get_full_registry(port).await?;
        Ok(Self { registry })
    }

    /// Generate response for requested types using the V2 approach
    pub fn generate_response(&self, requested_types: &[String]) -> TypeSchemaResponse {
        let mut response = TypeSchemaResponse {
            discovered_count: 0,
            requested_types:  requested_types.to_vec(),
            success:          true,
            summary:          TypeSchemaSummary {
                failed_discoveries:     0,
                successful_discoveries: 0,
                total_requested:        requested_types.len(),
            },
            type_info:        HashMap::new(),
        };

        for type_name in requested_types {
            let brp_type_name = BrpTypeName::from(type_name);

            if let Some(type_schema) = self.registry.get(&brp_type_name) {
                // Build TypeInfoV2 for this type
                let type_info = self.build_type_info(type_name, type_schema);

                response.type_info.insert(type_name.clone(), type_info);
                response.discovered_count += 1;
                response.summary.successful_discoveries += 1;
            } else {
                // Type not found - add error
                let type_info = TypeInfo {
                    type_name:            type_name.clone(),
                    type_category:        "Unknown".to_string(),
                    in_registry:          false,
                    has_serialize:        false,
                    has_deserialize:      false,
                    supported_operations: Vec::new(),
                    mutation_paths:       HashMap::new(),
                    example_values:       HashMap::new(),
                    enum_info:            None,
                    error:                Some("Type not found in registry".to_string()),
                };

                response.type_info.insert(type_name.clone(), type_info);
                response.summary.failed_discoveries += 1;
            }
        }

        response
    }

    /// Build `TypeInfoV2` for a single type
    fn build_type_info(&self, type_name: &str, type_schema: &Value) -> TypeInfo {
        // Use SchemaProcessor for this type
        let processor = SchemaProcessor::new(type_schema, &self.registry);

        // Build mutation paths
        let mutation_paths_vec = processor.build_mutation_paths();

        // Convert mutation paths to HashMap<String, MutationPathInfo> format
        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec);

        // Extract type category
        let type_category = type_schema
            .get_field("kind")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
            .to_string();

        // Extract reflection traits
        let reflect_types = extract_reflect_types(type_schema);

        // Check for serialization traits
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        // Determine supported operations
        let operations = determine_supported_operations(&reflect_types);
        let operations_strings: Vec<String> = operations
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        // Build enum info if it's an enum
        let enum_info = if type_category == "Enum" {
            Self::build_enum_info(type_schema)
        } else {
            None
        };

        TypeInfo {
            type_name: type_name.to_string(),
            type_category,
            in_registry: true,
            has_serialize,
            has_deserialize,
            supported_operations: operations_strings,
            mutation_paths,
            example_values: HashMap::new(), // V1 always has this empty
            enum_info,
            error: None,
        }
    }

    /// Convert Vec<MutationPath> to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(
        paths: &[super::types::MutationPath],
    ) -> HashMap<String, MutationPathInfo> {
        let mut result = HashMap::new();

        for path in paths {
            // Generate description based on path
            let description = Self::generate_mutation_description(&path.path);

            // Check if this is an Option type
            let is_option = path
                .type_name
                .as_ref()
                .is_some_and(|t| t.starts_with("core::option::Option<"));

            // Create MutationPathInfo from MutationPath
            let path_info = MutationPathInfo::from_mutation_path(path, description, is_option);

            result.insert(path.path.clone(), path_info);
        }

        result
    }

    /// Generate a description for a mutation path
    fn generate_mutation_description(path: &str) -> String {
        // Remove leading dot and split
        let parts: Vec<&str> = path.trim_start_matches('.').split('.').collect();

        if parts.len() == 1 {
            format!("Mutate the entire {} field", parts[0])
        } else if parts.len() == 2 {
            // Component path like .rotation.x
            format!("Mutate the {} component", parts[1])
        } else {
            format!("Mutate path {path}")
        }
    }

    /// Build enum info for enum types
    fn build_enum_info(type_schema: &Value) -> Option<Vec<super::result_types::EnumVariantInfo>> {
        use super::result_types::EnumVariantInfo;

        let one_of = type_schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)?;

        let variants: Vec<EnumVariantInfo> = one_of
            .iter()
            .filter_map(Self::extract_enum_variant)
            .collect();

        Some(variants)
    }

    /// Extract a single enum variant from schema
    fn extract_enum_variant(v: &Value) -> Option<super::result_types::EnumVariantInfo> {
        use super::result_types::{EnumFieldInfo, EnumVariantInfo};
        use crate::string_traits::IntoStrings;

        let name = v
            .get_field(SchemaField::ShortPath)
            .and_then(Value::as_str)?;

        // Check if this is a unit variant, tuple variant, or struct variant
        let variant_type = if v.get_field(SchemaField::PrefixItems).is_some() {
            EnumVariantKind::Tuple
        } else if v.get_field(SchemaField::Properties).is_some() {
            EnumVariantKind::Struct
        } else {
            EnumVariantKind::Unit
        };

        // Extract tuple types if present
        let tuple_types = if variant_type == EnumVariantKind::Tuple {
            v.get_field(SchemaField::PrefixItems)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            item.get_field(SchemaField::Type).and_then(Value::as_str)
                        })
                        .into_strings()
                })
        } else {
            None
        };

        // Extract struct fields if present
        let fields = if variant_type == EnumVariantKind::Struct {
            v.get_field(SchemaField::Properties)
                .and_then(Value::as_object)
                .map(|props| {
                    props
                        .iter()
                        .map(|(field_name, field_value)| {
                            let type_name = field_value
                                .get_field(SchemaField::Type)
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                                .to_string();
                            EnumFieldInfo {
                                name: field_name.clone(),
                                type_name,
                            }
                        })
                        .collect()
                })
        } else {
            None
        };

        Some(EnumVariantInfo {
            name: name.to_string(),
            variant_type,
            fields,
            tuple_types,
        })
    }
}
