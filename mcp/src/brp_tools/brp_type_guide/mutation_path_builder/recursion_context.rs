//! Context types for mutation path building
//!
//! This module contains the context structures and related types used for building mutation paths,
//! including the main context struct and location enums.
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::response_types::ReflectTrait;
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::path_kind::PathKind;
use super::types::{PathAction, VariantPathEntry};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Tracks enum-specific context during recursion
#[derive(Debug, Clone)]
pub enum EnumContext {
    /// This enum is establishing the root context
    Root,

    /// Building under enum variant(s)
    Child {
        /// Chain of variant constraints from parent to child with their paths
        /// e.g., `[VariantPathEntry { path: "", variant: "TestEnumWithSerDe::Nested" },`
        ///        `VariantPathEntry { path: ".nested_config", variant:`
        /// `"NestedConfigEnum::Conditional" }]`
        variant_chain: Vec<VariantPathEntry>,
    },
}

/// Context for mutation path building operations
///
/// This struct provides all the necessary context for building mutation paths,
/// including access to the registry, and enum variants.
#[derive(Debug)]
pub struct RecursionContext {
    /// The building context (root or field)
    pub path_kind:     PathKind,
    /// Reference to the type registry
    pub registry:      Arc<HashMap<BrpTypeName, Value>>,
    /// the accumulated mutation path as we recurse through the type
    pub mutation_path: String,
    /// Action to take regarding path creation (set by `MutationPathBuilder`)
    /// Design Review: Using enum instead of boolean for clarity and type safety
    pub path_action:   PathAction,
    /// Track enum context - None for non-enum types
    pub enum_context:  Option<EnumContext>,
}

impl RecursionContext {
    /// Create a new mutation path context
    pub const fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            path_kind,
            registry,
            mutation_path: String::new(),
            path_action: PathAction::Create, // Default to creating paths
            enum_context: None,              // Start with no enum context
        }
    }

    /// Get the type name being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        self.path_kind.type_name()
    }

    /// Generate the path segment string for a `PathKind` (private to this module)
    fn path_kind_to_segment(path_kind: &PathKind) -> String {
        match path_kind {
            PathKind::RootValue { .. } => String::new(),
            PathKind::StructField { field_name, .. } => format!(".{field_name}"),
            PathKind::IndexedElement { index, .. } => format!(".{index}"),
            PathKind::ArrayElement { index, .. } => format!("[{index}]"),
        }
    }

    /// Require the schema to be present, returning an error if missing
    /// This is the preferred method for migrated builders
    pub fn require_registry_schema(&self) -> crate::error::Result<&Value> {
        self.registry.get(self.type_name()).ok_or_else(|| {
            crate::error::Error::SchemaProcessing {
                message:   format!("No schema found for type: {}", self.type_name()),
                type_name: Some(self.type_name().to_string()),
                operation: Some("require_registry_schema".to_string()),
                details:   None,
            }
            .into()
        })
    }

    /// Look up a type in the registry
    pub fn get_registry_schema(&self, type_name: &BrpTypeName) -> Option<&Value> {
        self.registry.get(type_name)
    }

    /// Create a new context for protocol-driven recursion
    ///
    /// Key differences from `create_field_context` (which unmigrated builders use):
    /// - Takes a `PathAction` parameter to control child path creation
    /// - Ensures `Skip` mode propagates to all descendants (once `Skip`, always `Skip`)
    /// - Self-contained implementation (doesn't call `create_field_context`)
    /// - Propagates parent's `enum_context` to children by default
    pub fn create_recursion_context(
        &self,
        path_kind: PathKind,
        child_path_action: PathAction,
    ) -> Self {
        let new_path_prefix = format!(
            "{}{}",
            self.mutation_path,
            Self::path_kind_to_segment(&path_kind)
        );

        // Set path_action with proper propagation logic:
        // If parent is already Skip, stay Skip (regardless of what child wants)
        // Otherwise, use the child's preference
        let path_action = if matches!(self.path_action, PathAction::Skip) {
            PathAction::Skip // Once skipping, keep skipping for entire subtree
        } else {
            child_path_action
        };

        Self {
            path_kind,
            registry: Arc::clone(&self.registry),
            mutation_path: new_path_prefix,
            path_action,
            enum_context: self.enum_context.clone(), // Propagate parent's enum_context
        }
    }

    /// Check if a value type has serialization support
    /// Used to determine if opaque Value types like String can be mutated
    pub fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
        self.get_registry_schema(type_name).is_some_and(|schema| {
            let reflect_types: Vec<ReflectTrait> =
                Self::get_schema_field_as_array(schema, SchemaField::ReflectTypes)
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| s.parse().ok())
                    .collect();

            reflect_types.contains(&ReflectTrait::Serialize)
                && reflect_types.contains(&ReflectTrait::Deserialize)
        })
    }

    /// Extract all element types from Tuple/TupleStruct schema
    pub fn extract_tuple_element_types(schema: &Value) -> Option<Vec<BrpTypeName>> {
        Self::get_schema_field_as_array(schema, SchemaField::PrefixItems).map(|items| {
            items
                .iter()
                .filter_map(SchemaField::extract_field_type)
                .collect()
        })
    }

    /// Helper to get a schema field as an array
    fn get_schema_field_as_array(schema: &Value, field: SchemaField) -> Option<&Vec<Value>> {
        schema.get_field(field).and_then(Value::as_array)
    }

    /// Find mutation knowledge for this context
    ///
    /// This unified lookup method replaces the fragmented approach of separate lookup methods.
    /// It checks context-specific matches first, then falls back to exact type matches.
    ///
    /// Lookup order:
    /// 1. Struct field match (for field-specific values like `Camera3d.depth_texture_usages`) -
    ///    highest priority
    /// 2. Exact type match (handles most primitive and simple types) - fallback
    /// 3. Future: Enum signature match (for newtype variants - see plan-enum-variant-knowledge.md)
    pub fn find_knowledge(&self) -> Option<&'static super::mutation_knowledge::MutationKnowledge> {
        tracing::debug!(
            "find_knowledge for type: {}, path_kind: {:?}",
            self.type_name(),
            self.path_kind
        );

        // Try context-specific matches based on PathKind FIRST - these have higher priority
        match &self.path_kind {
            PathKind::StructField {
                field_name,
                parent_type,
                ..
            } => {
                // Try struct field-specific knowledge first - this overrides generic type knowledge
                // Example: Camera3d.depth_texture_usages needs value 20, not generic u32 value
                let key = KnowledgeKey::struct_field(parent_type.type_string(), field_name);
                tracing::debug!("Trying struct field match with key: {:?}", key);
                if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&key) {
                    tracing::debug!(
                        "Found struct field match for {}.{}: {:?}",
                        parent_type,
                        field_name,
                        knowledge.example()
                    );
                    return Some(knowledge);
                }
                tracing::debug!(
                    "No struct field match found for {}.{}, falling back to exact type match",
                    parent_type,
                    field_name
                );

                // Fall through to exact type match for struct fields without specific knowledge
            }
            PathKind::RootValue { .. }
            | PathKind::IndexedElement { .. }
            | PathKind::ArrayElement { .. } => {
                tracing::debug!(
                    "PathKind {:?} - checking exact type match only",
                    self.path_kind
                );
                // For these path kinds, only exact type matching applies
                // IndexedElement will be handled by enum signature matching in the future
            }
        }

        // Try exact type match as fallback - this handles most cases
        let exact_key = KnowledgeKey::exact(self.type_name().type_string());
        tracing::debug!("Trying exact type match with key: {:?}", exact_key);
        BRP_MUTATION_KNOWLEDGE.get(&exact_key).map_or_else(
            || {
                tracing::debug!("No exact type match found for {}", self.type_name());
                None
            },
            |knowledge| {
                tracing::debug!(
                    "Found exact type match for {}: {:?}",
                    self.type_name(),
                    knowledge.example()
                );
                Some(knowledge)
            },
        )
    }
}
