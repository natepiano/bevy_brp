use super::mutation_path_external::RootExample;
use super::variant_name::VariantName;

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
