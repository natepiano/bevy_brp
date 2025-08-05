//! High-level orchestration for format discovery with type state pattern
//!
//! This module provides the public API for format discovery, hiding the
//! complexity of the type state pattern behind a single function.

use either::Either;

use super::old_engine;
use super::recovery_result::FormatRecoveryResult;
use super::types::DiscoveryEngine;
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
    let engine = DiscoveryEngine::new(method, port, params.clone(), error.clone())?;

    // Initialize to SerializationCheck state
    let serialization_engine = engine.initialize().await?;

    // Check serialization (consumes engine)
    match serialization_engine.check_serialization() {
        Either::Left(result) => {
            // Terminal: serialization issues found
            Ok(result)
        }
        Either::Right(extras_engine) => {
            // TODO: For Phase 4, implement extras discovery
            // For now, create old engine with existing context and continue from Level 2
            let old_engine = old_engine::DiscoveryEngine::from_context(
                extras_engine.method,
                extras_engine.port,
                extras_engine.params,
                extras_engine.original_error,
                extras_engine.state.0, // Extract DiscoveryContext from ExtrasDiscovery state
            );
            old_engine.continue_after_serialization_check().await
        }
    }
}
