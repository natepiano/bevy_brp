//! Internal mutation path representation and conversion to external format
//!
//! This module contains `MutationPathInternal` and its conversion logic to `MutationPathExternal`.
//! The conversion is implemented as a consuming `into_mutation_path_external` method following
//! Rust's `into_*` pattern for efficient ownership transfer.

use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Value;
use serde_json::json;

use super::ecs_role::EcsRole;
use super::enum_path_info::EnumPathInfo;
use super::mutability::Mutability;
use super::mutability::MutabilityIssue;
use super::mutability::MutabilityIssueTarget;
use super::mutation_path::MutationPath;
use super::mutation_path_external::MutationPathExternal;
use super::mutation_path_external::PathInfo;
use super::mutation_path_external::RootExample;
use super::not_mutable_reason::NotMutableReason;
use super::path_example::Example;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::variant_name::VariantName;
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::brp_tools::brp_type_guide::constants::OPERATION_INSERT;
use crate::brp_tools::brp_type_guide::constants::OPERATION_SPAWN;
use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_DEFAULT;
use crate::brp_tools::brp_type_guide::type_kind::TypeKind;
use crate::support::JsonObjectAccess;
use crate::support::SchemaField;

type ResolvedEnumPathInfo = (
    Option<String>,
    Option<Vec<VariantName>>,
    Option<RootExample>,
);

/// Whether a path accepts an empty object `{}` for spawn, insert and root mutate operations
///
/// Only a root named-field `Struct` that reflects `Default` accepts `{}`: BRP fills the missing
/// fields from `Default`. An enum needs a single variant key, a single-field tuple struct takes
/// its field's value, and other tuple structs and value types do not deserialize from `{}`.
/// Gates the `{}` example fallback and its description guidance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmptyObjectRoot {
    Accepted,
    Rejected,
}

/// Parameters for constructing a `PathInfo`.
struct PathInfoParams {
    path_kind:           PathKind,
    type_name:           BrpTypeName,
    type_kind:           TypeKind,
    mutability:          Mutability,
    mutability_reason:   Option<Value>,
    applicable_variants: Option<Vec<VariantName>>,
    enum_instructions:   Option<String>,
    root_example:        Option<RootExample>,
}

impl From<PathInfoParams> for PathInfo {
    fn from(params: PathInfoParams) -> Self {
        Self {
            path_kind:           params.path_kind,
            type_name:           params.type_name,
            type_kind:           params.type_kind,
            mutability:          params.mutability,
            mutability_reason:   params.mutability_reason,
            applicable_variants: params.applicable_variants,
            enum_instructions:   params.enum_instructions,
            root_example:        params.root_example,
        }
    }
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub(super) struct MutationPathInternal {
    /// Example value for this path - now type-safe!
    pub(super) example:               PathExample,
    /// Path for mutation, e.g., ".translation.x"
    pub(super) mutation_path:         MutationPath,
    /// Type information for this path
    pub(super) type_name:             BrpTypeName,
    /// Context describing what kind of mutation this is
    pub(super) path_kind:             PathKind,
    /// Whether this path can be mutated
    pub(super) mutability:            Mutability,
    /// Reason if mutation is not possible
    pub(super) mutability_reason:     Option<NotMutableReason>,
    /// Consolidated enum-specific data
    pub(super) enum_path_info:        Option<EnumPathInfo>,
    /// Depth level of this path in the recursion tree (0 = root, 1 = .field, etc.)
    /// Used to identify direct children vs grandchildren during assembly
    pub(super) depth:                 usize,
    /// Maps variant chains to complete root examples for reaching nested enum paths.
    /// Populated during enum processing for paths where `matches!(example, PathExample::EnumRoot {
    /// .. })`. Built by `build_partial_root_examples()` in `enum_path_builder.rs` during
    /// ascent phase. None for non-enum paths and enum leaf paths.
    pub(super) partial_root_examples: Option<HashMap<Vec<VariantName>, RootExample>>,
}

impl MutationPathInternal {
    /// Check if this path is a direct child at the given parent depth
    pub(super) const fn is_direct_child_at_depth(&self, parent_depth: usize) -> bool {
        self.depth == parent_depth + 1
    }

    /// Create a `MutabilityIssue` from this mutation path (for non-enum types)
    pub(super) fn to_mutability_issue(&self) -> MutabilityIssue {
        MutabilityIssue {
            target:     MutabilityIssueTarget::Path(self.mutation_path.clone()),
            mutability: self.mutability,
        }
    }

    /// Convert this internal mutation path into external format for API responses
    ///
    /// This method consumes `self` to enable efficient data movement without cloning.
    /// Following Rust's `into_*` naming convention for consuming conversions.
    pub(super) fn into_mutation_path_external(
        mut self,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> MutationPathExternal {
        // Get schema and derive TypeKind for the field type
        let field_schema = registry.get(&self.type_name).unwrap_or(&Value::Null);
        let type_kind: TypeKind = field_schema.into();

        // Check once whether this is a root named-field struct with `Default`
        let empty_object_root = self.empty_object_root(&type_kind, field_schema);

        // `resolve_path_example` selects a `PathExample` from `self.mutability`,
        // `empty_object_root`, and `self.example`.
        let path_example = self.resolve_path_example(empty_object_root);

        // `resolve_description` uses `self.mutability`, `empty_object_root`, and whether
        // `path_example` provides an example to select guidance.
        let description = self.resolve_description(
            &type_kind,
            empty_object_root,
            &path_example,
            field_schema,
            registry,
        );

        // Extract enum-specific metadata only for mutable/partially mutable paths
        let (enum_instructions, applicable_variants, root_example) = self.resolve_enum_path_info();

        MutationPathExternal::new(
            self.mutation_path.clone(),
            description,
            PathInfoParams {
                path_kind: self.path_kind,
                type_name: self.type_name,
                type_kind,
                mutability: self.mutability,
                mutability_reason: self
                    .mutability_reason
                    .as_ref()
                    .and_then(Option::<Value>::from),
                applicable_variants,
                enum_instructions,
                root_example,
            }
            .into(),
            path_example,
        )
    }

    /// Check if this path is a root named-field struct with Default trait support
    fn empty_object_root(&self, type_kind: &TypeKind, field_schema: &Value) -> EmptyObjectRoot {
        if !matches!(self.path_kind, PathKind::RootValue { .. })
            || !matches!(type_kind, TypeKind::Struct)
        {
            return EmptyObjectRoot::Rejected;
        }

        let has_default = field_schema
            .get_field_array(SchemaField::ReflectTypes)
            .is_some_and(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .any(|t| t == REFLECT_TRAIT_DEFAULT)
            });
        if has_default {
            EmptyObjectRoot::Accepted
        } else {
            EmptyObjectRoot::Rejected
        }
    }

    /// Generate human-readable description for this mutation path
    ///
    /// Uses type-specific terminology (fields, elements, entries, variants) instead of
    /// generic "descendants". For `PartiallyMutable` and `NotMutable` paths, adds the empty
    /// object guidance when `empty_object_root` accepts `{}`, otherwise notes when
    /// `path_example` provides no example.
    ///
    /// `type_kind` is the kind of `self.type_name`; `Mutability::Mutable` paths name their
    /// containing type instead, so `PathKind::description` resolves that kind from `registry`.
    fn resolve_description(
        &self,
        type_kind: &TypeKind,
        empty_object_root: EmptyObjectRoot,
        path_example: &PathExample,
        field_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> String {
        let base_message = match self.mutability {
            Mutability::PartiallyMutable => format!(
                "This {} path is partially mutable due to some of its {} not being mutable",
                type_kind.description_label(),
                type_kind.child_terminology()
            ),
            Mutability::NotMutable => {
                format!("This {} is not mutable", type_kind.description_label())
            },
            Mutability::Mutable => {
                return self
                    .path_kind
                    .description(registry, self.enum_path_info.as_ref());
            },
        };

        match empty_object_root {
            EmptyObjectRoot::Accepted => {
                let guidance = Self::get_default_spawn_guidance(field_schema);
                format!("{base_message}.{guidance}")
            },
            EmptyObjectRoot::Rejected
                if matches!(path_example.preferred_example(), Example::NotApplicable) =>
            {
                format!("{base_message}. No example is provided.")
            },
            EmptyObjectRoot::Rejected => format!("{base_message}."),
        }
    }

    /// Get the appropriate Default spawn guidance based on the type's `EcsRole`
    fn get_default_spawn_guidance(field_schema: &Value) -> String {
        let reflect_traits = field_schema
            .get_field_array(SchemaField::ReflectTypes)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();

        // `EcsRole::Other` keeps the spawn wording used for components.
        let operation = match EcsRole::from(reflect_traits.as_slice()) {
            EcsRole::Resource => OPERATION_INSERT,
            EcsRole::Component | EcsRole::Other => OPERATION_SPAWN,
        };

        format!(
            " However this type implements Default and accepts empty object {{}} for {operation} or mutate operations on the root path"
        )
    }

    /// Resolve the appropriate `PathExample` based on mutability status
    ///
    /// - `NotMutable`: Returns `NotApplicable` (no example provided)
    /// - `PartiallyMutable`: Returns enum examples or a complete example; without a complete
    ///   example, an empty object if `empty_object_root` accepts one, otherwise `NotApplicable`
    /// - Mutable: Returns the original example
    fn resolve_path_example(&self, empty_object_root: EmptyObjectRoot) -> PathExample {
        match self.mutability {
            Mutability::NotMutable => PathExample::Simple(Example::NotApplicable),
            Mutability::PartiallyMutable => match &self.example {
                PathExample::EnumRoot { .. } => self.example.clone(),
                PathExample::Simple(example) if example.is_complete() => self.example.clone(),
                PathExample::Simple(_) => match empty_object_root {
                    EmptyObjectRoot::Accepted => PathExample::Simple(Example::Json(json!({}))),
                    EmptyObjectRoot::Rejected => PathExample::Simple(Example::NotApplicable),
                },
            },
            Mutability::Mutable => self.example.clone(),
        }
    }

    /// Extract enum-specific metadata for paths nested within enums
    ///
    /// Returns `(instructions, applicable_variants, root_example)` only for mutable/partially
    /// mutable paths. Returns `(None, None, None)` for
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
            .map_or((None, None,   None), |enum_path_info| {
                let instructions = match &enum_path_info.root_example {
                    Some(RootExample::Available { .. }) => Some("Current mutation path is nested within an enum variant. To mutate, first mutate path \"\" to the 'example' value in 'path_info', then this path.".to_string()),
                    _ => None,  // Unavailable - no instructions
                };

                let variants = if enum_path_info.applicable_variants.is_empty() {
                    None
                } else {
                    Some(enum_path_info.applicable_variants)
                };

                (
                    instructions,
                    variants,
                    enum_path_info.root_example,
                )
            })
    }
}

/// Collect all unique variant chains from direct children at the given depth.
pub(super) fn child_variant_chains(
    children: &[&MutationPathInternal],
    depth: usize,
) -> HashSet<Vec<VariantName>> {
    children
        .iter()
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

#[cfg(test)]
mod tests {
    use serde_json::Map;
    use serde_json::Value;

    use super::MutationPathInternal;
    use crate::brp_tools::brp_type_guide::constants::OPERATION_INSERT;
    use crate::brp_tools::brp_type_guide::constants::OPERATION_SPAWN;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_COMPONENT;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_DEFAULT;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_RESOURCE;
    use crate::support::JsonObjectAccess;
    use crate::support::SchemaField;

    fn schema_with_reflect_traits(reflect_traits: &[&str]) -> Value {
        let mut schema = Value::Object(Map::new());
        schema.insert_field(SchemaField::ReflectTypes.as_ref(), reflect_traits.to_vec());
        schema
    }

    #[test]
    fn default_guidance_for_resource_names_insert() {
        // Bevy 0.20 `#[reflect(Resource)]` also registers `ReflectComponent`.
        let schema = schema_with_reflect_traits(&[
            REFLECT_TRAIT_COMPONENT,
            REFLECT_TRAIT_DEFAULT,
            REFLECT_TRAIT_RESOURCE,
        ]);

        let guidance = MutationPathInternal::get_default_spawn_guidance(&schema);

        assert!(guidance.contains(&format!(" for {OPERATION_INSERT} ")));
    }

    #[test]
    fn default_guidance_for_component_names_spawn() {
        let schema = schema_with_reflect_traits(&[REFLECT_TRAIT_COMPONENT, REFLECT_TRAIT_DEFAULT]);

        let guidance = MutationPathInternal::get_default_spawn_guidance(&schema);

        assert!(guidance.contains(&format!(" for {OPERATION_SPAWN} ")));
    }
}
