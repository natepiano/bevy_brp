//! Mutation support status types for type schema analysis
use std::fmt::Display;

use super::super::response_types::BrpTypeName;

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
    /// Some children are mutable, others are not (results in `PartiallyMutable`)
    PartialChildMutability { parent_type: BrpTypeName },
}

impl NotMutableReason {
    /// Extract the deepest failing type from nested error contexts
    pub fn get_deepest_failing_type(&self) -> BrpTypeName {
        match self {
            Self::MissingSerializationTraits(type_name)
            | Self::NotInRegistry(type_name)
            | Self::RecursionLimitExceeded(type_name)
            | Self::ComplexCollectionKey(type_name) => type_name.clone(),
            Self::NonMutableHandle { element_type, .. } => element_type.clone(),
            Self::NoMutableChildren { parent_type }
            | Self::PartialChildMutability { parent_type } => parent_type.clone(),
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
            Self::PartialChildMutability { parent_type } => write!(
                f,
                "Type {parent_type} has partial child mutability - some children can be mutated, others cannot"
            ),
        }
    }
}

/// Convert `MutationSupport` to structured error reason string with detailed explanation
impl From<&NotMutableReason> for Option<String> {
    fn from(support: &NotMutableReason) -> Self {
        match support {
            NotMutableReason::MissingSerializationTraits(_) => {
                Some(format!("missing_serialization_traits: {support}"))
            }
            NotMutableReason::NonMutableHandle { .. } => {
                Some(format!("handle_wrapper_component: {support}"))
            }
            NotMutableReason::NotInRegistry(_) => Some(format!("not_in_registry: {support}")),
            NotMutableReason::RecursionLimitExceeded(_) => {
                Some(format!("recursion_limit_exceeded: {support}"))
            }
            NotMutableReason::ComplexCollectionKey(_) => {
                Some(format!("complex_collection_key: {support}"))
            }
            NotMutableReason::NoMutableChildren { .. } => {
                Some(format!("no_mutable_children: {support}"))
            }
            NotMutableReason::PartialChildMutability { .. } => {
                Some(format!("partial_child_mutability: {support}"))
            }
        }
    }
}
