//! Mutation path builders for different type kinds
//!
//! This module implements the TYPE-SYSTEM-002 refactor: Replace conditional chains
//! in mutation path building with type-directed dispatch using the `MutationPathBuilder` trait.
//!
//! The key insight is that different `TypeKind` variants need different logic for building
//! mutation paths, but this should be cleanly separated from the field-level logic.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::format_knowledge::BRP_FORMAT_KNOWLEDGE;
use super::response_types::{
    BrpTypeName, MutationPathInternal, MutationPathKind, SchemaField, TypeKind,
};
use super::wrapper_types::WrapperType;
use crate::string_traits::JsonFieldAccess;

/// Context for building mutation paths - handles root vs field scenarios
/// necessary because Struct, specifically, allows us to recurse down a level
/// for complex types that have Struct fields
#[derive(Debug, Clone)]
pub enum RootOrField {
    /// Building paths for a root type (used in root mutations)
    Root { type_name: BrpTypeName },
    /// Building paths for a field within a parent type
    Field {
        field_name:  String,
        field_type:  BrpTypeName,
        parent_type: BrpTypeName,
    },
}

impl RootOrField {
    /// Create a field context
    pub fn field(field_name: &str, field_type: &BrpTypeName, parent_type: &BrpTypeName) -> Self {
        Self::Field {
            field_name:  field_name.to_string(),
            field_type:  field_type.clone(),
            parent_type: parent_type.clone(),
        }
    }

    /// Create a root context
    pub fn root(type_name: &BrpTypeName) -> Self {
        Self::Root {
            type_name: type_name.clone(),
        }
    }

    /// Get the type being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        match self {
            Self::Root { type_name } => type_name,
            Self::Field { field_type, .. } => field_type,
        }
    }
}

/// Context for mutation path building operations
///
/// This struct provides all the necessary context for building mutation paths,
/// including access to the registry, wrapper type information, and enum variants.
#[derive(Debug)]
pub struct MutationPathContext<'a> {
    /// The building context (root or field)
    pub location:     RootOrField,
    /// Reference to the type registry
    pub registry:     &'a HashMap<BrpTypeName, Value>,
    /// Wrapper type information if applicable (Option, Handle, etc.)
    pub wrapper_info: Option<(WrapperType, &'a str)>,
    /// The schema for the current type
    pub schema:       Option<&'a Value>,
}

impl<'a> MutationPathContext<'a> {
    /// Create a new mutation path context
    pub const fn new(
        location: RootOrField,
        registry: &'a HashMap<BrpTypeName, Value>,
        wrapper_info: Option<(WrapperType, &'a str)>,
        schema: Option<&'a Value>,
    ) -> Self {
        Self {
            location,
            registry,
            wrapper_info,
            schema,
        }
    }

    /// Get the type name being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        self.location.type_name()
    }

    /// Look up a type in the registry
    pub fn get_type_schema(&self, type_name: &BrpTypeName) -> Option<&Value> {
        self.registry.get(type_name)
    }

    /// Create a new context for a field within the current type
    pub fn create_field_context(
        &self,
        field_name: &str,
        field_type: &BrpTypeName,
        wrapper_info: Option<(WrapperType, &'a str)>,
        schema: Option<&'a Value>,
    ) -> Self {
        let parent_type = self.type_name();
        Self::new(
            RootOrField::field(field_name, field_type, parent_type),
            self.registry,
            wrapper_info,
            schema,
        )
    }

    /// Wrap an example value based on the wrapper type context
    /// For Option types: creates {some: value, none: null}
    /// For other wrappers: creates appropriate mutation format
    /// For non-wrappers: returns the value as-is
    pub fn wrap_example(&self, inner_value: Value) -> Value {
        match self.wrapper_info {
            Some((wrapper, _)) => wrapper.mutation_examples(inner_value),
            None => inner_value,
        }
    }
}

/// Trait for building mutation paths for different type kinds
///
/// This trait provides type-directed dispatch for mutation path building,
/// replacing the large conditional match statement with clean separation of concerns.
/// Each type kind gets its own implementation that handles the specific logic needed.
pub trait MutationPathBuilder {
    /// Build mutation paths for this type kind
    ///
    /// This method takes a `MutationPathContext` which provides all necessary information
    /// including the registry, wrapper info, and enum variants.
    ///
    /// Returns a vector of `MutationPathInternal` representing all possible mutation paths.
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal>;
}

/// Implementation of `MutationPathBuilder` for `TypeKind`
///
/// This provides type-directed dispatch - each `TypeKind` variant gets routed
/// to the appropriate specialized builder for handling its specific logic.
impl MutationPathBuilder for TypeKind {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        match self {
            Self::Array => ArrayMutationBuilder.build_paths(ctx),
            Self::Enum => EnumMutationBuilder.build_paths(ctx),
            Self::Struct => StructMutationBuilder.build_paths(ctx),
            Self::Tuple | Self::TupleStruct => TupleMutationBuilder.build_paths(ctx),
            Self::List | Self::Map | Self::Option | Self::Value => {
                // For these types, build a simple standard path
                DefaultMutationBuilder.build_paths(ctx)
            }
        }
    }
}

// Specific builders for each type kind

/// Builder for Array types
///
/// Handles both fixed-size arrays like `[Vec3; 3]` and dynamic arrays.
/// Creates mutation paths for both the entire array and individual elements.
pub struct ArrayMutationBuilder;

impl MutationPathBuilder for ArrayMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.schema else {
            return paths;
        };

        // Get array element type from schema
        let element_type = schema
            .get("items")
            .and_then(|v| v.get_field(SchemaField::Type))
            .and_then(|t| t.get_field(SchemaField::Ref))
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("#/$defs/"))
            .map_or_else(BrpTypeName::unknown, BrpTypeName::from);

        // Build example element from hardcoded knowledge
        let example_element = BRP_FORMAT_KNOWLEDGE
            .get(&element_type)
            .map_or(json!(null), |k| k.example_value.clone());

        // Determine array size from type name (e.g., "[Vec3; 3]" -> 3)
        let array_size = ctx
            .type_name()
            .as_str()
            .rsplit(';')
            .next()
            .and_then(|s| s.trim_end_matches(']').trim().parse::<usize>().ok())
            .unwrap_or(3);

        // Build example array
        let array_example: Vec<Value> = (0..array_size).map(|_| example_element.clone()).collect();

        match &ctx.location {
            RootOrField::Root { type_name } => {
                // Add root mutation path for the entire array
                paths.push(MutationPathInternal {
                    path:          String::new(),
                    example:       json!(array_example),
                    enum_variants: None,
                    type_name:     type_name.clone(),
                    context:       MutationPathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                });

                // Add paths for all array elements
                for index in 0..array_size {
                    paths.push(MutationPathInternal {
                        path:          format!("[{index}]"),
                        example:       example_element.clone(),
                        enum_variants: None,
                        type_name:     element_type.clone(),
                        context:       MutationPathKind::ArrayElement {
                            index,
                            parent_type: type_name.clone(),
                        },
                    });
                }
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                // Add path for the entire array field
                paths.push(MutationPathInternal {
                    path:          format!(".{field_name}"),
                    example:       json!(array_example),
                    enum_variants: None,
                    type_name:     field_type.clone(),
                    context:       MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                });

                // Add paths for all array elements
                for index in 0..array_size {
                    paths.push(MutationPathInternal {
                        path:          format!(".{field_name}[{index}]"),
                        example:       example_element.clone(),
                        enum_variants: None,
                        type_name:     element_type.clone(),
                        context:       MutationPathKind::ArrayElement {
                            index,
                            parent_type: field_type.clone(),
                        },
                    });
                }
            }
        }

        paths
    }
}

/// Builder for Enum types
///
/// Handles enum mutation paths by extracting variant information and building
/// appropriate examples for each enum variant type (Unit, Tuple, Struct).
pub struct EnumMutationBuilder;

impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.schema else {
            return paths;
        };

        // Extract enum variants from schema
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(schema);

        match &ctx.location {
            RootOrField::Root { type_name } => {
                // For root enum mutations, add a root path with all variants
                if let Some(ref variants) = enum_variants
                    && let Some(first_variant) = variants.first()
                {
                    paths.push(MutationPathInternal {
                        path:          String::new(),
                        example:       json!(first_variant),
                        enum_variants: Some(variants.clone()),
                        type_name:     type_name.clone(),
                        context:       MutationPathKind::RootValue {
                            type_name: type_name.clone(),
                        },
                    });
                }
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                // For field enum mutations, handle wrapper types appropriately
                let final_example = ctx.wrap_example(enum_example);

                paths.push(MutationPathInternal {
                    path: format!(".{field_name}"),
                    example: final_example,
                    enum_variants,
                    type_name: field_type.clone(),
                    context: MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                });
            }
        }

        paths
    }
}

impl EnumMutationBuilder {
    /// Extract enum variants from type schema
    pub fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
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

    /// Build example value for an enum type
    pub fn build_enum_example(schema: &Value) -> Value {
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
                        BRP_FORMAT_KNOWLEDGE
                            .get(&BrpTypeName::from(inner_type))
                            .map_or(json!(null), |k| k.example_value.clone())
                    };

                    return json!({
                        variant_name: [inner_value]
                    });
                }
            } else if first_variant.get_field(SchemaField::Properties).is_some() {
                // Struct variant
                return json!({
                    variant_name: {}
                });
            }

            // Fallback for unit variants
            json!(variant_name)
        } else {
            json!(null)
        }
    }

    /// Get variant identifier from schema variant definition
    fn get_variant_identifier(variant: &Value) -> Option<&str> {
        variant.as_str().or_else(|| {
            variant.as_object().and_then(|obj| {
                // Object with single key (the variant name)
                obj.keys().next().map(String::as_str)
            })
        })
    }
}

/// Builder for Struct types
///
/// Handles the most complex case - struct mutations with one-level recursion.
/// For field contexts, adds both the struct field itself and nested field paths.
pub struct StructMutationBuilder;

impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.schema else {
            return paths;
        };

        match &ctx.location {
            RootOrField::Root { .. } => {
                // For root struct mutations, build paths for all properties
                paths.extend(Self::build_property_paths(ctx));
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                // First, add the struct field itself with null example
                let final_example = ctx.wrap_example(json!(null));

                paths.push(MutationPathInternal {
                    path:          format!(".{field_name}"),
                    example:       final_example,
                    enum_variants: None,
                    type_name:     field_type.clone(),
                    context:       MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                });

                // Then recursively expand nested fields (depth = 1 only)
                // Create a context for nested field building
                let nested_context = MutationPathContext::new(
                    RootOrField::root(field_type),
                    ctx.registry,
                    None, // No wrapper for nested fields
                    Some(schema),
                );

                let nested_paths = Self::build_property_paths(&nested_context);
                for nested_path in nested_paths {
                    // Convert to nested path by prepending the field name
                    let full_path = if nested_path.path.is_empty() {
                        format!(".{field_name}")
                    } else {
                        format!(".{field_name}{}", nested_path.path)
                    };

                    // Create new path with NestedPath context
                    let mut components = vec![field_name.clone()];
                    if let MutationPathKind::StructField {
                        field_name: nested_field,
                        ..
                    } = &nested_path.context
                    {
                        components.push(nested_field.clone());
                    }

                    paths.push(MutationPathInternal {
                        path:          full_path,
                        example:       nested_path.example,
                        enum_variants: nested_path.enum_variants,
                        type_name:     nested_path.type_name.clone(),
                        context:       MutationPathKind::NestedPath {
                            components,
                            final_type: nested_path.type_name,
                        },
                    });
                }
            }
        }

        paths
    }
}

impl StructMutationBuilder {
    /// Build mutation paths for all properties in a struct
    ///
    /// This method handles the property-level iteration and delegates to the
    /// field-level mutation path building logic via the main dispatch system.
    fn build_property_paths(ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.schema else {
            return paths;
        };

        let Some(properties) = schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            return paths;
        };

        // For each property, we need to build field mutation paths
        // This requires calling back into the main mutation path building system
        for (field_name, field_info) in properties {
            // Extract field type from field info
            let field_type = SchemaField::extract_field_type(field_info);

            let Some(ft) = field_type else {
                // No type info, add null mutation path
                paths.push(MutationPathInternal {
                    path:          format!(".{field_name}"),
                    example:       json!(null),
                    enum_variants: None,
                    type_name:     BrpTypeName::unknown(),
                    context:       match &ctx.location {
                        RootOrField::Root { type_name } => MutationPathKind::StructField {
                            field_name:  field_name.clone(),
                            parent_type: type_name.clone(),
                        },
                        RootOrField::Field {
                            field_type: parent_type,
                            ..
                        } => MutationPathKind::StructField {
                            field_name:  field_name.clone(),
                            parent_type: parent_type.clone(),
                        },
                    },
                });
                continue;
            };

            // Check if this is a wrapper type (Option, Handle) first
            let wrapper_info = WrapperType::detect(ft.as_str());

            // For wrapper types, check the inner type for hardcoded knowledge
            let type_to_check = wrapper_info.map_or(ft.as_str(), |(_, inner)| inner);

            // Check for hardcoded math types (Vec3, Quat, etc.) first
            if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&BrpTypeName::from(type_to_check)) {
                // Get enum variants if this is an enum
                let enum_variants = if wrapper_info.is_none() {
                    ctx.get_type_schema(&ft).and_then(|schema| {
                        if TypeKind::from_schema(schema, &ft) == TypeKind::Enum {
                            EnumMutationBuilder::extract_enum_variants(schema)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                let parent_type = ctx.type_name();
                paths.extend(Self::build_hardcoded_paths(
                    field_name,
                    &ft,
                    hardcoded,
                    wrapper_info,
                    enum_variants,
                    parent_type,
                ));
                continue;
            }

            // Look up the field type in the registry to determine its kind
            let field_type_schema = ctx.get_type_schema(&ft);
            let field_type_kind = field_type_schema
                .map_or(TypeKind::Value, |schema| TypeKind::from_schema(schema, &ft));

            // Create a field context for this property
            let field_ctx =
                ctx.create_field_context(field_name, &ft, wrapper_info, field_type_schema);

            // Dispatch to the appropriate builder based on field type kind
            let field_paths = field_type_kind.build_paths(&field_ctx);
            paths.extend(field_paths);
        }

        paths
    }

    /// Build paths for types with hardcoded knowledge (Vec3, Quat, etc.)
    pub fn build_hardcoded_paths(
        field_name: &str,
        field_type: &BrpTypeName,
        hardcoded: &super::format_knowledge::BrpFormatKnowledge,
        wrapper_info: Option<(WrapperType, &str)>,
        enum_variants: Option<Vec<String>>,
        parent_type: &BrpTypeName,
    ) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        // Build main path with appropriate example format
        let final_example = wrapper_info.map_or_else(
            || hardcoded.example_value.clone(),
            |(wrapper, _)| wrapper.mutation_examples(hardcoded.example_value.clone()),
        );

        paths.push(MutationPathInternal {
            path: format!(".{field_name}"),
            example: final_example,
            enum_variants,
            type_name: field_type.clone(),
            context: MutationPathKind::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        });

        // Add component paths if available (e.g., .x, .y, .z for Vec3)
        if let Some(subfield_paths) = &hardcoded.subfield_paths {
            for (component_name, component_example) in subfield_paths {
                let component_example = wrapper_info.map_or_else(
                    || component_example.clone(),
                    |(wrapper, _)| wrapper.mutation_examples(component_example.clone()),
                );

                paths.push(MutationPathInternal {
                    path:          format!(".{field_name}.{component_name}"),
                    example:       component_example,
                    enum_variants: None,
                    type_name:     BrpTypeName::from("f32"), // Components are always f32
                    context:       MutationPathKind::NestedPath {
                        components: vec![field_name.to_string(), component_name.to_string()],
                        final_type: BrpTypeName::from("f32"),
                    },
                });
            }
        }

        paths
    }
}

/// Builder for Tuple and `TupleStruct` types
///
/// Handles tuple mutations by extracting prefix items (tuple elements) and building
/// paths for both the entire tuple and individual elements by index.
pub struct TupleMutationBuilder;

impl TupleMutationBuilder {
    /// Build example value for a tuple type
    pub fn build_tuple_example(
        prefix_items: &Value,
        _registry: &HashMap<BrpTypeName, Value>,
    ) -> Value {
        prefix_items.as_array().map_or_else(
            || json!([]),
            |items| {
                let elements: Vec<Value> = items
                    .iter()
                    .map(|item| {
                        SchemaField::extract_field_type(item)
                            .and_then(|t| BRP_FORMAT_KNOWLEDGE.get(&t))
                            .map_or(json!(null), |k| k.example_value.clone())
                    })
                    .collect();
                json!(elements)
            },
        )
    }
}

impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.schema else {
            return paths;
        };

        // Get prefix items (tuple elements) from schema
        let prefix_items = schema
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array);

        // Build example tuple value using the extracted method
        let example = Self::build_tuple_example(
            schema
                .get_field(SchemaField::PrefixItems)
                .unwrap_or(&json!([])),
            ctx.registry,
        );

        // Add the main tuple path first (example is consumed here)
        let main_path = match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path: String::new(),
                example,
                enum_variants: None,
                type_name: type_name.clone(),
                context: MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path: format!(".{field_name}"),
                example,
                enum_variants: None,
                type_name: field_type.clone(),
                context: MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
            },
        };
        paths.push(main_path);

        // Add element paths
        match &ctx.location {
            RootOrField::Root { type_name } => {
                // Add paths for each tuple element
                if let Some(items) = prefix_items {
                    for (index, element_info) in items.iter().enumerate() {
                        if let Some(element_type) = SchemaField::extract_field_type(element_info) {
                            let elem_example = BRP_FORMAT_KNOWLEDGE
                                .get(&element_type)
                                .map_or(json!(null), |k| k.example_value.clone());

                            paths.push(MutationPathInternal {
                                path:          format!(".{index}"),
                                example:       elem_example,
                                enum_variants: None,
                                type_name:     element_type,
                                context:       MutationPathKind::TupleElement {
                                    index,
                                    parent_type: type_name.clone(),
                                },
                            });
                        }
                    }
                }
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type: _,
            } => {
                // Add paths for each tuple element
                if let Some(items) = prefix_items {
                    for (index, element_info) in items.iter().enumerate() {
                        if let Some(element_type) = SchemaField::extract_field_type(element_info) {
                            let elem_example = BRP_FORMAT_KNOWLEDGE
                                .get(&element_type)
                                .map_or(json!(null), |k| k.example_value.clone());

                            paths.push(MutationPathInternal {
                                path:          format!(".{field_name}.{index}"),
                                example:       elem_example,
                                enum_variants: None,
                                type_name:     element_type,
                                context:       MutationPathKind::TupleElement {
                                    index,
                                    parent_type: field_type.clone(),
                                },
                            });
                        }
                    }
                }
            }
        }

        paths
    }
}

/// Default builder for simple types (Value, List, Map, Option)
///
/// Handles simple types that don't need complex logic - just creates a standard mutation path
pub struct DefaultMutationBuilder;

impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path:          String::new(),
                    example:       json!(null),
                    enum_variants: None,
                    type_name:     type_name.clone(),
                    context:       MutationPathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                });
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                paths.push(MutationPathInternal {
                    path:          format!(".{field_name}"),
                    example:       json!(null),
                    enum_variants: None,
                    type_name:     field_type.clone(),
                    context:       MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                });
            }
        }

        paths
    }
}
