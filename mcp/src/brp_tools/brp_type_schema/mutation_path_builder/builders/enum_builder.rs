//! Builder for Enum types
//!
//! Handles enum mutation paths by extracting variant information and building
//! appropriate examples for each enum variant type (Unit, Tuple, Struct).
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::{PathLocation, RecursionContext};
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::{
    MAX_TYPE_RECURSION_DEPTH, RecursionDepth, SCHEMA_REF_PREFIX,
};
use crate::brp_tools::brp_type_schema::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, SchemaField};
use crate::brp_tools::brp_type_schema::type_info::TypeInfo;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumFieldInfo {
    /// Field name
    pub field_name: String,
    /// Field type
    #[serde(rename = "type")]
    pub type_name:  BrpTypeName,
}

/// Enum variant access patterns for building mutation paths
#[derive(Debug, Clone)]
pub enum VariantAccess {
    /// Tuple element access via index (e.g., `.0`, `.1`)
    TupleIndex(usize),
    /// Struct field access via field name (e.g., `.field_name`)
    StructField(String),
}

/// Variant signatures for deduplication - same signature means same inner structure
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum VariantSignature {
    /// Unit variants (no data)
    Unit,
    /// Tuple variants with specified types
    Tuple(Vec<BrpTypeName>),
    /// Struct variants with field names and types
    Struct(Vec<(String, BrpTypeName)>),
}

/// Type-safe enum variant information - replaces `EnumVariantInfoOld`
/// This enum makes invalid states impossible to construct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnumVariantInfo {
    /// Unit variant - just the variant name
    Unit(String),
    /// Tuple variant - name and guaranteed tuple types
    Tuple(String, Vec<BrpTypeName>),
    /// Struct variant - name and guaranteed struct fields
    Struct(String, Vec<EnumFieldInfo>),
}

impl EnumVariantInfo {
    /// Get the variant name regardless of variant type
    pub fn name(&self) -> &str {
        match self {
            Self::Unit(name) | Self::Tuple(name, _) | Self::Struct(name, _) => name,
        }
    }

    /// Extract inner types and their access methods from this variant
    /// Returns empty vector for unit variants, tuple indices for tuple variants,
    /// and field names for struct variants
    pub fn inner_types(&self) -> Vec<(BrpTypeName, VariantAccess)> {
        match self {
            Self::Unit(_) => Vec::new(),
            Self::Tuple(_, types) => types
                .iter()
                .enumerate()
                .map(|(index, type_name)| (type_name.clone(), VariantAccess::TupleIndex(index)))
                .collect(),
            Self::Struct(_, fields) => fields
                .iter()
                .map(|field| {
                    (
                        field.type_name.clone(),
                        VariantAccess::StructField(field.field_name.clone()),
                    )
                })
                .collect(),
        }
    }

    /// Get the signature of this variant for deduplication
    /// Unit variants return None, tuple variants return type list,
    /// struct variants return field name/type pairs
    pub fn signature(&self) -> VariantSignature {
        match self {
            Self::Unit(_) => VariantSignature::Unit,
            Self::Tuple(_, types) => VariantSignature::Tuple(types.clone()),
            Self::Struct(_, fields) => {
                let field_sig = fields
                    .iter()
                    .map(|f| (f.field_name.clone(), f.type_name.clone()))
                    .collect();
                VariantSignature::Struct(field_sig)
            }
        }
    }

    /// Constructor that infers variant type from JSON structure
    /// instead of relying on separate enum classification
    pub fn from_schema_variant(
        v: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: usize,
    ) -> Option<Self> {
        let name = extract_variant_name(v)?;

        // Infer variant type from JSON structure, not from string parsing
        if v.is_string() {
            Some(Self::Unit(name))
        } else if let Some(prefix_items) = v
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array)
        {
            let types = extract_tuple_types(prefix_items, registry, depth);
            Some(Self::Tuple(name, types))
        } else if let Some(properties) = v
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        {
            let fields = extract_struct_fields(properties, registry, depth);
            Some(Self::Struct(name, fields))
        } else {
            Some(Self::Unit(name)) // Default fallback
        }
    }

    /// Build example JSON for this enum variant
    pub fn build_example(
        &self,
        registry: &HashMap<BrpTypeName, Value>,
        depth: usize,
        enum_type: Option<&BrpTypeName>,
    ) -> Value {
        match self {
            Self::Unit(name) => {
                // NEW: Check for variant-specific knowledge first
                if let Some(enum_type) = enum_type {
                    let variant_key = KnowledgeKey::enum_variant(enum_type.type_string(), name);

                    if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&variant_key) {
                        return knowledge.example_value().clone();
                    }
                }
                // Fall back to default Unit variant behavior
                json!(name)
            }
            Self::Tuple(name, types) => {
                let tuple_values: Vec<Value> = types
                    .iter()
                    .map(|t| {
                        TypeInfo::build_example_value_for_type_with_depth(
                            t,
                            registry,
                            RecursionDepth::from_usize(depth).increment(),
                        )
                    }) // FIXED: Use depth-aware version with recursion tracking
                    .collect();
                // For single-element tuples (newtype pattern), unwrap the single value
                // For multi-element tuples, use array format
                let content = if tuple_values.len() == 1 {
                    // Safe: we just checked length is 1, so index 0 exists
                    tuple_values[0].clone()
                } else {
                    Value::Array(tuple_values)
                };
                json!({ name: content })
            }
            Self::Struct(name, fields) => {
                let struct_obj: serde_json::Map<String, Value> = fields
                    .iter()
                    .map(|f| {
                        (
                            f.field_name.clone(),
                            TypeInfo::build_example_value_for_type_with_depth(
                                &f.type_name,
                                registry,
                                RecursionDepth::from_usize(depth).increment(),
                            ), // FIXED: Use depth-aware version with recursion tracking
                        )
                    })
                    .collect();
                json!({ name: struct_obj })
            }
        }
    }
}

/// Helper function to extract variant name from schema variant
fn extract_variant_name(v: &Value) -> Option<String> {
    // For unit variants, the value is just a string
    if let Value::String(s) = v {
        return Some(s.clone());
    }

    // For tuple/struct variants, look for the shortPath field
    v.get_field(SchemaField::ShortPath)
        .and_then(Value::as_str)
        .map(String::from)
}

/// Helper function to check if recursion depth exceeds the maximum allowed
fn check_depth_exceeded(depth: usize, operation: &str) -> bool {
    if depth > MAX_TYPE_RECURSION_DEPTH {
        tracing::warn!("Max recursion depth reached while {operation}, using fallback");
        true
    } else {
        false
    }
}

/// Create a fallback type for when depth is exceeded
fn create_fallback_type() -> BrpTypeName {
    BrpTypeName::from("f32")
}

/// Create a fallback field for struct variants when depth is exceeded
fn create_fallback_field() -> EnumFieldInfo {
    EnumFieldInfo {
        field_name: "value".to_string(),
        type_name:  create_fallback_type(),
    }
}

/// Helper function to extract tuple types from prefixItems with depth control
/// This prevents stack overflow when processing deeply nested tuple structures
fn extract_tuple_types(
    prefix_items: &[Value],
    _registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> Vec<BrpTypeName> {
    if check_depth_exceeded(depth, "extracting tuple types") {
        return vec![create_fallback_type()];
    }

    prefix_items
        .iter()
        .filter_map(|item| {
            item.get_field(SchemaField::Type)
                .and_then(|t| t.get_field(SchemaField::Ref))
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix(SCHEMA_REF_PREFIX))
                .map(BrpTypeName::from)
        })
        .collect()
}

/// Helper function to extract struct fields from properties with depth control
/// This prevents stack overflow when processing deeply nested struct structures
fn extract_struct_fields(
    properties: &serde_json::Map<String, Value>,
    _registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> Vec<EnumFieldInfo> {
    if check_depth_exceeded(depth, "extracting struct fields") {
        return vec![create_fallback_field()];
    }

    properties
        .iter()
        .filter_map(|(field_name, field_schema)| {
            SchemaField::extract_field_type(field_schema).map(|type_name| EnumFieldInfo {
                field_name: field_name.clone(),
                type_name,
            })
        })
        .collect()
}

/// Build all enum examples - generates one example per unique variant type signature
pub fn build_all_enum_examples(
    schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
    enum_type: Option<&BrpTypeName>, // ADD enum_type parameter
) -> HashMap<String, Value> {
    let variants = extract_enum_variants(schema, registry, depth);

    // Group variants by their type signature and generate one example per group
    let mut examples = HashMap::new();
    let mut seen_unit = false;
    let mut seen_tuples: HashMap<Vec<BrpTypeName>, String> = HashMap::new();
    let mut seen_structs: HashMap<Vec<(String, BrpTypeName)>, String> = HashMap::new();

    for variant in variants {
        match &variant {
            EnumVariantInfo::Unit(name) => {
                if !seen_unit {
                    let example = variant.build_example(registry, depth, enum_type); // Pass both
                    examples.insert(name.clone(), example);
                    seen_unit = true;
                }
            }
            EnumVariantInfo::Tuple(name, types) => {
                if !seen_tuples.contains_key(types) {
                    let example = variant.build_example(registry, depth, enum_type); // Pass both
                    examples.insert(name.clone(), example);
                    seen_tuples.insert(types.clone(), name.clone());
                }
            }
            EnumVariantInfo::Struct(name, fields) => {
                let field_sig: Vec<(String, BrpTypeName)> = fields
                    .iter()
                    .map(|f| (f.field_name.clone(), f.type_name.clone()))
                    .collect();
                if let std::collections::hash_map::Entry::Vacant(e) = seen_structs.entry(field_sig)
                {
                    let example = variant.build_example(registry, depth, enum_type); // Pass both
                    examples.insert(name.clone(), example);
                    e.insert(name.clone());
                }
            }
        }
    }

    examples
}

/// Deduplicate variants by signature, returning first variant of each unique signature
/// This prevents redundant processing when multiple variants have the same type structure
pub fn deduplicate_variant_signatures(variants: Vec<EnumVariantInfo>) -> Vec<EnumVariantInfo> {
    use std::collections::HashSet;

    let mut seen_signatures = HashSet::new();
    let mut unique_variants = Vec::new();

    for variant in variants {
        let signature = variant.signature();
        if seen_signatures.insert(signature) {
            unique_variants.push(variant);
        }
    }

    unique_variants
}

/// Extract enum variants using the new `EnumVariantInfo` enum
pub fn extract_enum_variants(
    type_schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> Vec<EnumVariantInfo> {
    type_schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
        .map(|variants| {
            variants
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    EnumVariantInfo::from_schema_variant(v, registry, depth)
                        .or_else(|| {
                            tracing::warn!("Failed to parse enum variant {i} in schema - this is unexpected as BRP should provide valid variants");
                            None
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}
pub struct EnumMutationBuilder;

impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // Check depth limit first (like StructMutationBuilder does)
        if depth.exceeds_limit() {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
            )]);
        }

        let mut paths = Vec::new();

        // Step 1: Add the base enum path with ALL signature examples
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(
            schema,
            &ctx.registry,
            Some(ctx.type_name()),
            depth, // No increment here - just pass current depth
        );

        match &ctx.location {
            PathLocation::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path: String::new(),
                    example: enum_example,
                    enum_variants,
                    type_name: type_name.clone(),
                    path_kind: PathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
            PathLocation::Element {
                mutation_path: field_name,
                element_type: field_type,
                parent_type,
            } => {
                // When in field context, use the path_prefix which contains the full path
                let path = if ctx.path_prefix.is_empty() {
                    format!(".{field_name}")
                } else {
                    ctx.path_prefix.clone()
                };
                paths.push(MutationPathInternal {
                    path,
                    example: RecursionContext::wrap_example(enum_example),
                    enum_variants,
                    type_name: field_type.clone(),
                    path_kind: PathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
        }

        // Step 2: Recurse into unique signature inner types
        // ONLY add variant field paths when the enum is at the ROOT level
        // When an enum is a field, we don't recurse into its variants because:
        // 1. Only one variant can be active at a time
        // 2. The variant is selected when setting the field value
        // 3. Variant fields are accessed through the enum field path (e.g., .field.0.variant_field)
        if matches!(ctx.location, PathLocation::Root { .. }) {
            let variants = extract_enum_variants(schema, &ctx.registry, *depth);
            let unique_variants = deduplicate_variant_signatures(variants);

            for variant in unique_variants {
                for (type_name, variant_access) in variant.inner_types() {
                    // Get the schema for the inner type
                    let Some(inner_schema) = ctx.get_type_schema(&type_name) else {
                        continue; // Skip if we can't find the schema
                    };

                    let inner_kind = TypeKind::from_schema(inner_schema, &type_name);

                    // Create field context for recursion using existing infrastructure
                    let accessor = match &variant_access {
                        VariantAccess::TupleIndex(idx) => format!(".{idx}"),
                        VariantAccess::StructField(name) => format!(".{name}"),
                    };
                    let variant_ctx = ctx.create_field_context(&accessor, &type_name);

                    // Recurse with current depth (TypeKind::build_paths will increment if needed)
                    let nested_paths = inner_kind.build_paths(&variant_ctx, depth)?;
                    paths.extend(nested_paths);
                }
            }
        }

        Ok(paths)
    }
}

impl EnumMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            PathLocation::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This enum type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       PathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            PathLocation::Element {
                mutation_path: field_name,
                element_type: field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This enum field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       PathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }

    /// Extract enum variants from type schema
    pub fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
        let variants = extract_enum_variants(type_schema, &HashMap::new(), 0);
        if variants.is_empty() {
            None
        } else {
            Some(variants.iter().map(|v| v.name().to_string()).collect())
        }
    }

    /// Build example value for an enum type
    /// CHANGED: Now returns ALL variant examples instead of just the first one
    /// by calling the existing `build_all_enum_examples` function
    pub fn build_enum_example(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: Option<&BrpTypeName>,
        depth: RecursionDepth,
    ) -> Value {
        // Check for exact enum type knowledge first
        if let Some(enum_type) = enum_type
            && let Some(knowledge) =
                BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(enum_type.type_string()))
        {
            return knowledge.example_value().clone();
        }

        // CRITICAL: Reuse EXISTING build_all_enum_examples function
        // DO NOT reimplement the deduplication logic - it already exists!
        let all_examples = build_all_enum_examples(schema, registry, *depth, enum_type);

        // Return all variant examples as JSON
        if all_examples.is_empty() {
            json!(null)
        } else {
            json!(all_examples)
        }
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
                        depth, // Don't increment - TypeInfo will handle it
                    )
                })
                .unwrap_or(json!(null));

            example.insert(field_name.clone(), field_value);
        }

        json!(example)
    }
}
