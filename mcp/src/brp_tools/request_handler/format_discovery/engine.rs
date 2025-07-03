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

use std::collections::HashMap;

use serde_json::Value;
use tracing::{debug, trace};

use super::UnifiedTypeInfo;
use super::constants::FORMAT_DISCOVERY_METHODS;
use super::flow_types::{BrpRequestResult, FormatRecoveryResult};
use super::registry_integration::get_registry_type_info;
use super::unified_types::CorrectionInfo;
use crate::brp_tools::constants::BRP_ERROR_CODE_INVALID_REQUEST;
use crate::brp_tools::request_handler::format_discovery::recovery_engine;
use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};
use crate::error::Result;
use crate::tools::{BRP_METHOD_INSERT, BRP_METHOD_SPAWN};

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
    // Direct BRP execution - no format discovery overhead
    let result = execute_brp_method(method, params.clone(), port).await?;

    match result {
        BrpResult::Success(_) => {
            // Success - no format discovery needed
            Ok(BrpRequestResult::Success(result))
        }
        BrpResult::Error(ref error) => {
            // Get type information only when needed for error handling
            let type_infos = get_registry_type_info(method, params.as_ref(), port).await;
            // Check for serialization errors first (missing Serialize/Deserialize traits)
            // Only spawn/insert methods require full serialization
            if matches!(method, BRP_METHOD_SPAWN | BRP_METHOD_INSERT)
                && error.code == BRP_ERROR_CODE_INVALID_REQUEST
            {
                // Check if this is a serialization error that should be short-circuited
                if let Some(educational_message) = check_serialization_support(method, &type_infos)
                {
                    return Ok(BrpRequestResult::SerDeError {
                        error: result,
                        educational_message,
                    });
                }
            }

            // Check if this is a recoverable format error
            if is_format_error(error) && is_format_discovery_supported(method) {
                Ok(BrpRequestResult::FormatError {
                    error: result,
                    method: method.to_string(),
                    original_params: params,
                    type_infos,
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
    // Dynamic type errors indicate missing Serialize/Deserialize traits (any error code)
    if error
        .message
        .contains("Unknown component type: `bevy_reflect::DynamicEnum`")
        || error
            .message
            .contains("Unknown component type: `bevy_reflect::DynamicStruct`")
        || error
            .message
            .contains("Unknown resource type: `bevy_reflect::DynamicEnum`")
        || error
            .message
            .contains("Unknown resource type: `bevy_reflect::DynamicStruct`")
    {
        return true;
    }

    // Common format error codes that indicate type serialization issues
    // Include BRP_ERROR_CODE_INVALID_REQUEST (-23402) for mutation errors
    matches!(error.code, -32602 | -32603 | BRP_ERROR_CODE_INVALID_REQUEST)
        && (error.message.contains("failed to deserialize")
            || error.message.contains("invalid type")
            || error.message.contains("expected")
            || error.message.contains("AccessError")
            || error.message.contains("missing field")
            || error.message.contains("unknown variant")
            || error.message.contains("Error accessing element"))
}

/// Check if a method supports format discovery
fn is_format_discovery_supported(method: &str) -> bool {
    FORMAT_DISCOVERY_METHODS.contains(&method)
}

/// Check if any types lack serialization support using pre-fetched type infos
fn check_serialization_support(
    method: &str,
    type_infos: &HashMap<String, UnifiedTypeInfo>,
) -> Option<String> {
    debug!("Checking for serialization errors using pre-fetched type infos");

    for (component_type, type_info) in type_infos {
        debug!(
            "Component '{}' found in registry, brp_compatible={}",
            component_type, type_info.serialization.brp_compatible
        );
        // Component is registered but lacks serialization - short circuit
        if !type_info.serialization.brp_compatible {
            debug!(
                "Component '{}' lacks serialization, returning educational message",
                component_type
            );
            return Some(format!(
                "Component '{}' is registered but lacks Serialize and Deserialize traits required for {} operations. \
                Add #[derive(Serialize, Deserialize)] to the component definition.",
                component_type,
                method.split('/').next_back().unwrap_or(method)
            ));
        }
    }

    debug!("All components have serialization support");
    None
}

/// Execute exception path: Format error recovery using the 3-level decision tree
async fn execute_exception_path(
    method: String,
    original_params: Option<Value>,
    error: BrpResult,
    type_infos: HashMap<String, super::unified_types::UnifiedTypeInfo>,
    port: Option<u16>,
) -> FormatRecoveryResult {
    tracing::trace!("Discovery: Exception Path: Entering format recovery for method '{method}'");

    // Use the new recovery engine with 3-level decision tree, passing pre-fetched type infos
    recovery_engine::attempt_format_recovery_with_type_infos(
        &method,
        original_params,
        error,
        type_infos,
        port,
    )
    .await
}

/// Execute a BRP method with automatic format discovery using the new flow architecture
pub async fn execute_brp_method_with_format_discovery(
    method: &str,
    params: Option<Value>,
    port: Option<u16>,
) -> Result<EnhancedBrpResult> {
    trace!("Discovery: Format Discovery: Starting request for method '{method}'");

    // Phase 0: Direct BRP execution (normal path)
    trace!("Discovery: Phase 0: Attempting direct BRP execution");
    let phase_0_result = execute_phase_0(method, params, port).await?;

    match phase_0_result {
        BrpRequestResult::Success(result) => {
            trace!("Discovery: Phase 0: Direct execution succeeded, no discovery needed");
            Ok(EnhancedBrpResult {
                result,
                format_corrections: Vec::new(),
            })
        }
        BrpRequestResult::FormatError {
            error,
            method,
            original_params,
            type_infos,
        } => {
            trace!("Discovery: Phase 0: Format error detected, entering exception path");

            // Exception Path: Format error recovery with pre-fetched type infos
            let recovery_result =
                execute_exception_path(method, original_params, error, type_infos, port).await;

            // Convert recovery result to EnhancedBrpResult
            Ok(convert_recovery_to_enhanced_result(recovery_result))
        }
        BrpRequestResult::SerDeError {
            error,
            educational_message,
            ..
        } => {
            trace!(
                "Discovery: Phase 0: Serialization error detected, returning educational message"
            );

            // Replace the error message with the educational guidance
            let enhanced_error = match error {
                BrpResult::Error(mut error_info) => {
                    error_info.message = educational_message;
                    BrpResult::Error(error_info)
                }
                success @ BrpResult::Success(_) => success, /* Shouldn't happen but preserve if
                                                             * it does */
            };

            Ok(EnhancedBrpResult {
                result:             enhanced_error,
                format_corrections: Vec::new(),
            })
        }
        BrpRequestResult::OtherError(result) => {
            trace!("Discovery: Phase 0: Non-recoverable error, returning original result");
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
        FormatRecoveryResult::NotRecoverable {
            original_error,
            corrections,
        } => {
            let format_corrections = convert_corrections_to_legacy_format(corrections);
            EnhancedBrpResult {
                result: original_error,
                format_corrections,
            }
        }
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
