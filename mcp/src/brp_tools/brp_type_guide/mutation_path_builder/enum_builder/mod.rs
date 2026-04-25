mod enum_path_builder;
mod variant_kind;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::BuilderError;
use super::mutability::Mutability;
use super::mutation_path_external::RootExample;
use super::mutation_path_internal::MutationPathInternal;
use super::path_example::Example;
use super::recursion_context::RecursionContext;
use super::variant_name::VariantName;
use crate::brp_tools::brp_type_guide::variant_signature::VariantSignature;

/// Example group for enum variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ExampleGroup {
    /// List of variants that share this signature
    pub(super) applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) example:             Option<Value>,
    /// The variant signature (`Unit`, `Tuple`, or `Struct`)
    pub(super) signature:           VariantSignature,
    /// Mutation status for this signature/variant group
    pub(super) mutability:          Mutability,
}

/// Consolidated enum-specific data for mutation paths.
///
/// Added to a `MutationPathInternal` whenever that path is nested in an enum,
/// i.e. `!ctx.variant_chain.is_empty()` - whenever we have a variant chain.
#[derive(Debug, Clone)]
pub(super) struct EnumPathInfo {
    /// Chain of enum variants from root to this path
    pub(super) variant_chain: Vec<VariantName>,

    /// All variants that share the same signature and support this path
    pub(super) applicable_variants: Vec<VariantName>,

    /// Root example enum - handles mutual exclusivity
    ///
    /// Available: Complete root example for this specific variant chain
    /// Unavailable: Explanation for why `root_example` cannot be used to construct this variant
    /// via BRP.
    pub(super) root_example: Option<RootExample>,
}

pub(super) fn process_enum(
    ctx: &RecursionContext,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    enum_path_builder::process_enum(ctx)
}

pub(super) fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Example> {
    enum_path_builder::select_preferred_example(examples)
}
