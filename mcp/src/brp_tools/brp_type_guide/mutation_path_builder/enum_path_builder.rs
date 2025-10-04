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

use std::collections::{BTreeMap, HashMap, HashSet};

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
    type_name:  BrpTypeName,
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

    // Process children and collect BOTH examples AND child paths
    let (child_examples, child_paths) = process_children(&variant_groups, ctx, depth)?;

    // Use shared function to build examples
    let (enum_examples, default_example) =
        build_enum_examples(&variant_groups, child_examples, ctx)?;

    // Create result paths including both root AND child paths
    create_result_paths(ctx, enum_examples, default_example, child_paths)
        .map_err(BuilderError::from)
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

/// Build enum examples from variant groups and child examples
/// This handles all enum context logic in one place
fn build_enum_examples(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    child_examples: HashMap<MutationPathDescriptor, Value>,
    ctx: &RecursionContext,
) -> Result<(Vec<ExampleGroup>, Value)> {
    use error_stack::Report;

    // Build internal MutationExample to organize the enum logic
    tracing::debug!("build_enum_examples for {}", ctx.type_name());

    // Always build examples array for enums - no context check needed
    let mut examples = Vec::new();

    for (signature, variants_in_group) in variant_groups {
        let representative = variants_in_group
            .first()
            .ok_or_else(|| Report::new(Error::InvalidState("Empty variant group".to_string())))?;

        let example = build_variant_example(
            signature,
            representative.name(),
            &child_examples,
            ctx.type_name(),
        );

        let applicable_variants: Vec<VariantName> = variants_in_group
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        examples.push(ExampleGroup {
            applicable_variants,
            signature: signature.to_string(),
            example,
        });
    }

    let mutation_example = examples;

    // For enum roots, return both examples array and a default concrete value
    // Use the shared utility to prefer non-unit variants
    let default_example = select_preferred_example(&mutation_example).unwrap_or(json!(null));

    tracing::debug!(
        "build_enum_examples returning EnumRoot with {} examples",
        mutation_example.len()
    );

    Ok((mutation_example, default_example))
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
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(
    HashMap<MutationPathDescriptor, Value>,
    Vec<MutationPathInternal>,
)> {
    let mut child_examples = HashMap::new();
    let mut all_child_paths = Vec::new();

    // Process each variant group
    for (signature, variants_in_group) in variant_groups {
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
                    variant:            representative_variant.clone(),
                    instructions:       String::new(),
                    variant_example:    json!(null),
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
                if let Some(enum_data) = &mut child_path.enum_data {
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
            // When a child enum is processed with EnumContext::Child, it returns
            // a concrete example directly (not wrapped with enum_root_data)
            let child_example = child_paths
                .first()
                .map_or(json!(null), |p| p.example.clone());

            child_examples.insert(child_descriptor, child_example);

            // Collect ALL child paths for the final result
            all_child_paths.extend(child_paths);
        }
    }

    Ok((child_examples, all_child_paths))
}

/// Create `PathKind` objects for a signature
fn create_paths_for_signature(
    signature: &VariantSignature,
    ctx: &RecursionContext,
) -> Option<Vec<PathKind>> {
    use VariantSignature;

    match signature {
        VariantSignature::Unit => None, // Unit variants have no paths
        VariantSignature::Tuple(types) => Some(
            types
                .iter()
                .enumerate()
                .map(|(index, type_name)| PathKind::IndexedElement {
                    index,
                    type_name: type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                })
                .collect(),
        ),
        VariantSignature::Struct(fields) => Some(
            fields
                .iter()
                .map(|(field_name, type_name)| PathKind::StructField {
                    field_name:  field_name.clone(),
                    type_name:   type_name.clone(),
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
        if let Some(enum_data) = &mut child.enum_data
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

/// Build partial root examples for all unique variant chains in child paths
///
/// This function implements bottom-up building:
/// - At leaf enums: Build partial roots from scratch (nothing to wrap)
/// - At intermediate enums: Wrap child enums' already-built partial roots
/// - Each enum only does ONE level of wrapping
///
/// **Key insight**: Child paths contain FULL variant chains from root, but we only process
/// the portion relevant to this enum. We use `ctx.variant_chain.len()` to determine which
/// variant in the chain belongs to us.
///
/// Keys are FULL variant chains (e.g., `[WithMiddleStruct, VariantB]`) with NO stripping.
/// Uses `BTreeMap` for deterministic ordering in tests.
///
/// Returns an error if building fails, which indicates a bug in the algorithm.
fn build_partial_root_examples(
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Result<BTreeMap<Vec<VariantName>, Value>> {
    let mut partial_roots = BTreeMap::new();

    // Extract all unique FULL variant chains from child paths
    let unique_full_chains: HashSet<Vec<VariantName>> = child_paths
        .iter()
        .filter_map(|p| {
            p.enum_data
                .as_ref()
                .filter(|ed| !ed.variant_chain.is_empty())
                .map(|ed| extract_variant_names(&ed.variant_chain))
        })
        .collect();

    // For each unique FULL chain, build the partial root from this enum down
    let ancestor_len = ctx.variant_chain.len();
    for full_chain in unique_full_chains {
        // Skip chains that don't extend beyond ancestors (defensive check)
        if full_chain.len() <= ancestor_len {
            continue;
        }

        // Propagate errors - if building fails, the entire operation fails
        let root_example =
            build_partial_root_for_chain(&full_chain, enum_examples, child_paths, ctx)?;

        // Store using the FULL chain as key (no stripping)
        // This allows parent enums to look up by full chains
        partial_roots.insert(full_chain, root_example);
    }

    Ok(partial_roots)
}

/// Build a partial root example for a specific variant chain
///
/// **Important**: The `chain` parameter is the FULL chain from root. We use
/// `ctx.variant_chain.len()` to determine which variant in the chain belongs to
/// this enum (the variant at index `ancestor_len`).
///
/// Returns an error if partial roots are missing, which indicates a bug in the building algorithm.
fn build_partial_root_for_chain(
    chain: &[VariantName],
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Result<Value> {
    use error_stack::Report;

    // Determine which variant in the full chain belongs to this enum
    // Example: At BottomEnum with ctx.variant_chain=[WithMiddleStruct],
    //          full_chain=[WithMiddleStruct, VariantB] → our_variant=VariantB (index 1)
    let ancestor_len = ctx.variant_chain.len();
    let our_variant = chain.get(ancestor_len).ok_or_else(|| {
        Report::new(Error::InvalidState(format!(
            "Chain {chain:?} too short for ancestor depth {ancestor_len}"
        )))
    })?;

    // Find the example for this variant from our enum_examples
    let base_example = enum_examples
        .iter()
        .find(|ex| ex.applicable_variants.contains(our_variant))
        .map(|ex| ex.example.clone())
        .ok_or_else(|| {
            Report::new(Error::InvalidState(format!(
                "No example found for variant {our_variant:?} in enum {}",
                ctx.type_name()
            )))
        })?;

    // If chain has more levels (nested enums), wrap the child's partial root
    if chain.len() > ancestor_len + 1 {
        // Find child enum root path that has partial roots
        for child in child_paths {
            // Look for enum root paths with partial_root_examples
            if let Some(child_partial_roots) = &child.partial_root_examples {
                // Check if child has a partial root for the FULL chain
                // Children store their partial roots with FULL chains as keys
                if let Some(nested_partial_root) = child_partial_roots.get(chain) {
                    // Wrap the nested partial root into our base example
                    // This is ONE level of wrapping
                    if let Some(wrapped) =
                        wrap_nested_example(&base_example, nested_partial_root, child)
                    {
                        return Ok(wrapped);
                    }
                    // If wrapping failed, continue searching other children
                }
            }
        }

        // If we reach here, no child had the required partial root
        // This is an InvalidState - the child should have built partial roots during its recursion
        Err(Report::new(Error::InvalidState(format!(
            "Missing partial root for variant chain {chain:?}. \
             Bottom-up building failed for enum {} - child enum did not build required partial roots. \
             This indicates a bug in the building algorithm.",
            ctx.type_name()
        ))))
    } else {
        // Chain ends at this level - no more nesting, just return our example
        Ok(base_example)
    }
}

/// Wrap a nested partial root into a parent example at the correct field
///
/// This function handles the complex task of inserting a child's partial root example
/// into the correct nested location within a parent's example structure.
///
/// **Example:** For parent `{"WithMiddleStruct": {"middle_struct": {"nested_enum": {...}}}}`
/// and child path `".middle_struct.nested_enum"`, this will navigate to the `middle_struct`
/// object and replace its `nested_enum` field.
///
/// Returns None if wrapping fails (invalid structure or unsupported path kind).
fn wrap_nested_example(
    parent_example: &Value,
    nested_partial_root: &Value,
    child_path: &MutationPathInternal,
) -> Option<Value> {
    // Extract the field name from the child path's PathKind
    let field_name = match &child_path.path_kind {
        PathKind::StructField { field_name, .. } => field_name.as_str(),
        PathKind::RootValue { .. } => {
            tracing::debug!("Cannot wrap into RootValue path - no field name available");
            return None;
        }
        PathKind::IndexedElement { .. } | PathKind::ArrayElement { .. } => {
            tracing::warn!("Wrapping into indexed/array paths not currently supported");
            return None;
        }
    };

    // Step 1: Unwrap the variant wrapper from parent example
    // Parent structure: {"VariantName": <variant_content>}
    let parent_obj = parent_example.as_object()?;
    let (variant_name, variant_content) = parent_obj.iter().next()?;

    // Step 2: Parse the child's full mutation path into navigation segments
    // Example: ".middle_struct.nested_enum" -> ["middle_struct", "nested_enum"]
    let path_str = child_path.full_mutation_path.trim_start_matches('.');
    if path_str.is_empty() {
        tracing::warn!("Empty mutation path in wrap_nested_example");
        return None;
    }

    let segments: Vec<&str> = path_str.split('.').collect();
    let field_to_replace = segments.last()?;
    let path_to_parent = &segments[..segments.len() - 1];

    // Verify field name matches
    if *field_to_replace != field_name {
        tracing::warn!(
            "Field name mismatch: PathKind has '{field_name}' but path has '{field_to_replace}'"
        );
        return None;
    }

    // Step 3: Navigate through the JSON tree and replace the target field
    let new_content = navigate_and_replace(
        variant_content,
        path_to_parent,
        field_name,
        nested_partial_root,
    )?;

    // Step 4: Re-wrap with the variant name
    Some(json!({ variant_name: new_content }))
}

/// Recursively navigate through a JSON structure and replace a field at the target location
///
/// **Parameters:**
/// - `current`: The current JSON value being traversed
/// - `path`: Remaining path segments to navigate (e.g., `["middle_struct"]`)
/// - `field`: The field name to replace at the target location (e.g., `"nested_enum"`)
/// - `new_value`: The value to insert at the target field
///
/// **Returns:** A new JSON value with the field replaced, or None if navigation fails
fn navigate_and_replace(
    current: &Value,
    path: &[&str],
    field: &str,
    new_value: &Value,
) -> Option<Value> {
    let current_obj = current.as_object()?;

    if path.is_empty() {
        // We've reached the target parent - replace the field here
        let mut result = current_obj.clone();
        result.insert(field.to_string(), new_value.clone());
        return Some(Value::Object(result));
    }

    // Navigate deeper - get next segment and recurse
    let next_key = path[0];
    let next_val = current_obj.get(next_key)?;

    let replaced = navigate_and_replace(next_val, &path[1..], field, new_value)?;

    // Clone current object and update the navigated path
    let mut result = current_obj.clone();
    result.insert(next_key.to_string(), replaced);
    Some(Value::Object(result))
}

/// Populate `root_example` on all paths (root level only)
fn populate_root_example(
    paths: &mut [MutationPathInternal],
    partial_roots: &BTreeMap<Vec<VariantName>, Value>,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_data
            && !enum_data.variant_chain.is_empty()
        {
            let chain = extract_variant_names(&enum_data.variant_chain);
            if let Some(root_example) = partial_roots.get(&chain) {
                enum_data.root_example = Some(root_example.clone());
            } else {
                tracing::debug!("No root example found for variant chain: {chain:?}");
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
) -> Result<Vec<MutationPathInternal>> {
    // Generate enum data only if we have a variant chain (nested in another enum)
    let enum_data = if ctx.variant_chain.is_empty() {
        None
    } else {
        Some(EnumPathData {
            variant_chain:       populate_variant_path(ctx, &enum_examples, &default_example),
            applicable_variants: Vec::new(),
            root_example:        None,
        })
    };

    // Direct field assignment - enums ALWAYS generate examples arrays
    let mut root_mutation_path = MutationPathInternal {
        full_mutation_path: ctx.full_mutation_path.clone(),
        example: json!(null), /* Enums always use null for the example field - they use
                               * Vec<ExampleGroup> */
        enum_example_groups: Some(enum_examples.clone()),
        enum_example_for_parent: Some(default_example.clone()),
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutation_status: MutationStatus::Mutable, // Simplified for now
        mutation_status_reason: None,
        enum_data,
        partial_root_examples: None,
    };

    // ==================== NEW CODE ====================
    // Build partial root examples for all unique variant chains in children
    // This happens at EVERY enum root path (paths where enum_example_groups exists)
    // - For path "" (TestVariantChainEnum): builds roots for all descendants
    // - For path ".middle_struct.nested_enum" (BottomEnum): builds roots for its children
    //
    // Returns an error if building fails (InvalidState - indicates algorithm bug)
    let partial_roots = build_partial_root_examples(&enum_examples, &child_paths, ctx)?;

    // Store partial roots on this enum's root path so parent enums can access them
    // Parent finds these by searching child_paths for paths with partial_root_examples.is_some()
    root_mutation_path.partial_root_examples = Some(partial_roots.clone());

    // If we're at the actual root level (empty variant chain),
    // populate root_example on all paths
    if ctx.variant_chain.is_empty() {
        populate_root_example(&mut child_paths, &partial_roots);
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
    Ok(result)
}
