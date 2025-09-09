//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::response_types::BrpTypeName;
use super::TypeKind;
use super::path_kind::PathKind;

/// Status of whether a mutation path can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Path can be fully mutated
    Mutatable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutatable,
    /// Path is partially mutatable (some elements mutable, others not)
    PartiallyMutatable,
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:         Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:            String,
    /// Type information for this path
    pub type_name:       BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:       PathKind,
    /// Status of whether this path can be mutated
    pub mutation_status: MutationStatus,
    /// Error reason if mutation is not possible
    pub error_reason:    Option<String>,
}

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind: PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name: BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind: TypeKind,
}

/// Example group for the unified examples array
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature (only for enum types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants: Option<Vec<String>>,
    /// Human-readable signature description (only for enum types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature:           Option<String>,
    /// Example value for this group
    pub example:             Value,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description:     String,
    /// Combined path navigation and type metadata
    pub path_info:       PathInfo,
    /// Status of whether this path can be mutated
    pub mutation_status: MutationStatus,
    /// Error reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason:    Option<String>,
    /// List of applicable variants (for enum types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants:        Option<Vec<String>>,
    /// Array of example groups with variants, signatures, and examples (for enums)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples:        Vec<ExampleGroup>,
    /// Single example value (for non-enum types) 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:         Option<Value>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:            Option<String>,
}

impl MutationPath {
    /// Create from `MutationPathInternal` with proper formatting logic
    pub fn from_mutation_path_internal(
        path: &MutationPathInternal,
        description: String,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

        // Handle examples array creation based on type
        let examples = if path.example.is_null() {
            vec![]
        } else {
            path.example.as_object().map_or_else(
                || {
                    // Non-enum type: create single example group without variants/signature
                    vec![ExampleGroup {
                        applicable_variants: None,
                        signature:           None,
                        example:             path.example.clone(),
                    }]
                },
                |obj| {
                    // Check if this has variant context (enum field path)
                    obj.get("__variant_context").map_or_else(
                        || {
                            obj.get("__enum_signature_groups").map_or_else(
                                || {
                                    // DEBUG: Log the type detection for Transform
                                    if path.type_name.as_str()
                                        == "bevy_transform::components::transform::Transform"
                                    {
                                        tracing::warn!(
                                            "DEBUG: Transform type_kind = {:?}, obj keys = {:?}",
                                            type_kind,
                                            obj.keys().collect::<Vec<_>>()
                                        );
                                    }

                                    // Only treat as enum examples if this is actually an enum type
                                    // Use TypeKind to properly distinguish enums from structs
                                    if matches!(type_kind, TypeKind::Enum) && !obj.is_empty() {
                                        // This is an enum root path with variant examples
                                        // (fallback)
                                        Self::create_enum_example_groups(obj)
                                    } else {
                                        // This is a regular struct/object example - don't add enum
                                        // fields
                                        vec![ExampleGroup {
                                            applicable_variants: None,
                                            signature:           None,
                                            example:             path.example.clone(),
                                        }]
                                    }
                                },
                                |signature_groups| {
                                    // This is an enum root path with signature-grouped examples
                                    Self::create_enum_signature_groups(signature_groups)
                                },
                            )
                        },
                        |_variant_context| {
                            // Extract the actual example value
                            let actual_example = obj.get("example").unwrap_or(&Value::Null);
                            vec![ExampleGroup {
                                applicable_variants: None,
                                signature:           None,
                                example:             actual_example.clone(),
                            }]
                        },
                    )
                },
            )
        };

        // Extract variants from example if it's an enum or from variant context
        let variants = path
            .example
            .get("__variant_context")
            .and_then(Value::as_str)
            .map_or_else(
                || {
                    path.example
                        .as_object()
                        .and_then(|obj| obj.get("__enum_signature_groups"))
                        .and_then(Value::as_array)
                        .map_or_else(
                            || {
                                // Only extract variants if this is actually an enum type
                                if matches!(type_kind, TypeKind::Enum) {
                                    // This is a root enum path with variant examples (fallback)
                                    path.example.as_object().and_then(|obj| {
                                        if obj.is_empty() {
                                            None
                                        } else {
                                            let mut keys: Vec<String> =
                                                obj.keys().cloned().collect();
                                            keys.sort(); // Alphabetical sorting for consistency
                                            Some(keys)
                                        }
                                    })
                                } else {
                                    // Not an enum - no variants
                                    None
                                }
                            },
                            |signature_groups| {
                                // This is an enum root path - collect all variants from all groups
                                let mut all_variants = Vec::new();
                                for group in signature_groups {
                                    if let Some(group_variants) =
                                        group.get("variants").and_then(Value::as_array)
                                    {
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
                        )
                },
                |variant_context| Some(vec![variant_context.to_string()]),
            );

        // Determine if this should use examples array or single example
        let (final_examples, final_example) = if matches!(type_kind, TypeKind::Enum) && !examples.is_empty() {
            // Enum type with variants - use examples array
            (examples, None)
        } else if examples.len() == 1 && examples[0].applicable_variants.is_none() && examples[0].signature.is_none() {
            // Single example without enum context - use simple example field
            (vec![], Some(examples[0].example.clone()))
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
            },
            variants,
            examples: final_examples,
            example: final_example,
            note: None,
            mutation_status: path.mutation_status,
            error_reason: path.error_reason.clone(),
        }
    }

    /// Create example groups from signature groups (new enum structure)
    /// This handles the `__enum_signature_groups` format from the enum builder
    fn create_enum_signature_groups(signature_groups: &Value) -> Vec<ExampleGroup> {
        signature_groups
            .as_array()
            .map_or_else(Vec::new, |groups_array| {
                groups_array
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

                        Some(ExampleGroup {
                            applicable_variants: Some(variants),
                            signature: Some(signature),
                            example,
                        })
                    })
                    .collect()
            })
    }

    /// Create example groups for enum variants by analyzing their structure
    /// Groups variants with the same signature together and creates proper format
    fn create_enum_example_groups(
        variant_examples: &serde_json::Map<String, Value>,
    ) -> Vec<ExampleGroup> {
        use std::collections::HashMap;

        // Group variants by their structural signature
        let mut signature_groups: HashMap<String, Vec<(String, Value)>> = HashMap::new();

        for (variant_name, example_value) in variant_examples {
            let signature = Self::analyze_variant_signature(variant_name, example_value);
            signature_groups
                .entry(signature)
                .or_default()
                .push((variant_name.clone(), example_value.clone()));
        }

        // Convert groups to ExampleGroup entries
        let mut example_groups = Vec::new();
        for (signature, variants_with_examples) in signature_groups {
            if let Some((first_variant_name, first_example)) = variants_with_examples.first() {
                let variant_names: Vec<String> = variants_with_examples
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect();

                // For unit variants, use the variant name directly as the example
                // For other types, use the constructed example
                let example = if signature == "unit" {
                    Value::String(first_variant_name.clone())
                } else {
                    first_example.clone()
                };

                example_groups.push(ExampleGroup {
                    applicable_variants: Some(variant_names),
                    signature: Some(signature),
                    example,
                });
            }
        }

        // Sort by signature for consistent output (unit first, then alphabetically)
        example_groups.sort_by(
            |a, b| match (a.signature.as_deref(), b.signature.as_deref()) {
                (Some("unit"), Some("unit")) => std::cmp::Ordering::Equal,
                (Some("unit"), _) => std::cmp::Ordering::Less,
                (_, Some("unit")) => std::cmp::Ordering::Greater,
                (Some(a_sig), Some(b_sig)) => a_sig.cmp(b_sig),
                _ => std::cmp::Ordering::Equal,
            },
        );
        example_groups
    }

    /// Analyze the signature of a variant example to determine its structure
    /// This creates detailed type signatures as specified in the plan
    fn analyze_variant_signature(variant_name: &str, example_value: &Value) -> String {
        match example_value {
            // Simple string variant is a unit variant
            Value::String(_) if example_value.as_str() == Some(variant_name) => "unit".to_string(),

            // Object with variant name as key
            Value::Object(obj) if obj.len() == 1 => {
                if let Some((key, value)) = obj.iter().next() {
                    if key == variant_name {
                        match value {
                            Value::Array(arr) => {
                                // Tuple variant - analyze the actual types from the values
                                let type_names: Vec<String> =
                                    arr.iter().map(Self::infer_type_from_value).collect();
                                format!("tuple({})", type_names.join(", "))
                            }
                            Value::Object(struct_obj) => {
                                // Struct variant - analyze the actual fields and types
                                let mut field_sigs: Vec<String> = struct_obj
                                    .iter()
                                    .map(|(field_name, field_value)| {
                                        let type_name = Self::infer_type_from_value(field_value);
                                        format!("{field_name}: {type_name}")
                                    })
                                    .collect();
                                field_sigs.sort(); // Consistent ordering
                                format!("struct{{{}}}", field_sigs.join(", "))
                            }
                            _ => {
                                // Single value tuple (newtype pattern)
                                let type_name = Self::infer_type_from_value(value);
                                format!("tuple({type_name})")
                            }
                        }
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                }
            }

            _ => "unknown".to_string(),
        }
    }

    /// Infer a human-readable type name from a JSON value
    fn infer_type_from_value(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::Number(n) => {
                if n.is_f64() {
                    "f32".to_string()
                } else if n.is_i64() {
                    "i32".to_string()
                } else {
                    "u32".to_string()
                }
            }
            Value::String(_) => "String".to_string(),
            Value::Array(_) => "Array".to_string(),
            Value::Object(_) => "Object".to_string(),
        }
    }
}
