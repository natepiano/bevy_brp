//! Mutation support status types for type schema analysis
//!
//! This module implements an internal control flow mechanism where [`NotMutableReason`] acts as
//! structured error information that gets converted to user-facing output rather than propagated
//! as actual errors. The pattern enables clean separation between genuine errors (registry
//! failures, parsing errors) and expected "not mutable" conditions that are part of normal
//! operation.
//!
//! ## Control Flow Pattern
//!
//! Internal builders return [`MutationResult`] which uses [`NotMutableReason`] as the error type:
//! ```rust,ignore
//! pub(super) type MutationResult = Result<Vec<MutationPathInternal>, NotMutableReason>;
//! ```
//!
//! When a type cannot be mutated (missing `Reflect`, recursion limits, etc.), builders return
//! `Err(NotMutableReason::*)` rather than continuing processing. This gets caught at the choke
//! point in `recurse_mutation_paths()` and converted to user output via `build_not_mutable_path()`.
//!
//! This design allows:
//! - Clean early returns from deeply nested recursion
//! - Rich diagnostic information in the final output
//! - Clear separation between system errors and expected "not mutable" states
//! - Consistent formatting of all "not mutable" paths
use std::fmt::Display;

use serde_json::Value;
use serde_json::json;

use super::super::brp_type_name::BrpTypeName;
use super::types::Mutability;
use super::types::MutabilityIssue;

/// Represents detailed mutation support status for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotMutableReason {
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
    /// Leaf type registered in schema but has no hardcoded example value
    NoExampleAvailable(BrpTypeName),
    /// Some children are mutable, others are not (results in `PartiallyMutable`)
    PartialChildMutability {
        parent_type:       BrpTypeName,
        message:           String,
        mutable:           Vec<String>,
        not_mutable:       Vec<String>,
        partially_mutable: Vec<String>,
    },
}

impl NotMutableReason {
    /// Extract the deepest failing type from nested error contexts
    pub fn get_deepest_failing_type(&self) -> BrpTypeName {
        match self {
            Self::NotInRegistry(type_name)
            | Self::RecursionLimitExceeded(type_name)
            | Self::ComplexCollectionKey(type_name)
            | Self::NoExampleAvailable(type_name) => type_name.clone(),
            Self::NonMutableHandle { element_type, .. } => element_type.clone(),
            Self::NoMutableChildren { parent_type }
            | Self::PartialChildMutability { parent_type, .. } => parent_type.clone(),
        }
    }

    /// Construct `PartialChildMutability` from mutability issues
    ///
    /// # Deduplication Logic
    ///
    /// When the same path string appears multiple times with different statuses (which happens
    /// with enum variants where the same child path exists in multiple variants), this function
    /// deduplicates them and categorizes as `partially_mutable`.
    ///
    /// Example: `Handle<Image>` enum has two variants at `.0`:
    /// - `Strong` variant: `.0` is `not_mutable` (Arc<StrongHandle> not in registry)
    /// - `Uuid` variant: `.0` is `mutable` (Uuid has hardcoded knowledge)
    ///
    /// Both create the same path string `.color_lut.0.0`, so this function detects the
    /// conflict and correctly marks it as `partially_mutable`.
    pub fn from_partial_mutability(
        parent_type: BrpTypeName,
        mutability_issues: Vec<MutabilityIssue>,
        message: String,
    ) -> Self {
        use std::collections::HashMap;
        use std::collections::HashSet;

        // First pass: Collect all statuses for each unique path string
        // This detects when the same path appears with different statuses across variants
        let mut path_statuses: HashMap<String, HashSet<Mutability>> = HashMap::new();

        for mutability_issue in mutability_issues {
            let path_str = mutability_issue.target.to_string();
            path_statuses
                .entry(path_str)
                .or_default()
                .insert(mutability_issue.status);
        }

        // Second pass: Categorize each unique path based on its status diversity
        let mut mutable = Vec::new();
        let mut not_mutable = Vec::new();
        let mut partially_mutable = Vec::new();

        // Sort path_statuses for deterministic ordering
        let mut sorted_paths: Vec<_> = path_statuses.into_iter().collect();
        sorted_paths.sort_by(|a, b| a.0.cmp(&b.0));

        for (path_str, statuses) in sorted_paths {
            if statuses.len() > 1 {
                // Path has conflicting statuses across different variants → partially_mutable
                // Example: `.color_lut.0.0` is mutable in Uuid variant but not_mutable in Strong
                // variant
                partially_mutable.push(path_str);
            } else if let Some(&status) = statuses.iter().next() {
                // Path has consistent status across all variants → use that status
                match status {
                    Mutability::Mutable => mutable.push(path_str),
                    Mutability::NotMutable => not_mutable.push(path_str),
                    Mutability::PartiallyMutable => partially_mutable.push(path_str),
                }
            }
        }

        Self::PartialChildMutability {
            parent_type,
            message,
            mutable,
            not_mutable,
            partially_mutable,
        }
    }
}

impl Display for NotMutableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonMutableHandle {
                container_type,
                element_type,
            } => write!(
                f,
                "`{container_type}` is a TupleStruct wrapper around `{element_type}` which lacks the `ReflectDeserialize` type data required for mutation"
            ),
            Self::NotInRegistry(type_name) => {
                write!(f, "`{type_name}` not found in schema registry")
            },
            Self::RecursionLimitExceeded(type_name) => {
                write!(f, "`{type_name}` analysis exceeded maximum recursion depth")
            },
            Self::ComplexCollectionKey(type_name) => write!(
                f,
                "HashMap `{type_name}` has complex (enum/struct) keys that cannot be mutated through BRP - JSON requires string keys but complex types cannot currently be used with HashMap or HashSet"
            ),
            Self::NoMutableChildren { parent_type } => {
                write!(f, "`{parent_type}` has no mutable child paths")
            },
            Self::NoExampleAvailable(type_name) => write!(
                f,
                "`{type_name}` is registered in the schema but has no discoverable example value available for mutations. If you look up the type definition yourself you may be able to use it to mutate this type directly."
            ),
            Self::PartialChildMutability { parent_type, .. } => write!(
                f,
                "`{parent_type}` has partial child mutability - some children can be mutated, others cannot"
            ),
        }
    }
}

/// Convert `NotMutableReason` to structured JSON value with detailed explanation
impl From<&NotMutableReason> for Option<Value> {
    fn from(reason: &NotMutableReason) -> Self {
        match reason {
            NotMutableReason::NonMutableHandle { .. }
            | NotMutableReason::NotInRegistry(_)
            | NotMutableReason::RecursionLimitExceeded(_)
            | NotMutableReason::ComplexCollectionKey(_)
            | NotMutableReason::NoMutableChildren { .. }
            | NotMutableReason::NoExampleAvailable(_) => Some(Value::String(format!("{reason}"))),
            // PartialChildMutability returns structured JSON
            NotMutableReason::PartialChildMutability {
                parent_type: _,
                message,
                mutable,
                not_mutable,
                partially_mutable,
            } => {
                let mut reason = serde_json::Map::new();
                reason.insert("message".to_string(), json!(message));

                if !mutable.is_empty() {
                    reason.insert("mutable".to_string(), json!(mutable));
                }
                if !not_mutable.is_empty() {
                    reason.insert("not_mutable".to_string(), json!(not_mutable));
                }
                if !partially_mutable.is_empty() {
                    reason.insert("partially_mutable".to_string(), json!(partially_mutable));
                }

                Some(Value::Object(reason))
            },
        }
    }
}
