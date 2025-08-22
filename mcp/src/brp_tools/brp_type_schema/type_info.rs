//! This is the main response structure use to convey type information
//! to the caller
use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tracing::warn;

use super::format_knowledge::{BRP_FORMAT_KNOWLEDGE, BrpFormatKnowledge};
use super::response_types::{
    BrpSupportedOperation, BrpTypeName, EnumVariantInfo, EnumVariantKind, MutationPath,
    MutationPathInfo, OptionField, ReflectTrait, SchemaField, SchemaInfo, TypeKind,
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
    pub supported_operations: Vec<String>,
    /// Mutation paths available for this type - using same format as V1
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub mutation_paths:       HashMap<String, MutationPathInfo>,
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
    /// Builder method to create `TypeInfo` from schema data
    pub fn from_schema(
        brp_type_name: BrpTypeName,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Extract type category for enum check
        let type_kind = Self::get_type_kind(type_schema, &brp_type_name);

        // Extract reflection traits
        let reflect_types = Self::extract_reflect_types(type_schema);

        // Check for serialization traits
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        // Get supported operations directly as strings
        let supported_operations = Self::get_supported_operations(&reflect_types);

        // Only get mutation paths if mutation is supported
        let can_mutate = supported_operations.contains(&"mutate".to_string());
        let mutation_paths = if can_mutate {
            Self::get_mutation_paths(type_schema, registry)
        } else {
            HashMap::new()
        };

        // Build spawn format if spawn/insert is supported
        let can_spawn = supported_operations.contains(&"spawn".to_string())
            || supported_operations.contains(&"insert".to_string());
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

    /// Get supported BRP operations as strings based on reflection traits
    fn get_supported_operations(reflect_types: &[ReflectTrait]) -> Vec<String> {
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
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    }

    /// Get mutation paths for a type as a `HashMap`
    fn get_mutation_paths(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, MutationPathInfo> {
        let mutation_paths_vec = Self::build_mutation_paths(type_schema, registry);
        Self::convert_mutation_paths(&mutation_paths_vec)
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
        let wrapper_info = WrapperType::detect(ft.as_str());

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
    fn extract_field_type(field_info: &Value) -> Option<BrpTypeName> {
        field_info
            .get_field(SchemaField::Type)
            .and_then(|t| t.get_field(SchemaField::Ref))
            .and_then(Value::as_str)
            .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"))
            .map(BrpTypeName::from)
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

    /// Get example value and enum variants for a field
    fn get_field_example_and_variants(
        field_type: &BrpTypeName,
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
                    .get(field_type)
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
        field_type: &BrpTypeName,
        hardcoded: &BrpFormatKnowledge,
        wrapper_info: Option<(WrapperType, &str)>,
        enum_variants: Option<Vec<String>>,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        // Build main path with appropriate example format
        let final_example = if matches!(wrapper_info, Some((WrapperType::Option, _))) {
            let mut option_example = Map::new();
            option_example.insert_field(OptionField::Some, hardcoded.example_value.clone());
            option_example.insert_field(OptionField::None, json!(null));
            Value::Object(option_example)
        } else {
            hardcoded.example_value.clone()
        };

        paths.push(MutationPath {
            path: format!(".{field_name}"),
            example: final_example,
            enum_variants,
            type_name: field_type.clone(),
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
        field_type: &BrpTypeName,
        example_value: Value,
        enum_variants: Option<Vec<String>>,
        wrapper_info: Option<(WrapperType, &str)>,
    ) -> MutationPath {
        let final_example = if matches!(wrapper_info, Some((WrapperType::Option, _))) {
            let mut option_example = Map::new();
            option_example.insert_field(OptionField::Some, example_value);
            option_example.insert_field(OptionField::None, json!(null));
            Value::Object(option_example)
        } else {
            example_value
        };

        MutationPath {
            path: format!(".{field_name}"),
            example: final_example,
            enum_variants,
            type_name: field_type.clone(),
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

    /// Extract schema information from registry schema
    fn extract_schema_info(type_schema: &Value) -> Option<SchemaInfo> {
        let type_kind = type_schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| TypeKind::from_str(s).ok());

        let properties = type_schema.get_field(SchemaField::Properties).cloned();

        let required = type_schema
            .get("required")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });

        let module_path = type_schema
            .get("modulePath")
            .and_then(Value::as_str)
            .map(String::from);

        let crate_name = type_schema
            .get("crateName")
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
            let field_type = Self::extract_field_type(field_info);

            if let Some(ft) = field_type {
                // Check for hardcoded knowledge
                let example = BRP_FORMAT_KNOWLEDGE.get(&ft).map_or_else(
                    || {
                        // Check if it's an enum and build example
                        registry.get(&ft).map_or(json!(null), |field_schema| {
                            if field_schema
                                .get_field(SchemaField::Kind)
                                .and_then(Value::as_str)
                                == Some("Enum")
                            {
                                Self::build_enum_example(field_schema)
                            } else {
                                json!(null)
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
}
