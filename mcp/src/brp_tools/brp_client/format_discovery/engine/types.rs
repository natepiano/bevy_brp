//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::format_correction_fields::FormatCorrectionField;
use super::unified_types::UnifiedTypeInfo;
use crate::error::Error;
use crate::tool::{BrpMethod, ParameterName};

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

/// Can we use this data to attempt a correction
#[derive(Debug, Clone)]
pub enum Correction {
    /// Correction can be successfully applied
    Candidate { correction_info: CorrectionInfo },
    /// Correction cannot be applied but metadata was discovered
    Uncorrectable {
        type_info: UnifiedTypeInfo,
        reason:    String,
    },
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
    /// Format correction was attempted but the operation still failed
    AttemptedButFailed,
}

/// Information about a format correction applied during recovery
#[derive(Debug, Clone, Serialize)]
pub struct CorrectionInfo {
    /// Corrected value to use
    pub corrected_value:   Value,
    /// Human-readable explanation of the correction
    pub hint:              String,
    /// format information for error responses (usage, `valid_values`, examples)
    pub corrected_format:  Option<Value>,
    /// Type information discovered during correction
    pub type_info:         UnifiedTypeInfo,
    /// The correction method used
    pub correction_method: CorrectionMethod,
}

impl CorrectionInfo {
    /// Convert to JSON representation for API compatibility
    pub fn to_json(&self) -> Value {
        let mut correction_json = serde_json::json!({
            FormatCorrectionField::TypeName: self.type_info.type_name().as_str(),
            FormatCorrectionField::OriginalFormat: self.type_info.original_value,
            FormatCorrectionField::CorrectedFormat: self.corrected_value,
            FormatCorrectionField::Hint: self.hint
        });

        // Add rich metadata fields
        if let Some(obj) = correction_json.as_object_mut() {
            // Extract mutation_paths
            if !self.type_info.mutation_paths().is_empty() {
                let paths: Vec<String> = self.type_info.mutation_paths().keys().cloned().collect();
                obj.insert(
                    String::from(FormatCorrectionField::MutationPaths),
                    serde_json::json!(paths),
                );
            }

            // Extract type_category
            let type_kind = self
                .type_info
                .type_info
                .schema_info
                .as_ref()
                .and_then(|s| s.type_kind.clone())
                .unwrap_or_else(|| {
                    // Fallback: determine from enum_info
                    use crate::brp_tools::brp_type_schema::TypeKind;
                    if self.type_info.enum_info().is_some() {
                        TypeKind::Enum
                    } else {
                        TypeKind::Struct
                    }
                });
            obj.insert(
                String::from(FormatCorrectionField::TypeCategory),
                serde_json::json!(format!("{:?}", type_kind)),
            );
        }

        correction_json
    }
}

/// Method used to correct a format error
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CorrectionMethod {
    /// Direct replacement based on exact type knowledge
    DirectReplacement,
    /// Object to array conversion for math types
    ObjectToArray,
    /// Array to object conversion
    ArrayToObject,
    /// String to enum variant conversion
    StringToEnum,
    /// Nested structure correction
    NestedCorrection,
    /// Field name mapping or aliasing
    FieldMapping,
}

/// Format-specific information for correction
#[derive(Debug, Clone, Serialize, Default)]
pub struct FormatInfo {
    /// Original format that caused the error (if applicable)
    pub original_format:  Option<Value>,
    /// Corrected format to use instead
    pub corrected_format: Option<Value>,
}
