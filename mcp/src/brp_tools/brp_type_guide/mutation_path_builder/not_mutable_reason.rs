//! Mutation support status types for type schema analysis
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::brp_type_name::BrpTypeName;
use super::types::{MutationStatus, PathSummary};

/// Path detail for mutable paths
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathDetail {
    pub path:      String,
    pub type_name: BrpTypeName,
}

/// Path detail with reason for not/partially mutable paths
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathDetailWithReason {
    pub path:      String,
    pub type_name: BrpTypeName,
    pub reason:    String,
}

/// Represents detailed mutation support status for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotMutableReason {
    /// Type lacks required serialization traits
    MissingSerializationTraits(BrpTypeName),
    /// Container type has non-mutable element type
    NonMutableHandle {
        container_type: BrpTypeName,
        element_type:   BrpTypeName,
    },
    /// Type not found in registry
    NotInRegistry(BrpTypeName),
    /// Recursion depth limit exceeded during analysis
    RecursionLimitExceeded(BrpTypeName),
    /// `HashMap` or `HashSet` with complex (non-primitive) key type that cannot be mutated via BRP
    ComplexCollectionKey(BrpTypeName),
    /// All child paths are `NotMutable`
    NoMutableChildren { parent_type: BrpTypeName },
    /// Type has serialization support but no example value available
    NoExampleAvailable(BrpTypeName),
    /// Some children are mutable, others are not (results in `PartiallyMutable`)
    PartialChildMutability {
        parent_type:             BrpTypeName,
        mutable_paths:           Vec<PathDetail>,
        not_mutable_paths:       Vec<PathDetailWithReason>,
        partially_mutable_paths: Vec<PathDetailWithReason>,
    },
}

impl NotMutableReason {
    /// Extract the deepest failing type from nested error contexts
    pub fn get_deepest_failing_type(&self) -> BrpTypeName {
        match self {
            Self::MissingSerializationTraits(type_name)
            | Self::NotInRegistry(type_name)
            | Self::RecursionLimitExceeded(type_name)
            | Self::ComplexCollectionKey(type_name)
            | Self::NoExampleAvailable(type_name) => type_name.clone(),
            Self::NonMutableHandle { element_type, .. } => element_type.clone(),
            Self::NoMutableChildren { parent_type }
            | Self::PartialChildMutability { parent_type, .. } => parent_type.clone(),
        }
    }

    /// Construct `PartialChildMutability` from path summaries
    pub fn from_partial_mutability(parent_type: BrpTypeName, summaries: Vec<PathSummary>) -> Self {
        let mut mutable_paths = Vec::new();
        let mut not_mutable_paths = Vec::new();
        let mut partially_mutable_paths = Vec::new();

        for summary in summaries {
            match summary.status {
                MutationStatus::Mutable => {
                    mutable_paths.push(PathDetail {
                        path:      summary.path,
                        type_name: summary.type_name,
                    });
                }
                MutationStatus::NotMutable => {
                    // Extract reason string from Value if present
                    let reason = summary
                        .reason
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "unknown".to_string());

                    not_mutable_paths.push(PathDetailWithReason {
                        path: summary.path,
                        type_name: summary.type_name,
                        reason,
                    });
                }
                MutationStatus::PartiallyMutable => {
                    // Extract reason string from Value if present
                    let reason = summary
                        .reason
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "partial".to_string());

                    partially_mutable_paths.push(PathDetailWithReason {
                        path: summary.path,
                        type_name: summary.type_name,
                        reason,
                    });
                }
            }
        }

        Self::PartialChildMutability {
            parent_type,
            mutable_paths,
            not_mutable_paths,
            partially_mutable_paths,
        }
    }
}

impl Display for NotMutableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSerializationTraits(type_name) => write!(
                f,
                "Type {type_name} lacks Serialize/Deserialize traits required for mutation"
            ),
            Self::NonMutableHandle {
                container_type,
                element_type,
            } => write!(
                f,
                "Type `{container_type}` is a TupleStruct wrapper around `{element_type}` which lacks the `ReflectDeserialize` type data required for mutation"
            ),
            Self::NotInRegistry(type_name) => {
                write!(f, "Type {type_name} not found in schema registry")
            }
            Self::RecursionLimitExceeded(type_name) => write!(
                f,
                "Type {type_name} analysis exceeded maximum recursion depth"
            ),
            Self::ComplexCollectionKey(type_name) => write!(
                f,
                "HashMap type {type_name} has complex (enum/struct) keys that cannot be mutated through BRP - JSON requires string keys but complex types cannot currently be used with HashMap or HashSet"
            ),
            Self::NoMutableChildren { parent_type } => write!(
                f,
                "Type {parent_type} has no mutable child paths - all children lack required traits"
            ),
            Self::NoExampleAvailable(type_name) => write!(
                f,
                "Type {type_name} has serialization support but no example value is available for mutations"
            ),
            Self::PartialChildMutability { parent_type, .. } => write!(
                f,
                "Type {parent_type} has partial child mutability - some children can be mutated, others cannot"
            ),
        }
    }
}

/// Convert `NotMutableReason` to structured JSON value with detailed explanation
impl From<&NotMutableReason> for Option<Value> {
    fn from(reason: &NotMutableReason) -> Self {
        match reason {
            // Simple variants return strings
            NotMutableReason::MissingSerializationTraits(_) => Some(Value::String(format!(
                "missing_serialization_traits: {reason}"
            ))),
            NotMutableReason::NonMutableHandle { .. } => {
                Some(Value::String(format!("handle_wrapper_component: {reason}")))
            }
            NotMutableReason::NotInRegistry(_) => {
                Some(Value::String(format!("not_in_registry: {reason}")))
            }
            NotMutableReason::RecursionLimitExceeded(_) => {
                Some(Value::String(format!("recursion_limit_exceeded: {reason}")))
            }
            NotMutableReason::ComplexCollectionKey(_) => {
                Some(Value::String(format!("complex_collection_key: {reason}")))
            }
            NotMutableReason::NoMutableChildren { .. } => {
                Some(Value::String(format!("no_mutable_children: {reason}")))
            }
            NotMutableReason::NoExampleAvailable(_) => {
                Some(Value::String(format!("no_example_available: {reason}")))
            }
            // PartialChildMutability returns structured JSON
            NotMutableReason::PartialChildMutability {
                parent_type,
                mutable_paths,
                not_mutable_paths,
                partially_mutable_paths,
            } => Some(json!({
                "parent_type": parent_type,
                "mutable": mutable_paths,
                "not_mutable": not_mutable_paths,
                "partially_mutable": partially_mutable_paths,
            })),
        }
    }
}
