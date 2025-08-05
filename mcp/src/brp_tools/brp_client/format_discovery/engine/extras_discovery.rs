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
    /// This method processes types that were enriched with extras information
    /// and builds corrections for them. Only types with `DiscoverySource::RegistryPlusExtras`
    /// are processed.
    ///
    /// Returns `Either::Left(Either<Retry, Guidance>)` if corrections are found,
    /// or `Either::Right(engine)` to continue with `PatternCorrection`.
    pub fn try_extras_corrections(
        self,
    ) -> Either<
        Either<DiscoveryEngine<Retry>, DiscoveryEngine<Guidance>>,
        DiscoveryEngine<PatternCorrection>,
    > {
        debug!(
            "ExtrasDiscovery: Attempting direct discovery for {} types",
            self.state.type_names().len()
        );

        // Process only types that were enriched with extras information
        let corrections: Vec<Correction> = self
            .state
            .types()
            .filter(|type_info| {
                // Only process types that got information from extras
                matches!(
                    type_info.discovery_source,
                    DiscoverySource::RegistryPlusExtras
                )
            })
            .map(|type_info| {
                debug!(
                    "ExtrasDiscovery: Processing extras-enriched type '{}'",
                    type_info.type_name.as_str()
                );
                type_info.to_correction_for_method(self.method)
            })
            .collect();

        // Log the discovery results and evaluate corrections
        if corrections.is_empty() {
            debug!(
                "ExtrasDiscovery: No extras-based corrections found, proceeding to PatternCorrection with {} type infos",
                self.state.type_names().len()
            );
            Either::Right(self.transition_to_pattern_correction())
        } else {
            debug!(
                "ExtrasDiscovery: Found {} corrections from extras discovery",
                corrections.len()
            );

            // Extract the discovery context for terminal state creation
            let discovery_context = self.state.into_inner();

            // Evaluate whether corrections are retryable or guidance-only
            if are_corrections_retryable(&corrections) {
                debug!("ExtrasDiscovery: Corrections are retryable, creating Retry state");
                let retry_state = Retry::new(discovery_context, corrections);
                let retry_engine = DiscoveryEngine {
                    method:         self.method,
                    port:           self.port,
                    params:         self.params,
                    original_error: self.original_error,
                    state:          retry_state,
                };
                Either::Left(Either::Left(retry_engine))
            } else {
                debug!("ExtrasDiscovery: Corrections are guidance-only, creating Guidance state");
                let guidance_state = Guidance::new(discovery_context, corrections);
                let guidance_engine = DiscoveryEngine {
                    method:         self.method,
                    port:           self.port,
                    params:         self.params,
                    original_error: self.original_error,
                    state:          guidance_state,
                };
                Either::Left(Either::Right(guidance_engine))
            }
        }
    }

    /// Transition to `PatternCorrection` state, preserving the discovery context
    fn transition_to_pattern_correction(self) -> DiscoveryEngine<PatternCorrection> {
        DiscoveryEngine {
            method:         self.method,
            port:           self.port,
            params:         self.params,
            original_error: self.original_error,
            state:          PatternCorrection(self.state.into_inner()),
        }
    }
}
