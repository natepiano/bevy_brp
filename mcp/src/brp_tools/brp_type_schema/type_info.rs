//! This is the main response structure use to convey type information
//! to the caller
use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::format_knowledge::BRP_FORMAT_KNOWLEDGE;
use super::mutation_path_builders::{
    EnumMutationBuilder, MutationPathBuilder, MutationPathContext, RootOrField,
};
use super::response_types::{
    BrpSupportedOperation, BrpTypeName, EnumVariantInfo, EnumVariantKind, MutationPath,
    MutationPathInternal, ReflectTrait, SchemaField, SchemaInfo, TypeKind,
};
use super::wrapper_types::WrapperType;
use crate::string_traits::JsonFieldAccess;

/// this is all of the information we provide about a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Fully-qualified type name
    pub type_name:            BrpTypeName,
    /// Whether the type is registered in the Bevy registry
    pub in_registry:          bool,
    /// Whether the type has the Serialize trait
    pub has_serialize:        bool,
    /// Whether the type has the Deserialize trait
    pub has_deserialize:      bool,
    /// List of BRP operations supported by this type
    pub supported_operations: Vec<BrpSupportedOperation>,
    /// Mutation paths available for this type - using same format as V1
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub mutation_paths:       HashMap<String, MutationPath>,
    /// Example values for spawn/insert operations (currently empty to match V1)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub example_values:       HashMap<String, Value>,
    /// Example format for spawn/insert operations when supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_format:         Option<Value>,
    /// Information about enum variants if this is an enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_info:            Option<Vec<EnumVariantInfo>>,
    /// Schema information from the registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_info:          Option<SchemaInfo>,
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:                Option<String>,
}

impl TypeInfo {
    /// Check if this type is a math type (based on BRP format knowledge)
    // pub fn is_math_type(&self) -> bool {
    //     BRP_FORMAT_KNOWLEDGE
    //         .get(&self.type_name)
    //         .is_some_and(|knowledge| knowledge.subfield_paths.is_some())
    // }

    /// Builder method to create `TypeInfo` from schema data
    pub fn from_schema(
        brp_type_name: BrpTypeName,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Extract type category for enum check
        let type_kind = TypeKind::from_schema(type_schema, &brp_type_name);

        // Extract reflection traits
        let reflect_types = Self::extract_reflect_types(type_schema);

        // Check for serialization traits
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        // Get supported operations
        let supported_operations = Self::get_supported_operations(&reflect_types);

        // Only get mutation paths if mutation is supported
        let can_mutate = supported_operations.contains(&BrpSupportedOperation::Mutate);
        let mutation_paths = if can_mutate {
            Self::get_mutation_paths(&brp_type_name, type_schema, registry)
        } else {
            HashMap::new()
        };

        // Build spawn format if spawn/insert is supported
        let can_spawn = supported_operations.contains(&BrpSupportedOperation::Spawn)
            || supported_operations.contains(&BrpSupportedOperation::Insert);
        let spawn_format = if can_spawn {
            Self::build_spawn_format(type_schema, registry)
        } else {
            None
        };

        // Build enum info if it's an enum
        let enum_info = if type_kind == TypeKind::Enum {
            Self::extract_enum_info(type_schema)
        } else {
            None
        };

        // Extract schema info from registry
        let schema_info = Self::extract_schema_info(type_schema);

        Self {
            type_name: brp_type_name,
            in_registry: true,
            has_serialize,
            has_deserialize,
            supported_operations,
            mutation_paths,
            example_values: HashMap::new(), // V1 always has this empty
            spawn_format,
            enum_info,
            schema_info,
            error: None,
        }
    }

    /// Builder method to create `TypeInfo` for type not found in registry
    pub fn not_found(type_name: BrpTypeName, error_msg: String) -> Self {
        Self {
            type_name,
            in_registry: false,
            has_serialize: false,
            has_deserialize: false,
            supported_operations: Vec::new(),
            mutation_paths: HashMap::new(),
            example_values: HashMap::new(),
            spawn_format: None,
            enum_info: None,
            schema_info: None,
            error: Some(error_msg),
        }
    }

    // Private helper methods (alphabetically ordered)

    /// Build mutation paths for a type using the trait system
    fn build_mutation_paths(
        brp_type_name: &BrpTypeName,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<MutationPathInternal> {
        let type_kind = TypeKind::from_schema(type_schema, brp_type_name);

        // Create root context for the new trait system
        let location = RootOrField::root(brp_type_name);
        let ctx = MutationPathContext::new(location, registry, None);

        // Use the new trait dispatch system
        type_kind.build_paths(&ctx).unwrap_or_else(|_| Vec::new())
    }

    /// Build spawn format example for types that support spawn/insert
    fn build_spawn_format(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Option<Value> {
        let properties = type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)?;

        let mut spawn_example = Map::new();

        for (field_name, field_info) in properties {
            // Extract field type
            let field_type = SchemaField::extract_field_type(field_info);

            if let Some(ft) = field_type {
                // Check for hardcoded knowledge
                let example = BRP_FORMAT_KNOWLEDGE.get(&ft).map_or_else(
                    || {
                        // Check if it's an enum and build example
                        registry.get(&ft).map_or(json!(null), |field_schema| {
                            let field_kind = TypeKind::from_schema(field_schema, &ft);
                            match field_kind {
                                TypeKind::Enum => {
                                    EnumMutationBuilder::build_enum_example(field_schema)
                                }
                                _ => json!(null),
                            }
                        })
                    },
                    |hardcoded| hardcoded.example_value.clone(),
                );

                spawn_example.insert(field_name.clone(), example);
            }
        }

        if spawn_example.is_empty() {
            None
        } else {
            Some(Value::Object(spawn_example))
        }
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(paths: &[MutationPathInternal]) -> HashMap<String, MutationPath> {
        let mut result = HashMap::new();

        for path in paths {
            // Generate description using the context
            let description = path.context.description();

            // Check if this is an Option type using the proper wrapper detection
            let is_option = matches!(
                WrapperType::detect(path.type_name.as_str()),
                Some((WrapperType::Option, _))
            );

            // Create MutationPathInfo from MutationPath
            let path_info = MutationPath::from_mutation_path(path, description, is_option);

            // Keep empty path as empty for root mutations
            // BRP expects empty string for root replacements, not "."
            let key = path.path.clone();

            result.insert(key, path_info);
        }

        result
    }

    /// Extract enum information from schema
    fn extract_enum_info(type_schema: &Value) -> Option<Vec<EnumVariantInfo>> {
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
    fn extract_enum_variant(v: &Value) -> Option<EnumVariantInfo> {
        let name = Self::get_variant_identifier(v)?;

        // Use the explicit kind field if available, otherwise infer from structure
        let variant_type = v
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| EnumVariantKind::from_str(s).ok())
            .unwrap_or_else(|| {
                if v.is_string() {
                    // Simple string variant
                } else {
                    // Object without explicit kind = unit variant with metadata
                }
                EnumVariantKind::Unit
            });

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
                        .map(BrpTypeName::from)
                        .collect()
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
                                .unwrap_or("unknown");
                            super::response_types::EnumFieldInfo {
                                field_name: field_name.clone(),
                                type_name:  BrpTypeName::from(type_name),
                            }
                        })
                        .collect()
                })
        } else {
            None
        };

        Some(EnumVariantInfo {
            variant_name: name.to_string(),
            variant_kind: variant_type,
            fields,
            tuple_types,
        })
    }

    /// Extract reflect types from a registry schema
    fn extract_reflect_types(type_schema: &Value) -> Vec<ReflectTrait> {
        type_schema
            .get_field(SchemaField::ReflectTypes)
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| s.parse::<ReflectTrait>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract schema information from registry schema
    fn extract_schema_info(type_schema: &Value) -> Option<SchemaInfo> {
        let type_kind = type_schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| TypeKind::from_str(s).ok());

        let properties = type_schema.get_field(SchemaField::Properties).cloned();

        let required = type_schema
            .get_field(SchemaField::Required)
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });

        let module_path = type_schema
            .get_field(SchemaField::ModulePath)
            .and_then(Value::as_str)
            .map(String::from);

        let crate_name = type_schema
            .get_field(SchemaField::CrateName)
            .and_then(Value::as_str)
            .map(String::from);

        // Only return SchemaInfo if we have at least some information
        if type_kind.is_some()
            || properties.is_some()
            || required.is_some()
            || module_path.is_some()
            || crate_name.is_some()
        {
            Some(SchemaInfo {
                type_kind,
                properties,
                required,
                module_path,
                crate_name,
            })
        } else {
            None
        }
    }

    /// Get mutation paths for a type as a `HashMap`
    fn get_mutation_paths(
        brp_type_name: &BrpTypeName,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, MutationPath> {
        let mutation_paths_vec = Self::build_mutation_paths(brp_type_name, type_schema, registry);
        Self::convert_mutation_paths(&mutation_paths_vec)
    }

    /// Get supported BRP operations based on reflection traits
    fn get_supported_operations(reflect_types: &[ReflectTrait]) -> Vec<BrpSupportedOperation> {
        let mut operations = vec![BrpSupportedOperation::Query];

        let has_component = reflect_types.contains(&ReflectTrait::Component);
        let has_resource = reflect_types.contains(&ReflectTrait::Resource);
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        if has_component {
            operations.push(BrpSupportedOperation::Get);
            // Mutation only requires reflection support (being in the registry)
            operations.push(BrpSupportedOperation::Mutate);
            if has_serialize && has_deserialize {
                operations.push(BrpSupportedOperation::Spawn);
                operations.push(BrpSupportedOperation::Insert);
            }
        }

        if has_resource {
            // Mutation only requires reflection support (being in the registry)
            operations.push(BrpSupportedOperation::Mutate);
            if has_serialize && has_deserialize {
                operations.push(BrpSupportedOperation::Insert);
            }
        }

        operations
    }

    /// Extract variant identifier from either string or object representation
    /// This returns the discriminant/name that identifies which variant this is,
    /// regardless of whether the variant contains data
    fn get_variant_identifier(v: &Value) -> Option<&str> {
        v.as_str().map_or_else(
            || v.get_field(SchemaField::ShortPath).and_then(Value::as_str),
            Some,
        )
    }
}
