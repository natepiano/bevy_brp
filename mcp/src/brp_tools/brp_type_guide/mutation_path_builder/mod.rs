mod api;
mod enum_builder;
mod mutation_path_internal;
mod new_types;
mod not_mutable_reason;
mod path_builder;
mod path_kind;
mod recursion_context;
mod support;
mod type_kind_builders;
mod type_parser;
mod types;

// public use
pub(super) use api::{build_mutation_paths, extract_spawn_format};
pub(super) use enum_builder::VariantSignature;
use error_stack::Report;
use not_mutable_reason::NotMutableReason;
pub(super) use types::MutationPathExternal;

// local use
use crate::error::Error;

/// Internal error type for mutation path building that preserves semantic information.
///
/// This enum replaces the `MutationResult` type alias to properly handle both expected
/// mutation limitations (`NotMutableReason`) and actual system errors. The `BuilderError`
/// flows through all internal functions without conversion. Only at the module's public
/// interface in `recurse_mutation_paths()` do we convert `BuilderError` appropriately:
/// - `NotMutable` variants become success with `NotMutable` status
/// - `SystemError` variants propagate as errors
///
/// This design ensures that semantic information about why types cannot be mutated
/// is preserved throughout the internal processing and properly communicated to users.
#[derive(Debug)]
pub(super) enum BuilderError {
    NotMutable(NotMutableReason),
    SystemError(Report<Error>),
}

impl From<Report<Error>> for BuilderError {
    fn from(e: Report<Error>) -> Self {
        Self::SystemError(e)
    }
}

impl From<NotMutableReason> for BuilderError {
    fn from(reason: NotMutableReason) -> Self {
        Self::NotMutable(reason)
    }
}
