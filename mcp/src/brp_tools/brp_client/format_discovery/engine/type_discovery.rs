//! `TypeDiscovery` state implementation
//!
//! This module implements the `TypeDiscovery` state for the discovery engine.
//! This state creates the discovery context by calling the registry and optional extras plugin.

use super::discovery_context::DiscoveryContext;
use super::types::{DiscoveryEngine, SerializationCheck, TypeDiscovery};
use crate::error::Result;

impl DiscoveryEngine<TypeDiscovery> {
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
