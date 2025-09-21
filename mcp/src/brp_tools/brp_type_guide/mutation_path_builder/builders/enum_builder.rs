//! `PathBuilder` for Enum types using the new protocol

use std::collections::HashMap;

use error_stack::Report;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::path_builder::{MaybeVariants, PathBuilder};
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::{EnumContext, RecursionContext};
use super::super::types::{ExampleGroup, VariantSignature};
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

/// Represents a path with associated variant information
/// Used by the enum builder to track which variants a path applies to
#[derive(Debug, Clone)]
pub struct PathKindWithVariants {
    /// The path kind (None for unit variants)
    pub path:                Option<PathKind>,
    /// Variants this path applies to
    pub applicable_variants: Vec<String>,
}

impl MaybeVariants for PathKindWithVariants {
    fn applicable_variants(&self) -> Option<&[String]> {
        Some(&self.applicable_variants)
    }

    fn into_path_kind(self) -> Option<PathKind> {
        self.path
    }
}

/// Builder for enum mutation paths using the new protocol
pub struct EnumMutationBuilder;

// ============================================================================
// Helper Types and Functions (preserved from original)
// ============================================================================

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

impl EnumVariantInfo {
    fn name(&self) -> &str {
        match self {
            Self::Unit(name) | Self::Tuple(name, _) | Self::Struct(name, _) => name,
        }
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
            return Some(Self::Unit(variant_str.to_string()));
        }

        let variant_name = extract_variant_name(v)?;

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
// MutationPathBuilder Implementation
// ============================================================================

impl PathBuilder for EnumMutationBuilder {
    type Item = PathKindWithVariants;
    type Iter<'a>
        = std::vec::IntoIter<PathKindWithVariants>
    where
        Self: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;

        // Extract all variants from schema
        let variants = extract_enum_variants(schema, &ctx.registry);

        // Group variants by their signature (already handles deduplication)
        let variant_groups = group_variants_by_signature(variants);

        let mut children = Vec::new();

        // Create PathKindWithVariants for each signature group
        for (signature, variants_in_group) in variant_groups {
            let applicable_variants: Vec<String> = variants_in_group
                .iter()
                .map(|v| ctx.type_name().variant_name(v.name()))
                .collect();

            match signature {
                VariantSignature::Unit => {
                    // Unit variants have no path (no fields to mutate)
                    children.push(PathKindWithVariants {
                        path: None,
                        applicable_variants,
                    });
                }
                VariantSignature::Tuple(types) => {
                    // Create PathKindWithVariants for each tuple element
                    for (index, type_name) in types.iter().enumerate() {
                        children.push(PathKindWithVariants {
                            path:                Some(PathKind::IndexedElement {
                                index,
                                type_name: type_name.clone(),
                                parent_type: ctx.type_name().clone(),
                            }),
                            applicable_variants: applicable_variants.clone(),
                        });
                    }
                }
                VariantSignature::Struct(fields) => {
                    // Create PathKindWithVariants for each struct field
                    for (field_name, type_name) in fields {
                        children.push(PathKindWithVariants {
                            path:                Some(PathKind::StructField {
                                field_name:  field_name.clone(),
                                type_name:   type_name.clone(),
                                parent_type: ctx.type_name().clone(),
                            }),
                            applicable_variants: applicable_variants.clone(),
                        });
                    }
                }
            }
        }

        Ok(children.into_iter())
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> Result<Value> {
        let schema = ctx.require_registry_schema()?;
        let all_variants = extract_enum_variants(schema, &ctx.registry);
        let variant_groups = group_variants_by_signature(all_variants);

        // Build internal MutationExample to organize the enum logic
        tracing::debug!(
            "NewEnumBuilder for {} with enum_context: {:?}",
            ctx.type_name(),
            ctx.enum_context
        );
        let mutation_example = match &ctx.enum_context {
            Some(EnumContext::Root) => {
                // Build examples array for enum root path
                let mut examples = Vec::new();

                for (signature, variants_in_group) in &variant_groups {
                    let representative = variants_in_group.first().ok_or_else(|| {
                        Report::new(Error::InvalidState("Empty variant group".to_string()))
                    })?;

                    let example = Self::build_variant_example(
                        signature,
                        representative.name(),
                        &children,
                        ctx.type_name(),
                    );

                    let applicable_variants: Vec<String> = variants_in_group
                        .iter()
                        .map(|v| ctx.type_name().variant_name(v.name()))
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
                let example = Self::concrete_example(&variant_groups, &children, ctx.type_name());
                MutationExample::Simple(example)
            }

            None => {
                // Parent is not an enum - return a concrete example
                let example = Self::concrete_example(&variant_groups, &children, ctx.type_name());
                MutationExample::Simple(example)
            }
        };

        // Convert MutationExample to Value for MutationPathBuilder to process
        match mutation_example {
            MutationExample::Simple(val) => {
                tracing::debug!(
                    "NewEnumBuilder {} returning Simple value: {}",
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
                    "NewEnumBuilder returning EnumRoot with {} examples: {}",
                    examples.len(),
                    result
                );

                Ok(result)
            }
        }
    }
}
