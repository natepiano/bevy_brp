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
use crate::json_types::SchemaField;
use crate::string_traits::JsonFieldAccess;

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
}

impl RecursionContext {
    /// Create a new mutation path context
    pub const fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            path_kind,
            registry,
            mutation_path: String::new(),
            parent_knowledge: None,
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

    /// Require the schema to be present, logging a warning if missing
    /// Looks up the schema from the registry based on the current type
    pub fn require_schema(&self) -> Option<&Value> {
        self.registry.get(self.type_name()).or_else(|| {
            warn!(
                type_name = %self.type_name(),
                "Schema missing for type - mutation paths may be incomplete"
            );
            None
        })
    }

    /// Look up a type in the registry
    pub fn get_registry_type_schema(&self, type_name: &BrpTypeName) -> Option<&Value> {
        self.registry.get(type_name)
    }

    /// Create a new context for a child element (field, array element, tuple element)
    pub fn create_field_context(&self, path_kind: PathKind) -> Self {
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
        }
    }

    /// Check if a value type has serialization support
    /// Used to determine if opaque Value types like String can be mutated
    pub fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
        self.get_registry_type_schema(type_name)
            .is_some_and(|schema| {
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

    /// Extract element type from List or Array schema
    pub fn extract_list_element_type(schema: &Value) -> Option<BrpTypeName> {
        schema
            .get("items")
            .and_then(SchemaField::extract_field_type)
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
