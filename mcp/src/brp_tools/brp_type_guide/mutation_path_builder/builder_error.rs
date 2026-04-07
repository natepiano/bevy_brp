use error_stack::Report;

use super::not_mutable_reason::NotMutableReason;
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
    fn from(e: Report<Error>) -> Self { Self::SystemError(e) }
}

impl From<NotMutableReason> for BuilderError {
    fn from(reason: NotMutableReason) -> Self { Self::NotMutable(reason) }
}
