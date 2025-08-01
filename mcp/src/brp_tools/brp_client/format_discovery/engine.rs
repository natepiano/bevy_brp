//! Orchestration and flow control for format discovery
//!
//! # Architecture Overview
//!
//! The format discovery engine implements a clean two-phase architecture:
//!
//! ## Level 1: Normal Path (Direct BRP Execution)
//! Most requests succeed without any format discovery overhead.
//! ```text
//! Request: bevy/spawn with correct format
//! Result: Direct success, no discovery needed
//! ```
//!
//! ## Exception Path: Format Error Recovery
//! When Level 1 Normal Path fails with format errors, enter the exception path with a 3-level
//! decision tree:
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
//! Succinct call flow notes:
//! The format discovery engine makes the initial attempt at the BRP call. This MUST use
//! `execute_direct()` to avoid infinite recursion since it's part of the format discovery flow
//! itself.
//! - `format_discovery/engine.rs` - Makes the initial BRP call attempt
//! - `format_discovery/registry_integration.rs` - Queries registry for type info
//! - `format_discovery/extras_integration.rs` - Calls discovery endpoint
//! - `format_discovery/recovery_engine.rs` - Retries with corrected params

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::flow_types::FormatRecoveryResult;
use super::{recovery_engine, registry_integration};
use crate::brp_tools::{BrpClientError, BrpClientResult, Port};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Format correction information for a type (component or resource)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Try format recovery and retry with corrected format
pub(in super::super) async fn try_format_recovery_and_retry(
    method: BrpMethod,
    params: Value,
    port: Port,
    original_error: &BrpClientError,
) -> Result<FormatRecoveryResult> {
    // Skip Level 1 - we already failed
    // Check if error is format-related
    if !original_error.is_format_error() {
        return Ok(FormatRecoveryResult::NotRecoverable {
            corrections: Vec::new(),
        });
    }

    // Get type information only when needed for error handling
    let registry_type_info =
        registry_integration::get_registry_type_info(method, &params, port).await;

    // Continue with Level 2+ logic using the recovery engine directly
    let flow_result = recovery_engine::attempt_format_recovery_with_type_infos(
        method,
        params,
        BrpClientResult::Error(original_error.clone()),
        registry_type_info,
        port,
    )
    .await;

    // FormatRecoveryResult is already the correct type
    Ok(flow_result)
}
