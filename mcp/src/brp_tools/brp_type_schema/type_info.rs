//! This is the main response structure use to convey type information
//! to the caller
use std::collections::HashMap;
use std::str::FromStr;

use serde::Serialize;
use serde_json::{Map, Value, json};

use super::constants::{
    DEFAULT_EXAMPLE_ARRAY_SIZE, MAX_EXAMPLE_ARRAY_SIZE, MAX_TYPE_RECURSION_DEPTH, SCHEMA_REF_PREFIX,
};
use super::format_knowledge::{BRP_FORMAT_KNOWLEDGE, FormatKnowledgeKey};
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
#[derive(Debug, Clone, Serialize)]
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
    /// Type information for direct fields (struct fields only, one level deep)
    #[serde(skip)]
    pub field_type_infos:     HashMap<String, TypeInfo>,
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:                Option<String>,
}

impl TypeInfo {
    /// Check if this type is a math type (based on BRP format knowledge)
    pub fn is_math_type(&self) -> bool {
        BRP_FORMAT_KNOWLEDGE
            .get(&FormatKnowledgeKey::exact(&self.type_name))
            .is_some_and(|knowledge| knowledge.subfield_paths.is_some())
    }

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
            Self::build_spawn_format(type_schema, registry, &type_kind, &brp_type_name)
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

        // Build field TypeInfos for struct types (one level deep)
        let field_type_infos = if type_kind == TypeKind::Struct {
            Self::build_field_type_infos(type_schema, registry)
        } else {
            HashMap::new()
        };

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
            field_type_infos,
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
            field_type_infos: HashMap::new(),
            error: Some(error_msg),
        }
    }

    // Private helper methods (alphabetically ordered)

    /// Build `TypeInfos` for direct fields of a struct (one level deep)
    fn build_field_type_infos(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, Self> {
        let mut field_infos = HashMap::new();

        // Extract properties from the schema
        let Some(properties) = type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            return field_infos;
        };

        // Build TypeInfo for each field
        for (field_name, field_info) in properties {
            if let Some(field_type_name) = SchemaField::extract_field_type(field_info) {
                // Look up the field type in the registry and build its TypeInfo
                if let Some(field_schema) = registry.get(&field_type_name) {
                    let field_type_info =
                        Self::from_schema(field_type_name.clone(), field_schema, registry);
                    field_infos.insert(field_name.clone(), field_type_info);
                }
            }
        }

        field_infos
    }

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
        type_kind: &TypeKind,
        type_name: &BrpTypeName,
    ) -> Option<Value> {
        // Check for hardcoded format knowledge first - this fixes GlobalTransform and other types
        if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&FormatKnowledgeKey::exact(type_name)) {
            return Some(hardcoded.example_value.clone());
        }

        // Try generic match by stripping type parameters
        if let Some(generic_type) = type_name.as_str().split('<').next()
            && let Some(hardcoded) =
                BRP_FORMAT_KNOWLEDGE.get(&FormatKnowledgeKey::generic(generic_type))
        {
            return Some(hardcoded.example_value.clone());
        }

        match type_kind {
            TypeKind::TupleStruct | TypeKind::Tuple => {
                Self::build_tuple_spawn_format(type_schema, registry)
            }
            TypeKind::Struct => Self::build_struct_spawn_format(type_schema, registry),
            _ => None,
        }
    }

    /// Extract type name from a $ref field in schema
    ///
    /// Handles the common pattern of extracting a type reference from:
    /// ```json
    /// { "type": { "$ref": "#/$defs/TypeName" } }
    /// ```
    fn extract_type_ref_from_field(field: &Value) -> Option<BrpTypeName> {
        field
            .get_field(SchemaField::Type)
            .and_then(|t| t.get("$ref"))
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix(SCHEMA_REF_PREFIX))
            .map(BrpTypeName::from)
    }

    /// Extract type name from a type field using `SchemaField::Ref`
    ///
    /// Similar to `extract_type_ref_from_field` but uses `SchemaField::Ref`
    /// for accessing the $ref field
    fn extract_type_ref_with_schema_field(type_value: &Value) -> Option<BrpTypeName> {
        type_value
            .get_field(SchemaField::Ref)
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix(SCHEMA_REF_PREFIX))
            .map(BrpTypeName::from)
    }

    /// Build spawn format for struct types with named properties
    fn build_struct_spawn_format(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Option<Value> {
        let properties = type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)?;

        let mut spawn_example = Map::new();

        for (field_name, field_info) in properties {
            let field_type = SchemaField::extract_field_type(field_info);
            if let Some(ft) = field_type {
                let example = Self::build_example_value_for_type(&ft, registry);
                spawn_example.insert(field_name.clone(), example);
            }
        }

        if spawn_example.is_empty() {
            None
        } else {
            Some(Value::Object(spawn_example))
        }
    }

    /// Build spawn format for tuple struct types with indexed fields
    fn build_tuple_spawn_format(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Option<Value> {
        let prefix_items = type_schema
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array)?;

        let tuple_examples: Vec<Value> = prefix_items
            .iter()
            .map(|item| {
                Self::extract_type_ref_from_field(item).map_or_else(
                    || json!(null),
                    |ft| Self::build_example_value_for_type(&ft, registry),
                )
            })
            .collect();

        if tuple_examples.is_empty() {
            None
        } else if tuple_examples.len() == 1 {
            // Special case: single-field tuple structs are unwrapped by BRP
            // Return the inner value directly, not as an array
            tuple_examples.into_iter().next()
        } else {
            Some(Value::Array(tuple_examples))
        }
    }

    /// Build an example value for a specific type
    pub fn build_example_value_for_type(
        type_name: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Value {
        Self::build_example_value_for_type_with_depth(type_name, registry, 0)
    }

    /// Build an example value for a specific type with recursion depth tracking
    fn build_example_value_for_type_with_depth(
        type_name: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        depth: usize,
    ) -> Value {
        // Prevent stack overflow from deep recursion
        if depth > MAX_TYPE_RECURSION_DEPTH {
            return json!(null);
        }

        // Check for hardcoded knowledge first - try exact match
        if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&FormatKnowledgeKey::exact(type_name)) {
            return hardcoded.example_value.clone();
        }

        // Try generic match by stripping type parameters
        if let Some(generic_type) = type_name.as_str().split('<').next()
            && let Some(hardcoded) =
                BRP_FORMAT_KNOWLEDGE.get(&FormatKnowledgeKey::generic(generic_type))
        {
            return hardcoded.example_value.clone();
        }

        // Check for wrapper types (Option, Handle) and use their default examples
        if let Some((wrapper_type, _)) = WrapperType::detect(type_name.as_str()) {
            // For wrapper types, use the wrapper's default example
            // This handles Option::None -> null, Handle::Weak -> {"Weak": {}}
            return wrapper_type.default_example();
        }

        // Check if we have the type in the registry
        let Some(field_schema) = registry.get(type_name) else {
            return json!(null);
        };

        let field_kind = TypeKind::from_schema(field_schema, type_name);
        match field_kind {
            TypeKind::Enum => EnumMutationBuilder::build_enum_example(field_schema, registry),
            TypeKind::Array => {
                // Handle array types like [f32; 4] or [glam::Vec2; 3]
                // Arrays have an "items" field with the element type
                let item_type = field_schema
                    .get_field(SchemaField::Items)
                    .and_then(|items| items.get_field(SchemaField::Type))
                    .and_then(Self::extract_type_ref_with_schema_field);

                item_type.map_or(json!(null), |item_type_name| {
                    // Generate example value for the item type
                    let item_example = Self::build_example_value_for_type_with_depth(
                        &item_type_name,
                        registry,
                        depth + 1,
                    );

                    // Parse the array size from the type name (e.g., "[f32; 4]" -> 4)
                    let size = type_name
                        .as_str()
                        .rsplit_once("; ")
                        .and_then(|(_, rest)| rest.strip_suffix(']'))
                        .and_then(|s| s.parse::<usize>().ok())
                        .map_or(DEFAULT_EXAMPLE_ARRAY_SIZE, |s| {
                            s.min(MAX_EXAMPLE_ARRAY_SIZE)
                        });

                    // Create array with the appropriate number of elements
                    let array = vec![item_example; size];
                    json!(array)
                })
            }
            TypeKind::Tuple | TypeKind::TupleStruct => {
                // Handle tuple types with prefixItems
                field_schema
                    .get_field(SchemaField::PrefixItems)
                    .and_then(Value::as_array)
                    .map_or(json!(null), |prefix_items| {
                        let tuple_examples: Vec<Value> = prefix_items
                            .iter()
                            .map(|item| {
                                item.get_field(SchemaField::Type)
                                    .and_then(Self::extract_type_ref_with_schema_field)
                                    .map_or_else(
                                        || json!(null),
                                        |ft| {
                                            Self::build_example_value_for_type_with_depth(
                                                &ft,
                                                registry,
                                                depth + 1,
                                            )
                                        },
                                    )
                            })
                            .collect();

                        if tuple_examples.is_empty() {
                            json!(null)
                        } else {
                            json!(tuple_examples)
                        }
                    })
            }
            TypeKind::Struct => {
                // Build struct example from properties
                field_schema
                    .get_field(SchemaField::Properties)
                    .map_or(json!(null), |properties| {
                        EnumMutationBuilder::build_struct_example_from_properties(
                            properties, registry,
                        )
                    })
            }
            _ => json!(null),
        }
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(paths: &[MutationPathInternal]) -> HashMap<String, MutationPath> {
        let mut result = HashMap::new();

        for path in paths {
            // Generate description using the context
            let description = path.path_kind.description();

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
                        .filter_map(Self::extract_type_ref_from_field)
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
