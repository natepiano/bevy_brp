//! Flow control types for the format discovery system
//!
//! This module defines the core result types that control the flow between
//! normal BRP execution (Phase 0) and the format error recovery system (exception path).
//!
//! # Architecture Overview
//!
//! The format discovery system operates in two distinct phases:
//!
//! ## Phase 0: Normal Path
//! Direct BRP execution without format discovery overhead. Most requests succeed here.
//!
//! ## Exception Path: Format Error Recovery
//! When Phase 0 fails with format errors, the system enters a 3-level decision tree:
//! - Level 1: Registry/serialization checks
//! - Level 2: Direct discovery via `bevy_brp_extras`
//! - Level 3: Pattern-based transformations

use std::collections::HashMap;

use super::{CorrectionInfo, UnifiedTypeInfo};
use crate::brp_tools::support::brp_client::BrpResult;

/// Result of a BRP request attempt, determining whether to enter format recovery
#[derive(Debug, Clone)]
pub enum BrpRequestResult {
    /// Request succeeded - no format discovery needed
    Success(BrpResult),
    /// Request failed with recoverable format error - enter exception path
    FormatError {
        error:           BrpResult,
        method:          String,
        original_params: Option<serde_json::Value>,
        type_infos:      HashMap<String, UnifiedTypeInfo>,
    },
    /// Request failed with missing Serialize/Deserialize traits - return educational message
    SerDeError {
        error:               BrpResult,
        educational_message: String,
    },
    /// Request failed with non-recoverable error - return immediately
    OtherError(BrpResult),
}

/// Result of format error recovery attempt in the exception path
#[derive(Debug, Clone)]
pub enum FormatRecoveryResult {
    /// Recovery successful with corrections applied
    Recovered {
        corrected_result: BrpResult,
        corrections:      Vec<CorrectionInfo>,
    },
    /// Recovery not possible but guidance available
    NotRecoverable {
        original_error: BrpResult,
        corrections:    Vec<CorrectionInfo>,
    },
}

/// Result of individual correction attempts during recovery
#[derive(Debug, Clone)]
pub enum CorrectionResult {
    /// Correction was successfully applied
    Corrected { correction_info: CorrectionInfo },
    /// Correction could not be applied but metadata was discovered
    CannotCorrect {
        type_info: UnifiedTypeInfo,
        reason:    String,
    },
}
