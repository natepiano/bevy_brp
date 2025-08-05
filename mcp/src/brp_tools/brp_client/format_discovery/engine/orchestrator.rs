//! High-level orchestration for format discovery with type state pattern
//!
//! This module provides the public API for format discovery, hiding the
//! complexity of the type state pattern behind a single function.

use either::Either;

use super::recovery_result::FormatRecoveryResult;
use super::types::DiscoveryEngine;
use crate::brp_tools::{BrpClientError, Port};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Main entry point for format discovery with error recovery
///
/// This orchestrates the entire discovery flow through the type state pattern:
/// 1. Create engine in `TypeDiscovery` state
/// 2. Initialize to gather discovery context (`SerializationCheck` state)
/// 3. Check for serialization issues (terminal if issues found)
/// 4. Try extras-based discovery (`ExtrasDiscovery` state, terminal if corrections found)
/// 5. Apply pattern-based corrections (`PatternCorrection` state, terminal)
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
            // Phase 4: Try extras-based discovery
            match extras_engine.build_extras_corrections() {
                Either::Left(result) => {
                    // Terminal: extras discovery succeeded
                    Ok(result)
                }
                Either::Right(pattern_engine) => {
                    // Phase 5: Apply pattern-based corrections (terminal state)
                    pattern_engine.apply_pattern_corrections().await
                }
            }
        }
    }
}
