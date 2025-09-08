//! Builder for Enum types
//!
//! Handles enum mutation paths by extracting variant information and building
//! appropriate examples for each enum variant type (Unit, Tuple, Struct).
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::{
    MAX_TYPE_RECURSION_DEPTH, RecursionDepth, SCHEMA_REF_PREFIX,
};
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, SchemaField};
use crate::brp_tools::brp_type_schema::type_info::TypeInfo;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

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

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumFieldInfo {
    /// Field name
    pub field_name: String,
    /// Field type
    #[serde(rename = "type")]
    pub type_name:  BrpTypeName,
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

impl std::fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Tuple(types) => {
                let short_types: Vec<String> = types
                    .iter()
                    .map(|t| shorten_type_name(t.as_str()))
                    .collect();
                if short_types.len() == 1 {
                    write!(f, "tuple({})", short_types[0])
                } else {
                    write!(f, "tuple({})", short_types.join(", "))
                }
            }
            Self::Struct(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, typ)| format!("{name}: {}", shorten_type_name(typ.as_str())))
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
}

/// Convert a fully-qualified type name to a short readable name
fn shorten_type_name(type_name: &str) -> String {
    // Handle common fully-qualified type names
    match type_name {
        "alloc::string::String" => "String".to_string(),
        "core::option::Option" => "Option".to_string(),
        name if name.starts_with("alloc::string::String") => "String".to_string(),
        name if name.starts_with("core::option::Option<") => {
            // Extract the inner type and shorten it recursively
            if let Some(inner) = name
                .strip_prefix("core::option::Option<")
                .and_then(|s| s.strip_suffix('>'))
            {
                format!("Option<{}>", shorten_type_name(inner))
            } else {
                "Option".to_string()
            }
        }
        name => {
            // For other types, just take the last segment after ::
            name.split("::").last().unwrap_or(name).to_string()
        }
    }
}

impl EnumVariantInfo {
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

    /// Get the signature of this variant for deduplication
    /// Unit variants return None, tuple variants return type list,
    /// struct variants return field name/type pairs
    fn signature(&self) -> VariantSignature {
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

    /// Build example JSON for this enum variant
    fn build_example(&self, registry: &HashMap<BrpTypeName, Value>, depth: usize) -> Value {
        match self {
            Self::Unit(name) => {
                // just output the name
                json!(name)
            }
            Self::Tuple(name, types) => {
                let tuple_values: Vec<Value> = types
                    .iter()
                    .map(|t| {
                        TypeInfo::build_type_example(
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
                            TypeInfo::build_type_example(
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

/// Build all enum examples - groups variants by signature and creates proper structure
/// This creates a special structured object that the conversion layer can understand
fn build_all_enum_examples(
    schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> HashMap<String, Value> {
    let variants = extract_enum_variants(schema, registry, depth);
    let variant_groups = group_variants_by_signature(variants);

    let mut signature_examples = Vec::new();

    for (signature, variants_in_group) in variant_groups {
        // Use the first variant in the group as the representative example
        if let Some(representative_variant) = variants_in_group.first() {
            let example = representative_variant.build_example(registry, depth);
            let variant_names: Vec<String> = variants_in_group
                .iter()
                .map(|v| match v {
                    EnumVariantInfo::Unit(name) => name.clone(),
                    EnumVariantInfo::Tuple(name, _) => name.clone(),
                    EnumVariantInfo::Struct(name, _) => name.clone(),
                })
                .collect();

            signature_examples.push(json!({
                "signature": signature.to_string(),
                "variants": variant_names,
                "example": example
            }));
        }
    }

    // Return a special structure that indicates this is grouped enum examples
    let mut result = HashMap::new();
    result.insert(
        "__enum_signature_groups".to_string(),
        json!(signature_examples),
    );
    result
}

/// Group variants by their signature, keeping ALL variants in each group
/// Returns a mapping from signature to all variants that share that signature
fn group_variants_by_signature(
    variants: Vec<EnumVariantInfo>,
) -> HashMap<VariantSignature, Vec<EnumVariantInfo>> {
    let mut signature_groups: HashMap<VariantSignature, Vec<EnumVariantInfo>> = HashMap::new();

    for variant in variants {
        let signature = variant.signature();
        signature_groups.entry(signature).or_default().push(variant);
    }

    signature_groups
}

/// Deduplicate variants by signature, returning first variant of each unique signature
/// This prevents redundant processing when multiple variants have the same type structure
fn deduplicate_variant_signatures(variants: Vec<EnumVariantInfo>) -> Vec<EnumVariantInfo> {
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
fn extract_enum_variants(
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
        let example = Self::build_enum_example(
            schema,
            &ctx.registry,
            Some(ctx.type_name()),
            depth, // No increment here - just pass current depth
        );

        paths.push(MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        });

        // Step 2: Add mutation paths for fields WITHIN variants
        // When we encounter an enum, we create paths to mutate fields inside the active variant.
        // For example, an enum with a struct variant containing field "foo" gets a path ".foo"
        // This applies to both root enums and field enums to support nested mutation.
        {
            let variants = extract_enum_variants(schema, &ctx.registry, *depth);
            let unique_variants = deduplicate_variant_signatures(variants);

            for variant in unique_variants {
                match variant {
                    EnumVariantInfo::Unit(_) => {} /* Unit variants have no inner fields to */
                    // recurse into
                    EnumVariantInfo::Tuple(variant_name, types) => {
                        for (index, type_name) in types.iter().enumerate() {
                            let Some(inner_schema) = ctx.get_type_schema(type_name) else {
                                continue;
                            };

                            // Create field context using PathKind
                            let field_path_kind = PathKind::new_indexed_element(
                                index,
                                type_name.clone(),
                                ctx.type_name().clone(),
                            );
                            let field_ctx = ctx.create_field_context(field_path_kind);
                            let inner_kind = TypeKind::from_schema(inner_schema, type_name);
                            let mut field_paths = inner_kind.build_paths(&field_ctx, depth)?;

                            // Add variant information to indicate which enum variant contains this
                            // field
                            for path in &mut field_paths {
                                path.example = Self::add_variant_context_to_example(
                                    &path.example,
                                    &variant_name,
                                );
                            }

                            paths.extend(field_paths);
                        }
                    }
                    EnumVariantInfo::Struct(variant_name, fields) => {
                        for field in fields {
                            let Some(inner_schema) = ctx.get_type_schema(&field.type_name) else {
                                continue;
                            };

                            // Create field context using PathKind
                            let field_path_kind = PathKind::new_struct_field(
                                field.field_name.clone(),
                                field.type_name.clone(),
                                ctx.type_name().clone(),
                            );
                            let field_ctx = ctx.create_field_context(field_path_kind);
                            let inner_kind = TypeKind::from_schema(inner_schema, &field.type_name);
                            let mut field_paths = inner_kind.build_paths(&field_ctx, depth)?;

                            // Add variant information to indicate which enum variant contains this
                            // field
                            for path in &mut field_paths {
                                path.example = Self::add_variant_context_to_example(
                                    &path.example,
                                    &variant_name,
                                );
                            }

                            paths.extend(field_paths);
                        }
                    }
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
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This enum type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }

    /// Add variant context to an example to indicate which enum variant contains this field
    /// This creates a wrapper object that includes variant information
    fn add_variant_context_to_example(example: &Value, variant_name: &str) -> Value {
        json!({
            "__variant_context": variant_name,
            "example": example
        })
    }

    /// Build example value for an enum type
    /// Now returns ALL variant examples instead of just the first one
    /// by calling the existing `build_all_enum_examples` function
    /// Build enum example for mutation paths (returns grouped structure)
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
            return knowledge.example().clone();
        }

        let all_examples = build_all_enum_examples(schema, registry, *depth);

        // Return all variant examples as JSON
        if all_examples.is_empty() {
            json!(null)
        } else {
            json!(all_examples)
        }
    }

    /// Build single concrete enum example for spawn format (returns usable value)
    pub fn build_enum_spawn_example(
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
            return knowledge.example().clone();
        }

        let variants = extract_enum_variants(schema, registry, *depth);

        // Pick the first variant as our concrete spawn example
        if let Some(first_variant) = variants.first() {
            first_variant.build_example(registry, *depth)
        } else {
            json!(null)
        }
    }
}
