//! Builder for Enum types
//!
//! Handles enum mutation paths by extracting variant information and building
//! appropriate examples for each enum variant type (Unit, Tuple, Struct).
//!
//! **Recursion**: YES - Enums fully support mutation:
//! - Root path `""` can replace the entire enum, INCLUDING changing the active variant
//! - Struct variants recurse into fields (e.g., `MyEnum::Config.enabled`)
//! - Tuple variants recurse into elements (e.g., `Option::Some.0`)
//! - Unit variants have no fields to recurse into
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::{MAX_TYPE_RECURSION_DEPTH, RecursionDepth};
use crate::brp_tools::brp_type_schema::example_builder::ExampleBuilder;
use crate::brp_tools::brp_type_schema::response_types::BrpTypeName;
use crate::error::Result;
use crate::json_types::SchemaField;
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
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
            name.strip_prefix("core::option::Option<")
                .and_then(|s| s.strip_suffix('>'))
                .map_or_else(
                    || "Option".to_string(),
                    |inner| format!("Option<{}>", shorten_type_name(inner)),
                )
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
                        ExampleBuilder::build_example(
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
                            ExampleBuilder::build_example(
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
        .filter_map(SchemaField::extract_field_type)
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

/// Group variants by their signature, keeping ALL variants in each group
/// Returns a mapping from signature to all variants that share that signature
fn group_variants_by_signature(
    variants: Vec<EnumVariantInfo>,
) -> Vec<(VariantSignature, Vec<EnumVariantInfo>)> {
    let mut signature_groups: HashMap<VariantSignature, Vec<EnumVariantInfo>> = HashMap::new();

    for variant in variants {
        let signature = variant.signature();
        signature_groups.entry(signature).or_default().push(variant);
    }

    // Convert to sorted Vec for deterministic ordering
    let mut groups: Vec<(VariantSignature, Vec<EnumVariantInfo>)> =
        signature_groups.into_iter().collect();
    groups.sort_by_key(|(signature, _)| signature.clone());
    groups
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
        tracing::error!("ENUM {} - Starting build_paths", ctx.type_name());
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

        // Step 1: Process all variants to accumulate field examples
        let variants = extract_enum_variants(schema, &ctx.registry, *depth);
        let unique_variants = deduplicate_variant_signatures(variants.clone());

        // Collect variant examples from accumulated child paths
        let mut variant_examples_map = HashMap::new();

        for variant in &unique_variants {
            match variant {
                EnumVariantInfo::Unit(name) => {
                    // Unit variants have no fields to accumulate
                    variant_examples_map.insert(name.clone(), json!(name));
                }
                EnumVariantInfo::Tuple(variant_name, types) => {
                    let mut tuple_values = Vec::new();

                    for (index, type_name) in types.iter().enumerate() {
                        let Some(inner_schema) = ctx.get_type_schema(type_name) else {
                            tuple_values.push(json!(null));
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
                        tracing::error!(
                            "ENUM VARIANT {} - Before build_paths for tuple element {} (parent: {})",
                            variant_name,
                            index,
                            ctx.type_name()
                        );
                        let mut field_paths = inner_kind.build_paths(&field_ctx, depth)?;
                        tracing::error!(
                            "ENUM VARIANT {} - After build_paths for tuple element {}, got {} paths (parent: {})",
                            variant_name,
                            index,
                            field_paths.len(),
                            ctx.type_name()
                        );

                        // Extract the example from the field's root path
                        tracing::error!(
                            "ENUM VARIANT {} - Extracting field example (parent: {})",
                            variant_name,
                            ctx.type_name()
                        );
                        let field_example = field_paths
                            .iter()
                            .find(|p| p.path == field_ctx.mutation_path)
                            .map(|p| p.example.clone())
                            .unwrap_or_else(|| {
                                // Fallback to trait dispatch if no direct path
                                tracing::error!("ENUM VARIANT {} - No direct path, using trait dispatch (parent: {})", variant_name, ctx.type_name());
                                inner_kind
                                    .builder()
                                    .build_schema_example(&field_ctx, depth.increment())
                            });

                        tracing::error!(
                            "ENUM VARIANT {} - Pushing field example to tuple_values (parent: {})",
                            variant_name,
                            ctx.type_name()
                        );
                        tuple_values.push(field_example);

                        // Add variant context to field paths for proper inheritance
                        tracing::error!(
                            "ENUM VARIANT {} - Before extending paths, current: {} (parent: {})",
                            variant_name,
                            paths.len(),
                            ctx.type_name()
                        );

                        // Add variant context to each field path before extending
                        for field_path in &mut field_paths {
                            // Create variant context for this field path
                            let variant_context = json!([variant_name.clone()]);
                            // Store variant context in the example - the conversion will extract it
                            if let Some(obj) = field_path.example.as_object_mut() {
                                obj.insert("__variant_context".to_string(), variant_context);
                            } else {
                                // For non-object examples, wrap in object with variant context
                                field_path.example = json!({
                                    "value": field_path.example,
                                    "__variant_context": variant_context
                                });
                            }
                        }

                        paths.extend(field_paths);
                        tracing::error!(
                            "ENUM VARIANT {} - After extending paths, new total: {} (parent: {})",
                            variant_name,
                            paths.len(),
                            ctx.type_name()
                        );
                    }

                    // Build tuple variant example from accumulated values
                    let content = if tuple_values.len() == 1 {
                        tuple_values[0].clone()
                    } else {
                        Value::Array(tuple_values)
                    };
                    variant_examples_map
                        .insert(variant_name.clone(), json!({ variant_name: content }));
                }
                EnumVariantInfo::Struct(variant_name, fields) => {
                    let mut struct_obj = serde_json::Map::new();

                    for field in fields {
                        let Some(inner_schema) = ctx.get_type_schema(&field.type_name) else {
                            struct_obj.insert(field.field_name.clone(), json!(null));
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
                        tracing::error!(
                            "ENUM VARIANT {} - Before build_paths for struct field {} (parent: {})",
                            variant_name,
                            field.field_name,
                            ctx.type_name()
                        );
                        let mut field_paths = inner_kind.build_paths(&field_ctx, depth)?;
                        tracing::error!(
                            "ENUM VARIANT {} - After build_paths for struct field {}, got {} paths (parent: {})",
                            variant_name,
                            field.field_name,
                            field_paths.len(),
                            ctx.type_name()
                        );

                        // Extract the example from the field's root path
                        let field_example = field_paths
                            .iter()
                            .find(|p| p.path == field_ctx.mutation_path)
                            .map(|p| p.example.clone())
                            .unwrap_or_else(|| {
                                // Fallback to trait dispatch if no direct path
                                inner_kind
                                    .builder()
                                    .build_schema_example(&field_ctx, depth.increment())
                            });

                        struct_obj.insert(field.field_name.clone(), field_example);

                        // Add variant context to field paths for proper inheritance
                        tracing::error!(
                            "ENUM VARIANT {} - Before extending paths, current: {} (parent: {})",
                            variant_name,
                            paths.len(),
                            ctx.type_name()
                        );

                        // Add variant context to each field path before extending
                        for field_path in &mut field_paths {
                            // Create variant context for this field path
                            let variant_context = json!([variant_name.clone()]);
                            // Store variant context in the example - the conversion will extract it
                            if let Some(obj) = field_path.example.as_object_mut() {
                                obj.insert("__variant_context".to_string(), variant_context);
                            } else {
                                // For non-object examples, wrap in object with variant context
                                field_path.example = json!({
                                    "value": field_path.example,
                                    "__variant_context": variant_context
                                });
                            }
                        }

                        paths.extend(field_paths);
                        tracing::error!(
                            "ENUM VARIANT {} - After extending paths, new total: {} (parent: {})",
                            variant_name,
                            paths.len(),
                            ctx.type_name()
                        );
                    }

                    // Build struct variant example from accumulated fields
                    variant_examples_map
                        .insert(variant_name.clone(), json!({ variant_name: struct_obj }));
                }
            }
        }

        // Step 2: Build the enum example using accumulated variant examples
        let example = Self::build_enum_example_from_accumulated(
            &variants,
            &variant_examples_map,
            ctx.type_name(),
            ctx,
        );

        paths.insert(
            0,
            MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example,
                type_name: ctx.type_name().clone(),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            },
        );

        tracing::error!("ENUM {} - Returning {} paths", ctx.type_name(), paths.len());
        Ok(paths)
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        let Some(schema) = ctx.require_schema() else {
            return json!(null);
        };

        // Check depth limit to prevent infinite recursion
        if depth.exceeds_limit() {
            return json!("...");
        }

        // Use the existing build_enum_spawn_example for concrete spawn format
        Self::build_enum_spawn_example(schema, &ctx.registry, Some(ctx.type_name()), depth)
    }
}

impl EnumMutationBuilder {
    /// Build enum example from accumulated variant examples
    fn build_enum_example_from_accumulated(
        variants: &[EnumVariantInfo],
        variant_examples_map: &HashMap<String, Value>,
        enum_type: &BrpTypeName,
        ctx: &RecursionContext,
    ) -> Value {
        // Check for exact enum type knowledge first
        if let Some(knowledge) =
            BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(enum_type.type_string()))
        {
            return knowledge.example().clone();
        }

        // Note: Always return signature groups for enum mutation paths
        // Concrete examples are only used when building parent struct examples

        // For non-root paths, continue with signature groups format
        let variant_groups = group_variants_by_signature(variants.to_vec());
        let mut signature_examples = Vec::new();

        for (signature, variants_in_group) in variant_groups {
            // Use the first variant in the group as the representative
            if let Some(representative_variant) = variants_in_group.first() {
                let variant_name = match representative_variant {
                    EnumVariantInfo::Unit(name)
                    | EnumVariantInfo::Tuple(name, _)
                    | EnumVariantInfo::Struct(name, _) => name,
                };

                // Get the accumulated example for this variant
                let example = variant_examples_map
                    .get(variant_name)
                    .cloned()
                    .unwrap_or(json!(null));

                let example = Self::apply_option_transformation(
                    example,
                    representative_variant,
                    Some(enum_type),
                );

                let variant_names: Vec<String> = variants_in_group
                    .iter()
                    .map(|v| match v {
                        EnumVariantInfo::Unit(name)
                        | EnumVariantInfo::Tuple(name, _)
                        | EnumVariantInfo::Struct(name, _) => name.clone(),
                    })
                    .collect();

                signature_examples.push(json!({
                    "signature": signature.to_string(),
                    "variants": variant_names,
                    "example": example
                }));
            }
        }

        // Return signature examples array directly in the NEW format
        if signature_examples.is_empty() {
            json!(null)
        } else {
            json!(signature_examples)
        }
    }

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

    /// Check if a type is Option<T>
    fn is_option_type(type_name: &BrpTypeName) -> bool {
        type_name.as_str().starts_with("core::option::Option<")
    }

    /// Transform Option<T> examples to proper format
    /// None -> null, Some(value) -> value
    fn transform_option_example(example: Value, variant_name: &str) -> Value {
        match variant_name {
            "None" => json!(null),
            "Some" => {
                // Extract the inner value from {"Some": value}
                if let Some(obj) = example.as_object()
                    && let Some(value) = obj.get("Some")
                {
                    return value.clone();
                }
                example
            }
            _ => example,
        }
    }

    /// Apply Option<T> transformation if needed
    fn apply_option_transformation(
        example: Value,
        variant: &EnumVariantInfo,
        enum_type: Option<&BrpTypeName>,
    ) -> Value {
        if let Some(enum_type) = enum_type
            && Self::is_option_type(enum_type)
        {
            let variant_name = match variant {
                EnumVariantInfo::Unit(name)
                | EnumVariantInfo::Tuple(name, _)
                | EnumVariantInfo::Struct(name, _) => name,
            };
            return Self::transform_option_example(example, variant_name);
        }
        example
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
        variants.first().map_or(json!(null), |first_variant| {
            let example = first_variant.build_example(registry, *depth);
            Self::apply_option_transformation(example, first_variant, enum_type)
        })
    }
}
