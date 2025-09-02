//! Mutation path builders for different type kinds
//!
//! This module implements the TYPE-SYSTEM-002 refactor: Replace conditional chains
//! in mutation path building with type-directed dispatch using the `MutationPathBuilder` trait.
//!
//! The key insight is that different `TypeKind` variants need different logic for building
//! mutation paths, but this should be cleanly separated from the field-level logic.

use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::warn;

/// Result of attempting to build mutation paths using hardcoded knowledge
#[derive(Debug)]
enum HardcodedPathsResult {
    /// Hardcoded knowledge handled this type with these paths
    Handled(Vec<MutationPathInternal>),
    /// Hardcoded knowledge says this type cannot be mutated with explanatory reason
    NotMutatable(String),
    /// No hardcoded knowledge found, should try fallback approach
    Fallback,
}

use super::constants::SCHEMA_REF_PREFIX;
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeGuidance, MutationKnowledge};
use super::response_types::{
    BrpTypeName, EnumVariantInfo, MutationPathInternal, MutationPathKind, SchemaField, TypeKind,
};
use super::type_info::TypeInfo;
use super::wrapper_types::WrapperType;
use crate::brp_tools::brp_type_schema::mutation_knowledge::KnowledgeKey;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

/// Extract type name from a type field using `SchemaField::Ref`
///
/// Helper for extracting type references from schema fields
fn extract_type_ref_with_schema_field(type_value: &Value) -> Option<BrpTypeName> {
    type_value
        .get_field(SchemaField::Ref)
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix(SCHEMA_REF_PREFIX))
        .map(BrpTypeName::from)
}

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
    registry:         &'a HashMap<BrpTypeName, Value>,
    /// Wrapper type information if applicable (Option, Handle, etc.)
    pub wrapper_info: Option<(WrapperType, BrpTypeName)>,
}

impl<'a> MutationPathContext<'a> {
    /// Create a new mutation path context
    pub const fn new(
        location: RootOrField,
        registry: &'a HashMap<BrpTypeName, Value>,
        wrapper_info: Option<(WrapperType, BrpTypeName)>,
    ) -> Self {
        Self {
            location,
            registry,
            wrapper_info,
        }
    }

    /// Get the type name being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        self.location.type_name()
    }

    /// Require the schema to be present, logging a warning if missing
    /// Looks up the schema from the registry based on the current type
    pub fn require_schema(&self) -> Option<&Value> {
        self.registry.get(self.type_name()).or_else(|| {
            warn!(
                type_name = %self.type_name(),
                "Schema missing for type - mutation paths may be incomplete"
            );
            None
        })
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
        wrapper_info: Option<(WrapperType, BrpTypeName)>,
    ) -> Self {
        let parent_type = self.type_name();
        Self::new(
            RootOrField::field(field_name, field_type, parent_type),
            self.registry,
            wrapper_info,
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
    /// Returns a `Result` containing a vector of `MutationPathInternal` representing
    /// all possible mutation paths, or an error if path building failed.
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>>;
}

/// Implementation of `MutationPathBuilder` for `TypeKind`
///
/// This provides type-directed dispatch - each `TypeKind` variant gets routed
/// to the appropriate specialized builder for handling its specific logic.
impl MutationPathBuilder for TypeKind {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
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
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.require_schema() else {
            return Ok(paths);
        };

        // Get array element type from schema
        let element_type = schema
            .get("items")
            .and_then(|v| v.get_field(SchemaField::Type))
            .and_then(extract_type_ref_with_schema_field)
            .unwrap_or_else(BrpTypeName::unknown);

        // Build example element from hardcoded knowledge
        let example_element = BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::exact(&element_type))
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
                    path_kind:     MutationPathKind::RootValue {
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
                        path_kind:     MutationPathKind::ArrayElement {
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
                    path_kind:     MutationPathKind::StructField {
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
                        path_kind:     MutationPathKind::ArrayElement {
                            index,
                            parent_type: field_type.clone(),
                        },
                    });
                }
            }
        }

        Ok(paths)
    }
}

/// Builder for Enum types
///
/// Handles enum mutation paths by extracting variant information and building
/// appropriate examples for each enum variant type (Unit, Tuple, Struct).
pub struct EnumMutationBuilder;

impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.require_schema() else {
            return Ok(paths);
        };

        // Extract enum variants from schema
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(schema, ctx.registry, Some(ctx.type_name()));

        match &ctx.location {
            RootOrField::Root { type_name } => {
                // For root enum mutations, add a root path with all variants
                if let Some(ref variants) = enum_variants
                    && !variants.is_empty()
                {
                    paths.push(MutationPathInternal {
                        path:          String::new(),
                        example:       enum_example,
                        enum_variants: Some(variants.clone()),
                        type_name:     type_name.clone(),
                        path_kind:     MutationPathKind::RootValue {
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
                    path_kind: MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                });
            }
        }

        Ok(paths)
    }
}

impl EnumMutationBuilder {
    /// Extract enum variants from type schema
    pub fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
        use super::response_types::extract_enum_variants as extract_variants_new;

        let variants = extract_variants_new(type_schema, &HashMap::new(), 0);
        if variants.is_empty() {
            None
        } else {
            Some(variants.iter().map(|v| v.name().to_string()).collect())
        }
    }

    /// Build example value for an enum type
    ///
    /// Updated to use type-safe pattern matching instead of conditional chains.
    /// For tuple/newtype variants, builds proper examples based on the inner type
    /// by looking up struct definitions in the registry.
    pub fn build_enum_example(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: Option<&BrpTypeName>,
    ) -> Value {
        if let Some(one_of) = schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)
            && let Some(first_variant) = one_of.first()
        {
            // Use the new type-safe EnumVariantInfo with pattern matching instead of conditional
            // chains
            EnumVariantInfo::from_schema_variant(first_variant, registry, 0).map_or(
                json!(null),
                |variant_info| {
                    match variant_info {
                        EnumVariantInfo::Unit(name) => {
                            // Simple unit variant - just return the string
                            json!(name)
                        }
                        EnumVariantInfo::Tuple(name, types) => {
                            if types.len() == 1 {
                                // Newtype variant - single field tuple
                                let inner_value = Self::build_variant_data_example(
                                    &types[0],
                                    registry,
                                    enum_type,
                                    Some(&name),
                                );

                                // For newtype variants, BRP expects the struct directly, not in an
                                // array
                                json!({
                                    name: inner_value
                                })
                            } else if !types.is_empty() {
                                // Multi-field tuple variant (rare in Bevy)
                                let tuple_values: Vec<Value> = types
                                    .iter()
                                    .map(|t| {
                                        Self::build_variant_data_example(
                                            t,
                                            registry,
                                            enum_type,
                                            Some(&name),
                                        )
                                    })
                                    .collect();

                                json!({
                                    name: tuple_values
                                })
                            } else {
                                // Empty tuple - treat as unit variant
                                json!(name)
                            }
                        }
                        EnumVariantInfo::Struct(name, fields) => {
                            // Struct variant - build example from fields
                            let struct_obj: serde_json::Map<String, Value> = fields
                                .iter()
                                .map(|f| {
                                    (
                                        f.field_name.clone(),
                                        Self::build_variant_data_example(
                                            &f.type_name,
                                            registry,
                                            enum_type,
                                            Some(&name),
                                        ),
                                    )
                                })
                                .collect();

                            json!({
                                name: struct_obj
                            })
                        }
                    }
                },
            )
        } else {
            json!(null)
        }
    }

    /// Build example data for enum variant inner types
    fn build_variant_data_example(
        type_name: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: Option<&BrpTypeName>,
        variant_name: Option<&str>,
    ) -> Value {
        // Check for enum variant-specific knowledge first
        if let Some(enum_type) = enum_type
            && let Some(variant_name) = variant_name
        {
            let variant_key = KnowledgeKey::EnumVariant {
                enum_type:       enum_type.to_string(),
                variant_name:    variant_name.to_string(),
                variant_pattern: format!("{variant_name}({type_name})"),
            };

            if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&variant_key) {
                return knowledge.example_value.clone();
            }
        }

        // Fall back to the existing TypeInfo helper that already handles all the complexity
        TypeInfo::build_example_value_for_type(type_name, registry)
    }

    /// Build example struct from properties
    pub fn build_struct_example_from_properties(
        properties: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Value {
        let Some(props_map) = properties.as_object() else {
            return json!({});
        };

        let mut example = serde_json::Map::new();

        for (field_name, field_schema) in props_map {
            // Use TypeInfo to build example for each field type
            let field_value = SchemaField::extract_field_type(field_schema)
                .map(|field_type| TypeInfo::build_example_value_for_type(&field_type, registry))
                .unwrap_or(json!(null));

            example.insert(field_name.clone(), field_value);
        }

        json!(example)
    }
}

/// Builder for Struct types
///
/// Handles the most complex case - struct mutations with one-level recursion.
/// For field contexts, adds both the struct field itself and nested field paths.
pub struct StructMutationBuilder;

impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        let Some(_schema) = ctx.require_schema() else {
            return Ok(paths);
        };

        match &ctx.location {
            RootOrField::Root { .. } => {
                // For root struct mutations, build paths for all properties
                paths.extend(Self::build_property_paths(ctx)?);
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                // First, add the struct field itself
                paths.push(Self::build_field_mutation_path(
                    field_name,
                    field_type,
                    parent_type,
                    ctx,
                ));

                // Then expand nested fields (depth = 1 only)
                paths.extend(Self::expand_nested_fields(field_name, field_type, ctx)?);
            }
        }

        Ok(paths)
    }
}

impl StructMutationBuilder {
    /// Build a single field mutation path
    fn build_field_mutation_path(
        field_name: &str,
        field_type: &BrpTypeName,
        parent_type: &BrpTypeName,
        ctx: &MutationPathContext<'_>,
    ) -> MutationPathInternal {
        let final_example = ctx.wrap_example(json!(null));

        MutationPathInternal {
            path:          format!(".{field_name}"),
            example:       final_example,
            enum_variants: None,
            type_name:     field_type.clone(),
            path_kind:     MutationPathKind::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        }
    }

    /// Expand nested fields for a struct field (depth = 1 only)
    fn expand_nested_fields(
        field_name: &str,
        field_type: &BrpTypeName,
        ctx: &MutationPathContext<'_>,
    ) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        // Create a context for nested field building
        let nested_context = MutationPathContext::new(
            RootOrField::root(field_type),
            ctx.registry,
            None, // No wrapper for nested fields
        );

        let nested_paths = Self::build_property_paths(&nested_context)?;
        for nested_path in nested_paths {
            // Convert to nested path by prepending the field name
            let full_path = if nested_path.path.is_empty() {
                format!(".{field_name}")
            } else {
                format!(".{field_name}{}", nested_path.path)
            };

            // Create new path with NestedPath context
            let mut components = vec![field_name.to_string()];
            if let MutationPathKind::StructField {
                field_name: nested_field,
                ..
            } = &nested_path.path_kind
            {
                components.push(nested_field.clone());
            }

            paths.push(MutationPathInternal {
                path:          full_path,
                example:       nested_path.example,
                enum_variants: nested_path.enum_variants,
                type_name:     nested_path.type_name.clone(),
                path_kind:     MutationPathKind::NestedPath {
                    components,
                    final_type: nested_path.type_name,
                },
            });
        }

        Ok(paths)
    }

    /// Build mutation paths for all properties in a struct
    ///
    /// This method handles the property-level iteration and delegates to the
    /// field-level mutation path building logic via the main dispatch system.
    fn build_property_paths(ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        let properties = Self::extract_properties(ctx);
        if properties.is_empty() {
            return Ok(paths);
        }

        for (field_name, field_info) in properties {
            // Extract field type to check for skip directive
            let field_type = SchemaField::extract_field_type(field_info);
            let Some(ft) = field_type else {
                paths.push(Self::build_null_mutation_path(ctx, &field_name));
                continue;
            };

            let wrapper_info = WrapperType::detect(ft.as_str());

            // Check hardcoded knowledge first
            match Self::try_build_hardcoded_paths(ctx, &field_name, &ft, wrapper_info.as_ref()) {
                HardcodedPathsResult::NotMutatable(reason) => {
                    paths.push(Self::build_not_mutatable_path(&field_name, &ft, reason));
                }
                HardcodedPathsResult::Handled(field_paths) => {
                    paths.extend(field_paths);
                }
                HardcodedPathsResult::Fallback => {
                    // Fall back to type-based building
                    let field_paths =
                        Self::build_type_based_paths(ctx, &field_name, &ft, wrapper_info)?;
                    paths.extend(field_paths);
                }
            }
        }

        Ok(paths)
    }

    /// Extract properties from the schema
    fn extract_properties<'a>(ctx: &'a MutationPathContext<'_>) -> Vec<(String, &'a Value)> {
        let Some(schema) = ctx.require_schema() else {
            return Vec::new();
        };

        let Some(properties) = schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            warn!(
                type_name = %ctx.type_name(),
                "No properties field found in struct schema - mutation paths may be incomplete"
            );
            return Vec::new();
        };

        properties.iter().map(|(k, v)| (k.clone(), v)).collect()
    }

    /// Build a null mutation path for fields without type information
    fn build_null_mutation_path(
        ctx: &MutationPathContext<'_>,
        field_name: &str,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:          format!(".{field_name}"),
            example:       json!(null),
            enum_variants: None,
            type_name:     BrpTypeName::unknown(),
            path_kind:     match &ctx.location {
                RootOrField::Root { type_name } => MutationPathKind::StructField {
                    field_name:  field_name.to_string(),
                    parent_type: type_name.clone(),
                },
                RootOrField::Field {
                    field_type: parent_type,
                    ..
                } => MutationPathKind::StructField {
                    field_name:  field_name.to_string(),
                    parent_type: parent_type.clone(),
                },
            },
        }
    }

    /// Try to build paths using hardcoded knowledge
    fn try_build_hardcoded_paths(
        ctx: &MutationPathContext<'_>,
        field_name: &str,
        field_type: &BrpTypeName,
        wrapper_info: Option<&(WrapperType, BrpTypeName)>,
    ) -> HardcodedPathsResult {
        // Check for hardcoded knowledge - first try the full type, then inner type for wrappers
        let Some(hardcoded) = BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::exact(field_type))
            .or_else(|| {
                // For wrapper types, check the inner type for hardcoded knowledge
                wrapper_info
                    .and_then(|(_, inner)| BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(inner)))
            })
        else {
            return HardcodedPathsResult::Fallback;
        };

        // Handle different knowledge variants
        match &hardcoded.guidance {
            KnowledgeGuidance::NotMutatable { reason } => {
                return HardcodedPathsResult::NotMutatable(reason.clone());
            }
            KnowledgeGuidance::TreatAsValue { .. } | KnowledgeGuidance::Teach => {
                // TreatAsValue is handled elsewhere in get_mutation_paths - continue for now
            }
        }

        // Get enum variants if this is an enum
        let enum_variants = if wrapper_info.is_none() {
            ctx.get_type_schema(field_type).and_then(|schema| {
                if TypeKind::from_schema(schema, field_type) == TypeKind::Enum {
                    EnumMutationBuilder::extract_enum_variants(schema)
                } else {
                    None
                }
            })
        } else {
            None
        };

        let parent_type = ctx.type_name();
        let paths = Self::build_hardcoded_paths(
            field_name,
            field_type,
            hardcoded,
            wrapper_info.cloned(),
            enum_variants,
            parent_type,
        );
        HardcodedPathsResult::Handled(paths)
    }

    /// Build paths based on the field's type kind
    fn build_type_based_paths(
        ctx: &MutationPathContext<'_>,
        field_name: &str,
        field_type: &BrpTypeName,
        wrapper_info: Option<(WrapperType, BrpTypeName)>,
    ) -> Result<Vec<MutationPathInternal>> {
        // Look up the field type in the registry to determine its kind
        let field_type_schema = ctx.get_type_schema(field_type);
        let field_type_kind = field_type_schema.map_or(TypeKind::Value, |schema| {
            TypeKind::from_schema(schema, field_type)
        });

        // Create a field context for this property
        let field_ctx = ctx.create_field_context(field_name, field_type, wrapper_info);

        // Dispatch to the appropriate builder based on field type kind
        field_type_kind.build_paths(&field_ctx)
    }

    /// Build paths for types with hardcoded knowledge (Vec3, Quat, etc.)
    pub fn build_hardcoded_paths(
        field_name: &str,
        field_type: &BrpTypeName,
        hardcoded: &MutationKnowledge,
        wrapper_info: Option<(WrapperType, BrpTypeName)>,
        enum_variants: Option<Vec<String>>,
        parent_type: &BrpTypeName,
    ) -> Vec<MutationPathInternal> {
        let mut paths = Vec::new();

        // Build main path with appropriate example format
        // When format knowledge exists for wrapper types, use it directly without wrapper
        // transformation This fixes Handle<Image> where format knowledge provides the
        // correct Weak format but wrapper.mutation_examples() wraps it in incorrect complex
        // format
        let final_example = if wrapper_info.is_some()
            && BRP_MUTATION_KNOWLEDGE.contains_key(&KnowledgeKey::exact(field_type))
        {
            // Use format knowledge directly when the full wrapper type (e.g., Handle<Image>)
            // has format knowledge This avoids wrapping the correct format in
            // incorrect wrapper mutation examples
            hardcoded.example_value.clone()
        } else {
            // Use wrapper transformation when hardcoded knowledge comes from inner type
            wrapper_info.as_ref().map_or_else(
                || hardcoded.example_value.clone(),
                |(wrapper, _)| wrapper.mutation_examples(hardcoded.example_value.clone()),
            )
        };

        paths.push(MutationPathInternal {
            path: format!(".{field_name}"),
            example: final_example,
            enum_variants,
            type_name: field_type.clone(),
            path_kind: MutationPathKind::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
        });

        // Add component paths if available (e.g., .x, .y, .z for Vec3)
        if let Some(subfield_paths) = &hardcoded.subfield_paths {
            for (component_name, component_example) in subfield_paths {
                let component_example = wrapper_info.as_ref().map_or_else(
                    || component_example.clone(),
                    |(wrapper, _)| wrapper.mutation_examples(component_example.clone()),
                );

                paths.push(MutationPathInternal {
                    path:          format!(".{field_name}.{component_name}"),
                    example:       component_example,
                    enum_variants: None,
                    type_name:     BrpTypeName::from("f32"), // Components are always f32
                    path_kind:     MutationPathKind::NestedPath {
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
    pub fn build_tuple_example(prefix_items: &Value) -> Value {
        prefix_items.as_array().map_or_else(
            || json!([]),
            |items| {
                let elements: Vec<Value> = items
                    .iter()
                    .map(|item| {
                        SchemaField::extract_field_type(item)
                            .and_then(|t| BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(&t)))
                            .map_or(json!(null), |k| k.example_value.clone())
                    })
                    .collect();

                // Special case: single-field tuple structs are unwrapped by BRP
                // Return the inner value directly, not as an array
                if elements.len() == 1 {
                    elements.into_iter().next().unwrap_or(json!(null))
                } else {
                    json!(elements)
                }
            },
        )
    }
}

impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        let Some(schema) = ctx.require_schema() else {
            return Ok(paths);
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
        );

        // Add the main tuple path first (example is consumed here)
        let main_path = match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path: String::new(),
                example,
                enum_variants: None,
                type_name: type_name.clone(),
                path_kind: MutationPathKind::RootValue {
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
                path_kind: MutationPathKind::StructField {
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
                            let elem_example = BRP_MUTATION_KNOWLEDGE
                                .get(&KnowledgeKey::exact(&element_type))
                                .map_or(json!(null), |k| k.example_value.clone());

                            paths.push(MutationPathInternal {
                                path:          format!(".{index}"),
                                example:       elem_example,
                                enum_variants: None,
                                type_name:     element_type,
                                path_kind:     MutationPathKind::TupleElement {
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
                            let elem_example = BRP_MUTATION_KNOWLEDGE
                                .get(&KnowledgeKey::exact(&element_type))
                                .map_or(json!(null), |k| k.example_value.clone());

                            paths.push(MutationPathInternal {
                                path:          format!(".{field_name}.{index}"),
                                example:       elem_example,
                                enum_variants: None,
                                type_name:     element_type,
                                path_kind:     MutationPathKind::TupleElement {
                                    index,
                                    parent_type: field_type.clone(),
                                },
                            });
                        }
                    }
                }
            }
        }

        Ok(paths)
    }
}

/// Default builder for simple types (Value, List, Map, Option)
///
/// Handles simple types that don't need complex logic - just creates a standard mutation path
pub struct DefaultMutationBuilder;

impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path:          String::new(),
                    example:       json!(null),
                    enum_variants: None,
                    type_name:     type_name.clone(),
                    path_kind:     MutationPathKind::RootValue {
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
                    path_kind:     MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                });
            }
        }

        Ok(paths)
    }
}

impl StructMutationBuilder {
    /// Build a mutation path for non-mutatable types with explanatory reason
    fn build_not_mutatable_path(
        field_name: &str,
        field_type: &BrpTypeName,
        reason: String,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:          field_name.to_string(),
            example:       json!({
                "NotMutatable": reason,
                "agent_directive": "This path cannot be mutated - do not attempt mutation operations"
            }),
            enum_variants: None,
            type_name:     field_type.clone(),
            path_kind:     MutationPathKind::NotMutatable,
        }
    }
}
