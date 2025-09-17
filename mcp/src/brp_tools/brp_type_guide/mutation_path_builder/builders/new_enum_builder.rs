//! Migrated Builder for Enum types using the new protocol
//!
//! This is the migrated version of EnumMutationBuilder that uses the new
//! protocol-driven pattern with ProtocolEnforcer handling all the common
//! protocol concerns while this builder focuses only on enum-specific logic.

use std::collections::{HashMap, HashSet};

use error_stack::Report;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::MutationPathBuilder;
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::{EnumContext, RecursionContext};
use super::super::types::{ExampleGroup, VariantSignature};
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
        example:             Value,
        applicable_variants: Vec<String>,
    },
}

/// Builder for enum mutation paths using the new protocol
pub struct NewEnumMutationBuilder;

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

fn deduplicate_variant_signatures(variants: Vec<EnumVariantInfo>) -> Vec<EnumVariantInfo> {
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

impl NewEnumMutationBuilder {
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

    /// Separator used for flattening nested enum variant chains into dot notation
    const VARIANT_PATH_SEPARATOR: &str = ".";

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
                        full_path.join(Self::VARIANT_PATH_SEPARATOR)
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

impl MutationPathBuilder for NewEnumMutationBuilder {
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let schema = ctx.require_registry_schema()?;

        // Use existing variant processing logic
        let variants = extract_enum_variants(schema, &ctx.registry);
        tracing::debug!(
            "Found {} total variants for {}",
            variants.len(),
            ctx.type_name()
        );

        let unique_variants = deduplicate_variant_signatures(variants);
        tracing::debug!(
            "After deduplication: {} unique signatures for {}",
            unique_variants.len(),
            ctx.type_name()
        );

        let mut children = Vec::new();

        for variant in unique_variants {
            match variant {
                EnumVariantInfo::Unit(name) => {
                    tracing::debug!("Unit variant: {}", name);
                    // Unit variants have no children
                }
                EnumVariantInfo::Tuple(name, types) => {
                    tracing::debug!("Tuple variant: {} with {} types", name, types.len());
                    // Create standard IndexedElement for each tuple element
                    // Results in paths like ".0", ".1" (flat, no variant prefix)
                    for (index, type_name) in types.iter().enumerate() {
                        tracing::debug!("Adding IndexedElement .{} for type {}", index, type_name);
                        children.push(PathKind::IndexedElement {
                            index,
                            type_name: type_name.clone(),
                            parent_type: ctx.type_name().clone(),
                        });
                    }
                }
                EnumVariantInfo::Struct(name, fields) => {
                    tracing::debug!("Struct variant: {} with {} fields", name, fields.len());
                    // Create standard StructField for each struct field
                    // Results in paths like ".enabled", ".name" (flat, no variant prefix)
                    for field in fields {
                        tracing::debug!(
                            "Adding StructField .{} for type {}",
                            field.field_name,
                            field.type_name
                        );
                        children.push(PathKind::StructField {
                            field_name:  field.field_name.clone(),
                            type_name:   field.type_name.clone(),
                            parent_type: ctx.type_name().clone(),
                        });
                    }
                }
            }
        }

        tracing::debug!(
            "collect_children returning {} total children for {}",
            children.len(),
            ctx.type_name()
        );
        Ok(children)
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
                        signature: signature.clone(),
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
                MutationExample::Simple(example)
            }
        };

        // Convert MutationExample to Value for ProtocolEnforcer to process
        match mutation_example {
            MutationExample::Simple(val) => Ok(val),
            MutationExample::EnumRoot(examples) => {
                // For enum roots, return both examples array and a default concrete value
                // ProtocolEnforcer will extract these to populate MutationPathInternal fields
                let default_example = examples
                    .first()
                    .map(|g| g.example.clone())
                    .unwrap_or(json!(null));

                Ok(json!({
                    "enum_root_data": {
                        "examples": examples,
                        "default": default_example
                    }
                }))
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
