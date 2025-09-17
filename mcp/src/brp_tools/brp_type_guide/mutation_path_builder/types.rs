//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.
use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use super::super::response_types::BrpTypeName;
use super::TypeKind;
use super::path_kind::PathKind;

/// Action to take regarding path creation during recursion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAction {
    /// Create mutation paths during recursion
    Create,
    /// Skip path creation during recursion
    Skip,
}

/// Status of whether a mutation path can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Path can be fully mutated
    Mutable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutable,
    /// Path is partially mutable (some elements mutable, others not)
    PartiallyMutable,
}

/// Variant signature types for enum variants - used for grouping similar structures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantSignature {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with ordered types
    Tuple(Vec<BrpTypeName>),
    /// Struct variant with named fields and types
    Struct(Vec<(String, BrpTypeName)>),
}

impl std::fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Tuple(types) => {
                let type_names: Vec<String> = types
                    .iter()
                    .map(|t| shorten_type_name(t.as_str()))
                    .collect();
                write!(f, "tuple({})", type_names.join(", "))
            }
            Self::Struct(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, type_name)| {
                        format!("{}: {}", name, shorten_type_name(type_name.as_str()))
                    })
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
}

/// Convert a fully-qualified type name to a short readable name
fn shorten_type_name(type_name: &str) -> String {
    match type_name {
        "alloc::string::String" => "String".to_string(),
        "core::option::Option" => "Option".to_string(),
        name if name.starts_with("alloc::string::String") => "String".to_string(),
        name if name.starts_with("core::option::Option<") => name
            .strip_prefix("core::option::Option<")
            .and_then(|s| s.strip_suffix('>'))
            .map_or_else(
                || "Option".to_string(),
                |inner| format!("Option<{}>", shorten_type_name(inner)),
            ),
        _ => {
            // For other types, just take the last segment after ::
            type_name
                .split("::")
                .last()
                .unwrap_or(type_name)
                .to_string()
        }
    }
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:                Value,
    /// For enum roots only: the examples array with all variant groups
    /// None for all other paths (including enum children and regular types)
    pub enum_root_examples:     Option<Vec<ExampleGroup>>,
    /// Path for mutation, e.g., ".translation.x"
    pub path:                   String,
    /// Type information for this path
    pub type_name:              BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:              PathKind,
    /// Status of whether this path can be mutated
    pub mutation_status:        MutationStatus,
    /// Reason if mutation is not possible
    pub mutation_status_reason: Option<Value>,
}

impl MutationPathInternal {
    /// Convert to summary for reason reporting
    pub fn to_path_summary(&self) -> PathSummary {
        PathSummary {
            path:      self.path.clone(),
            type_name: self.type_name.clone(),
            status:    self.mutation_status,
            reason:    self.mutation_status_reason.clone(),
        }
    }
}

/// Summary of a mutation path for reason reporting
#[derive(Debug, Clone)]
pub struct PathSummary {
    pub path:      String,
    pub type_name: BrpTypeName,
    pub status:    MutationStatus,
    pub reason:    Option<Value>,
}

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind:              PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:              BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind:              TypeKind,
    /// Status of whether this path can be mutated
    pub mutation_status:        MutationStatus,
    /// Reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_status_reason: Option<Value>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<String>,

    /// The variant signature type (serialized as string using Display)
    #[serde(serialize_with = "serialize_signature")]
    pub signature: VariantSignature,

    /// Example value for this group
    pub example: Value,
}

fn serialize_signature<S>(sig: &VariantSignature, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&sig.to_string())
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description: String,
    /// Combined path navigation and type metadata
    pub path_info:   PathInfo,
    /// List of applicable variants (for enum types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants:    Option<Vec<String>>,
    /// Array of example groups with variants, signatures, and examples (for enums)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples:    Vec<ExampleGroup>,
    /// Single example value (for non-enum types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:     Option<Value>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:        Option<String>,
}

impl MutationPath {
    /// Create from `MutationPathInternal` with proper formatting logic
    pub fn from_mutation_path_internal(
        path: &MutationPathInternal,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

        // Generate description using the context
        let description = path.path_kind.description(&type_kind);

        // Handle examples array creation - clean up variant context wrapping
        let clean_example = path.example.get("__variant_context").map_or_else(
            || path.example.clone(),
            |_variant_context| {
                path.example.get("value").map_or_else(
                    || {
                        // Remove the variant context and keep the rest
                        let mut clean = path.example.clone();
                        if let Some(obj) = clean.as_object_mut() {
                            obj.remove("__variant_context");
                        }
                        clean
                    },
                    std::clone::Clone::clone,
                )
            },
        );

        // Handle examples array creation - new clean logic
        let examples = if clean_example.is_null() {
            vec![]
        } else if let Some(signature_groups) = clean_example.as_array() {
            // New format: direct array of signature groups from consolidated enum builder
            Self::convert_signature_groups_array(signature_groups)
        } else {
            // Single value: create simple example group (temporary - will be fixed in Step 6)
            vec![]
        };

        // Extract variants - check both signature groups and variant context
        let variants = path.example.as_array().map_or_else(
            || {
                path.example
                    .get("__variant_context")
                    .and_then(Value::as_array)
                    .and_then(|variant_context| {
                        // Extract variants from variant context (for enum sub-paths)
                        let mut variants = Vec::new();
                        for variant in variant_context {
                            if let Some(variant_name) = variant.as_str() {
                                variants.push(variant_name.to_string());
                            }
                        }
                        if variants.is_empty() {
                            None
                        } else {
                            Some(variants)
                        }
                    })
            },
            |signature_groups| {
                // Extract all variants from signature groups array (for enum root paths)
                let mut all_variants = Vec::new();
                for group in signature_groups {
                    if let Some(group_variants) = group.get("variants").and_then(Value::as_array) {
                        for variant in group_variants {
                            if let Some(variant_name) = variant.as_str() {
                                all_variants.push(variant_name.to_string());
                            }
                        }
                    }
                }
                all_variants.sort();
                if all_variants.is_empty() {
                    None
                } else {
                    Some(all_variants)
                }
            },
        );

        // Only process examples if the path is mutable
        let (final_examples, final_example) = if path.mutation_status != MutationStatus::Mutable {
            // Not mutable - no examples to show
            (vec![], None)
        } else if matches!(type_kind, TypeKind::Enum) && !examples.is_empty() {
            // Enum type with variants - use examples array
            (examples, None)
        } else if examples.len() == 1 && examples[0].applicable_variants.is_empty() {
            // Single example without enum context - use simple example field (temporary check -
            // will be fixed in Step 6)
            (vec![], Some(examples[0].example.clone()))
        } else if examples.is_empty() && !clean_example.is_null() {
            // Direct example without going through examples array (for TupleStruct, Array, etc.)
            (vec![], Some(clean_example))
        } else {
            // Multiple examples or enum context - keep examples array
            (examples, None)
        };

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
            },
            variants,
            examples: final_examples,
            example: final_example,
            note: None,
        }
    }

    /// Convert clean signature groups array from consolidated enum builder
    /// This handles the new direct array format: [{"example": ..., "signature": ..., "variants":
    /// [...]}]
    fn convert_signature_groups_array(signature_groups: &[Value]) -> Vec<ExampleGroup> {
        signature_groups
            .iter()
            .filter_map(|group| {
                let signature = group.get("signature")?.as_str()?.to_string();
                let variants = group
                    .get("variants")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>();
                let example = group.get("example")?.clone();

                // Temporary conversion - will be properly fixed in Step 6
                // For now, parse the signature string to create a VariantSignature
                let variant_sig = if signature == "unit" {
                    VariantSignature::Unit
                } else if signature.starts_with("tuple(") {
                    // For now, just use an empty tuple signature - will be fixed properly in Step 6
                    VariantSignature::Tuple(vec![])
                } else {
                    // For struct, use empty struct signature - will be fixed properly in Step 6
                    VariantSignature::Struct(vec![])
                };

                Some(ExampleGroup {
                    applicable_variants: variants,
                    signature: variant_sig,
                    example,
                })
            })
            .collect()
    }
}
