//! Standalone enum path builder - no `PathBuilder` dependency

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::constants::RecursionDepth;
use super::builder::recurse_mutation_paths;
use super::path_kind::MutationPathDescriptor;
use super::recursion_context::{EnumContext, RecursionContext};
use super::types::{ExampleGroup, StructFieldName, VariantName, VariantSignature};
use super::{MutationPathInternal, MutationStatus, PathAction, PathKind, TypeKind, VariantPath};
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

// ============================================================================
// Types moved from enum_builder.rs
// ============================================================================

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
/// Uses the same shared functions as `EnumMutationBuilder` for identical output
pub fn process_enum(
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<Vec<MutationPathInternal>> {
    tracing::debug!("EnumPathBuilder processing type: {}", ctx.type_name());

    // Use shared function to get variant information - same as EnumMutationBuilder
    let variant_groups = extract_and_group_variants(ctx)?;

    // Process children and collect BOTH examples AND child paths
    let (child_examples, child_paths) = process_children(&variant_groups, ctx, depth)?;

    // Use shared function to build examples - same as EnumMutationBuilder
    let assembled_value = build_enum_examples(&variant_groups, child_examples, ctx)?;

    // Create result paths including both root AND child paths
    Ok(create_result_paths(ctx, assembled_value, child_paths))
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
        .filter_map(SchemaField::extract_field_type)
        .collect()
}

fn extract_struct_fields(
    properties: &serde_json::Map<String, Value>,
    _registry: &HashMap<BrpTypeName, Value>,
) -> Vec<EnumFieldInfo> {
    properties
        .iter()
        .filter_map(|(field_name, field_schema)| {
            SchemaField::extract_field_type(field_schema).map(|type_name| EnumFieldInfo {
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

// ============================================================================
// Static methods moved from EnumMutationBuilder
// ============================================================================

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

/// Create a concrete example value for embedding in a parent structure
fn concrete_example(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {
    // Pick first unit variant if available, otherwise first example
    let unit_variant = variant_groups
        .iter()
        .find(|(sig, _)| matches!(sig, VariantSignature::Unit))
        .and_then(|(_, variants)| variants.first());

    if let Some(variant) = unit_variant {
        return json!(variant.name());
    }

    // Fall back to first available example with full structure
    variant_groups
        .iter()
        .next()
        .map(|(sig, variants)| {
            variants.first().map_or(json!(null), |representative| {
                build_variant_example(sig, representative.name(), children, enum_type)
            })
        })
        .unwrap_or(json!(null))
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
) -> Result<Value> {
    use error_stack::Report;

    // Build internal MutationExample to organize the enum logic
    tracing::debug!(
        "build_enum_examples for {} with enum_context: {:?}",
        ctx.type_name(),
        ctx.enum_context
    );

    let mutation_example = match &ctx.enum_context {
        Some(EnumContext::Root) => {
            // Build examples array for enum root path
            let mut examples = Vec::new();

            for (signature, variants_in_group) in variant_groups {
                let representative = variants_in_group.first().ok_or_else(|| {
                    Report::new(Error::InvalidState("Empty variant group".to_string()))
                })?;

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

            examples
        }

        Some(EnumContext::Child) => {
            // Building under another enum - return Simple example
            let example = concrete_example(variant_groups, &child_examples, ctx.type_name());
            return Ok(example);
        }

        None => {
            // Parent is not an enum - return a concrete example
            let example = concrete_example(variant_groups, &child_examples, ctx.type_name());
            return Ok(example);
        }
    };

    // For enum roots, return both examples array and a default concrete value
    // MutationPathBuilder will extract these to populate MutationPathInternal fields
    let default_example = mutation_example
        .first()
        .map(|g| g.example.clone())
        .unwrap_or(json!(null));

    let result = json!({
        "enum_root_data": {
            "enum_root_examples": mutation_example,
            "enum_root_example_for_parent": default_example
        }
    });

    tracing::debug!(
        "build_enum_examples returning EnumRoot with {} examples: {}",
        mutation_example.len(),
        result
    );

    Ok(result)
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

    // Process each variant group (same logic as EnumMutationBuilder::collect_children)
    for (signature, variants_in_group) in variant_groups {
        let applicable_variants: Vec<VariantName> = variants_in_group
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        // Create paths for this signature group
        let paths = create_paths_for_signature(signature, &applicable_variants, ctx);

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
            child_ctx.enum_context = Some(EnumContext::Child);

            // Recursively process child and collect paths
            let child_descriptor = path.to_mutation_path_descriptor();
            let child_schema = child_ctx.require_registry_schema()?;
            let child_type_kind = TypeKind::from_schema(child_schema, child_ctx.type_name());

            // Use the same recursion function as MutationPathBuilder
            let child_paths =
                recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

            // Extract example from first path (same logic as MutationPathBuilder)
            let child_example = child_paths.first().map_or(json!(null), |p| {
                p.enum_root_example_for_parent
                    .as_ref()
                    .map_or_else(|| p.example.clone(), Clone::clone)
            });

            child_examples.insert(child_descriptor, child_example);

            // Collect ALL child paths for the final result
            all_child_paths.extend(child_paths);
        }
    }

    Ok((child_examples, all_child_paths))
}

/// Create `PathKind` objects for a signature - mirrors `EnumMutationBuilder` logic
fn create_paths_for_signature(
    signature: &VariantSignature,
    _applicable_variants: &[VariantName],
    ctx: &RecursionContext,
) -> Vec<Option<PathKind>> {
    use VariantSignature;

    match signature {
        VariantSignature::Unit => {
            vec![None] // Unit variants have no paths
        }
        VariantSignature::Tuple(types) => types
            .iter()
            .enumerate()
            .map(|(index, type_name)| {
                Some(PathKind::IndexedElement {
                    index,
                    type_name: type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                })
            })
            .collect(),
        VariantSignature::Struct(fields) => fields
            .iter()
            .map(|(field_name, type_name)| {
                Some(PathKind::StructField {
                    field_name:  field_name.clone(),
                    type_name:   type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                })
            })
            .collect(),
    }
}

/// Create final result paths - includes both root and child paths
fn create_result_paths(
    ctx: &RecursionContext,
    assembled_value: Value,
    child_paths: Vec<MutationPathInternal>,
) -> Vec<MutationPathInternal> {
    // Process assembled value for enum context (same logic as MutationPathBuilder)
    let (parent_example, enum_root_examples, enum_root_example_for_parent) =
        process_enum_context(ctx, assembled_value);

    // Create the main mutation path for this enum root
    let root_mutation_path = MutationPathInternal {
        full_mutation_path: ctx.full_mutation_path.clone(),
        example: parent_example,
        enum_root_examples,
        enum_root_example_for_parent,
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutation_status: MutationStatus::Mutable, // Simplified for now
        mutation_status_reason: None,
        enum_instructions: None, // Simplified for now
        enum_variant_path: ctx.variant_chain.clone(),
    };

    // Return root path plus all child paths (like MutationPathBuilder does)
    let mut result = vec![root_mutation_path];
    result.extend(child_paths);
    result
}

/// Process enum context - same logic as `MutationPathBuilder::process_enum_context`
fn process_enum_context(
    ctx: &RecursionContext,
    assembled_example: Value,
) -> (Value, Option<Vec<ExampleGroup>>, Option<Value>) {
    match &ctx.enum_context {
        Some(EnumContext::Root) => assembled_example
            .get("enum_root_data")
            .cloned()
            .map_or_else(
                || (assembled_example, None, None),
                |enum_data| {
                    let default_example = enum_data
                        .get("enum_root_example_for_parent")
                        .cloned()
                        .unwrap_or(json!(null));
                    let examples_json = enum_data
                        .get("enum_root_examples")
                        .cloned()
                        .unwrap_or(json!([]));
                    let examples: Vec<ExampleGroup> =
                        serde_json::from_value(examples_json).unwrap_or_default();

                    (json!(null), Some(examples), Some(default_example))
                },
            ),
        Some(EnumContext::Child) | None => (assembled_example, None, None),
    }
}
