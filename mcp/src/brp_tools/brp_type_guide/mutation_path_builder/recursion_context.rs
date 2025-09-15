//! Context types for mutation path building
//!
//! This module contains the context structures and related types used for building mutation paths,
//! including the main context struct and location enums.
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use super::super::response_types::{BrpTypeName, ReflectTrait};
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey, MutationKnowledge};
use super::path_kind::PathKind;
use super::types::PathAction;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Context for mutation path building operations
///
/// This struct provides all the necessary context for building mutation paths,
/// including access to the registry, and enum variants.
#[derive(Debug)]
pub struct RecursionContext {
    /// The building context (root or field)
    pub path_kind:        PathKind,
    /// Reference to the type registry
    pub registry:         Arc<HashMap<BrpTypeName, Value>>,
    /// the accumulated mutation path as we recurse through the type
    pub mutation_path:    String,
    /// Parent's mutation knowledge for extracting component examples
    pub parent_knowledge: Option<&'static MutationKnowledge>,
    /// Action to take regarding path creation (set by `ProtocolEnforcer`)
    /// Design Review: Using enum instead of boolean for clarity and type safety
    pub path_action:      PathAction,
}

impl RecursionContext {
    /// Create a new mutation path context
    pub const fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            path_kind,
            registry,
            mutation_path: String::new(),
            parent_knowledge: None,
            path_action: PathAction::Create, // Default to creating paths
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

    /// Legacy method for unmigrated builders - returns Option and logs warning
    /// Will be removed once all builders are migrated to protocol-driven pattern
    #[deprecated(
        since = "0.1.0",
        note = "Use require_registry_schema instead. This method is only for unmigrated builders."
    )]
    pub fn require_registry_schema_legacy(&self) -> Option<&Value> {
        self.registry.get(self.type_name()).or_else(|| {
            warn!(
                type_name = %self.type_name(),
                "Schema missing for type - mutation paths may be incomplete"
            );
            None
        })
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

    /// Create a new context for a child element (field, array element, tuple element)
    #[deprecated(
        since = "0.1.0",
        note = "Use create_recursion_context instead. This method is only for unmigrated builders and will be removed once all builders are migrated to the protocol-driven pattern."
    )]
    pub fn create_unmigrated_recursion_context(&self, path_kind: PathKind) -> Self {
        let parent_type = self.type_name();
        let new_path_prefix = format!(
            "{}{}",
            self.mutation_path,
            Self::path_kind_to_segment(&path_kind)
        );

        // Look up mutation knowledge based on path kind
        // Only struct fields can have field-specific knowledge
        let field_knowledge = match &path_kind {
            PathKind::StructField { field_name, .. } => {
                // Check for struct field-specific knowledge, then fall back to exact type
                BRP_MUTATION_KNOWLEDGE
                    .get(&KnowledgeKey::struct_field(parent_type, field_name))
                    .or_else(|| {
                        BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(path_kind.type_name()))
                    })
            }
            PathKind::IndexedElement { .. }
            | PathKind::ArrayElement { .. }
            | PathKind::RootValue { .. } => {
                // Non-struct types only have exact type knowledge
                BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(path_kind.type_name()))
            }
        };

        Self {
            path_kind,
            registry: Arc::clone(&self.registry),
            mutation_path: new_path_prefix,
            parent_knowledge: field_knowledge,
            path_action: self.path_action, // Preserve parent's setting
        }
    }

    /// Create a new context for protocol-driven recursion
    ///
    /// Key differences from create_field_context (which unmigrated builders use):
    /// - Takes a PathAction parameter to control child path creation
    /// - Ensures Skip mode propagates to all descendants (once Skip, always Skip)
    /// - Self-contained implementation (doesn't call create_field_context)
    pub fn create_recursion_context(
        &self,
        path_kind: PathKind,
        child_path_action: PathAction,
    ) -> Self {
        let parent_type = self.type_name();
        let new_path_prefix = format!(
            "{}{}",
            self.mutation_path,
            Self::path_kind_to_segment(&path_kind)
        );

        // Look up mutation knowledge based on path kind
        // Only struct fields can have field-specific knowledge
        let field_knowledge = match &path_kind {
            PathKind::StructField { field_name, .. } => {
                // Check for struct field-specific knowledge, then fall back to exact type
                BRP_MUTATION_KNOWLEDGE
                    .get(&KnowledgeKey::struct_field(parent_type, field_name))
                    .or_else(|| {
                        BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(path_kind.type_name()))
                    })
            }
            PathKind::IndexedElement { .. }
            | PathKind::ArrayElement { .. }
            | PathKind::RootValue { .. } => {
                // Non-struct types only have exact type knowledge
                BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(path_kind.type_name()))
            }
        };

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
            parent_knowledge: field_knowledge,
            path_action,
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
}
