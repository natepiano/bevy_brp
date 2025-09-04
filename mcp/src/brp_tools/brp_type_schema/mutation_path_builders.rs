//! Mutation path builders for different type kinds
//!
//! This module implements the TYPE-SYSTEM-002 refactor: Replace conditional chains
//! in mutation path building with type-directed dispatch using the `MutationPathBuilder` trait.
//!
//! The key insight is that different `TypeKind` variants need different logic for building
//! mutation paths, but this should be cleanly separated from the field-level logic.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use tracing::warn;

use super::constants::{RecursionDepth, SCHEMA_REF_PREFIX};
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, MutationKnowledge};
use super::response_types::{
    BrpTypeName, EnumVariantInfo, MathComponent, MutationPathInternal, MutationPathKind,
    MutationStatus, SchemaField, TypeKind,
};
use super::type_info::{MutationSupport, TypeInfo};
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

/// Helper to get a schema field as an array
fn get_schema_field_as_array(schema: &Value, field: SchemaField) -> Option<&Vec<Value>> {
    schema.get_field(field).and_then(Value::as_array)
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
pub struct MutationPathContext {
    /// The building context (root or field)
    pub location:         RootOrField,
    /// Reference to the type registry
    registry:             Arc<HashMap<BrpTypeName, Value>>,
    /// Path prefix for nested structures (e.g., ".translation" when building Vec3 fields)
    pub path_prefix:      String,
    /// Parent's mutation knowledge for extracting component examples
    pub parent_knowledge: Option<&'static MutationKnowledge>,
}

impl MutationPathContext {
    /// Create a new mutation path context
    pub const fn new(location: RootOrField, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            location,
            registry,
            path_prefix: String::new(),
            parent_knowledge: None,
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
    pub fn create_field_context(&self, field_name: &str, field_type: &BrpTypeName) -> Self {
        let parent_type = self.type_name();
        // Build the new path prefix by appending the field name to the current prefix
        let new_path_prefix = if self.path_prefix.is_empty() {
            format!(".{field_name}")
        } else {
            format!("{}.{field_name}", self.path_prefix)
        };

        // Check if field type has hardcoded knowledge to pass to children
        let field_knowledge = BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type));

        Self {
            location:         RootOrField::field(field_name, field_type, parent_type),
            registry:         Arc::clone(&self.registry),
            path_prefix:      new_path_prefix,
            parent_knowledge: field_knowledge,
        }
    }

    /// Return an example value unchanged (wrapper functionality removed)
    pub const fn wrap_example(inner_value: Value) -> Value {
        inner_value
    }

    /// Check if a value type has serialization support
    /// Used to determine if opaque Value types like String can be mutated
    pub fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
        use super::response_types::ReflectTrait;

        self.get_type_schema(type_name).is_some_and(|schema| {
            let reflect_types: Vec<ReflectTrait> =
                get_schema_field_as_array(schema, SchemaField::ReflectTypes)
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| s.parse().ok())
                    .collect();

            reflect_types.contains(&ReflectTrait::Serialize)
                && reflect_types.contains(&ReflectTrait::Deserialize)
        })
    }

    /// Extract element type from List or Array schema
    fn extract_list_element_type(schema: &Value) -> Option<BrpTypeName> {
        schema
            .get("items")
            .and_then(|items| items.get_field(SchemaField::Type))
            .and_then(extract_type_ref_with_schema_field)
    }

    /// Extract value type from Map schema
    fn extract_map_value_type(schema: &Value) -> Option<BrpTypeName> {
        schema
            .get("additionalProperties")
            .and_then(|props| props.get_field(SchemaField::Type))
            .and_then(extract_type_ref_with_schema_field)
    }

    /// Extract all element types from Tuple/TupleStruct schema
    fn extract_tuple_element_types(schema: &Value) -> Option<Vec<BrpTypeName>> {
        get_schema_field_as_array(schema, SchemaField::PrefixItems).map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get_field(SchemaField::Type)
                        .and_then(extract_type_ref_with_schema_field)
                })
                .collect()
        })
    }
}

/// Trait for building mutation paths for different type kinds
///
/// This trait provides type-directed dispatch for mutation path building,
/// replacing the large conditional match statement with clean separation of concerns.
/// Each type kind gets its own implementation that handles the specific logic needed.
pub trait MutationPathBuilder {
    /// Build mutation paths with depth tracking for recursion safety
    ///
    /// This method takes a `MutationPathContext` which provides all necessary information
    /// including the registry, wrapper info, and enum variants, plus a `RecursionDepth`
    /// parameter to track recursion depth and prevent infinite loops.
    ///
    /// Returns a `Result` containing a vector of `MutationPathInternal` representing
    /// all possible mutation paths, or an error if path building failed.
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>>;
}

impl TypeKind {
    /// Build `NotMutatable` path from `MutationSupport` error details
    fn build_not_mutatable_path_from_support(
        ctx: &MutationPathContext,
        support: &MutationSupport,
        directive_suffix: &str,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This type cannot be mutated{directive_suffix} - see error message for details")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This field cannot be mutated{directive_suffix} - see error message for details")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(support),
            },
        }
    }
}

/// Implementation of `MutationPathBuilder` for `TypeKind`
///
/// This provides type-directed dispatch - each `TypeKind` variant gets routed
/// to the appropriate specialized builder for handling its specific logic.
impl MutationPathBuilder for TypeKind {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check recursion limit first
        if depth.exceeds_limit() {
            let recursion_limit_path = Self::build_not_mutatable_path_from_support(
                ctx,
                &MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
                "",
            );
            return Ok(vec![recursion_limit_path]);
        }

        // Only increment depth for container types that recurse into nested structures
        let builder_depth = match self {
            // Container types that recurse - increment depth
            Self::Struct
            | Self::Tuple
            | Self::TupleStruct
            | Self::Array
            | Self::List
            | Self::Map
            | Self::Enum => depth.increment(),
            // Leaf types and wrappers - preserve current depth
            Self::Value => depth,
        };

        match self {
            Self::Struct => StructMutationBuilder.build_paths(ctx, builder_depth),
            Self::Tuple | Self::TupleStruct => TupleMutationBuilder.build_paths(ctx, builder_depth),
            Self::Array => ArrayMutationBuilder.build_paths(ctx, builder_depth),
            Self::List => ListMutationBuilder.build_paths(ctx, builder_depth),
            Self::Map => MapMutationBuilder.build_paths(ctx, builder_depth),
            Self::Enum => EnumMutationBuilder.build_paths(ctx, builder_depth),
            Self::Value => {
                // Check serialization inline, no recursion needed
                if ctx.value_type_has_serialization(ctx.type_name()) {
                    DefaultMutationBuilder.build_paths(ctx, builder_depth)
                } else {
                    let not_mutatable_path = Self::build_not_mutatable_path_from_support(
                        ctx,
                        &MutationSupport::MissingSerializationTraits(ctx.type_name().clone()),
                        "",
                    );
                    Ok(vec![not_mutatable_path])
                }
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
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_type) = MutationPathContext::extract_list_element_type(schema) else {
            // If we have a schema but can't extract element type, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // RECURSE DEEPER - don't stop at array level
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(element_type),
            )]);
        };
        let element_kind = TypeKind::from_schema(element_schema, &element_type);

        // Create a new root context for the element type
        let element_ctx =
            MutationPathContext::new(RootOrField::root(&element_type), Arc::clone(&ctx.registry));

        // Continue recursion to actual mutation endpoints
        element_kind.build_paths(&element_ctx, depth) // depth already incremented by TypeKind
    }
}

impl ArrayMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This array type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This array field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }
}

/// Builder for List types (Vec, etc.)
///
/// Similar to `ArrayMutationBuilder` but for dynamic containers like Vec<T>.
/// Uses single-pass recursion to extract element type and recurse deeper.
pub struct ListMutationBuilder;

impl MutationPathBuilder for ListMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_type) = MutationPathContext::extract_list_element_type(schema) else {
            // If we have a schema but can't extract element type, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // RECURSE DEEPER - don't stop at list level
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(element_type),
            )]);
        };
        let element_kind = TypeKind::from_schema(element_schema, &element_type);

        // Create a new root context for the element type
        let element_ctx =
            MutationPathContext::new(RootOrField::root(&element_type), Arc::clone(&ctx.registry));

        // Continue recursion to actual mutation endpoints
        element_kind.build_paths(&element_ctx, depth) // depth already incremented by TypeKind
    }
}

impl ListMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This list type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This list field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }
}

/// Builder for Enum types
///
/// Handles enum mutation paths by extracting variant information and building
/// appropriate examples for each enum variant type (Unit, Tuple, Struct).
pub struct EnumMutationBuilder;

impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // Build enum mutation path inline (following the existing implementation)
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(
            schema,
            &ctx.registry,
            Some(ctx.type_name()),
            depth.increment(),
        );

        match &ctx.location {
            RootOrField::Root { type_name } => Ok(vec![MutationPathInternal {
                path: String::new(),
                example: enum_example,
                enum_variants,
                type_name: type_name.clone(),
                path_kind: MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            }]),
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => Ok(vec![MutationPathInternal {
                path: format!(".{field_name}"),
                example: MutationPathContext::wrap_example(enum_example),
                enum_variants,
                type_name: field_type.clone(),
                path_kind: MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            }]),
        }
    }
}

impl EnumMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This enum type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This enum field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
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
        depth: RecursionDepth, // ADD: Accept recursion depth
    ) -> Value {
        // NEW: Check for exact enum type knowledge first (restores old behavior)
        if let Some(enum_type) = enum_type
            && let Some(knowledge) =
                BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(enum_type.type_string()))
        {
            return knowledge.example_value().clone();
        }

        // Fall back to existing variant building logic...
        if let Some(one_of) = get_schema_field_as_array(schema, SchemaField::OneOf)
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
                                    depth.increment(),
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
                                            depth.increment(),
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
                                            depth.increment(),
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
        depth: RecursionDepth, // ADD: Accept recursion depth
    ) -> Value {
        // Check for enum variant-specific knowledge first
        if let Some(enum_type) = enum_type
            && let Some(variant_name) = variant_name
        {
            // First try newtype variant (for cases like Clear(f32))
            let newtype_key = KnowledgeKey::newtype_variant(
                enum_type.to_string(),
                variant_name.to_string(),
                type_name.to_string(),
            );

            if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&newtype_key) {
                return knowledge.example_value().clone();
            }

            // Fall back to regular enum variant
            let variant_key =
                KnowledgeKey::enum_variant(enum_type.to_string(), variant_name.to_string());

            if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&variant_key) {
                return knowledge.example_value().clone();
            }
        }

        // FIXED: Pass depth through to maintain recursion limits
        TypeInfo::build_example_value_for_type_with_depth(type_name, registry, depth.increment())
    }

    /// Build example struct from properties
    pub fn build_struct_example_from_properties(
        properties: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // Check depth limit to prevent infinite recursion
        if depth.exceeds_limit() {
            return json!("...");
        }

        let Some(props_map) = properties.as_object() else {
            return json!({});
        };

        let mut example = serde_json::Map::new();

        for (field_name, field_schema) in props_map {
            // Use TypeInfo to build example for each field type with depth tracking
            let field_value = SchemaField::extract_field_type(field_schema)
                .map(|field_type| {
                    TypeInfo::build_example_value_for_type_with_depth(
                        &field_type, 
                        registry, 
                        depth.increment()
                    )
                })
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
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check depth limit to prevent infinite recursion
        if depth.exceeds_limit() {
            return Ok(vec![Self::build_not_mutatable_path_from_support(
                ctx,
                MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
            )]);
        }

        let Some(_schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path_from_support(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        let properties = Self::extract_properties(ctx);

        for (field_name, field_info) in properties {
            let Some(field_type) = SchemaField::extract_field_type(field_info) else {
                paths.push(Self::build_not_mutatable_field_from_support(
                    &field_name,
                    &BrpTypeName::from(field_name.as_str()), /* Use field name as type name when
                                                              * extraction fails */
                    ctx,
                    MutationSupport::NotInRegistry(BrpTypeName::from(field_name.as_str())),
                ));
                continue;
            };

            // Create field context using existing method
            let field_ctx = ctx.create_field_context(&field_name, &field_type);

            // Check if field is a Value type needing serialization
            let Some(field_schema) = ctx.get_type_schema(&field_type) else {
                paths.push(Self::build_not_mutatable_field_from_support(
                    &field_name,
                    &field_type,
                    ctx,
                    MutationSupport::NotInRegistry(field_type.clone()),
                ));
                continue;
            };
            let field_kind = TypeKind::from_schema(field_schema, &field_type);

            // Check if this type has hardcoded knowledge (like Vec3, Vec4, etc.)
            let has_hardcoded_knowledge = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(&field_type))
                .is_some();

            if matches!(field_kind, TypeKind::Value) {
                if ctx.value_type_has_serialization(&field_type) {
                    paths.push(Self::build_field_mutation_path(
                        &field_name,
                        &field_type,
                        ctx.type_name(),
                        ctx,
                        depth,
                    ));
                } else {
                    paths.push(Self::build_not_mutatable_field_from_support(
                        &field_name,
                        &field_type,
                        ctx,
                        MutationSupport::MissingSerializationTraits(field_type.clone()),
                    ));
                }
            } else {
                // Recurse for nested containers or structs
                let field_paths = field_kind.build_paths(&field_ctx, depth)?;
                paths.extend(field_paths);
            }

            // Special case: Types with hardcoded knowledge that are also structs
            // (like Vec3, Quat, etc.) should have their direct path AND nested paths
            if has_hardcoded_knowledge && matches!(field_kind, TypeKind::Struct) {
                // We already added paths above through normal recursion,
                // but we also need the direct field path with hardcoded example
                if ctx.value_type_has_serialization(&field_type) {
                    // Build the field path using the context's prefix
                    let field_path = if ctx.path_prefix.is_empty() {
                        format!(".{field_name}")
                    } else {
                        format!("{}.{field_name}", ctx.path_prefix)
                    };

                    // Find and update the direct field path to use hardcoded example
                    if let Some(path) = paths.iter_mut().find(|p| p.path == field_path) {
                        if let Some(knowledge) =
                            BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(&field_type))
                        {
                            path.example = knowledge.example_value().clone();
                        }
                    } else {
                        // If no direct path was created, add it now with hardcoded example
                        paths.push(Self::build_field_mutation_path(
                            &field_name,
                            &field_type,
                            ctx.type_name(),
                            ctx,
                            depth,
                        ));
                    }
                }
            }
        }

        Self::propagate_struct_immutability(&mut paths);
        Ok(paths)
    }
}

impl StructMutationBuilder {
    /// Build a not mutatable path from `MutationSupport` for struct-level errors
    fn build_not_mutatable_path_from_support(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This struct type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This struct field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }

    /// Build a not mutatable field path from `MutationSupport` for field-level errors
    fn build_not_mutatable_field_from_support(
        field_name: &str,
        field_type: &BrpTypeName,
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        // Build path using the context's prefix
        let path = if ctx.path_prefix.is_empty() {
            format!(".{field_name}")
        } else {
            format!("{}.{field_name}", ctx.path_prefix)
        };

        MutationPathInternal {
            path,
            example: json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": "This field cannot be mutated - see error message for details"
            }),
            enum_variants: None,
            type_name: field_type.clone(),
            path_kind: MutationPathKind::StructField {
                field_name:  field_name.to_string(),
                parent_type: ctx.type_name().clone(),
            },
            mutation_status: MutationStatus::NotMutatable,
            error_reason: Option::<String>::from(&support),
        }
    }

    /// Build a single field mutation path
    fn build_field_mutation_path(
        field_name: &str,
        field_type: &BrpTypeName,
        parent_type: &BrpTypeName,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        // First check if parent has math components and this field is a component
        let example_value = ctx.parent_knowledge.map_or_else(
            || {
                // No parent knowledge, use normal logic
                BRP_MUTATION_KNOWLEDGE
                    .get(&KnowledgeKey::exact(field_type))
                    .map_or_else(
                        || {
                            let next_depth = depth.increment();
                            if next_depth.exceeds_limit() {
                                Value::String("...".to_string())
                            } else {
                                TypeInfo::build_example_value_for_type_with_depth(
                                    field_type,
                                    &ctx.registry,
                                    next_depth,
                                )
                            }
                        },
                        |k| k.example_value().clone(),
                    )
            },
            |parent_knowledge| {
                MathComponent::try_from(field_name)
                    .ok()
                    .and_then(|component| parent_knowledge.get_component_example(component))
                    .map_or_else(
                        || {
                            // Either not a math component or no example available
                            BRP_MUTATION_KNOWLEDGE
                                .get(&KnowledgeKey::exact(field_type))
                                .map_or_else(
                                    || {
                                        let next_depth = depth.increment();
                                        if next_depth.exceeds_limit() {
                                            Value::String("...".to_string())
                                        } else {
                                            TypeInfo::build_example_value_for_type_with_depth(
                                                field_type,
                                                &ctx.registry,
                                                next_depth,
                                            )
                                        }
                                    },
                                    |k| k.example_value().clone(),
                                )
                        },
                        std::clone::Clone::clone,
                    )
            },
        );

        let final_example = MutationPathContext::wrap_example(example_value);

        // Build path using the context's prefix
        let path = if ctx.path_prefix.is_empty() {
            format!(".{field_name}")
        } else {
            format!("{}.{field_name}", ctx.path_prefix)
        };

        MutationPathInternal {
            path,
            example: final_example,
            enum_variants: None,
            type_name: field_type.clone(),
            path_kind: MutationPathKind::StructField {
                field_name:  field_name.to_string(),
                parent_type: parent_type.clone(),
            },
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        }
    }

    /// Extract properties from the schema
    fn extract_properties(ctx: &MutationPathContext) -> Vec<(String, &Value)> {
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
                            .map_or(json!(null), |k| k.example_value().clone())
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

    /// Build a mutation path for a single tuple element with registry checking
    fn build_tuple_element_path(
        ctx: &MutationPathContext,
        index: usize,
        element_info: &Value,
        path_prefix: &str,
        parent_type: &BrpTypeName,
    ) -> Option<MutationPathInternal> {
        let element_type = SchemaField::extract_field_type(element_info)?;
        let path = if path_prefix.is_empty() {
            format!(".{index}")
        } else {
            format!("{path_prefix}.{index}")
        };

        // Inline validation for Value types only (similar to TypeKind::build_paths)
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            // Element type not in registry - build error path
            return Some(MutationPathInternal {
                path,
                example: json!({
                    "NotMutatable": format!("{}", super::type_info::MutationSupport::NotInRegistry(element_type.clone())),
                    "agent_directive": "Element type not found in registry"
                }),
                enum_variants: None,
                type_name: element_type.clone(),
                path_kind: MutationPathKind::TupleElement {
                    index,
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason: Option::<String>::from(
                    &super::type_info::MutationSupport::NotInRegistry(element_type),
                ),
            });
        };

        let element_kind =
            super::response_types::TypeKind::from_schema(element_schema, &element_type);
        let supports_mutation = match element_kind {
            super::response_types::TypeKind::Value => {
                ctx.value_type_has_serialization(&element_type)
            }
            // Other types are assumed mutatable (their builders handle validation)
            _ => true,
        };

        if supports_mutation {
            // Element is mutatable, build normal path
            let elem_example = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(&element_type))
                .map_or(json!(null), |k| k.example_value().clone());

            Some(MutationPathInternal {
                path,
                example: elem_example,
                enum_variants: None,
                type_name: element_type,
                path_kind: MutationPathKind::TupleElement {
                    index,
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            })
        } else {
            // Element not mutatable, build error path
            let missing_support =
                super::type_info::MutationSupport::MissingSerializationTraits(element_type.clone());
            Some(MutationPathInternal {
                path,
                example: json!({
                    "NotMutatable": format!("{missing_support}"),
                    "agent_directive": "Element type cannot be mutated through BRP"
                }),
                enum_variants: None,
                type_name: element_type,
                path_kind: MutationPathKind::TupleElement {
                    index,
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason: Option::<String>::from(&missing_support),
            })
        }
    }

    /// Propagate mixed mutability from tuple elements to root path according to DESIGN-001
    fn propagate_tuple_mixed_mutability(paths: &mut [MutationPathInternal]) {
        let has_root = paths.iter().any(|p| p.path.is_empty());

        if has_root {
            let (mutable_count, immutable_count) =
                paths.iter().filter(|p| !p.path.is_empty()).fold(
                    (0, 0),
                    |(mut_count, immut_count), path| match path.mutation_status {
                        MutationStatus::NotMutatable => (mut_count, immut_count + 1),
                        _ => (mut_count + 1, immut_count),
                    },
                );

            // Root mutation strategy based on element composition
            if let Some(root) = paths.iter_mut().find(|p| p.path.is_empty()) {
                match (mutable_count, immutable_count) {
                    (0, _) => {
                        // All elements immutable - root cannot be mutated
                        root.mutation_status = MutationStatus::NotMutatable;
                        root.error_reason = Some("non_mutatable_elements".to_string());
                        root.example = json!({
                            "NotMutatable": format!("Type {} contains non-mutatable element types", root.type_name),
                            "agent_directive": "This tuple cannot be mutated - all elements contain non-mutatable types"
                        });
                    }
                    (_, 0) => {
                        // All elements mutable - keep existing mutable root path
                    }
                    (_, _) => {
                        // Mixed mutability - root cannot be replaced, but individual elements can
                        // be mutated
                        root.mutation_status = MutationStatus::PartiallyMutatable;
                        root.error_reason = Some("partially_mutable_elements".to_string());
                        root.example = json!({
                            "PartialMutation": format!("Some elements of {} are immutable", root.type_name),
                            "agent_directive": "Use individual element paths - root replacement not supported",
                            "mutable_elements": mutable_count,
                            "immutable_elements": immutable_count
                        });
                    }
                }
            }
        }
    }
}

impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        let elements = MutationPathContext::extract_tuple_element_types(schema).unwrap_or_default();

        // Build root tuple path
        Self::build_root_tuple_path(&mut paths, ctx, schema);

        // Build paths for each element
        Self::build_element_paths(&mut paths, ctx, schema, &elements, depth)?;

        // Propagate mixed mutability status to root path
        Self::propagate_tuple_mixed_mutability(&mut paths);
        Ok(paths)
    }
}

impl TupleMutationBuilder {
    fn build_root_tuple_path(
        paths: &mut Vec<MutationPathInternal>,
        ctx: &MutationPathContext,
        schema: &Value,
    ) {
        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path:            String::new(),
                    example:         Self::build_tuple_example(
                        schema
                            .get_field(SchemaField::PrefixItems)
                            .unwrap_or(&json!([])),
                    ),
                    enum_variants:   None,
                    type_name:       type_name.clone(),
                    path_kind:       MutationPathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason:    None,
                });
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                paths.push(MutationPathInternal {
                    path:            format!(".{field_name}"),
                    example:         Self::build_tuple_example(
                        schema
                            .get_field(SchemaField::PrefixItems)
                            .unwrap_or(&json!([])),
                    ),
                    enum_variants:   None,
                    type_name:       field_type.clone(),
                    path_kind:       MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason:    None,
                });
            }
        }
    }

    fn build_element_paths(
        paths: &mut Vec<MutationPathInternal>,
        ctx: &MutationPathContext,
        schema: &Value,
        elements: &[BrpTypeName],
        depth: RecursionDepth,
    ) -> Result<()> {
        for (index, element_type) in elements.iter().enumerate() {
            // Use existing create_field_context with index as field name
            let element_ctx = ctx.create_field_context(&index.to_string(), element_type);
            let Some(element_schema) = ctx.get_type_schema(element_type) else {
                // Build not mutatable element path for missing registry entry
                paths.push(MutationPathInternal {
                    path: format!(".{index}"),
                    example: json!({
                        "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                        "agent_directive": "Element type not found in registry"
                    }),
                    enum_variants: None,
                    type_name: element_type.clone(),
                    path_kind: MutationPathKind::TupleElement {
                        index,
                        parent_type: ctx.type_name().clone(),
                    },
                    mutation_status: MutationStatus::NotMutatable,
                    error_reason: Option::<String>::from(&MutationSupport::NotInRegistry(element_type.clone())),
                });
                continue;
            };
            let element_kind = TypeKind::from_schema(element_schema, element_type);

            // Similar to struct fields - check Value types for serialization
            if matches!(element_kind, TypeKind::Value) {
                if ctx.value_type_has_serialization(element_type) {
                    // Use existing build_tuple_element_path method for Value types
                    if let Some(element_info) = schema
                        .get_field(SchemaField::PrefixItems)
                        .and_then(|items| items.as_array())
                        .and_then(|arr| arr.get(index))
                        && let Some(element_path) = Self::build_tuple_element_path(
                            ctx,
                            index,
                            element_info,
                            "",
                            ctx.type_name(),
                        )
                    {
                        paths.push(element_path);
                    }
                } else {
                    // Build not mutatable element path inline
                    paths.push(MutationPathInternal {
                        path: format!(".{index}"),
                        example: json!({
                            "NotMutatable": format!("{}", MutationSupport::MissingSerializationTraits(element_type.clone())),
                            "agent_directive": "Element type cannot be mutated through BRP"
                        }),
                        enum_variants: None,
                        type_name: element_type.clone(),
                        path_kind: MutationPathKind::TupleElement {
                            index,
                            parent_type: ctx.type_name().clone(),
                        },
                        mutation_status: MutationStatus::NotMutatable,
                        error_reason: Option::<String>::from(&MutationSupport::MissingSerializationTraits(element_type.clone())),
                    });
                }
            } else {
                // Recurse for nested types
                let element_paths = element_kind.build_paths(&element_ctx, depth)?;
                paths.extend(element_paths);
            }
        }
        Ok(())
    }
}

impl TupleMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This tuple type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This tuple field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }
}

/// Builder for Map types
///
/// Handles Map mutation paths with inline value type checking to avoid redundant precheck
pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(value_type) = MutationPathContext::extract_map_value_type(schema) else {
            // If we have a schema but can't extract value type, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // Maps are currently treated as opaque (cannot mutate individual keys)
        // So we just validate value type has serialization and build a single path
        if !ctx.value_type_has_serialization(&value_type) {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::MissingSerializationTraits(value_type),
            )]);
        }

        // Build single opaque mutation path for the entire map
        DefaultMutationBuilder.build_paths(ctx, depth)
    }
}

impl MapMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This map type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This map field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }
}

/// Default builder for simple types (Value, List, Option)
///
/// Handles simple types that don't need complex logic - just creates a standard mutation path
pub struct DefaultMutationBuilder;

impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path:            String::new(),
                    example:         json!(null),
                    enum_variants:   None,
                    type_name:       type_name.clone(),
                    path_kind:       MutationPathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason:    None,
                });
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                paths.push(MutationPathInternal {
                    path:            format!(".{field_name}"),
                    example:         json!(null),
                    enum_variants:   None,
                    type_name:       field_type.clone(),
                    path_kind:       MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason:    None,
                });
            }
        }

        Ok(paths)
    }
}

impl StructMutationBuilder {
    /// Propagate `NotMutatable` status from all struct fields to the root path
    fn propagate_struct_immutability(paths: &mut [MutationPathInternal]) {
        let field_paths: Vec<_> = paths
            .iter()
            .filter(|p| matches!(p.path_kind, MutationPathKind::StructField { .. }))
            .collect();

        if !field_paths.is_empty() {
            let all_fields_not_mutatable = field_paths
                .iter()
                .all(|p| matches!(p.mutation_status, MutationStatus::NotMutatable));

            if all_fields_not_mutatable {
                // Mark any root-level paths as NotMutatable
                for path in paths.iter_mut() {
                    if matches!(path.path_kind, MutationPathKind::RootValue { .. }) {
                        path.mutation_status = MutationStatus::NotMutatable;
                        path.error_reason = Some("non_mutatable_fields".to_string());
                        path.example = json!({
                            "NotMutatable": format!("Type {} contains non-mutatable field types", path.type_name),
                            "agent_directive": "This struct cannot be mutated - all fields contain non-mutatable types"
                        });
                    }
                }
            }
        }
    }
}
