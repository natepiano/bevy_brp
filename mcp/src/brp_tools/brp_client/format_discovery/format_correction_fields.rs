use strum::{Display, EnumString};

use crate::tool::JsonFieldAccess;

/// Format correction field names enum for type-safe field access
///
/// This enum provides compile-time safety for format correction field names
/// used throughout the BRP tools handler, response builder, and components.
/// Using strum's `IntoStaticStr` derive allows `.into()` to get string representation.
/// Using strum's `AsRefStr` derive allows `.as_ref()` to get string representation.
#[derive(Display, EnumString, Clone, Copy, Debug, strum::IntoStaticStr, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum FormatCorrectionField {
    /// Available mutation paths
    AvailablePaths,
    /// Error code
    Code,
    /// Component type being corrected
    Component,
    /// Corrected format to use instead
    CorrectedFormat,
    /// Source of the type discovery (e.g., "`TypeRegistry`", "`PatternMatching`",
    /// "`RegistryPlusExtras`")
    DiscoverySource,
    /// Error data in error responses
    ErrorData,
    /// Examples of valid usage
    Examples,
    /// Status indicating if format correction was applied
    FormatCorrected,
    /// Array of format corrections that were applied
    FormatCorrections,
    /// Human-readable hint for using the corrected format
    Hint,
    /// Available mutation paths for this component
    MutationPaths,
    /// Original error message when enhanced
    OriginalError,
    /// Original format that was incorrect
    OriginalFormat,
    /// Path for mutation operations
    Path,
    /// Operations supported by this component type
    SupportedOperations,
    /// Category of the component type (e.g., "Component", "Resource")
    TypeCategory,
    /// Full type name (path) of the type being corrected
    TypeName,
    /// Valid values for enum fields
    ValidValues,
    /// Value for mutation operations
    Value,
}

impl JsonFieldAccess for FormatCorrectionField {}
