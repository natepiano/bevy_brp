//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::format_correction_fields::FormatCorrectionField;
use super::unified_types::UnifiedTypeInfo;
use crate::error::Error;
use crate::tool::{BrpMethod, ParameterName};

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrpTypeName(String);

impl BrpTypeName {
    /// Get the underlying string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrpTypeName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrpTypeName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for BrpTypeName {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl std::fmt::Display for BrpTypeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
            FormatCorrectionField::TypeName.as_ref(): self.type_info.type_name.as_str(),
            FormatCorrectionField::OriginalFormat.as_ref(): self.type_info.original_value,
            FormatCorrectionField::CorrectedFormat.as_ref(): self.corrected_value,
            FormatCorrectionField::Hint.as_ref(): self.hint
        });

        // Add rich metadata fields
        if let Some(obj) = correction_json.as_object_mut() {
            // Extract mutation_paths
            if !self.type_info.format_info.mutation_paths.is_empty() {
                let paths: Vec<String> = self
                    .type_info
                    .format_info
                    .mutation_paths
                    .keys()
                    .cloned()
                    .collect();
                obj.insert(
                    FormatCorrectionField::MutationPaths.as_ref().to_string(),
                    serde_json::json!(paths),
                );
            }

            // Extract type_category
            obj.insert(
                FormatCorrectionField::TypeCategory.as_ref().to_string(),
                serde_json::json!(format!("{:?}", self.type_info.type_category)),
            );

            // Extract discovery_source (always present now)
            obj.insert(
                FormatCorrectionField::DiscoverySource.as_ref().to_string(),
                serde_json::json!(format!("{:?}", self.type_info.discovery_source)),
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

/// Method used to discover or correct a type format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoverySource {
    /// Information from Bevy's type registry
    TypeRegistry,
    /// Information inferred from error patterns
    PatternMatching,
    /// Information combined from registry and extras sources
    RegistryPlusExtras,
}

/// Information about an enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    /// The name of the variant
    pub name:         String,
    /// The type of the variant (Unit, Tuple, Struct)
    pub variant_type: String,
}

/// Information about an enum type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    /// List of enum variants
    pub variants: Vec<EnumVariant>,
}

/// Format-specific information and examples
#[derive(Debug, Clone, Serialize)]
pub struct FormatInfo {
    /// Real example values for different BRP operations
    pub examples:         HashMap<Operation, Value>,
    /// Available mutation paths if the type supports mutation
    pub mutation_paths:   HashMap<String, String>,
    /// Original format that caused the error (if applicable)
    pub original_format:  Option<Value>,
    /// Corrected format to use instead
    pub corrected_format: Option<Value>,
}

/// Registry and reflection status for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatus {
    /// Whether the type is registered in Bevy's type registry
    pub in_registry: bool,
    /// Whether the type has reflection support
    pub has_reflect: bool,
    /// Type path as registered in the registry
    pub type_path:   Option<String>,
}

/// Serialization trait support for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationSupport {
    /// Whether the type implements Serialize
    pub has_serialize:   bool,
    /// Whether the type implements Deserialize
    pub has_deserialize: bool,
    /// Whether the type can be used in BRP operations requiring serialization
    pub brp_compatible:  bool,
}

/// Result of a transformation operation containing the corrected value and a hint
#[derive(Debug, Clone)]
pub struct TransformationResult {
    /// The corrected value after transformation
    pub corrected_value: Value,
    /// Human-readable hint about the transformation
    pub hint:            String,
}

/// Category of type for quick identification and processing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TypeCategory {
    /// Unknown or unclassified type
    Unknown,
    /// Regular struct type
    Struct,
    /// Tuple struct type
    TupleStruct,
    /// Enum type
    Enum,
    /// Math type (Vec2, Vec3, Quat, etc.)
    MathType,
    /// Component type
    Component,
}

impl From<&str> for TypeCategory {
    fn from(s: &str) -> Self {
        match s {
            "Struct" => Self::Struct,
            "TupleStruct" => Self::TupleStruct,
            "Enum" => Self::Enum,
            _ => Self::Unknown,
        }
    }
}
