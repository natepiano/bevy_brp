mod builder;
mod option_classification;
mod variant_kind;
mod variant_signature;

pub use builder::{process_enum, select_preferred_example};
pub use variant_signature::VariantSignature;
