//! Shared enum processing functions for both EnumPathBuilder and legacy code

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::path_kind::MutationPathDescriptor;
use super::super::recursion_context::{EnumContext, RecursionContext};
use super::super::types::{ExampleGroup, VariantName, VariantSignature};
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Internal example data for enum mutation paths - used only within enum builder
/// Other builders continue using Value directly
#[derive(Debug, Clone)]
enum MutationExample {
    /// Simple example value (for non-enum types and embedded enum values)
    Simple(Value),

    /// Multiple examples with signatures (for enum root paths)
    /// Each group has `applicable_variants`, `signature`, and `example`
    EnumRoot(Vec<ExampleGroup>),
}

// PathKindWithVariants and MaybeVariants removed - no longer needed with EnumPathBuilder

/// Builder for enum mutation paths using the new protocol
pub struct EnumMutationBuilder;

// ============================================================================
// Helper Types and Functions (preserved from original)
// ============================================================================

/// Type-safe enum variant information - replaces `EnumVariantInfoOld`
/// This enum makes invalid states impossible to construct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnumVariantInfo {
    /// Unit variant - qualified variant name (e.g., "Color::Srgba")
    Unit(VariantName),
    /// Tuple variant - qualified name and guaranteed tuple types
    Tuple(VariantName, Vec<BrpTypeName>),
    /// Struct variant - qualified name and guaranteed struct fields
    Struct(VariantName, Vec<EnumFieldInfo>),
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

impl EnumVariantInfo {
    /// Get the fully qualified variant name (e.g., "Color::Srgba")
    pub fn variant_name(&self) -> &VariantName {
        match self {
            Self::Unit(name) | Self::Tuple(name, _) | Self::Struct(name, _) => name,
        }
    }

    /// Get just the variant name without the enum prefix (e.g., "Srgba" from "Color::Srgba")
    fn short_name(&self) -> &str {
        self.variant_name()
            .as_str()
            .rsplit_once("::")
            .map(|(_, name)| name)
            .unwrap_or(self.variant_name().as_str())
    }

    /// Compatibility method - delegates to short_name
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
    pub fn from_schema_variant(v: &Value, registry: &HashMap<BrpTypeName, Value>) -> Option<Self> {
        // Handle Unit variants which show up as simple strings
        if let Some(variant_str) = v.as_str() {
            // For simple string variants, create a VariantName from just the string
            return Some(Self::Unit(VariantName::from(variant_str.to_string())));
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

// Helper functions for variant processing
fn extract_variant_name(v: &Value) -> Option<String> {
    v.get_field(SchemaField::ShortPath)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// Extract the fully qualified variant name from schema (e.g., "Color::Srgba")
fn extract_variant_qualified_name(v: &Value) -> Option<VariantName> {
    // First try to get the type path for the full qualified name
    if let Some(type_path) = v.get_field(SchemaField::TypePath).and_then(Value::as_str) {
        // Find second-to-last :: to extract "EnumType::Variant"
        // e.g., "bevy_color::color::Color::Srgba" -> "Color::Srgba"
        let parts: Vec<&str> = type_path.rsplitn(3, "::").collect();
        if parts.len() >= 3 {
            return Some(VariantName::from(format!("{}::{}", parts[1], parts[0])));
        }
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
                field_name: field_name.clone(),
                type_name,
            })
        })
        .collect()
}

fn extract_enum_variants(
    registry_schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
) -> Vec<EnumVariantInfo> {
    let one_of_field = registry_schema.get_field(SchemaField::OneOf);

    one_of_field
        .and_then(Value::as_array)
        .map(|variants| {
            variants
                .iter()
                .filter_map(|v| EnumVariantInfo::from_schema_variant(v, registry))
                .collect()
        })
        .unwrap_or_default()
}

fn group_variants_by_signature(
    variants: Vec<EnumVariantInfo>,
) -> HashMap<VariantSignature, Vec<EnumVariantInfo>> {
    let mut groups = HashMap::new();
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
// NewEnumMutationBuilder Helper Methods
// ============================================================================

impl EnumMutationBuilder {
    /// Check if a type is Option<T>
    fn is_option_type(type_name: &BrpTypeName) -> bool {
        type_name.as_str().starts_with("core::option::Option<")
    }

    /// Apply Option<T> transformation if needed: {"Some": value} → value, "None" → null
    fn apply_option_transformation(
        example: Value,
        variant_name: &str,
        enum_type: &BrpTypeName,
    ) -> Value {
        // Only transform if this is actually core::option::Option<T>
        if !Self::is_option_type(enum_type) {
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
                    let descriptor = MutationPathDescriptor::from(field_name.clone());
                    let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                    field_values.insert(field_name.clone(), value);
                }
                json!({ variant_name: field_values })
            }
        };

        // Apply Option<T> transformation only for actual Option types
        Self::apply_option_transformation(example, variant_name, enum_type)
    }

    /// Create a concrete example value for embedding in a parent structure
    fn concrete_example(
        variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>,
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
                    Self::build_variant_example(sig, representative.name(), children, enum_type)
                })
            })
            .unwrap_or(json!(null))
    }
}

// ============================================================================
// Shared Enum Processing Functions
// ============================================================================

/// Extract all variants from schema and group them by signature
/// This is the single source of truth for enum variant processing
pub fn extract_and_group_variants(
    ctx: &RecursionContext,
) -> Result<HashMap<VariantSignature, Vec<EnumVariantInfo>>> {
    let schema = ctx.require_registry_schema()?;
    let variants = extract_enum_variants(schema, &ctx.registry);
    Ok(group_variants_by_signature(variants))
}

/// Build enum examples from variant groups and child examples
/// This handles all enum context logic in one place
pub fn build_enum_examples(
    variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>,
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

                let example = EnumMutationBuilder::build_variant_example(
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

            MutationExample::EnumRoot(examples)
        }

        Some(EnumContext::Child) => {
            // Building under another enum - return Simple example
            let example = EnumMutationBuilder::concrete_example(
                variant_groups,
                &child_examples,
                ctx.type_name(),
            );
            MutationExample::Simple(example)
        }

        None => {
            // Parent is not an enum - return a concrete example
            let example = EnumMutationBuilder::concrete_example(
                variant_groups,
                &child_examples,
                ctx.type_name(),
            );
            MutationExample::Simple(example)
        }
    };

    // Convert MutationExample to Value for MutationPathBuilder to process
    match mutation_example {
        MutationExample::Simple(val) => {
            tracing::debug!(
                "build_enum_examples {} returning Simple value: {}",
                ctx.type_name(),
                val
            );
            Ok(val)
        }
        MutationExample::EnumRoot(examples) => {
            // For enum roots, return both examples array and a default concrete value
            // MutationPathBuilder will extract these to populate MutationPathInternal fields
            let default_example = examples
                .first()
                .map(|g| g.example.clone())
                .unwrap_or(json!(null));

            let result = json!({
                "enum_root_data": {
                    "enum_root_examples": examples,
                    "enum_root_example_for_parent": default_example
                }
            });

            tracing::debug!(
                "build_enum_examples returning EnumRoot with {} examples: {}",
                examples.len(),
                result
            );

            Ok(result)
        }
    }
}

// ============================================================================
// EnumMutationBuilder is now deprecated - use EnumPathBuilder instead
// ============================================================================

// PathBuilder implementation removed - enums now use EnumPathBuilder directly
