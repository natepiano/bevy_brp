mod builders;
mod mutation_support;
mod path_kind;
mod recursion_context;
mod type_kind;
mod types;

pub use builders::{EnumMutationBuilder, EnumVariantInfo, build_all_enum_examples};
pub use mutation_support::MutationSupport;
pub use recursion_context::{RecursionContext, RootOrField};
pub use type_kind::TypeKind;
pub use types::{MutationPath, MutationPathInternal};

use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::error::Result;

/// Trait for building mutation paths for different type kinds
///
/// This trait provides type-directed dispatch for mutation path building,
/// replacing the large conditional match statement with clean separation of concerns.
/// Each type kind gets its own implementation that handles the specific logic needed.
pub trait MutationPathBuilder {
    /// Build mutation paths with depth tracking for recursion safety
    ///
    /// This method takes a `MutationPathContext` which provides all necessary information
    /// including the registry, wrapper info, and enum variants, plus a `RecursionDepth`
    /// parameter to track recursion depth and prevent infinite loops.
    ///
    /// Returns a `Result` containing a vector of `MutationPathInternal` representing
    /// all possible mutation paths, or an error if path building failed.
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>>;
}
