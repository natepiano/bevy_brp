//! `SerializationCheck` state implementation
//!
//! This module implements the `SerializationCheck` state for the discovery engine.
//! This state checks if types have required serialization traits to work around
//! a bug in Bevy 0.16 where "Unknown component type" errors are thrown even for
//! components with proper serialization support.

use either::Either;
use serde_json::json;
use tracing::debug;

use super::super::types::{CorrectionInfo, CorrectionMethod};
use super::recovery_result::FormatRecoveryResult;
use super::types::{DiscoveryEngine, ExtrasDiscovery, SerializationCheck};
use crate::tool::BrpMethod;

impl DiscoveryEngine<SerializationCheck> {
    /// Check for serialization issues that prevent BRP operations
    ///
    /// This method examines types in the discovery context to detect components
    /// that are registered in the type registry but lack the required Serialize
    /// and Deserialize traits for BRP operations.
    ///
    /// Returns `Either::Left(result)` if serialization issues are found,
    /// or `Either::Right(engine)` to continue with `ExtrasDiscovery`.
    pub fn check_serialization(
        self,
    ) -> Either<FormatRecoveryResult, DiscoveryEngine<ExtrasDiscovery>> {
        // Only check for spawn/insert methods with UnknownComponentType errors
        if !matches!(self.method, BrpMethod::BevySpawn | BrpMethod::BevyInsert) {
            debug!("SerializationCheck: Not a spawn/insert method, proceeding to ExtrasDiscovery");
            return Either::Right(self.transition_to_extras_discovery());
        }

        // Check if error message indicates a serialization issue
        if !self
            .original_error
            .message
            .contains("Unknown component type")
        {
            debug!(
                "SerializationCheck: Error message doesn't indicate serialization issue, proceeding to ExtrasDiscovery"
            );
            return Either::Right(self.transition_to_extras_discovery());
        }

        debug!("SerializationCheck: Checking for serialization errors in registry type infos");

        // Check each type for serialization support
        for type_info in self.state.types() {
            debug!(
                "SerializationCheck: Component '{}' found, brp_compatible={}",
                type_info.type_name, type_info.serialization.brp_compatible
            );

            // Component is registered but lacks serialization - create terminal result
            if type_info.registry_status.in_registry && !type_info.serialization.brp_compatible {
                debug!(
                    "SerializationCheck: Component '{}' lacks serialization, building corrections",
                    type_info.type_name
                );

                let educational_message = format!(
                    "Component '{}' is registered but lacks Serialize and Deserialize traits required for {} operations. \
                    Add #[derive(Serialize, Deserialize)] to the component definition.",
                    type_info.type_name,
                    self.method.as_str()
                );

                let corrections: Vec<CorrectionInfo> = self
                    .state
                    .types()
                    .map(|type_info| CorrectionInfo {
                        type_name:         type_info.type_name.clone(),
                        original_value:    type_info
                            .original_value
                            .clone()
                            .unwrap_or_else(|| json!({})),
                        corrected_value:   json!({}),
                        hint:              educational_message.clone(),
                        target_type:       type_info.type_name.clone(),
                        corrected_format:  None,
                        type_info:         Some(type_info.clone()),
                        correction_method: CorrectionMethod::DirectReplacement,
                    })
                    .collect();

                // Build terminal result for serialization issues
                let recovery_result = FormatRecoveryResult::NotRecoverable { corrections };

                return Either::Left(recovery_result);
            }
        }

        debug!(
            "SerializationCheck: All components have serialization support or are not in registry, proceeding to ExtrasDiscovery"
        );
        Either::Right(self.transition_to_extras_discovery())
    }

    /// Transition to `ExtrasDiscovery` state, preserving the discovery context
    fn transition_to_extras_discovery(self) -> DiscoveryEngine<ExtrasDiscovery> {
        DiscoveryEngine {
            method:         self.method,
            port:           self.port,
            params:         self.params,
            original_error: self.original_error,
            state:          ExtrasDiscovery(self.state.into_inner()),
        }
    }
}
