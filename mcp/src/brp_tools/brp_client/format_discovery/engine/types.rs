//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

use std::ops::Deref;

use serde_json::Value;

use super::super::format_correction_fields::FormatCorrectionField;
use super::super::types::{Correction, CorrectionInfo};
use super::discovery_context::DiscoveryContext;
use crate::brp_tools::{BrpClientError, Port};
use crate::tool::BrpMethod;

/// Generic discovery engine with type state validation
///
/// The `State` parameter ensures that only valid operations can be called
/// for the current discovery phase.
pub struct DiscoveryEngine<State> {
    pub method:         BrpMethod,
    pub port:           Port,
    pub params:         Value,
    pub original_error: BrpClientError,
    #[allow(dead_code)] // Used in future phases when State contains data
    pub state: State,
}

/// Marker type for the `TypeDiscovery` state.
/// This state is responsible for creating the discovery context by calling
/// the registry and optional extras plugin.
pub struct TypeDiscovery;

/// State type for the `SerializationCheck` state.
/// This state holds a discovery context and is responsible for checking
/// if types have required serialization traits (Bevy 0.16 workaround).
pub struct SerializationCheck(pub DiscoveryContext);

/// State type for the `ExtrasDiscovery` state.
/// This state holds a discovery context and is responsible for building
/// corrections from extras data when no serialization issues are found.
#[allow(dead_code)]
pub struct ExtrasDiscovery(pub DiscoveryContext);

/// State type for the `PatternCorrection` state.
/// This state holds a discovery context and is responsible for applying
/// pattern-based corrections when extras discovery is unavailable or fails.
#[allow(dead_code)]
pub struct PatternCorrection(pub DiscoveryContext);

/// Terminal state for retryable corrections.
/// This state holds a discovery context and retryable corrections that can
/// be applied to modify parameters and retry the original BRP call.
pub struct Retry {
    pub context:     DiscoveryContext,
    pub corrections: Vec<Correction>, /* Only retryable corrections (Correction::Candidate with
                                       * real values) */
}

/// Terminal state for educational/guidance corrections.
/// This state holds a discovery context and educational corrections that
/// provide guidance but cannot be automatically retried.
pub struct Guidance {
    pub context:     DiscoveryContext,
    pub corrections: Vec<Correction>, /* Educational/metadata corrections (Uncorrectable +
                                       * guidance-only Candidates) */
}

// Implement Deref for states that wrap DiscoveryContext
impl Deref for SerializationCheck {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for ExtrasDiscovery {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for PatternCorrection {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Retry {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl Deref for Guidance {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

// Provide methods to extract the inner DiscoveryContext
impl SerializationCheck {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl ExtrasDiscovery {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl PatternCorrection {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl Retry {
    pub const fn new(context: DiscoveryContext, corrections: Vec<Correction>) -> Self {
        Self {
            context,
            corrections,
        }
    }
}

impl Guidance {
    pub const fn new(context: DiscoveryContext, corrections: Vec<Correction>) -> Self {
        Self {
            context,
            corrections,
        }
    }
}

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl std::fmt::Display for BrpTypeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Determine if corrections are retryable (should go to Retry state) or educational only (should go
/// to Guidance state)
///
/// This function evaluates a collection of corrections to determine the appropriate terminal state.
/// Returns `true` if corrections contain actionable values that can be used for retrying the BRP
/// call, `false` if corrections are educational/metadata only.
pub fn are_corrections_retryable(corrections: &[Correction]) -> bool {
    // Extract CorrectionInfo from Correction::Candidate variants and check if any can be retried
    let correction_infos: Vec<CorrectionInfo> = corrections
        .iter()
        .filter_map(|correction| match correction {
            Correction::Candidate { correction_info } => Some(correction_info.clone()),
            Correction::Uncorrectable { .. } => None, // Uncorrectable are never retryable
        })
        .collect();

    can_retry_with_corrections(&correction_infos)
}

/// Check if corrections can be applied for a retry
///
/// This is the core retry validation logic extracted from the old engine.
/// Only retry if we have corrections with actual values (not just metadata/hints).
pub fn can_retry_with_corrections(corrections: &[CorrectionInfo]) -> bool {
    // Only retry if we have corrections with actual values
    if corrections.is_empty() {
        return false;
    }

    // Check if all corrections have valid corrected values
    for correction in corrections {
        // Skip if the corrected value is just a placeholder or metadata
        if correction.corrected_value.is_null()
            || (correction.corrected_value.is_object()
                && correction.corrected_value.as_object().is_some_and(|o| {
                    o.contains_key(FormatCorrectionField::Hint.as_ref())
                        || o.contains_key(FormatCorrectionField::Examples.as_ref())
                        || o.contains_key(FormatCorrectionField::ValidValues.as_ref())
                }))
        {
            return false;
        }
    }

    true
}
