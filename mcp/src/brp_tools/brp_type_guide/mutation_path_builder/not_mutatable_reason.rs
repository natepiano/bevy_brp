//! Mutation support status types for type schema analysis
use std::fmt::Display;

use super::super::response_types::BrpTypeName;

/// Represents detailed mutation support status for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotMutatableReason {
    /// Type lacks required serialization traits
    MissingSerializationTraits(BrpTypeName),
    /// Container type has non-mutatable element type
    NonMutatableHandle {
        container_type: BrpTypeName,
        element_type:   BrpTypeName,
    },
    /// Type not found in registry
    NotInRegistry(BrpTypeName),
    /// Recursion depth limit exceeded during analysis
    RecursionLimitExceeded(BrpTypeName),
    /// HashMap or HashSet with complex (non-primitive) key type that cannot be mutated via BRP
    ComplexCollectionKey(BrpTypeName),
}

impl NotMutatableReason {
    /// Extract the deepest failing type from nested error contexts
    pub fn get_deepest_failing_type(&self) -> Option<BrpTypeName> {
        match self {
            Self::MissingSerializationTraits(type_name)
            | Self::NotInRegistry(type_name)
            | Self::RecursionLimitExceeded(type_name)
            | Self::ComplexCollectionKey(type_name) => Some(type_name.clone()),
            Self::NonMutatableHandle { element_type, .. } => Some(element_type.clone()),
        }
    }
}

impl Display for NotMutatableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSerializationTraits(type_name) => write!(
                f,
                "Type {type_name} lacks Serialize/Deserialize traits required for mutation"
            ),
            Self::NonMutatableHandle {
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
                "HashMap type {type_name} has complex (enum/struct) keys that cannot be mutated through BRP - JSON requires string keys but complex types cannot be reliably serialized/deserialized as HashMap keys"
            ),
        }
    }
}

/// Convert `MutationSupport` to structured error reason string
impl From<&NotMutatableReason> for Option<String> {
    fn from(support: &NotMutatableReason) -> Self {
        match support {
            NotMutatableReason::MissingSerializationTraits(_) => {
                Some("missing_serialization_traits".to_string())
            }
            NotMutatableReason::NonMutatableHandle { .. } => {
                Some("handle_wrapper_component".to_string())
            }
            NotMutatableReason::NotInRegistry(_) => Some("not_in_registry".to_string()),
            NotMutatableReason::RecursionLimitExceeded(_) => {
                Some("recursion_limit_exceeded".to_string())
            }
            NotMutatableReason::ComplexCollectionKey(_) => {
                Some("complex_collection_key".to_string())
            }
        }
    }
}
