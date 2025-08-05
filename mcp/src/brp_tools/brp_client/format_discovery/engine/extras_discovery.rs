//! `ExtrasDiscovery` state implementation
//!
//! This module implements the `ExtrasDiscovery` state for the discovery engine.
//! This state builds correction candidates using already-gathered extras data
//! when no serialization issues are found.

use either::Either;
use tracing::debug;

use super::super::types::{Correction, DiscoverySource};
use super::recovery_result::FormatRecoveryResult;
use super::types::{DiscoveryEngine, ExtrasDiscovery, PatternCorrection};

impl DiscoveryEngine<ExtrasDiscovery> {
    /// Build corrections from extras data
    ///
    /// This method processes types that were enriched with extras information
    /// and builds corrections for them. Only types with `DiscoverySource::RegistryPlusExtras`
    /// are processed.
    ///
    /// Returns `Either::Left(result)` if corrections are found and successfully built,
    /// or `Either::Right(engine)` to continue with `PatternCorrection`.
    pub fn build_extras_corrections(
        self,
    ) -> Either<FormatRecoveryResult, DiscoveryEngine<PatternCorrection>> {
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
                    type_info.type_name
                );
                type_info.to_correction_for_method(self.method)
            })
            .collect();

        // Log the discovery results
        if corrections.is_empty() {
            debug!(
                "ExtrasDiscovery: No extras-based corrections found, proceeding to PatternCorrection with {} type infos",
                self.state.type_names().len()
            );
        } else {
            debug!(
                "ExtrasDiscovery: Found {} corrections from extras discovery",
                corrections.len()
            );
            // TODO: In Phase 5, when corrections are found, this will build and return
            // a terminal FormatRecoveryResult instead of transitioning to PatternCorrection
        }

        // For Phase 4, we always transition to PatternCorrection and let the orchestrator
        // handle calling the old engine's continuation method to build the actual result
        Either::Right(self.transition_to_pattern_correction())
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
