//! Mutation support status types for type schema analysis
use std::fmt::Display;

use super::super::response_types::BrpTypeName;

/// Represents detailed mutation support status for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationSupport {
    /// Type fully supports mutation operations
    Supported,
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
}

impl MutationSupport {
    /// Extract the deepest failing type from nested error contexts
    pub fn get_deepest_failing_type(&self) -> Option<BrpTypeName> {
        match self {
            Self::Supported => None,
            Self::MissingSerializationTraits(type_name)
            | Self::NotInRegistry(type_name)
            | Self::RecursionLimitExceeded(type_name) => Some(type_name.clone()),
            Self::NonMutatableHandle { element_type, .. } => Some(element_type.clone()),
        }
    }
}

impl Display for MutationSupport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Supported => write!(f, "Type supports mutation"),
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
        }
    }
}

/// Convert `MutationSupport` to structured error reason string
impl From<&MutationSupport> for Option<String> {
    fn from(support: &MutationSupport) -> Self {
        match support {
            MutationSupport::Supported => None,
            MutationSupport::MissingSerializationTraits(_) => {
                Some("missing_serialization_traits".to_string())
            }
            MutationSupport::NonMutatableHandle { .. } => {
                Some("handle_wrapper_component".to_string())
            }
            MutationSupport::NotInRegistry(_) => Some("not_in_registry".to_string()),
            MutationSupport::RecursionLimitExceeded(_) => {
                Some("recursion_limit_exceeded".to_string())
            }
        }
    }
}
