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
use super::super::recursion_context::RecursionContext;
use super::super::types::VariantSignature;
use crate::brp_tools::brp_type_guide::response_types::BrpTypeName;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

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

        // Group all variants by their signature
        let variant_groups = group_variants_by_signature(all_variants);

        // Build one example per signature group
        let mut examples = Vec::new();

        for (signature, variants_in_group) in variant_groups {
            // Use first variant in group as representative for the example
            let representative = variants_in_group.first().ok_or_else(|| {
                Report::new(Error::InvalidState("Empty variant group".to_string()))
            })?;

            let example = match &signature {
                VariantSignature::Unit => {
                    // Unit variants: just use the variant name
                    json!(representative.name())
                }
                VariantSignature::Tuple(types) => {
                    // Tuple variants: assemble from indexed children
                    let mut tuple_values = Vec::new();
                    for index in 0..types.len() {
                        let descriptor = MutationPathDescriptor::from(index.to_string());
                        let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                        tuple_values.push(value);
                    }
                    // Format: {"VariantName": [val1, val2]}
                    json!({ representative.name(): tuple_values })
                }
                VariantSignature::Struct(field_types) => {
                    // Struct variants: assemble from field children
                    let mut field_values = serde_json::Map::new();
                    for (field_name, _) in field_types {
                        let descriptor = MutationPathDescriptor::from(field_name.clone());
                        let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                        field_values.insert(field_name.clone(), value);
                    }
                    // Format: {"VariantName": {field1: val1, field2: val2}}
                    json!({ representative.name(): field_values })
                }
            };

            // Collect all variant names that share this signature
            let applicable_variants: Vec<String> = variants_in_group
                .iter()
                .map(|v| v.name().to_string())
                .collect();

            // Create the signature example with all applicable variants
            examples.push(json!({
                "applicable_variants": applicable_variants,
                "example": example,
                "signature": signature.to_string(),
            }));
        }

        // Return root enum example structure
        // Note: When used as the root path, we'll just return the first example's value
        // The "examples" array structure is for the mutation_paths output format
        if examples.len() == 1 && examples[0].get("applicable_variants").is_some() {
            // For single-signature enums, return just the example value
            Ok(examples[0].get("example").cloned().unwrap_or(json!(null)))
        } else {
            // For multi-signature enums, pick the first unit variant if available,
            // otherwise the first example
            let unit_example = examples
                .iter()
                .find(|e| {
                    e.get("signature")
                        .and_then(|s| s.as_str())
                        .map_or(false, |s| s == "unit")
                })
                .or_else(|| examples.first());

            Ok(unit_example
                .and_then(|e| e.get("example"))
                .cloned()
                .unwrap_or(json!(null)))
        }
    }
}
