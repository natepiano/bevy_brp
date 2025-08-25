//! `ExtrasDiscovery` state implementation
//!
//! This module implements the `ExtrasDiscovery` state for the discovery engine.
//! This state builds correction candidates using already-gathered extras data
//! when no serialization issues are found.

use either::Either;
use tracing::debug;

use super::state::{DiscoveryEngine, ExtrasDiscovery, Guidance, PatternCorrection, Retry};
use super::types::{Correction, DiscoverySource, are_corrections_retryable};

impl DiscoveryEngine<ExtrasDiscovery> {
    /// Try to build corrections from extras data
    ///
    /// This method processes types from the `TypeRegistry` to build corrections.
    ///
    /// Returns `Either::Left(Either<Retry, Guidance>)` if corrections are found,
    /// or `Either::Right(engine)` to continue with `PatternCorrection`.
    pub fn try_corrections(
        self,
    ) -> Either<
        Either<DiscoveryEngine<Retry>, DiscoveryEngine<Guidance>>,
        DiscoveryEngine<PatternCorrection>,
    > {
        debug!(
            "ExtrasDiscovery: Attempting direct discovery for {} types",
            self.context.types().count()
        );

        // Process types from TypeRegistry that have sufficient information for corrections
        let corrections: Vec<Correction> = self
            .context
            .types()
            .filter(|type_info| {
                // Process types from TypeRegistry that have spawn_format or mutation_paths
                matches!(type_info.discovery_source, DiscoverySource::TypeRegistry)
                    && (type_info.type_info.spawn_format.is_some()
                        || !type_info.type_info.mutation_paths.is_empty())
            })
            .map(|type_info| {
                debug!(
                    "ExtrasDiscovery: Processing type '{}' from TypeRegistry",
                    type_info.type_info.type_name.as_str()
                );
                type_info.to_correction(self.operation)
            })
            .collect();

        // Log the discovery results and evaluate corrections
        if corrections.is_empty() {
            debug!(
                "ExtrasDiscovery: No extras-based corrections found, proceeding to PatternCorrection with {} type infos",
                self.context.types().count()
            );
            Either::Right(self.transition_to_pattern_correction())
        } else {
            debug!(
                "ExtrasDiscovery: Found {} corrections from extras discovery",
                corrections.len()
            );

            // Extract the discovery context for terminal state creation
            let discovery_context = self.context.into_inner();

            // Evaluate whether corrections are retryable or guidance-only
            if are_corrections_retryable(&corrections) {
                debug!("ExtrasDiscovery: Corrections are retryable, creating Retry state");
                let retry_state = Retry::new(discovery_context, corrections);
                let retry_engine = DiscoveryEngine {
                    method:         self.method,
                    operation:      self.operation,
                    port:           self.port,
                    params:         self.params,
                    original_error: self.original_error,
                    context:        retry_state,
                };
                Either::Left(Either::Left(retry_engine))
            } else {
                debug!("ExtrasDiscovery: Corrections are guidance-only, creating Guidance state");
                let guidance_state = Guidance::new(discovery_context, corrections);
                let guidance_engine = DiscoveryEngine {
                    method:         self.method,
                    operation:      self.operation,
                    port:           self.port,
                    params:         self.params,
                    original_error: self.original_error,
                    context:        guidance_state,
                };
                Either::Left(Either::Right(guidance_engine))
            }
        }
    }

    /// Transition to `PatternCorrection` state, preserving the discovery context
    fn transition_to_pattern_correction(self) -> DiscoveryEngine<PatternCorrection> {
        DiscoveryEngine {
            method:         self.method,
            operation:      self.operation,
            port:           self.port,
            params:         self.params,
            original_error: self.original_error,
            context:        PatternCorrection(self.context.into_inner()),
        }
    }
}
