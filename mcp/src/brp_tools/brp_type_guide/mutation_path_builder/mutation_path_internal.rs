//! Internal mutation path representation and conversion to external format
//!
//! This module contains `MutationPathInternal` and its conversion logic to `MutationPathExternal`.
//! The conversion is implemented as a consuming `into_mutation_path_external` method following
//! Rust's `into_*` pattern for efficient ownership transfer.

use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Value;
use serde_json::json;

use super::super::brp_type_name::BrpTypeName;
use super::super::constants::DEFAULT_SPAWN_GUIDANCE;
use super::super::constants::REFLECT_TRAIT_DEFAULT;
use super::super::type_kind::TypeKind;
use super::new_types::MutationPath;
use super::new_types::VariantName;
use super::not_mutable_reason::NotMutableReason;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::types::EnumPathInfo;
use super::types::Mutability;
use super::types::MutabilityIssue;
use super::types::MutabilityIssueTarget;
use super::types::MutationPathExternal;
use super::types::PathInfo;
use super::types::RootExample;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

type ResolvedEnumPathInfo = (
    Option<String>,
    Option<Vec<VariantName>>,
    Option<RootExample>,
);

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path - now type-safe!
    pub example: PathExample,
    /// Path for mutation, e.g., ".translation.x"
    pub mutation_path: MutationPath,
    /// Type information for this path
    pub type_name: BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind: PathKind,
    /// Whether this path can be mutated
    pub mutability: Mutability,
    /// Reason if mutation is not possible
    pub mutability_reason: Option<NotMutableReason>,
    /// Consolidated enum-specific data
    pub enum_path_info: Option<EnumPathInfo>,
    /// Depth level of this path in the recursion tree (0 = root, 1 = .field, etc.)
    /// Used to identify direct children vs grandchildren during assembly
    pub depth: usize,
    /// Maps variant chains to complete root examples for reaching nested enum paths.
    /// Populated during enum processing for paths where `matches!(example, PathExample::EnumRoot {
    /// .. })`. Built by `build_partial_root_examples()` in `enum_path_builder.rs` during
    /// ascent phase. None for non-enum paths and enum leaf paths.
    pub partial_root_examples: Option<HashMap<Vec<VariantName>, RootExample>>,
}

impl MutationPathInternal {
    /// Check if this path is a direct child at the given parent depth
    pub const fn is_direct_child_at_depth(&self, parent_depth: usize) -> bool {
        self.depth == parent_depth + 1
    }

    /// Create a `MutabilityIssue` from this mutation path (for non-enum types)
    pub fn to_mutability_issue(&self) -> MutabilityIssue {
        MutabilityIssue {
            target: MutabilityIssueTarget::Path(self.mutation_path.clone()),
            type_name: self.type_name.clone(),
            status: self.mutability,
            reason: self
                .mutability_reason
                .as_ref()
                .and_then(Option::<Value>::from),
        }
    }

    /// Convert this internal mutation path into external format for API responses
    ///
    /// This method consumes `self` to enable efficient data movement without cloning.
    /// Following Rust's `into_*` naming convention for consuming conversions.
    pub fn into_mutation_path_external(
        mut self,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> MutationPathExternal {
        // Get schema and derive TypeKind for the field type
        let field_schema = registry.get(&self.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema);

        // Check for Default trait once at the top for root paths
        let has_default_for_root = self.has_default_for_root(field_schema);

        // Generate description with proper handling of PartiallyMutable status
        let description = self.resolve_description(&type_kind, has_default_for_root);

        // Resolve the appropriate path example based on mutability status
        let path_example = self.resolve_path_example(has_default_for_root);

        // Extract enum-specific metadata only for mutable/partially mutable paths
        let (enum_instructions, applicable_variants, root_example) = self.resolve_enum_path_info();

        MutationPathExternal {
            description,
            path_info: PathInfo {
                path_kind: self.path_kind,
                type_name: self.type_name,
                type_kind,
                mutability: self.mutability,
                mutability_reason: self
                    .mutability_reason
                    .as_ref()
                    .and_then(Option::<Value>::from),
                enum_instructions,
                applicable_variants,
                root_example,
            },
            path_example,
        }
    }

    /// Check if this path is a root path with Default trait support
    fn has_default_for_root(&self, field_schema: &Value) -> bool {
        if !matches!(self.path_kind, PathKind::RootValue { .. }) {
            return false;
        }

        field_schema
            .get_field_array(SchemaField::ReflectTypes)
            .is_some_and(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .any(|t| t == REFLECT_TRAIT_DEFAULT)
            })
    }

    /// Generate human-readable description for this mutation path
    ///
    /// Uses type-specific terminology (fields, elements, entries, variants) instead of
    /// generic "descendants". Adds spawn guidance for `PartiallyMutable` root paths with Default.
    fn resolve_description(&self, type_kind: &TypeKind, has_default_for_root: bool) -> String {
        match self.mutability {
            Mutability::PartiallyMutable => {
                let base_msg = format!(
                    "This {} path is partially mutable due to some of its {} not being mutable",
                    type_kind.as_ref().to_lowercase(),
                    type_kind.child_terminology()
                );
                if has_default_for_root {
                    format!("{base_msg}.{DEFAULT_SPAWN_GUIDANCE}")
                } else {
                    base_msg
                }
            }
            _ => self.path_kind.description(type_kind),
        }
    }

    /// Resolve the appropriate `PathExample` based on mutability status
    ///
    /// - `NotMutable`: Returns null (no example provided)
    /// - `PartiallyMutable`: Returns enum examples or empty object if Default trait exists
    /// - Mutable: Returns the original example (moved out of self)
    fn resolve_path_example(&mut self, has_default_for_root: bool) -> PathExample {
        match self.mutability {
            Mutability::NotMutable => PathExample::Simple(Value::Null),
            Mutability::PartiallyMutable => match &self.example {
                PathExample::EnumRoot { .. } => self.example.clone(),
                PathExample::Simple(_) => {
                    if has_default_for_root {
                        PathExample::Simple(json!({}))
                    } else {
                        PathExample::Simple(Value::Null)
                    }
                }
            },
            Mutability::Mutable => {
                // Move the example out for Mutable case
                std::mem::replace(&mut self.example, PathExample::Simple(Value::Null))
            }
        }
    }

    /// Extract enum-specific metadata for paths nested within enums
    ///
    /// Returns `(instructions, applicable_variants, root_example, root_example_unavailable_reason)`
    /// only for mutable/partially mutable paths. Returns `(None, None, None, None)` for
    /// `NotMutable` paths to avoid showing contradictory mutation instructions for paths that
    /// cannot be mutated.
    fn resolve_enum_path_info(&mut self) -> ResolvedEnumPathInfo {
        if !matches!(
            self.mutability,
            Mutability::Mutable | Mutability::PartiallyMutable
        ) {
            return (None, None, None);
        }

        self.enum_path_info
            .take()
            .map_or((None, None,   None), |enum_data| {
                let instructions = match &enum_data.root_example {
                    Some(RootExample::Available { .. }) => Some(format!(
                        "First, set the root mutation path to 'root_example', then you can mutate the '{}' path. See 'applicable_variants' for which variants support this field.",
                        &self.mutation_path
                    )),
                    _ => None,  // Unavailable or NotPresent - no instructions
                };

                let variants = if enum_data.applicable_variants.is_empty() {
                    None
                } else {
                    Some(enum_data.applicable_variants)
                };

                (
                    instructions,
                    variants,
                    enum_data.root_example,
                )
            })
    }
}

/// Extension trait for collecting variant chains from slices of `MutationPathInternal`
pub trait MutationPathSliceExt {
    /// Collect all unique variant chains from direct children at the given depth
    ///
    /// Extracts variant chains from `partial_root_examples` for all direct children,
    /// enabling variant-specific root example assembly during enum processing.
    fn child_variant_chains(&self, depth: usize) -> HashSet<Vec<VariantName>>;
}

impl MutationPathSliceExt for [&MutationPathInternal] {
    fn child_variant_chains(&self, depth: usize) -> HashSet<Vec<VariantName>> {
        self.iter()
            .filter(|child| child.is_direct_child_at_depth(depth))
            .flat_map(|child| {
                child
                    .partial_root_examples
                    .as_ref()
                    .into_iter()
                    .flat_map(|partials| partials.keys().cloned())
            })
            .collect()
    }
}
