//! High-level orchestration for format discovery with type state pattern
//!
//! This module provides the public API for format discovery, hiding the
//! complexity of the type state pattern behind a single function.

use either::Either::{self, Left, Right};
use serde_json::Value;

use super::discovery_context::DiscoveryContext;
use super::recovery_result::FormatRecoveryResult;
use super::state::{DiscoveryEngine, SerializationCheck, TypeDiscovery};
use super::types::Operation;
use crate::brp_tools::{BrpClientError, Port};
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

/// Main entry point for format discovery with error recovery
///
/// This orchestrates the entire discovery flow through the type state pattern:
/// 1. Create engine in `TypeDiscovery` state and initialize with `TypeSchemaEngine` data
/// 2. Check for serialization issues (`SerializationCheck` state, terminal if issues found)
/// 3. Try to build corrections from TypeSchema data (`TypeSchemaDiscovery` state, terminal if
///    corrections found)
/// 4. Apply pattern-based transformations (`PatternCorrection` state, terminal) returning either
///    `Retry` or `Guidance`
pub async fn discover_format_with_recovery(
    method: BrpMethod,
    port: Port,
    params: Option<Value>,
    error: BrpClientError,
) -> Result<FormatRecoveryResult> {
    let engine = match initialize_discovery_engine(method, port, params, error).await? {
        Left(engine) => engine,
        Right(early_result) => return Ok(early_result),
    };

    // Chain the discovery strategies - flatten the nested Either structure and walk through the
    // states systematically - if it's terminal (including the final call totry_pattern_corrections)
    // then it returns either `Retry` or `Guidance`
    let terminal = engine.check_serialization().either(
        |terminal| terminal,
        |extras| {
            extras.try_corrections().either(
                |terminal| terminal,
                DiscoveryEngine::try_pattern_corrections,
            )
        },
    );

    // Execute terminal state - either retry or provide guidance
    match terminal {
        Left(retry) => Ok(retry.apply_corrections_and_retry().await),
        Right(guidance) => Ok(guidance.provide_guidance()),
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

        let operation = Operation::try_from(method)?; // Compute once

        Ok(Self {
            method,
            operation,
            port,
            params,
            original_error,
            context: TypeDiscovery,
        })
    }

    /// This method extracts type information from the method parameters,
    /// creates a `DiscoveryContext` by calling the registry and optional extras plugin,
    /// and returns a `SerializationCheck` state containing the context to allow further processing.
    pub async fn initialize_context(self) -> Result<DiscoveryEngine<SerializationCheck>> {
        // Create discovery context from method parameters
        let discovery_context = DiscoveryContext::new(self.method, self.port, &self.params).await?;

        // Return SerializationCheck state with the context
        Ok(DiscoveryEngine {
            method:         self.method,
            operation:      self.operation,
            port:           self.port,
            params:         self.params,
            original_error: self.original_error,
            context:        SerializationCheck(discovery_context),
        })
    }
}

/// Initialize discovery engine, handling `TypeNotRegistered` by returning early result
async fn initialize_discovery_engine(
    method: BrpMethod,
    port: Port,
    params: Option<Value>,
    error: BrpClientError,
) -> Result<Either<DiscoveryEngine<SerializationCheck>, FormatRecoveryResult>> {
    match DiscoveryEngine::new(method, port, params, error)?
        .initialize_context()
        .await
    {
        Ok(engine) => Ok(Either::Left(engine)),

        // special case handling for type not found in registry
        Err(err) if matches!(err.current_context(), Error::TypeNotRegistered { .. }) => {
            tracing::debug!("Converting TypeNotRegistered error to NotRecoverable");
            Ok(Either::Right(FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            }))
        }
        Err(err) => Err(err),
    }
}
