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
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) example:             Option<Value>,
    /// The variant signature (`Unit`, `Tuple`, or `Struct`)
    pub(super) signature:           VariantSignature,
    /// Mutation status for this signature/variant group
    pub(super) mutability:          Mutability,
}
