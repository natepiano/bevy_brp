//! `TypeSchemaDiscovery` state implementation
//!
//! This module implements the `TypeSchemaDiscovery` state for the discovery engine.
//! This state builds correction candidates using `TypeSchema` data from the registry
//! when no serialization issues are found. This is a terminal state.

use either::Either;
use tracing::debug;

use super::state::{DiscoveryEngine, Guidance, Retry, TypeSchemaDiscovery};
use super::types::{Correction, are_corrections_retryable};

impl DiscoveryEngine<TypeSchemaDiscovery> {
    /// Try to build corrections from `TypeSchema` data (terminal state)
    ///
    /// This method processes types from the `TypeSchema` registry to build corrections.
    /// Since every Component/Resource in the registry has mutation support and `mutation_paths`,
    /// corrections are always found, making this a terminal state.
    ///
    /// Returns `Either<Retry, Guidance>` based on correction evaluation.
    pub fn try_corrections(self) -> Either<DiscoveryEngine<Retry>, DiscoveryEngine<Guidance>> {
        debug!(
            "TypeSchemaDiscovery: Attempting discovery for {} types",
            self.context.type_names().count()
        );

        // Process all types from TypeSchema registry to build corrections
        // Since every Component/Resource automatically gets mutation_paths, corrections will never
        // be empty
        let corrections: Vec<Correction> = self
            .context
            .0
            .type_names()
            .map(|type_name| {
                debug!(
                    "TypeSchemaDiscovery: Processing type '{}' from TypeSchema registry",
                    type_name.as_str()
                );
                self.context.0.to_correction(type_name)
            })
            .collect();

        debug!(
            "TypeSchemaDiscovery: Found {} corrections from TypeSchema discovery",
            corrections.len()
        );

        // Extract the discovery context for terminal state creation
        let discovery_context = self.context.0;

        // Evaluate whether corrections are retryable or guidance-only
        if are_corrections_retryable(&corrections) {
            debug!("TypeSchemaDiscovery: Corrections are retryable, creating Retry state");
            let retry_state = Retry::new(discovery_context, corrections);
            let retry_engine = DiscoveryEngine {
                method:         self.method,
                operation:      self.operation,
                port:           self.port,
                params:         self.params,
                original_error: self.original_error,
                context:        retry_state,
            };
            Either::Left(retry_engine)
        } else {
            debug!("TypeSchemaDiscovery: Corrections are guidance-only, creating Guidance state");
            let guidance_state = Guidance::new(discovery_context, corrections);
            let guidance_engine = DiscoveryEngine {
                method:         self.method,
                operation:      self.operation,
                port:           self.port,
                params:         self.params,
                original_error: self.original_error,
                context:        guidance_state,
            };
            Either::Right(guidance_engine)
        }
    }
}
