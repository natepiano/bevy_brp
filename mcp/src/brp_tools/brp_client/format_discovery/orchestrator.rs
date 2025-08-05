//! High-level orchestration for format discovery with type state pattern
//!
//! This module provides the public API for format discovery, hiding the
//! complexity of the type state pattern behind a single function.

use super::engine::DiscoveryEngine;
use super::recovery_result::FormatRecoveryResult;
use crate::brp_tools::{BrpClientError, Port};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Main entry point for format discovery with error recovery
///
/// This orchestrates the entire discovery flow through the type state pattern:
/// 1. Create engine in `TypeDiscovery` state
/// 2. Initialize to gather discovery context
/// 3. Check for serialization issues (Phase 3)
/// 4. Try extras-based discovery (Phase 4)
/// 5. Fall back to pattern-based corrections (Phase 5)
pub async fn discover_format_with_recovery(
    method: BrpMethod,
    port: Port,
    params: Option<serde_json::Value>,
    error: BrpClientError,
) -> Result<FormatRecoveryResult> {
    // Create engine in TypeDiscovery state
    let engine = DiscoveryEngine::new(method, port, params, error)?;

    // For now, just delegate to initialize which calls old_engine
    // Future phases will add more state transitions here
    engine.initialize().await
}
