//! `SerializationCheck` state implementation
//!
//! This module implements the `SerializationCheck` state for the discovery engine.
//! This state checks if types have required serialization traits to work around
//! a bug in Bevy 0.16 where "Unknown component type" errors are thrown even for
//! components with proper serialization support.

use either::Either;
use tracing::debug;

use super::state::{DiscoveryEngine, ExtrasDiscovery, Guidance, Retry, SerializationCheck};
use super::types::{Correction, Operation, are_corrections_retryable};

impl DiscoveryEngine<SerializationCheck> {
    /// Check for serialization issues that prevent BRP operations
    ///
    /// This method examines types in the discovery context to detect components
    /// that are registered in the type registry but lack the required Serialize
    /// and Deserialize traits for BRP operations.
    ///
    /// Returns `Either::Left(Either<Retry, Guidance>)` if serialization issues are found,
    /// or `Either::Right(engine)` to continue with `ExtrasDiscovery`.
    pub fn check_serialization(
        self,
    ) -> Either<
        Either<DiscoveryEngine<Retry>, DiscoveryEngine<Guidance>>,
        DiscoveryEngine<ExtrasDiscovery>,
    > {
        // Only check for spawn/insert methods with UnknownComponentType errors
        if !matches!(self.operation, Operation::SpawnInsert { .. }) {
            debug!("SerializationCheck: Not a spawn/insert method, proceeding to ExtrasDiscovery");
            return Either::Right(self.transition_to_type_discovery());
        }

        // Check if error message indicates a serialization issue
        // This is a known spurious response when we try to spawn or insert a component that does
        // exist but didn't derive serialization - so we know we need to be seeing this particular
        // error to continue on to build a Correction
        if !self
            .original_error
            .message
            .contains("Unknown component type")
        {
            debug!(
                "SerializationCheck: Error message doesn't indicate serialization issue, proceeding to ExtrasDiscovery"
            );
            return Either::Right(self.transition_to_type_discovery());
        }

        debug!("SerializationCheck: Checking for serialization errors in registry type infos");

        // First, check if any types have serialization issues before building corrections
        let has_serialization_issues = self
            .context
            .types()
            .any(|type_info| type_info.in_registry() && !type_info.is_brp_compatible());

        if !has_serialization_issues {
            debug!(
                "SerializationCheck: All components have serialization support or are not in registry, proceeding to ExtrasDiscovery"
            );
            return Either::Right(self.transition_to_type_discovery());
        }

        // Build corrections for all types with serialization issues
        debug!("SerializationCheck: Building corrections for serialization issues");

        // Extract the discovery context since we know there are serialization issues
        let discovery_context = self.context.into_inner();

        let educational_message = format!(
            "Component is registered but lacks Serialize and Deserialize traits required for {} operations. \
            Add #[derive(Serialize, Deserialize)] to the component definition.",
            self.method.as_str()
        );

        let corrections: Vec<Correction> = discovery_context
            .types()
            .filter(|type_info| type_info.in_registry() && !type_info.is_brp_compatible())
            .map(|type_info| {
                debug!(
                    "SerializationCheck: Component '{}' lacks serialization, building correction",
                    type_info.type_name().as_str()
                );
                let reason = format!(
                    "Component '{}' lacks serialization support. {}",
                    type_info.type_name().as_str(),
                    educational_message
                );
                // Since serialization issues can't be fixed by BRP calls, these are always
                // guidance-only
                Correction::Uncorrectable {
                    type_info: type_info.clone(),
                    reason,
                }
            })
            .collect();

        // Evaluate whether corrections are retryable or guidance-only
        // For serialization issues, corrections are always educational/guidance-only
        if are_corrections_retryable(&corrections) {
            // This shouldn't happen for serialization issues, but handle it just in case
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
            // Create guidance state for educational corrections
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

    /// Transition to `ExtrasDiscovery` state, preserving the discovery context
    fn transition_to_type_discovery(self) -> DiscoveryEngine<ExtrasDiscovery> {
        DiscoveryEngine {
            method:         self.method,
            operation:      self.operation,
            port:           self.port,
            params:         self.params,
            original_error: self.original_error,
            context:        ExtrasDiscovery(self.context.into_inner()),
        }
    }
}
