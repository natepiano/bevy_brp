use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::mutability::Mutability;
use super::variant_name::VariantName;
use crate::brp_tools::brp_type_guide::variant_signature::VariantSignature;

/// Example group for enum variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ExampleGroup {
    /// List of variants that share this signature
    pub(super) applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for variants with a field that has no complete
    /// example)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) example:             Option<Value>,
    /// The variant signature (`Unit`, `Tuple`, or `Struct`)
    pub(super) signature:           VariantSignature,
    /// Mutation status for this signature/variant group
    pub(super) mutability:          Mutability,
}

impl ExampleGroup {
    /// Rank this group for choosing an enum's example; see `VariantPreference`
    pub(super) const fn preference(&self) -> VariantPreference {
        if self.example.is_none() {
            return VariantPreference::Unconstructible;
        }
        match (&self.signature, self.mutability) {
            (VariantSignature::Unit, _) => VariantPreference::Unit,
            (_, Mutability::Mutable) => VariantPreference::MutableFields,
            (_, Mutability::PartiallyMutable | Mutability::NotMutable) => {
                VariantPreference::ConstructibleFields
            },
        }
    }
}

/// Order in which an enum's variant groups supply its example (`select_preferred_example`: the
/// enum's spawn and parent example), first to last
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum VariantPreference {
    /// Tuple or struct variant whose fields are all `Mutable`
    MutableFields,
    /// Tuple or struct variant with non-mutable descendants whose fields all have complete
    /// examples (e.g. a field holding an enum with a `Mutable` variant)
    ConstructibleFields,
    /// Unit variant
    Unit,
    /// Variant without an example
    Unconstructible,
}
