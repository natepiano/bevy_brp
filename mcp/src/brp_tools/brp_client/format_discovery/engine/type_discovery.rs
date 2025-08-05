//! `TypeDiscovery` state implementation
//!
//! This module implements the `TypeDiscovery` state for the discovery engine.
//! In Phase 2, this simply delegates to the existing engine implementation.

use super::super::recovery_result::FormatRecoveryResult;
use super::new::DiscoveryEngine;
use super::old_engine;
use super::types::TypeDiscovery;
use crate::error::Result;

impl DiscoveryEngine<TypeDiscovery> {
    /// Initialize the discovery process by delegating to the existing engine
    ///
    /// In Phase 2, this method simply creates an old-style engine and delegates
    /// to `attempt_discovery_with_recovery()`. In later phases, this will be
    /// refactored to only handle context creation and return a `SerializationCheck` state.
    pub async fn initialize(self) -> Result<FormatRecoveryResult> {
        // Create an old-style engine and delegate the work
        let old_engine = old_engine::DiscoveryEngine::new(
            self.method,
            self.port,
            Some(self.params),
            self.original_error,
        )
        .await?;

        // Delegate to existing implementation
        old_engine.attempt_discovery_with_recovery().await
    }
}
