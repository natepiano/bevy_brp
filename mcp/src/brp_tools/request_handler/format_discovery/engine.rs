//! Orchestration and flow control for format discovery
//!
//! # Architecture Overview
//!
//! The format discovery engine implements a clean two-phase architecture:
//!
//! ## Phase 0: Normal Path (Direct BRP Execution)
//! Most requests succeed without any format discovery overhead.
//! ```text
//! Request: bevy/spawn with correct format
//! Result: Direct success, no discovery needed
//! ```
//!
//! ## Exception Path: Format Error Recovery
//! When Phase 0 fails with format errors, enter the exception path with a 3-level decision tree:
//!
//! ### Level 1: Registry/Serialization Checks
//! Verify type registration and serialization support.
//! ```text
//! Check: Is type in registry? Does it have Serialize/Deserialize?
//! Result: Educational guidance for unsupported types
//! ```
//!
//! ### Level 2: Direct Discovery (requires `bevy_brp_extras`)
//! Query the Bevy app for authoritative format information.
//! ```text
//! Query: `bevy_brp_extras`/discover_format for type
//! Result: Corrected format with rich metadata
//! ```
//!
//! ### Level 3: Pattern-Based Transformations
//! Apply deterministic transformations based on error patterns.
//! ```text
//! Pattern: Vec3 object→array conversion, enum variant access
//! Result: Corrected format with transformation hints
//! ```

use serde_json::Value;
use tracing::debug;

use super::constants::FORMAT_DISCOVERY_METHODS;
use super::flow_types::{BrpRequestResult, FormatRecoveryResult};
use super::unified_types::CorrectionInfo;
use crate::brp_tools::support::brp_client::BrpResult;
use crate::error::Result;

/// Format correction information for a type (component or resource)
#[derive(Debug, Clone)]
pub struct FormatCorrection {
    pub component:            String, // Keep field name for API compatibility
    pub original_format:      Value,
    pub corrected_format:     Value,
    pub hint:                 String,
    pub supported_operations: Option<Vec<String>>,
    pub mutation_paths:       Option<Vec<String>>,
    pub type_category:        Option<String>,
}

impl FormatCorrection {}

/// Enhanced response with format corrections
#[derive(Debug, Clone)]
pub struct EnhancedBrpResult {
    pub result:             BrpResult,
    pub format_corrections: Vec<FormatCorrection>,
}

/// Execute Phase 0: Direct BRP request without format discovery overhead
async fn execute_phase_0(
    method: &str,
    params: Option<Value>,
    port: Option<u16>,
) -> Result<BrpRequestResult> {
    use crate::brp_tools::support::brp_client::execute_brp_method;

    // Direct BRP execution - no format discovery overhead
    let result = execute_brp_method(method, params.clone(), port).await?;

    match result {
        BrpResult::Success(_) => {
            // Success - no format discovery needed
            Ok(BrpRequestResult::Success(result))
        }
        BrpResult::Error(ref error) => {
            // Check if this is a recoverable format error
            if is_format_error(error) && is_format_discovery_supported(method) {
                Ok(BrpRequestResult::FormatError {
                    error:           result,
                    method:          method.to_string(),
                    original_params: params,
                })
            } else {
                // Non-recoverable error - return immediately
                Ok(BrpRequestResult::OtherError(result))
            }
        }
    }
}

/// Check if an error indicates a format issue that can be recovered
fn is_format_error(error: &crate::brp_tools::support::brp_client::BrpError) -> bool {
    // Common format error codes that indicate type serialization issues
    matches!(error.code, -32602 | -32603)
        && (error.message.contains("failed to deserialize")
            || error.message.contains("invalid type")
            || error.message.contains("expected")
            || error.message.contains("AccessError")
            || error.message.contains("missing field")
            || error.message.contains("unknown variant"))
}

/// Check if a method supports format discovery
fn is_format_discovery_supported(method: &str) -> bool {
    FORMAT_DISCOVERY_METHODS.contains(&method)
}

/// Execute exception path: Format error recovery using the 3-level decision tree
async fn execute_exception_path(
    method: String,
    original_params: Option<Value>,
    error: BrpResult,
    port: Option<u16>,
) -> FormatRecoveryResult {
    use super::phases::context::DiscoveryContext;

    DiscoveryContext::add_debug(format!(
        "Exception Path: Entering format recovery for method '{method}'"
    ));

    // Use the new recovery engine with 3-level decision tree
    super::recovery_engine::attempt_format_recovery(&method, original_params, error, port).await
}

/// Execute a BRP method with automatic format discovery using the new flow architecture
pub async fn execute_brp_method_with_format_discovery(
    method: &str,
    params: Option<Value>,
    port: Option<u16>,
) -> Result<EnhancedBrpResult> {
    use super::phases::context::DiscoveryContext;

    DiscoveryContext::add_debug(format!(
        "Format Discovery: Starting request for method '{method}'"
    ));

    // Phase 0: Direct BRP execution (normal path)
    DiscoveryContext::add_debug("Phase 0: Attempting direct BRP execution".to_string());
    let phase_0_result = execute_phase_0(method, params, port).await?;

    match phase_0_result {
        BrpRequestResult::Success(result) => {
            DiscoveryContext::add_debug(
                "Phase 0: Direct execution succeeded, no discovery needed".to_string(),
            );
            Ok(EnhancedBrpResult {
                result,
                format_corrections: Vec::new(),
            })
        }
        BrpRequestResult::FormatError {
            error,
            method,
            original_params,
        } => {
            DiscoveryContext::add_debug(
                "Phase 0: Format error detected, entering exception path".to_string(),
            );

            // Exception Path: Format error recovery
            let recovery_result =
                execute_exception_path(method, original_params, error, port).await;

            // Convert recovery result to EnhancedBrpResult
            Ok(convert_recovery_to_enhanced_result(recovery_result))
        }
        BrpRequestResult::OtherError(result) => {
            DiscoveryContext::add_debug(
                "Phase 0: Non-recoverable error, returning original result".to_string(),
            );
            Ok(EnhancedBrpResult {
                result,
                format_corrections: Vec::new(),
            })
        }
    }
}

/// Convert `FormatRecoveryResult` to `EnhancedBrpResult` for API compatibility
fn convert_recovery_to_enhanced_result(recovery_result: FormatRecoveryResult) -> EnhancedBrpResult {
    match recovery_result {
        FormatRecoveryResult::Recovered {
            corrected_result,
            corrections,
        } => {
            let format_corrections = convert_corrections_to_legacy_format(corrections);
            EnhancedBrpResult {
                result: corrected_result,
                format_corrections,
            }
        }
        FormatRecoveryResult::Educational { original_error, .. } => EnhancedBrpResult {
            result:             original_error,
            format_corrections: Vec::new(),
        },
        FormatRecoveryResult::NotRecoverable(result) => EnhancedBrpResult {
            result,
            format_corrections: Vec::new(),
        },
    }
}

/// Convert new `CorrectionInfo` to legacy `FormatCorrection` for API compatibility
fn convert_corrections_to_legacy_format(corrections: Vec<CorrectionInfo>) -> Vec<FormatCorrection> {
    corrections
        .into_iter()
        .map(|correction| FormatCorrection {
            component:            correction.type_name,
            original_format:      correction.original_value,
            corrected_format:     correction.corrected_value,
            hint:                 correction.hint,
            supported_operations: correction
                .type_info
                .as_ref()
                .map(|ti| ti.supported_operations.clone()),
            mutation_paths:       correction
                .type_info
                .as_ref()
                .map(|ti| ti.format_info.mutation_paths.keys().cloned().collect()),
            type_category:        correction
                .type_info
                .as_ref()
                .map(|ti| ti.type_category.clone()),
        })
        .collect()
}
