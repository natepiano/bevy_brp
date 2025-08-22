//! This is the main response structure use to convey type information
//! to the caller
use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tracing::warn;

use super::format_knowledge::{BRP_FORMAT_KNOWLEDGE, BrpFormatKnowledge};
use super::response_types::{
    BrpSupportedOperation, BrpTypeName, EnumVariantInfo, EnumVariantKind, MutationContext,
    MutationPath, MutationPathInfo, OptionField, ReflectTrait, SchemaField, SchemaInfo, TypeKind,
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

    // Private helper methods (alphabetically ordered)

    /// Build mutation paths for an array field within a struct
    fn build_array_field_mutation_paths(
        field_name: &str,
        array_type: &BrpTypeName,
        array_schema: &Value,
        parent_type: &BrpTypeName,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        // Get array element type
        let element_type = array_schema
            .get("items")
            .and_then(|v| v.get_field(SchemaField::Type))
            .and_then(|t| t.get_field(SchemaField::Ref))
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("#/$defs/"))
            .map(BrpTypeName::from)
            .unwrap_or_else(|| BrpTypeName::from("unknown"));

        // Build example array
        let example_element = BRP_FORMAT_KNOWLEDGE
            .get(&element_type)
            .map_or(json!(null), |k| k.example_value.clone());

        // Determine array size from type path (e.g., "[Vec3; 3]" -> 3)
        let array_size = array_type
            .as_str()
            .rsplit(';')
            .next()
            .and_then(|s| s.trim_end_matches(']').trim().parse::<usize>().ok())
            .unwrap_or(3);

        // Add path for the entire array field
        let array_example: Vec<Value> = (0..array_size).map(|_| example_element.clone()).collect();
        paths.push(MutationPath {
            path:          format!(".{field_name}"),
            example:       json!(array_example),
            enum_variants: None,
            type_name:     array_type.clone(),
            context:       MutationContext::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        });

        // Add paths for array elements (usually just first few as examples)
        for index in 0..array_size.min(3) {
            paths.push(MutationPath {
                path:          format!(".{field_name}[{index}]"),
                example:       example_element.clone(),
                enum_variants: None,
                type_name:     element_type.clone(),
                context:       MutationContext::ArrayElement {
                    index,
                    parent_type: array_type.clone(),
                },
            });
        }

        paths
    }

    /// Build mutation paths for array types
    fn build_array_mutation_paths(
        type_schema: &Value,
        _registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        let parent_type = type_schema
            .get_field(SchemaField::TypePath)
            .and_then(Value::as_str)
            .map(BrpTypeName::from)
            .unwrap_or_else(|| BrpTypeName::from("unknown"));

        // Get array item type
        let Some(items) = type_schema
            .get("items")
            .and_then(|v| v.get_field(SchemaField::Type))
        else {
            return paths;
        };

        let element_type = items
            .get_field(SchemaField::Ref)
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("#/$defs/"))
            .map(BrpTypeName::from)
            .unwrap_or_else(|| BrpTypeName::from("unknown"));

        // Build example array
        let example_element = BRP_FORMAT_KNOWLEDGE
            .get(&element_type)
            .map_or(json!(null), |k| k.example_value.clone());

        // Determine array size from type path (e.g., "[Vec3; 3]" -> 3)
        let array_size = parent_type
            .as_str()
            .rsplit(';')
            .next()
            .and_then(|s| s.trim_end_matches(']').trim().parse::<usize>().ok())
            .unwrap_or(3);

        // Add root mutation path for the entire array
        let array_example: Vec<Value> = (0..array_size).map(|_| example_element.clone()).collect();
        paths.push(MutationPath {
            path:          String::new(),
            example:       json!(array_example),
            enum_variants: None,
            type_name:     parent_type.clone(),
            context:       MutationContext::RootValue {
                type_name: parent_type.clone(),
            },
        });

        // Add paths for array elements (usually just first few as examples)
        for index in 0..array_size.min(3) {
            paths.push(MutationPath {
                path:          format!("[{index}]"),
                example:       example_element.clone(),
                enum_variants: None,
                type_name:     element_type.clone(),
                context:       MutationContext::ArrayElement {
                    index,
                    parent_type: parent_type.clone(),
                },
            });
        }

        paths
    }

    /// Build example value for an enum type
    fn build_enum_example(schema: &Value) -> Value {
        if let Some(one_of) = schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)
            && let Some(first_variant) = one_of.first()
        {
            let Some(variant_name) = Self::get_variant_identifier(first_variant) else {
                return json!(null);
            };

            // Check variant type to build appropriate example
            if first_variant.is_string() {
                // Simple unit variant - just return the string
                return json!(variant_name);
            } else if let Some(prefix_items) = first_variant
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
            } else {
                // Unit variant (object format)
                return json!(variant_name);
            }
        }
        json!(null)
    }

    /// Build mutation paths for enum types
    fn build_enum_mutation_paths(type_schema: &Value) -> Vec<MutationPath> {
        let type_name = type_schema
            .get_field(SchemaField::TypePath)
            .and_then(Value::as_str)
            .map(BrpTypeName::from)
            .unwrap_or_else(|| BrpTypeName::from("unknown"));

        // Extract enum info using our existing function
        let Some(enum_info) = Self::extract_enum_info(type_schema) else {
            return Vec::new();
        };

        // Get variant names for the root mutation
        let variants: Vec<String> = enum_info
            .iter()
            .map(|info| info.variant_name.clone())
            .collect();

        let mut paths = Vec::new();

        // Always add root path for replacing entire enum
        if let Some(first_variant) = variants.first() {
            paths.push(MutationPath {
                path:          String::new(),
                example:       json!(first_variant),
                enum_variants: Some(variants),
                type_name:     type_name.clone(),
                context:       MutationContext::RootValue {
                    type_name: type_name.clone(),
                },
            });
        }

        // Add paths based on variant kinds (simplified for now)
        // TODO: Add variant-specific field paths when needed

        paths
    }

    /// Build mutation paths for a single field
    fn build_field_mutation_paths(
        field_name: &str,
        field_info: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        parent_type: &BrpTypeName,
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
                context:       MutationContext::StructField {
                    field_name:  field_name.to_string(),
                    parent_type: parent_type.clone(),
                },
            });
            return paths;
        };

        // Check if this is a wrapper type (Option, Handle) first
        let wrapper_info = WrapperType::detect(ft.as_str());

        // For wrapper types, check the inner type for hardcoded knowledge
        let type_to_check = wrapper_info.map_or(ft.as_str(), |(_, inner)| inner);

        // Check for hardcoded math types (Vec3, Quat, etc.)
        if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&BrpTypeName::from(type_to_check)) {
            // Get enum variants if this is an enum
            let enum_variants = if wrapper_info.is_none() {
                registry.get(&ft).and_then(|schema| {
                    if Self::get_type_kind(schema, &ft) == TypeKind::Enum {
                        Self::extract_enum_variants(schema)
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            paths.extend(Self::build_hardcoded_paths(
                field_name,
                &ft,
                hardcoded,
                wrapper_info,
                enum_variants,
                parent_type,
            ));
            return paths;
        }

        // Look up the field type in the registry to determine its kind
        let field_type_schema = registry.get(&ft);
        let field_type_kind = field_type_schema
            .map(|schema| Self::get_type_kind(schema, &ft))
            .unwrap_or(TypeKind::Value);

        // Handle different type kinds
        match field_type_kind {
            TypeKind::Array => {
                // Array field - generate element paths
                if let Some(schema) = field_type_schema {
                    paths.extend(Self::build_array_field_mutation_paths(
                        field_name,
                        &ft,
                        schema,
                        parent_type,
                    ));
                }
            }
            TypeKind::Enum => {
                // Enum field - include enum variants
                let enum_variants = field_type_schema.and_then(Self::extract_enum_variants);
                let example = field_type_schema
                    .map(Self::build_enum_example)
                    .unwrap_or(json!(null));

                paths.push(Self::build_standard_path(
                    field_name,
                    &ft,
                    example,
                    enum_variants,
                    wrapper_info,
                    parent_type,
                ));
            }
            TypeKind::Tuple | TypeKind::TupleStruct => {
                // Tuple field - generate element paths
                if let Some(schema) = field_type_schema {
                    paths.extend(Self::build_tuple_field_mutation_paths(
                        field_name,
                        &ft,
                        schema,
                        parent_type,
                    ));
                }
            }
            _ => {
                // All other types (Struct, Value, List, Map, etc.)
                paths.push(Self::build_standard_path(
                    field_name,
                    &ft,
                    json!(null),
                    None,
                    wrapper_info,
                    parent_type,
                ));
            }
        }

        paths
    }

    /// Build paths for types with hardcoded knowledge
    fn build_hardcoded_paths(
        field_name: &str,
        field_type: &BrpTypeName,
        hardcoded: &BrpFormatKnowledge,
        wrapper_info: Option<(WrapperType, &str)>,
        enum_variants: Option<Vec<String>>,
        parent_type: &BrpTypeName,
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
            context: MutationContext::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        });

        // Add component paths if available (e.g., .x, .y, .z for Vec3)
        if let Some(component_paths) = &hardcoded.subfield_paths {
            for (component, example_value) in component_paths {
                paths.push(MutationPath {
                    path:          format!(".{field_name}.{component}"),
                    example:       example_value.clone(),
                    enum_variants: None,
                    type_name:     BrpTypeName::from("f32"),
                    context:       MutationContext::NestedPath {
                        components: vec![field_name.to_string(), component.to_string()],
                        final_type: BrpTypeName::from("f32"),
                    },
                });
            }
        }

        paths
    }

    /// Build mutation paths for a type
    fn build_mutation_paths(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<MutationPath> {
        let type_kind = Self::get_type_kind(type_schema, &BrpTypeName::from("unknown"));

        match type_kind {
            TypeKind::Enum => Self::build_enum_mutation_paths(type_schema),
            TypeKind::Struct => Self::build_struct_mutation_paths(type_schema, registry),
            TypeKind::Tuple | TypeKind::TupleStruct => {
                Self::build_tuple_mutation_paths(type_schema, registry)
            }
            TypeKind::Array => Self::build_array_mutation_paths(type_schema, registry),
            _ => Vec::new(),
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

    /// Build standard mutation path
    fn build_standard_path(
        field_name: &str,
        field_type: &BrpTypeName,
        example_value: Value,
        enum_variants: Option<Vec<String>>,
        wrapper_info: Option<(WrapperType, &str)>,
        parent_type: &BrpTypeName,
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
            context: MutationContext::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        }
    }

    /// Build mutation paths for struct types
    fn build_struct_mutation_paths(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        let parent_type = type_schema
            .get_field(SchemaField::TypePath)
            .and_then(Value::as_str)
            .map(BrpTypeName::from)
            .unwrap_or_else(|| BrpTypeName::from("unknown"));

        let Some(properties) = type_schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            return paths;
        };

        for (field_name, field_info) in properties {
            let field_paths =
                Self::build_field_mutation_paths(field_name, field_info, registry, &parent_type);
            paths.extend(field_paths);
        }

        paths
    }

    /// Build example value for a tuple
    fn build_tuple_example(
        prefix_items: &[Value],
        _registry: &HashMap<BrpTypeName, Value>,
    ) -> Value {
        let elements: Vec<Value> = prefix_items
            .iter()
            .map(|item| {
                Self::extract_field_type(item)
                    .and_then(|t| BRP_FORMAT_KNOWLEDGE.get(&t))
                    .map_or(json!(null), |k| k.example_value.clone())
            })
            .collect();

        json!(elements)
    }

    /// Build mutation paths for a tuple field within a struct
    fn build_tuple_field_mutation_paths(
        field_name: &str,
        tuple_type: &BrpTypeName,
        tuple_schema: &Value,
        parent_type: &BrpTypeName,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        // Get prefix items (tuple elements)
        let prefix_items = tuple_schema
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array);

        // Build example tuple value
        let example = if let Some(items) = prefix_items {
            let elements: Vec<Value> = items
                .iter()
                .map(|item| {
                    Self::extract_field_type(item)
                        .and_then(|t| BRP_FORMAT_KNOWLEDGE.get(&t))
                        .map_or(json!(null), |k| k.example_value.clone())
                })
                .collect();
            json!(elements)
        } else {
            json!([])
        };

        // Add path for the entire tuple field
        paths.push(MutationPath {
            path: format!(".{field_name}"),
            example,
            enum_variants: None,
            type_name: tuple_type.clone(),
            context: MutationContext::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        });

        // Add paths for each tuple element
        if let Some(items) = prefix_items {
            for (index, element_info) in items.iter().enumerate() {
                if let Some(element_type) = Self::extract_field_type(element_info) {
                    let elem_example = BRP_FORMAT_KNOWLEDGE
                        .get(&element_type)
                        .map_or(json!(null), |k| k.example_value.clone());

                    paths.push(MutationPath {
                        path:          format!(".{field_name}.{index}"),
                        example:       elem_example,
                        enum_variants: None,
                        type_name:     element_type,
                        context:       MutationContext::TupleElement {
                            index,
                            parent_type: tuple_type.clone(),
                        },
                    });
                }
            }
        }

        paths
    }

    /// Build mutation paths for tuple and tuple struct types
    fn build_tuple_mutation_paths(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<MutationPath> {
        let mut paths = Vec::new();

        let parent_type = type_schema
            .get_field(SchemaField::TypePath)
            .and_then(Value::as_str)
            .map(BrpTypeName::from)
            .unwrap_or_else(|| BrpTypeName::from("unknown"));

        // Get prefix items (tuple elements)
        let Some(prefix_items) = type_schema
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array)
        else {
            return paths;
        };

        // Add root mutation path for the entire tuple
        paths.push(MutationPath {
            path:          String::new(),
            example:       Self::build_tuple_example(prefix_items, registry),
            enum_variants: None,
            type_name:     parent_type.clone(),
            context:       MutationContext::RootValue {
                type_name: parent_type.clone(),
            },
        });

        // Add paths for each tuple element
        for (index, element_info) in prefix_items.iter().enumerate() {
            if let Some(element_type) = Self::extract_field_type(element_info) {
                let example = BRP_FORMAT_KNOWLEDGE
                    .get(&element_type)
                    .map_or(json!(null), |k| k.example_value.clone());

                paths.push(MutationPath {
                    path: format!(".{index}"),
                    example,
                    enum_variants: None,
                    type_name: element_type,
                    context: MutationContext::TupleElement {
                        index,
                        parent_type: parent_type.clone(),
                    },
                });
            }
        }

        paths
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(paths: &[MutationPath]) -> HashMap<String, MutationPathInfo> {
        let mut result = HashMap::new();

        for path in paths {
            // Generate description using the context
            let description = path.context.description();

            // Check if this is an Option type
            let is_option = path.type_name.as_str().starts_with("core::option::Option<");

            // Create MutationPathInfo from MutationPath
            let path_info = MutationPathInfo::from_mutation_path(path, description, is_option);

            // Use "." for root mutations instead of empty string
            let key = if path.path.is_empty() {
                ".".to_string()
            } else {
                path.path.clone()
            };

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
                    EnumVariantKind::Unit
                } else {
                    // Object without explicit kind = unit variant with metadata
                    EnumVariantKind::Unit
                }
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

    /// Extract enum variants from a type schema
    fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
        type_schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)
            .map(|one_of| {
                one_of
                    .iter()
                    .filter_map(|v| Self::get_variant_identifier(v).map(String::from))
                    .collect()
            })
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

    /// Get mutation paths for a type as a `HashMap`
    fn get_mutation_paths(
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, MutationPathInfo> {
        let mutation_paths_vec = Self::build_mutation_paths(type_schema, registry);
        Self::convert_mutation_paths(&mutation_paths_vec)
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
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
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

    /// Extract variant identifier from either string or object representation
    /// This returns the discriminant/name that identifies which variant this is,
    /// regardless of whether the variant contains data
    fn get_variant_identifier(v: &Value) -> Option<&str> {
        if let Some(s) = v.as_str() {
            // Simple string variant (unit variant with no data)
            Some(s)
        } else {
            // Complex variant (may have tuple or struct data) - get its identifier
            v.get_field(SchemaField::ShortPath).and_then(Value::as_str)
        }
    }
}