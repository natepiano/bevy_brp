//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

use serde_json::Value;
use strum::{Display, EnumString};

use crate::brp_tools::brp_type_schema::BrpTypeName;
use crate::error::Error;
use crate::tool::{BrpMethod, ParameterName};

/// Can we use this data to attempt a correction
#[derive(Debug, Clone)]
pub enum Correction {
    /// Correction can be successfully applied
    Candidate { correction_info: CorrectionInfo },
    /// Correction cannot be applied but metadata was discovered
    Uncorrectable {
        type_name: BrpTypeName,
        reason:    String,
    },
}

/// Determine if corrections are retryable (should go to Retry state) or educational only (should go
/// to Guidance state)
///
/// This function evaluates a collection of corrections to determine the appropriate terminal state.
/// Returns `true` if corrections contain actionable values that can be used for retrying the BRP
/// call, `false` if corrections are educational/metadata only.
pub fn are_corrections_retryable(corrections: &[Correction]) -> bool {
    // Any Candidate correction is retryable by definition
    corrections
        .iter()
        .any(|correction| matches!(correction, Correction::Candidate { .. }))
}

/// Information about a format correction applied during recovery
#[derive(Debug, Clone)]
pub struct CorrectionInfo {
    /// Corrected value to use
    pub corrected_value: Value,
    /// Human-readable explanation of the correction
    pub hint:            String,
    /// Type name for this correction
    pub type_name:       BrpTypeName,
    /// Original value that was incorrect
    pub original_value:  Value,
}

impl CorrectionInfo {
    /// Convert to JSON representation for API compatibility
    /// Note: Additional metadata (`mutation_paths`, `type_category`) should be added by the caller
    /// who has access to the full `DiscoveryContext`
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            FormatCorrectionField::TypeName: self.type_name.as_str(),
            FormatCorrectionField::OriginalFormat: self.original_value,
            FormatCorrectionField::CorrectedFormat: self.corrected_value,
            FormatCorrectionField::Hint: self.hint
        })
    }
}

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
    /// Corrected format to use instead
    CorrectedFormat,
    /// Examples of valid usage
    Examples,
    /// Human-readable hint for using the corrected format
    Hint,
    /// Available mutation paths for this component
    MutationPaths,
    /// Original format that was incorrect
    OriginalFormat,
    /// Path for mutation operations
    Path,
    /// Category of the component type (e.g., "Component", "Resource")
    TypeCategory,
    /// Full type name (path) of the type being corrected
    TypeName,
    /// Valid values for enum fields
    ValidValues,
    /// Value for mutation operations
    Value,
}

impl From<FormatCorrectionField> for String {
    fn from(field: FormatCorrectionField) -> Self {
        field.as_ref().to_string()
    }
}

/// Status of format correction attempts
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCorrectionStatus {
    /// Format discovery was not enabled for this request
    NotApplicable,
    /// No format correction was attempted
    NotAttempted,
    /// Format correction was applied and the operation succeeded
    Succeeded,
}

/// Type of BRP operation being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Operations that create or replace entire components/resources
    /// Includes: `BevySpawn`, `BevyInsert`, `BevyInsertResource`
    /// Serializes as: `spawn_insert`
    SpawnInsert {
        /// Which parameter name to use when building requests
        /// Components for `BevySpawn`/`BevyInsert`, Value for `BevyInsertResource`
        #[serde(skip)]
        parameter_name: ParameterName,
    },
    /// Operations that modify specific fields
    /// Includes: `BevyMutateComponent`, `BevyMutateResource`
    /// Serializes as: `mutate`
    Mutate {
        /// Which parameter name to use when building requests
        /// Component for `BevyMutateComponent`, Resource for `BevyMutateResource`
        #[serde(skip)]
        parameter_name: ParameterName,
    },
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use consistent string representation for both variants
        let s = match self {
            Self::SpawnInsert { .. } => "spawn_insert",
            Self::Mutate { .. } => "mutate",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<BrpMethod> for Operation {
    type Error = Error;

    fn try_from(method: BrpMethod) -> Result<Self, Self::Error> {
        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => Ok(Self::SpawnInsert {
                parameter_name: ParameterName::Components,
            }),

            BrpMethod::BevyInsertResource => Ok(Self::SpawnInsert {
                parameter_name: ParameterName::Value,
            }),

            BrpMethod::BevyMutateComponent => Ok(Self::Mutate {
                parameter_name: ParameterName::Component,
            }),

            BrpMethod::BevyMutateResource => Ok(Self::Mutate {
                parameter_name: ParameterName::Resource,
            }),

            _ => Err(Error::InvalidArgument(format!(
                "Method {method:?} is not supported for format discovery"
            ))),
        }
    }
}
