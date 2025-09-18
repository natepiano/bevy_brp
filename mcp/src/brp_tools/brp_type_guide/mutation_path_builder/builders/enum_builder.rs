//! Migrated Builder for Enum types using the new protocol
//!
//! This is the migrated version of EnumMutationBuilder that uses the new
//! protocol-driven pattern with ProtocolEnforcer handling all the common
//! protocol concerns while this builder focuses only on enum-specific logic.

use std::collections::HashMap;

use error_stack::Report;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::path_builder::{MaybeVariants, PathBuilder};
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::{EnumContext, RecursionContext};
use super::super::types::{ExampleGroup, VariantSignature};
use crate::brp_tools::brp_type_guide::constants::VARIANT_PATH_SEPARATOR;
use crate::brp_tools::brp_type_guide::response_types::BrpTypeName;
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
    /// Each group has applicable_variants, signature, and example
    EnumRoot(Vec<ExampleGroup>),

    /// Example with variant context (for enum child paths like .0, .1, .enabled)
    EnumChild {
        example: Value,
        applicable_variants: Vec<String>,
    },
}

/// Represents a path with associated variant information
/// Used by the enum builder to track which variants a path applies to
#[derive(Debug, Clone)]
pub struct PathKindWithVariants {
    /// The path kind (None for unit variants)
    pub path: Option<PathKind>,
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
    pub type_name: BrpTypeName,
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
    pub fn from_schema_variant(
        v: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Option<EnumVariantInfo> {
        let variant_name = extract_variant_name(v)?;

        // Check what type of variant this is
        if let Some(prefix_items) = v.get_field(SchemaField::PrefixItems) {
            // Tuple variant
            if let Some(prefix_array) = prefix_items.as_array() {
                let tuple_types = extract_tuple_types(prefix_array, registry);
                return Some(EnumVariantInfo::Tuple(variant_name, tuple_types));
            }
        } else if let Some(properties) = v.get_field(SchemaField::Properties) {
            // Struct variant
            if let Some(props_map) = properties.as_object() {
                let struct_fields = extract_struct_fields(props_map, registry);
                if !struct_fields.is_empty() {
                    return Some(EnumVariantInfo::Struct(variant_name, struct_fields));
                }
            }
        }

        // Unit variant (no fields)
        Some(EnumVariantInfo::Unit(variant_name))
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
    registry_schema
        .get_field(SchemaField::OneOf)
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
    /// Build a complete example for a variant with all its fields
    fn build_variant_example(
        &self,
        signature: &VariantSignature,
        variant_name: &str,
        children: &HashMap<MutationPathDescriptor, Value>,
    ) -> Value {
        match signature {
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
                json!({ variant_name: tuple_values })
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
        }
    }

    /// Create a concrete example value for embedding in a parent structure
    fn concrete_example(
        &self,
        variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>,
        children: &HashMap<MutationPathDescriptor, Value>,
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
                let representative = variants.first().unwrap();
                self.build_variant_example(sig, representative.name(), children)
            })
            .unwrap_or(json!(null))
    }

    /// Flatten variant chain into dot notation for nested enums
    fn flatten_variant_chain(variant_chain: &[(BrpTypeName, Vec<String>)]) -> Vec<String> {
        // e.g., [(TestEnum, ["Nested"]), (NestedEnum, ["Conditional"])] → ["Nested.Conditional"]
        if variant_chain.is_empty() {
            return vec![];
        }

        // Only return the variants from the last level in the chain
        if let Some((_, last_variants)) = variant_chain.last() {
            let prefix_parts: Vec<String> = variant_chain
                .iter()
                .take(variant_chain.len() - 1)
                .filter_map(|(_, v)| v.first().cloned())
                .collect();

            if prefix_parts.is_empty() {
                last_variants.clone()
            } else {
                last_variants
                    .iter()
                    .map(|v| {
                        let mut full_path = prefix_parts.clone();
                        full_path.push(v.clone());
                        full_path.join(VARIANT_PATH_SEPARATOR)
                    })
                    .collect()
            }
        } else {
            vec![]
        }
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
                .map(|v| v.name().to_string())
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
                            path: Some(PathKind::IndexedElement {
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
                            path: Some(PathKind::StructField {
                                field_name: field_name.clone(),
                                type_name: type_name.clone(),
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

                    let example =
                        self.build_variant_example(signature, representative.name(), &children);

                    let applicable_variants: Vec<String> = variants_in_group
                        .iter()
                        .map(|v| v.name().to_string())
                        .collect();

                    examples.push(ExampleGroup {
                        applicable_variants,
                        signature: signature.to_string(),
                        example,
                    });
                }

                MutationExample::EnumRoot(examples)
            }

            Some(EnumContext::Child { variant_chain }) => {
                // Building under another enum - return EnumChild
                let example = self.concrete_example(&variant_groups, &children);
                let applicable_variants = Self::flatten_variant_chain(variant_chain);

                MutationExample::EnumChild {
                    example,
                    applicable_variants,
                }
            }

            None => {
                // Parent is not an enum - return a concrete example
                let example = self.concrete_example(&variant_groups, &children);
                tracing::debug!(
                    "NewEnumBuilder {} with None context returning Simple: {}",
                    ctx.type_name(),
                    example
                );
                MutationExample::Simple(example)
            }
        };

        // Convert MutationExample to Value for ProtocolEnforcer to process
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
                // ProtocolEnforcer will extract these to populate MutationPathInternal fields
                let default_example = examples
                    .first()
                    .map(|g| g.example.clone())
                    .unwrap_or(json!(null));

                let result = json!({
                    "enum_root_data": {
                        "examples": examples,
                        "default": default_example
                    }
                });

                tracing::debug!(
                    "NewEnumBuilder returning EnumRoot with {} examples: {}",
                    examples.len(),
                    result
                );

                Ok(result)
            }
            MutationExample::EnumChild {
                example,
                applicable_variants,
            } => {
                // For enum children, wrap with applicable_variants info
                Ok(json!({
                    "value": example,
                    "applicable_variants": applicable_variants
                }))
            }
        }
    }
}
