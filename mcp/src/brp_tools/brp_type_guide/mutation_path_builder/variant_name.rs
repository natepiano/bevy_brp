//! Variant-name newtype used throughout enum mutation-path handling.

use serde::Deserialize;
use serde::Serialize;

/// Newtype for variant name from a Bevy enum type (e.g., "`Option<String>::Some`",
/// "`Color::Srgba`")
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct VariantName(String);

impl From<String> for VariantName {
    fn from(name: String) -> Self { Self(name) }
}

impl std::fmt::Display for VariantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl VariantName {
    /// Get just the short variant name without the enum prefix (e.g., "Srgba" from
    /// "`Color::Srgba`")
    pub fn short_name(&self) -> &str { self.0.rsplit_once("::").map_or(&self.0, |(_, name)| name) }
}
