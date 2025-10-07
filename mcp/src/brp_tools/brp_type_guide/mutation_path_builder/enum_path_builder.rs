//! Enum-specific mutation path builder handling variant selection and path generation
//!
//! This module exclusively handles enum types, which require special processing due to:
//! - Multiple variant signatures (unit, tuple, struct) that share the same mutation interface
//! - Variant selection requirements that cascade through the type hierarchy
//! - Generation of variant path examples showing how to reach nested mutation targets these
//!   examples also show the signature and the qualifying applicable variants for each signature
//!   These more sophisticated examples are necessary because the bevy remote protocol mutation
//!   paths are the same for all variants with the same signature.
//!
//! ## Key Responsibilities
//!
//! 1. **Variant Processing**: Groups variants by signature and generates examples for each
//! 2. **Variant Path Management**: Creates and populates the variant chain showing how to select
//!    specific variants to reach a mutation target
//! 3. **Child Path Updates**: Propagates variant examples down to children via
//!    `update_child_variant_paths`
//!
//! ## Example
//!
//! For `Option<GameState>` where `GameState` has a `mode: GameMode` field:
//! - To mutate `.mode`, you must first select `Some` variant: `{"Some": {...}}`
//! - The variant path shows this requirement with instructions and examples
//!
//! ## Integration
//!
//! Called directly by `recurse_mutation_paths` in builder.rs when `TypeKind::Enum` is detected.
//! Unlike other types that use `MutationPathBuilder`, enums bypass the trait system for
//! their specialized processing, then calls back into `recurse_mutation_paths` for its children.

/// Result type for `process_children` containing example groups, child paths, and partial roots
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    BTreeMap<Vec<VariantName>, Value>,
);

use std::collections::{BTreeMap, BTreeSet, HashMap};

use error_stack::Report;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::constants::RecursionDepth;
use super::super::type_kind::TypeKind;
use super::path_kind::MutationPathDescriptor;
use super::recursion_context::RecursionContext;
use super::types::{
    EnumPathData, ExampleGroup, PathAction, StructFieldName, VariantName, VariantPath,
    VariantSignature,
};
use super::{BuilderError, MutationPathInternal, MutationStatus, PathKind, builder};
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::brp_tools::brp_type_guide::mutation_path_builder::types::FullMutationPath;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Type-safe enum variant information
#[derive(Debug, Clone, Serialize, Deserialize)]
enum EnumVariantInfo {
    /// Unit variant - qualified variant name (e.g., "`Color::Srgba`")
    Unit(VariantName),
    /// Tuple variant - qualified name and guaranteed tuple types
    Tuple(VariantName, Vec<BrpTypeName>),
    /// Struct variant - qualified name and guaranteed struct fields
    Struct(VariantName, Vec<EnumFieldInfo>),
}

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnumFieldInfo {
    /// Field name
    field_name: StructFieldName,
    /// Field type
    #[serde(rename = "type")]
    type_name: BrpTypeName,
}

impl EnumVariantInfo {
    /// Get the fully qualified variant name (e.g., "`Color::Srgba`")
    const fn variant_name(&self) -> &VariantName {
        match self {
            Self::Unit(name) | Self::Tuple(name, _) | Self::Struct(name, _) => name,
        }
    }

    /// Get just the variant name without the enum prefix (e.g., "Srgba" from "`Color::Srgba`")
    fn short_name(&self) -> &str {
        self.variant_name()
            .as_str()
            .rsplit_once("::")
            .map_or_else(|| self.variant_name().as_str(), |(_, name)| name)
    }

    /// Compatibility method - delegates to `short_name`
    fn name(&self) -> &str {
        self.short_name()
    }

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

    /// Extract variant information from a schema variant
    fn from_schema_variant(
        v: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: &BrpTypeName,
    ) -> Option<Self> {
        // Handle Unit variants which show up as simple strings
        if let Some(variant_str) = v.as_str() {
            // For simple string variants, we need to construct the full variant name
            // Extract just the type name without module path
            let type_name = enum_type
                .as_str()
                .rsplit("::")
                .next()
                .unwrap_or(enum_type.as_str());

            let qualified_name = format!("{type_name}::{variant_str}");
            return Some(Self::Unit(VariantName::from(qualified_name)));
        }

        // Extract the fully qualified variant name
        let variant_name = extract_variant_qualified_name(v)?;

        // Check what type of variant this is
        if let Some(prefix_items) = v.get_field(SchemaField::PrefixItems) {
            // Tuple variant
            if let Some(prefix_array) = prefix_items.as_array() {
                let tuple_types = extract_tuple_types(prefix_array, registry);
                return Some(Self::Tuple(variant_name, tuple_types));
            }
        } else if let Some(properties) = v.get_field(SchemaField::Properties) {
            // Struct variant
            if let Some(props_map) = properties.as_object() {
                let struct_fields = extract_struct_fields(props_map, registry);
                if !struct_fields.is_empty() {
                    return Some(Self::Struct(variant_name, struct_fields));
                }
            }
        }

        // Unit variant (no fields)
        Some(Self::Unit(variant_name))
    }
}

/// Process enum type directly, bypassing `PathBuilder` trait
///
/// # Simplified Design
///
/// This function always generates examples arrays for all enums, regardless of where
/// they appear in the type hierarchy. This simplification:
///
/// - Removes the need for `EnumContext` tracking
/// - Ensures all enum fields show their available variants
/// - Improves discoverability for nested enums
/// - Makes the behavior predictable and consistent
///
/// Every enum will output:
/// - `example`: null (the example field is always null for enums)
/// - `enum_root_examples`: array of all variant examples
/// - `enum_root_example_for_parent`: concrete value for parent assembly
pub fn process_enum(
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    // Use shared function to get variant information
    let variant_groups = extract_and_group_variants(ctx)?;

    // Process children - now builds examples immediately to avoid HashMap overwrites
    let (enum_examples, child_paths, partial_roots) =
        process_children(&variant_groups, ctx, depth)?;

    // Select default example from the generated examples
    let default_example = select_preferred_example(&enum_examples).unwrap_or(json!(null));

    // Create result paths including both root AND child paths
    Ok(create_result_paths(
        ctx,
        enum_examples,
        default_example,
        child_paths,
        partial_roots,
        depth,
    ))
}

// Helper functions for variant processing
fn extract_variant_name(v: &Value) -> Option<String> {
    v.get_field(SchemaField::ShortPath)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// Extract the fully qualified variant name from schema (e.g., "`Color::Srgba`")
fn extract_variant_qualified_name(v: &Value) -> Option<VariantName> {
    // First try to get the type path for the full qualified name
    if let Some(type_path) = v.get_field(SchemaField::TypePath).and_then(Value::as_str) {
        // Use the new parser to handle nested generics properly
        let simplified_name = super::type_parser::extract_simplified_variant_name(type_path);
        return Some(VariantName::from(simplified_name));
    }

    // Fallback to just the variant name if we can't parse it
    extract_variant_name(v).map(VariantName::from)
}

fn extract_tuple_types(
    prefix_items: &[Value],
    _registry: &HashMap<BrpTypeName, Value>,
) -> Vec<BrpTypeName> {
    prefix_items
        .iter()
        .filter_map(Value::extract_field_type)
        .collect()
}

fn extract_struct_fields(
    properties: &serde_json::Map<String, Value>,
    _registry: &HashMap<BrpTypeName, Value>,
) -> Vec<EnumFieldInfo> {
    properties
        .iter()
        .filter_map(|(field_name, field_schema)| {
            field_schema
                .extract_field_type()
                .map(|type_name| EnumFieldInfo {
                    field_name: StructFieldName::from(field_name.clone()),
                    type_name,
                })
        })
        .collect()
}

fn extract_enum_variants(
    registry_schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    enum_type: &BrpTypeName,
) -> Vec<EnumVariantInfo> {
    let one_of_field = registry_schema.get_field(SchemaField::OneOf);

    one_of_field
        .and_then(Value::as_array)
        .map(|variants| {
            variants
                .iter()
                .filter_map(|v| EnumVariantInfo::from_schema_variant(v, registry, enum_type))
                .collect()
        })
        .unwrap_or_default()
}

fn group_variants_by_signature(
    variants: Vec<EnumVariantInfo>,
) -> BTreeMap<VariantSignature, Vec<EnumVariantInfo>> {
    let mut groups = BTreeMap::new();
    for variant in variants {
        let signature = variant.signature();
        groups
            .entry(signature)
            .or_insert_with(Vec::new)
            .push(variant);
    }
    groups
}

#[derive(Debug, Clone, PartialEq)]
enum TypeCategory {
    Option { inner_type: BrpTypeName },
    Regular(BrpTypeName),
}

impl TypeCategory {
    fn from_type_name(type_name: &BrpTypeName) -> Self {
        Self::extract_option_inner(type_name).map_or_else(
            || Self::Regular(type_name.clone()),
            |inner_type| Self::Option { inner_type },
        )
    }

    const fn is_option(&self) -> bool {
        matches!(self, Self::Option { .. })
    }

    fn extract_option_inner(type_name: &BrpTypeName) -> Option<BrpTypeName> {
        const OPTION_PREFIX: &str = "core::option::Option<";
        const OPTION_SUFFIX: char = '>';

        let type_str = type_name.as_str();
        type_str
            .strip_prefix(OPTION_PREFIX)
            .and_then(|inner_with_suffix| {
                inner_with_suffix
                    .strip_suffix(OPTION_SUFFIX)
                    .map(|inner| BrpTypeName::from(inner.to_string()))
            })
    }
}

/// Apply `Option<T>` transformation if needed: {"Some": value} → value, "None" → null
fn apply_option_transformation(
    example: Value,
    variant_name: &str,
    enum_type: &BrpTypeName,
) -> Value {
    let type_category = TypeCategory::from_type_name(enum_type);
    if !type_category.is_option() {
        return example;
    }

    // Transform Option variants for BRP mutations
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

/// Build a complete example for a variant with all its fields
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &str,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {
    let example = match signature {
        VariantSignature::Unit => {
            json!(variant_name)
        }
        VariantSignature::Tuple(types) => {
            let mut tuple_values = Vec::new();
            for index in 0..types.len() {
                let descriptor = MutationPathDescriptor::from(index.to_string());
                let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                tuple_values.push(value);
            }
            // Fix: Single-element tuples should not be wrapped in arrays
            // Vec<Value> always serializes as JSON array, but BRP expects single-element
            // tuples to use direct value format for mutations, not array format
            if tuple_values.len() == 1 {
                json!({ variant_name: tuple_values[0] })
            } else {
                json!({ variant_name: tuple_values })
            }
        }
        VariantSignature::Struct(field_types) => {
            let mut field_values = serde_json::Map::new();
            for (field_name, _) in field_types {
                let descriptor = MutationPathDescriptor::from(field_name);
                let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                field_values.insert(field_name.to_string(), value);
            }
            json!({ variant_name: field_values })
        }
    };

    // Apply `Option<T>` transformation only for actual Option types
    apply_option_transformation(example, variant_name, enum_type)
}

/// Select the preferred example from a collection of `ExampleGroups`.
/// Prefers non-unit variants for richer examples, falling back to unit variants if needed.
pub fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Value> {
    // Try to find a non-unit variant first
    examples
        .iter()
        .find(|example_group| example_group.signature != "unit")
        .map(|example_group| example_group.example.clone())
        .or_else(|| {
            // Fall back to first example (likely unit variant) if no non-unit variants exist
            examples
                .first()
                .map(|example_group| example_group.example.clone())
        })
}

// ============================================================================
// Public functions moved from enum_builder.rs
// ============================================================================

/// Extract all variants from schema and group them by signature
/// This is the single source of truth for enum variant processing
fn extract_and_group_variants(
    ctx: &RecursionContext,
) -> Result<BTreeMap<VariantSignature, Vec<EnumVariantInfo>>> {
    let schema = ctx.require_registry_schema()?;
    let variants = extract_enum_variants(schema, &ctx.registry, ctx.type_name());
    Ok(group_variants_by_signature(variants))
}

/// Generate single-step mutation instructions for enum paths
///
/// Guides users to use the `root_example` field for single-step mutations
/// instead of the old multi-step `enum_variant_path` approach.
///
/// Note: Don't duplicate `applicable_variants` in the instructions - it's already a separate
/// field in the JSON output
pub fn generate_enum_instructions(
    _enum_data: &EnumPathData,
    full_mutation_path: &FullMutationPath,
) -> String {
    format!(
        "First, set the root mutation path to 'root_example', then you can mutate the '{full_mutation_path}' path. See 'applicable_variants' for which variants support this field."
    )
}

/// Populate variant path with proper instructions and variant examples
fn populate_variant_path(
    ctx: &RecursionContext,
    enum_examples: &[ExampleGroup],
    default_example: &Value,
) -> Vec<VariantPath> {
    let mut populated_paths = Vec::new();

    for variant_path in &ctx.variant_chain {
        let mut populated_path = variant_path.clone();

        // Generate instructions for this variant step
        populated_path.instructions = format!(
            "Mutate '{}' 'full_mutation_path' to the '{}' variant using 'variant_example'",
            if populated_path.full_mutation_path.is_empty() {
                "root".to_string()
            } else {
                populated_path.full_mutation_path.to_string()
            },
            variant_path.variant
        );

        // Find the appropriate example for this variant
        populated_path.variant_example = enum_examples
            .iter()
            .find(|ex| ex.applicable_variants.contains(&variant_path.variant))
            .map_or_else(|| default_example.clone(), |ex| ex.example.clone());

        populated_paths.push(populated_path);
    }

    populated_paths
}

/// Process child paths - simplified version of `MutationPathBuilder`'s child processing
///
/// Now builds examples immediately for each variant group to avoid `HashMap` collision issues
/// where multiple variant groups with the same signature would overwrite each other's examples.
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<ProcessChildrenResult> {
    let mut all_examples = Vec::new();
    let mut all_child_paths = Vec::new();

    // Process each variant group
    for (signature, variants_in_group) in variant_groups {
        // Create FRESH child_examples HashMap for each variant group to avoid overwrites
        let mut child_examples = HashMap::new();

        let applicable_variants: Vec<VariantName> = variants_in_group
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        // Create paths for this signature group
        let paths = create_paths_for_signature(signature, ctx);

        // Process each path
        for path in paths.into_iter().flatten() {
            let mut child_ctx = ctx.create_recursion_context(path.clone(), PathAction::Create);

            // Set up enum context for children
            if let Some(representative_variant) = applicable_variants.first() {
                child_ctx.variant_chain.push(VariantPath {
                    full_mutation_path: ctx.full_mutation_path.clone(),
                    variant: representative_variant.clone(),
                    instructions: String::new(),
                    variant_example: json!(null),
                });
            }
            // Recursively process child and collect paths
            let child_descriptor = path.to_mutation_path_descriptor();
            let child_schema = child_ctx.require_registry_schema()?;
            let child_type_kind = TypeKind::from_schema(child_schema);

            // No enum context needed - each type handles its own behavior

            // Use the same recursion function as MutationPathBuilder
            let mut child_paths =
                builder::recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

            // ==================== NEW: POPULATE applicable_variants ====================
            // Track which variants make these child paths valid
            // Only populate for DIRECT children (not grandchildren nested deeper)
            for child_path in &mut child_paths {
                if let Some(enum_data) = &mut child_path.enum_path_data {
                    // Check if this path is a direct child of the current enum level
                    // Direct children have variant_chain.len() == ctx.variant_chain.len() + 1
                    if enum_data.variant_chain.len() == ctx.variant_chain.len() + 1 {
                        // Add all variants from this signature group
                        // (all variants in a group share the same signature/structure)
                        for variant_name in &applicable_variants {
                            enum_data.applicable_variants.push(variant_name.clone());
                        }
                    }
                }
            }
            // ==================== END NEW CODE ====================

            // Extract example from first path
            // For enum children: use enum_example_for_parent (the concrete variant example)
            // For non-enum children: use example (works for structs/values)
            tracing::debug!(
                "process_children: about to extract example for descriptor={child_descriptor:?}, child_paths.len()={}",
                child_paths.len()
            );

            let child_example = child_paths
                .first()
                .ok_or_else(|| {
                    tracing::error!("Empty child_paths for descriptor {child_descriptor:?}");
                    Report::new(Error::InvalidState(format!(
                        "Empty child_paths returned for descriptor {child_descriptor:?}"
                    )))
                })
                .map(|p| {
                    tracing::debug!(
                        "First path: full_mutation_path={}, has_enum_example_for_parent={}, example={:?}",
                        p.full_mutation_path,
                        p.enum_example_for_parent.is_some(),
                        p.example
                    );
                    // For enum children, use enum_example_for_parent
                    p.enum_example_for_parent.as_ref().map_or_else(
                        || {
                            // For non-enum children, use example
                            tracing::debug!("Using example (no enum_example_for_parent)");
                            p.example.clone()
                        },
                        |enum_example| {
                            tracing::debug!("Using enum_example_for_parent: {enum_example:?}");
                            enum_example.clone()
                        },
                    )
                })?;

            tracing::debug!(
                "process_children: inserting child_descriptor={child_descriptor:?}, child_example={child_example:?}"
            );
            child_examples.insert(child_descriptor, child_example);

            // Collect ALL child paths for the final result
            all_child_paths.extend(child_paths);
        }

        // Build example immediately for this variant group
        let representative = variants_in_group
            .first()
            .ok_or_else(|| Report::new(Error::InvalidState("Empty variant group".to_string())))?;

        let example = build_variant_example(
            signature,
            representative.name(),
            &child_examples,
            ctx.type_name(),
        );

        tracing::debug!(
            "process_children: built example for signature={:?}, example={:?}",
            signature,
            example
        );

        all_examples.push(ExampleGroup {
            applicable_variants,
            signature: signature.to_string(),
            example,
        });
    }

    // Build partial roots using assembly during ascent
    let partial_roots =
        build_partial_roots(variant_groups, &all_examples, &all_child_paths, ctx, depth);

    Ok((all_examples, all_child_paths, partial_roots))
}

/// Create `PathKind` objects for a signature
fn create_paths_for_signature(
    signature: &VariantSignature,
    ctx: &RecursionContext,
) -> Option<Vec<PathKind>> {
    use VariantSignature;

    match signature {
        VariantSignature::Unit => None, // Unit variants have no paths
        VariantSignature::Tuple(types) => {
            let paths: Vec<PathKind> = types
                .iter()
                .enumerate()
                .map(|(index, type_name)| {
                    let path = PathKind::IndexedElement {
                        index,
                        type_name: type_name.clone(),
                        parent_type: ctx.type_name().clone(),
                    };
                    tracing::debug!(
                        "create_paths_for_signature TUPLE: index={}, type_name={}, path_descriptor={:?}",
                        index,
                        type_name,
                        path.to_mutation_path_descriptor()
                    );
                    path
                })
                .collect();
            Some(paths)
        }
        VariantSignature::Struct(fields) => Some(
            fields
                .iter()
                .map(|(field_name, type_name)| PathKind::StructField {
                    field_name: field_name.clone(),
                    type_name: type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                })
                .collect(),
        ),
    }
}

/// Updates `variant_path` entries in child paths with level-appropriate examples
fn update_child_variant_paths(
    paths: &mut [MutationPathInternal],
    current_path: &FullMutationPath,
    current_example: &Value,
    enum_examples: Option<&Vec<ExampleGroup>>,
) {
    // For each child path that has enum variant requirements
    for child in paths.iter_mut() {
        if let Some(enum_data) = &mut child.enum_path_data
            && !enum_data.is_empty()
        {
            // Find matching entry in child's variant_chain that corresponds to our level
            for entry in &mut enum_data.variant_chain {
                if entry.full_mutation_path == *current_path {
                    // This entry represents our current level - update its instructions
                    entry.instructions = format!(
                        "Mutate '{}' mutation 'path' to the '{}' variant using 'variant_example'",
                        if entry.full_mutation_path.is_empty() {
                            "root"
                        } else {
                            &entry.full_mutation_path
                        },
                        &entry.variant
                    );

                    // find the matching variant example
                    if let Some(examples) = enum_examples {
                        entry.variant_example = examples
                            .iter()
                            .find(|ex| ex.applicable_variants.contains(&entry.variant))
                            .map_or_else(|| current_example.clone(), |ex| ex.example.clone());
                    }
                }
            }
        }
    }
}

/// Build partial root examples using assembly during ascent
///
/// Builds partial roots IMMEDIATELY during recursion by wrapping child partial roots
/// as we receive them during the ascent phase.
#[allow(clippy::too_many_lines)]
fn build_partial_roots(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> BTreeMap<Vec<VariantName>, Value> {
    let mut partial_roots = BTreeMap::new();

    // For each variant at THIS level
    for (signature, variants) in variant_groups {
        for variant in variants {
            let our_variant = variant.variant_name().clone();

            // Build our variant chain by extending parent's chain
            let mut our_chain = ctx
                .variant_chain
                .iter()
                .map(|vp| vp.variant.clone())
                .collect::<Vec<_>>();
            our_chain.push(our_variant.clone());

            // Get base example for this variant
            let base_example = enum_examples
                .iter()
                .find(|ex| ex.applicable_variants.contains(&our_variant))
                .map(|ex| ex.example.clone())
                .unwrap_or(json!(null));

            // Collect all unique child chains that start with our_chain
            let mut child_chains_to_wrap = BTreeSet::new();
            for child in child_paths {
                // Skip grandchildren - only process direct children
                if !child.is_direct_child_at_depth(*depth) {
                    continue;
                }

                if let Some(child_partials) = &child.partial_root_examples {
                    for child_chain in child_partials.keys() {
                        if child_chain.starts_with(&our_chain) {
                            child_chains_to_wrap.insert(child_chain.clone());
                        }
                    }
                }
            }

            // Build wrapped examples for each child variant chain
            //
            // VARIANT CHAIN COMPATIBILITY RULE:
            // When building partial roots for a specific `child_chain`, we must only include
            // child paths whose `variant_chain` is compatible with that `child_chain`.
            //
            // Compatibility means: the child's `variant_chain` must be a prefix of `child_chain`.
            //
            // Example: Given `Handle<Image>` enum with two variants (Weak, Strong):
            //   - Weak variant: `.image.0` → `AssetId<Image>` (another enum with Uuid, Index)
            //   - Strong variant: `.image.0` → `Arc<StrongHandle>` (not an enum)
            //
            // When building for `child_chain = ["Handle<Image>::Weak", "AssetId<Image>::Uuid"]`:
            //   - Child with variant_chain `["Handle<Image>::Weak"]` IS compatible ✅ (prefix of
            //     target chain)
            //   - Child with variant_chain `["Handle<Image>::Strong"]` is NOT compatible ❌
            //     (different variant path)
            //
            // Without this filtering, both children share the same descriptor ("0" for tuple
            // index), causing HashMap collisions where the last insert overwrites correct values
            // with incompatible ones (e.g., Strong's null overwrites Weak's nested structure).
            //
            // This ensures deeply nested enum paths like `.image.0.uuid` get correct
            // `root_example` values: `{"Weak": {"Uuid": {"uuid": "..."}}}` rather than
            // `{"Weak": null}`.
            let mut found_child_chains = false;
            for child_chain in child_chains_to_wrap {
                let mut children = HashMap::new();

                // Collect children with variant-specific or regular values
                for child in child_paths {
                    // Skip grandchildren - only process direct children
                    if !child.is_direct_child_at_depth(*depth) {
                        continue;
                    }

                    // Filter by variant_chain compatibility: child's variant_chain must be a
                    // prefix of the child_chain we're building for
                    if let Some(child_enum_data) = &child.enum_path_data {
                        let child_variant_chain =
                            extract_variant_names(&child_enum_data.variant_chain);

                        // Child's variant_chain cannot be longer than target chain
                        if child_variant_chain.len() > child_chain.len() {
                            continue;
                        }

                        // Check prefix compatibility: all elements must match
                        let is_compatible = child_variant_chain
                            .iter()
                            .zip(child_chain.iter())
                            .all(|(child_v, chain_v)| child_v == chain_v);

                        if !is_compatible {
                            continue;
                        }
                    }

                    let descriptor = child.path_kind.to_mutation_path_descriptor();

                    let value = child
                        .partial_root_examples
                        .as_ref()
                        .and_then(|partials| partials.get(&child_chain))
                        .cloned()
                        .unwrap_or_else(|| child.example.clone());

                    children.insert(descriptor, value);
                }

                // Use existing build_variant_example with SHORT variant name
                let wrapped =
                    build_variant_example(signature, variant.name(), &children, ctx.type_name());

                partial_roots.insert(child_chain, wrapped);
                found_child_chains = true;
            }

            // After processing all child chains, also create entry for n-variant chain
            // This handles paths that only specify the outer variant(s)
            if found_child_chains {
                // Build n-variant entry using SAME approach as child chains:
                // Assemble from ALL children with their REGULAR (non-variant-specific) examples
                // This gives us a representative nested structure without tying to specific inner
                // variants
                let mut children = HashMap::new();
                for child in child_paths {
                    // Skip grandchildren - only process direct children
                    if !child.is_direct_child_at_depth(*depth) {
                        continue;
                    }

                    let descriptor = child.path_kind.to_mutation_path_descriptor();
                    children.insert(descriptor, child.example.clone());
                }

                // Wrap with this variant using regular child examples
                let wrapped =
                    build_variant_example(signature, variant.name(), &children, ctx.type_name());
                partial_roots.insert(our_chain.clone(), wrapped);
                tracing::debug!(
                    "[ENUM] Added n-variant chain entry for {:?}",
                    our_chain
                        .iter()
                        .map(super::types::VariantName::as_str)
                        .collect::<Vec<_>>()
                );
            } else {
                // No child chains found, this is a leaf variant - store base example
                partial_roots.insert(our_chain, base_example);
            }
        }
    }

    partial_roots
}

/// Populate `root_example` field using assembly approach
///
/// Uses the `partial_root_examples` already propagated to each path from its wrapping parent.
fn populate_root_example(paths: &mut [MutationPathInternal]) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
        {
            let chain = extract_variant_names(&enum_data.variant_chain);

            tracing::debug!(
                "[POPULATE_ROOT] Path: {}, variant_chain: {:?}",
                path.full_mutation_path,
                chain.iter().map(VariantName::as_str).collect::<Vec<_>>()
            );

            // Use the partial_root_examples that was propagated to this path
            if let Some(ref partials) = path.partial_root_examples {
                tracing::debug!(
                    "[POPULATE_ROOT]   Available keys in partial_root_examples: {:?}",
                    partials
                        .keys()
                        .map(|k| k.iter().map(VariantName::as_str).collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                );

                if let Some(root_example) = partials.get(&chain) {
                    tracing::debug!("[POPULATE_ROOT]   FOUND root_example: {root_example:?}");
                    enum_data.root_example = Some(root_example.clone());
                } else {
                    tracing::debug!(
                        "[POPULATE_ROOT]   NOT FOUND - no entry for chain {:?}",
                        chain.iter().map(VariantName::as_str).collect::<Vec<_>>()
                    );
                }
            } else {
                tracing::debug!("[POPULATE_ROOT]   NO partial_root_examples on path");
            }
        }
    }
}

/// Helper to extract variant names from variant path chain
fn extract_variant_names(variant_chain: &[VariantPath]) -> Vec<VariantName> {
    variant_chain.iter().map(|vp| vp.variant.clone()).collect()
}

/// Create final result paths - includes both root and child paths
fn create_result_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_paths: Vec<MutationPathInternal>,
    partial_roots: BTreeMap<Vec<VariantName>, Value>,
    depth: RecursionDepth,
) -> Vec<MutationPathInternal> {
    // Generate enum data only if we have a variant chain (nested in another enum)
    let enum_data = if ctx.variant_chain.is_empty() {
        None
    } else {
        Some(EnumPathData {
            variant_chain: populate_variant_path(ctx, &enum_examples, &default_example),
            applicable_variants: Vec::new(),
            root_example: None,
        })
    };

    // Direct field assignment - enums ALWAYS generate examples arrays
    let mut root_mutation_path = MutationPathInternal {
        full_mutation_path: ctx.full_mutation_path.clone(),
        example: json!(null), /* Enums always use null for the example field -
                               * they use
                               * Vec<ExampleGroup> */
        enum_example_groups: Some(enum_examples),
        enum_example_for_parent: Some(default_example.clone()),
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutation_status: MutationStatus::Mutable, // Simplified for now
        mutation_status_reason: None,
        enum_path_data: enum_data,
        depth: *depth,
        partial_root_examples: None,
    };

    // Build partial root examples using assembly during ascent
    // Store partial_roots built during ascent in process_children
    root_mutation_path.partial_root_examples = Some(partial_roots.clone());
    tracing::debug!(
        "[ENUM] Built partial_roots for {} with {} chains",
        ctx.type_name(),
        partial_roots.len()
    );
    for (chain, value) in &partial_roots {
        tracing::debug!(
            "[ENUM]   Chain {:?} -> {}",
            chain
                .iter()
                .map(super::types::VariantName::as_str)
                .collect::<Vec<_>>(),
            serde_json::to_string(value).unwrap_or_else(|_| "???".to_string())
        );
    }

    // If we're at the actual root level (empty variant chain),
    // propagate and populate
    if ctx.variant_chain.is_empty() {
        // Propagate to children (overwriting struct-level propagations)
        for child in &mut child_paths {
            child.partial_root_examples = Some(partial_roots.clone());
            tracing::debug!(
                "[ENUM] Propagated partial_roots to child {}",
                child.full_mutation_path
            );
        }

        populate_root_example(&mut child_paths);
    }
    // ==================== END NEW CODE ====================

    // Update variant_path entries in child paths with level-appropriate examples
    // Use the default_example which contains actual data, not the null example
    let example_for_children = &default_example;
    update_child_variant_paths(
        &mut child_paths,
        &ctx.full_mutation_path,
        example_for_children,
        root_mutation_path.enum_example_groups.as_ref(),
    );

    // Return root path plus all child paths (like MutationPathBuilder does)
    let mut result = vec![root_mutation_path];
    result.extend(child_paths);
    result
}
