//! High-level orchestration for format discovery with type state pattern
//!
//! This module provides the public API for format discovery, hiding the
//! complexity of the type state pattern behind a single function.

use either::Either;
use serde_json::Value;

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
    params: Option<Value>,
    error: BrpClientError,
) -> Result<FormatRecoveryResult> {
    // Create engine in `TypeDiscovery` state, then initialize it to transition to
    // `SerializationCheck` state state
    let engine = DiscoveryEngine::new(method, port, params, error)?
        .initialize()
        .await?;

    // Check serialization and route to terminal states or continue discovery
    let terminal_engine = match engine.check_serialization() {
        Either::Left(terminal) => terminal, // Either<Retry, Guidance> from serialization
        Either::Right(extras_engine) => {
            // Try extras-based discovery
            match extras_engine.try_extras_corrections() {
                Either::Left(terminal) => terminal, // Either<Retry, Guidance> from extras
                Either::Right(pattern_engine) => {
                    // Apply pattern-based corrections (terminal state)
                    pattern_engine.try_pattern_corrections() // Either<Retry, Guidance> from patterns
                }
            }
        }
    };

    // Execute terminal state - either retry or provide guidance
    match terminal_engine {
        Either::Left(retry_engine) => Ok(retry_engine.apply_corrections_and_retry().await),
        Either::Right(guidance_engine) => Ok(guidance_engine.provide_guidance()),
    }
}
