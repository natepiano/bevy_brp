//! High-level orchestration for format discovery with type state pattern
//!
//! This module provides the public API for format discovery, hiding the
//! complexity of the type state pattern behind a single function.

use either::Either;
use serde_json::Value;

use super::discovery_context::DiscoveryContext;
use super::recovery_result::FormatRecoveryResult;
use super::state::{DiscoveryEngine, SerializationCheck, TypeDiscovery};
use crate::brp_tools::{BrpClientError, Port};
use crate::error::{Error, Result};
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

    // execute the engine against the varous possible terminal states
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

/// `DiscoveryEngine<TypeDiscovery>` is the entry point to the engine. Having this state pattern is
/// not only for type safety but also for readability. We use new as the only constructor and it
/// does some initial validation and then returns a `TypeDiscovery` instance - on which the only
/// thing you can do is initialize it to then move it into the `SerializationCheck`
///
/// Ultimately this starts the pipeline of discovery and retry process that is
/// executed by the `orchestrator` method `discover_format_with_recovery`
impl DiscoveryEngine<TypeDiscovery> {
    /// Create a new format discovery engine for a specific method and port
    ///
    /// Returns an error if the parameters are invalid for format discovery
    /// (e.g., None when format discovery requires parameters, or error is not a format error)
    pub fn new(
        method: BrpMethod,
        port: Port,
        params: Option<Value>,
        original_error: BrpClientError,
    ) -> Result<Self> {
        // Check if we can recover from this error type
        // Format discovery can only work with specific error types
        if !original_error.is_format_error() {
            return Err(Error::InvalidArgument(
                "Format discovery can only be used with format errors".to_string(),
            )
            .into());
        }

        // Validate that parameters exist for format discovery
        // Format discovery requires parameters to extract type information
        let params = params.ok_or_else(|| {
            Error::InvalidArgument(
                "Format discovery requires parameters to extract type information".to_string(),
            )
        })?;

        Ok(Self {
            method,
            port,
            params,
            original_error,
            state: TypeDiscovery,
        })
    }

    /// Initialize the discovery process by creating a discovery context
    ///
    /// This method extracts type information from the method parameters,
    /// creates a `DiscoveryContext` by calling the registry and optional extras plugin,
    /// and returns a `SerializationCheck` state containing the context.
    pub async fn initialize(self) -> Result<DiscoveryEngine<SerializationCheck>> {
        // Create discovery context from method parameters
        let mut discovery_context =
            DiscoveryContext::from_params(self.method, self.port, Some(&self.params)).await?;

        // Enrich context with extras discovery upfront (don't fail if enrichment fails)
        if let Err(e) = discovery_context.enrich_with_extras().await {
            tracing::debug!("TypeDiscovery: Failed to enrich with extras: {e:?}");
        }

        // Return SerializationCheck state with the context
        Ok(DiscoveryEngine {
            method:         self.method,
            port:           self.port,
            params:         self.params,
            original_error: self.original_error,
            state:          SerializationCheck(discovery_context),
        })
    }
}
