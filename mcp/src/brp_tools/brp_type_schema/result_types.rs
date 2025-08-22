//! Public API result types for the `brp_type_schema` tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::warn;

use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::types::{
    BrpSupportedOperation, BrpTypeName, EnumVariantKind, MutationPath, ReflectTrait, SchemaField,
    TypeKind,
};
use super::wrapper_types::WrapperType;
use crate::string_traits::{IntoStrings, JsonFieldAccess};

/// Summary statistics for the discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaSummary {
    /// Number of types that failed discovery
    pub failed_discoveries:     usize,
    /// Number of types successfully discovered
    pub successful_discoveries: usize,
    /// Total number of types requested
    pub total_requested:        usize,
}

/// Information about a mutation path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathInfo {
    /// Human-readable description of what this path mutates
    pub description:   String,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:     String,
    /// Example value for mutations (for non-Option types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:       Option<Value>,
    /// Example value for setting Some variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_some:  Option<Value>,
    /// Example value for setting None variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_none:  Option<Value>,
    /// List of valid enum variants for this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_variants: Option<Vec<String>>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:          Option<String>,
}

/// Information about an enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantInfo {
    /// Name of the variant
    pub name:         String,
    /// Type of the variant (Unit, Tuple, Struct)
    pub variant_type: EnumVariantKind,
    /// Fields for struct variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields:       Option<Vec<EnumFieldInfo>>,
    /// Types for tuple variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuple_types:  Option<Vec<String>>,
}

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumFieldInfo {
    /// Field name
    pub name:      String,
    /// Field type
    #[serde(rename = "type")]
    pub type_name: String,
}

impl MutationPathInfo {
    /// Create from internal `MutationPath` with proper formatting logic
    pub fn from_mutation_path(path: &MutationPath, description: String, is_option: bool) -> Self {
        if is_option {
            // For Option types, check if we have the special format
            if let Some(examples_obj) = path.example.as_object()
                && examples_obj.contains_key("some")
                && examples_obj.contains_key("none")
            {
                return Self {
                    description,
                    type_name: path.type_name.to_string(),
                    example: None,
                    example_some: Some(examples_obj["some"].clone()),
                    example_none: Some(examples_obj["none"].clone()),
                    enum_variants: path.enum_variants.clone(),
                    note: Some(
                        "For Option fields: pass the value directly to set Some, null to set None"
                            .to_string(),
                    ),
                };
            }
        }

        // Regular non-Option path
        Self {
            description,
            type_name: path.type_name.to_string(),
            example: if path.example.is_null() {
                None
            } else {
                Some(path.example.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants: path.enum_variants.clone(),
            note: None,
        }
    }
}

// V2 Response Types for parallel implementation

/// V2 response structure - same as V1 but uses `TypeInfoV2`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaResponse {
    /// Number of types successfully discovered
    pub discovered_count: usize,
    /// List of type names that were requested
    pub requested_types:  Vec<String>,
    /// Summary statistics for the discovery operation
    pub summary:          TypeSchemaSummary,
    /// Detailed information for each type, keyed by type name
    pub type_info:        HashMap<BrpTypeName, TypeInfo>,
}

/// V2 version of `TypeInfo` - same structure as V1 but without `registry_schema` field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Fully-qualified type name
    pub type_name:            BrpTypeName,
    /// Category of the type (Struct, Enum, etc.)
    pub type_kind:            TypeKind,
    /// Whether the type is registered in the Bevy registry
    pub in_registry:          bool,
    /// Whether the type has the Serialize trait
    pub has_serialize:        bool,
    /// Whether the type has the Deserialize trait
    pub has_deserialize:      bool,
    /// List of BRP operations supported by this type
    pub supported_operations: Vec<String>,
    /// Mutation paths available for this type - using same format as V1
    pub mutation_paths:       HashMap<String, MutationPathInfo>,
    /// Example values for spawn/insert operations (currently empty to match V1)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub example_values:       HashMap<String, Value>,
    /// Information about enum variants if this is an enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_info:            Option<Vec<EnumVariantInfo>>,
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:                Option<String>,
}

impl TypeInfo {
    /// Builder method to create `TypeInfo` from schema data
    pub fn from_schema(
        brp_type_name: BrpTypeName,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Extract type category - default to Value if missing/invalid
        let type_kind = Self::get_type_kind(type_schema, &brp_type_name);

        // Extract reflection traits
        let reflect_types = Self::extract_reflect_types(type_schema);

        // Check for serialization traits
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        // Determine supported operations
        let operations = Self::determine_supported_operations(&reflect_types);
        let operations_strings: Vec<String> = operations
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        // Build mutation paths
        let mutation_paths_vec = Self::build_mutation_paths(type_schema, registry);
        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec);

        // Build enum info if it's an enum
        let enum_info = if type_kind == TypeKind::Enum {
            Self::extract_enum_info(type_schema)
        } else {
            None
        };

        Self {
            type_name: brp_type_name,
            type_kind,
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

    /// Builder method to create `TypeInfo` for type not found in registry
    pub fn not_found(type_name: BrpTypeName, error_msg: String) -> Self {
        Self {
            type_name,
            type_kind: TypeKind::Value, // Default to Value for unknown types
            in_registry: false,
            has_serialize: false,
            has_deserialize: false,
            supported_operations: Vec::new(),
            mutation_paths: HashMap::new(),
            example_values: HashMap::new(),
            enum_info: None,
            error: Some(error_msg),
        }
    }

    // Private helper methods

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

    /// Determine supported BRP operations based on reflection traits
    fn determine_supported_operations(
        reflect_types: &[ReflectTrait],
    ) -> Vec<BrpSupportedOperation> {
        let mut operations = vec![BrpSupportedOperation::Query];

        let has_component = reflect_types.contains(&ReflectTrait::Component);
        let has_resource = reflect_types.contains(&ReflectTrait::Resource);
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        if has_component {
            operations.push(BrpSupportedOperation::Get);
            if has_serialize && has_deserialize {
                operations.push(BrpSupportedOperation::Spawn);
                operations.push(BrpSupportedOperation::Insert);
            }
            if has_serialize {
                operations.push(BrpSupportedOperation::Mutate);
            }
        }

        if has_resource {
            if has_serialize && has_deserialize {
                operations.push(BrpSupportedOperation::Insert);
            }
            if has_serialize {
                operations.push(BrpSupportedOperation::Mutate);
            }
        }

        operations
    }

    /// Build mutation paths for a type
    fn build_mutation_paths(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<MutationPath> {
        let mut mutation_paths = Vec::new();

        let Some(properties) = type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            return mutation_paths;
        };

        for (field_name, field_info) in properties {
            let paths = Self::build_field_mutation_paths(field_name, field_info, registry);
            mutation_paths.extend(paths);
        }

        mutation_paths
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(paths: &[MutationPath]) -> HashMap<String, MutationPathInfo> {
        let mut result = HashMap::new();

        for path in paths {
            // Generate description based on path
            let description = Self::generate_mutation_description(&path.path);

            // Check if this is an Option type
            let is_option = path.type_name.as_str().starts_with("core::option::Option<");

            // Create MutationPathInfo from MutationPath
            let path_info = MutationPathInfo::from_mutation_path(path, description, is_option);

            result.insert(path.path.clone(), path_info);
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

    /// Build mutation paths for a single field
    fn build_field_mutation_paths(
        field_name: &str,
        field_info: &Value,
        registry: &HashMap<BrpTypeName, Value>,
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
                type_name:     BrpTypeName::from("unknown"),
            });
            return paths;
        };

        // Check if this is a wrapper type (Option, Handle)
        let wrapper_info = WrapperType::detect(&ft);

        // Get example value and enum variants based on type
        let (example_value, enum_variants) =
            Self::get_field_example_and_variants(&ft, wrapper_info, registry);

        // Check for hardcoded knowledge
        // For wrapper types, check the inner type for hardcoded knowledge
        let type_to_check = wrapper_info.map_or(ft.as_str(), |(_, inner)| inner);

        if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&BrpTypeName::from(type_to_check)) {
            paths.extend(Self::build_hardcoded_paths(
                field_name,
                &ft,
                hardcoded,
                wrapper_info,
                enum_variants,
            ));
        } else {
            paths.push(Self::build_standard_path(
                field_name,
                &ft,
                example_value,
                enum_variants,
                wrapper_info,
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

    /// Extract a single enum variant from schema
    fn extract_enum_variant(v: &Value) -> Option<EnumVariantInfo> {
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
                            super::result_types::EnumFieldInfo {
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

    /// Get example value and enum variants for a field
    fn get_field_example_and_variants(
        field_type: &str,
        wrapper_info: Option<(WrapperType, &str)>,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> (Value, Option<Vec<String>>) {
        match wrapper_info {
            Some((wrapper_type, inner_type)) => {
                // Handle wrapper types - they don't get enum_variants
                let inner_example = BRP_FORMAT_KNOWLEDGE
                    .get(&BrpTypeName::from(inner_type))
                    .map(|h| h.example_value.clone())
                    .unwrap_or(json!(null));

                // Build the appropriate example based on wrapper type
                let example = match wrapper_type {
                    WrapperType::Option => inner_example, /* For Option, we return the inner */
                    // example
                    WrapperType::Handle => wrapper_type.wrap_example(json!({})), /* For Handle,
                                                                                  * wrap it */
                };

                (example, None) // Wrapper types never get enum_variants
            }
            None => {
                // Regular types - check if they're enums
                registry
                    .get(&BrpTypeName::from(field_type))
                    .map_or((json!(null), None), |schema| {
                        if schema
                            .get_field(SchemaField::Kind)
                            .and_then(Value::as_str)
                            .and_then(|s| TypeKind::from_str(s).ok())
                            == Some(TypeKind::Enum)
                        {
                            let variants = Self::extract_enum_variants(schema);
                            let example = Self::build_enum_example(schema);
                            (example, variants)
                        } else {
                            (json!(null), None)
                        }
                    })
            }
        }
    }

    /// Build paths for types with hardcoded knowledge
    fn build_hardcoded_paths(
        field_name: &str,
        field_type: &str,
        hardcoded: &super::types::BrpFormatKnowledge,
        wrapper_info: Option<(WrapperType, &str)>,
        enum_variants: Option<Vec<String>>,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        // Build main path with appropriate example format
        let final_example = if matches!(wrapper_info, Some((WrapperType::Option, _))) {
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
            type_name: BrpTypeName::from(field_type),
        });

        // Add component paths if available (e.g., .x, .y, .z for Vec3)
        if let Some(component_paths) = &hardcoded.subfield_paths {
            for (component, example_value) in component_paths {
                paths.push(MutationPath {
                    path:          format!(".{field_name}.{component}"),
                    example:       example_value.clone(),
                    enum_variants: None,
                    type_name:     BrpTypeName::from("f32"),
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
        wrapper_info: Option<(WrapperType, &str)>,
    ) -> MutationPath {
        let final_example = if matches!(wrapper_info, Some((WrapperType::Option, _))) {
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
            type_name: BrpTypeName::from(field_type),
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
    fn get_type_kind(schema: &Value, type_name: &BrpTypeName) -> TypeKind {
        schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| TypeKind::from_str(s).ok())
            .unwrap_or_else(|| {
                warn!(
                    "Type '{}' has missing or invalid 'kind' field in registry schema, defaulting to TypeKind::Value",
                    type_name
                );
                TypeKind::Value
            })
    }

    /// Build example value for an enum type
    fn build_enum_example(schema: &Value) -> Value {
        if let Some(one_of) = schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)
            && let Some(first_variant) = one_of.first()
            && let Some(variant_name) = first_variant
                .get_field(SchemaField::ShortPath)
                .and_then(Value::as_str)
        {
            // Check variant type to build appropriate example
            if let Some(prefix_items) = first_variant
                .get_field(SchemaField::PrefixItems)
                .and_then(Value::as_array)
            {
                // Tuple variant
                if let Some(first_item) = prefix_items.first()
                    && let Some(type_ref) = first_item
                        .get_field(SchemaField::Type)
                        .and_then(|t| t.get_field(SchemaField::Ref))
                        .and_then(Value::as_str)
                {
                    let inner_type = type_ref.strip_prefix("#/$defs/").unwrap_or(type_ref);

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
                return json!({ variant_name: [] });
            } else if first_variant.get_field(SchemaField::Properties).is_some() {
                // Struct variant
                return json!({ variant_name: {} });
            }
            // Unit variant
            return json!(variant_name);
        }
        json!(null)
    }
}
