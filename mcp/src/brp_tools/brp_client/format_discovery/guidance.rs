//! Terminal guidance logic for the format discovery engine
//!
//! This module implements the terminal guidance state that provides educational
//! corrections and metadata but cannot be automatically retried.

use serde_json::Value;
use tracing::debug;

use super::discovery_context::TypeContext;
use super::format_correction_fields::FormatCorrectionField;
use super::recovery_result::FormatRecoveryResult;
use super::state::{DiscoveryEngine, Guidance};
use super::types::{Correction, CorrectionInfo, Operation};
use crate::string_traits::JsonFieldAccess;
use crate::tool::ParameterName;

impl DiscoveryEngine<Guidance> {
    /// Provide guidance based on educational corrections
    ///
    /// This terminal method processes educational corrections and metadata
    /// to provide guidance to the user, always returning `NotRecoverable`.
    pub fn provide_guidance(self) -> FormatRecoveryResult {
        debug!("Guidance Engine: Processing educational corrections and metadata");

        let mut corrections = Vec::new();

        for correction_result in &self.context.corrections {
            match correction_result {
                Correction::Candidate { correction_info } => {
                    // Include guidance-only candidates (with metadata/hints but no retry values)
                    corrections.push(correction_info.clone());
                }
                Correction::Uncorrectable { type_info, reason } => {
                    debug!(
                        "Guidance Engine: Found metadata for type '{}' but no correction: {}",
                        type_info.type_name().as_str(),
                        reason
                    );
                    // Create a CorrectionInfo from metadata-only result to provide guidance
                    let correction_info = CorrectionInfo {
                        corrected_value: self.build_corrected_value_from_type_info(type_info),
                        hint:            reason.to_string(),
                        type_info:       type_info.clone(),
                    };
                    corrections.push(correction_info);
                }
            }
        }

        if corrections.is_empty() {
            debug!("Guidance Engine: No corrections found, returning original error");
            return FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            };
        }

        debug!(
            "Guidance Engine: Returning guidance with {} corrections",
            corrections.len()
        );
        // Guidance always returns NotRecoverable as corrections are educational only
        FormatRecoveryResult::NotRecoverable { corrections }
    }

    /// Build a corrected value from type info for guidance
    fn build_corrected_value_from_type_info(&self, type_info: &TypeContext) -> Value {
        debug!(
            "build_corrected_value_from_type_info: Building for type '{}' with operation '{:?}', enum_info present: {}",
            type_info.type_name().as_str(),
            self.operation,
            type_info.enum_info().is_some()
        );

        // Check if we have examples for this operation
        if let Some(example) = type_info.get_example_for_operation(self.operation) {
            debug!(
                "build_corrected_value_from_type_info: Found example for operation, returning it"
            );
            return example.clone();
        }

        // For mutations, provide mutation path guidance
        debug!(
            "build_corrected_value_from_type_info: Checking mutation operation match - operation: {:?}",
            self.operation
        );
        if matches!(self.operation, Operation::Mutate { .. }) {
            debug!(
                "build_corrected_value_from_type_info: Operation matches mutation, proceeding with guidance"
            );
            // Check if we have a mutate example
            debug!(
                "build_corrected_value_from_type_info: Checking for mutate example, examples keys: {:?}",
                vec![&Operation::SpawnInsert {
                    parameter_name: ParameterName::Component,
                }] // Available example operations
            );
            if let Some(mutate_example) = type_info.get_example_for_operation(Operation::Mutate {
                parameter_name: ParameterName::Component,
            }) {
                debug!(
                    "build_corrected_value_from_type_info: Found mutate example, returning early: {}",
                    serde_json::to_string_pretty(&mutate_example)
                        .unwrap_or_else(|_| "Failed to serialize".to_string())
                );
                return mutate_example.clone();
            }
            debug!(
                "build_corrected_value_from_type_info: No mutate example found, proceeding to generate guidance"
            );

            let mut guidance = serde_json::json!({
                FormatCorrectionField::Hint: "Use appropriate path and value for mutation"
            });

            if !type_info.mutation_paths().is_empty() {
                let paths: Vec<String> = type_info.mutation_paths().keys().cloned().collect();
                guidance.insert_field(
                    FormatCorrectionField::AvailablePaths,
                    serde_json::json!(paths),
                );
            }

            // Add enum-specific guidance if this is an enum
            if let Some(enum_info) = type_info.enum_info() {
                let variants: Vec<String> =
                    enum_info.iter().map(|v| v.variant_name.clone()).collect();
                debug!(
                    "build_corrected_value_from_type_info: Adding enum guidance with {} variants: {:?}",
                    variants.len(),
                    variants
                );
                guidance.insert_field(
                    FormatCorrectionField::ValidValues,
                    serde_json::json!(variants),
                );
                guidance.insert_field(
                    FormatCorrectionField::Hint,
                    serde_json::json!("Use empty path with variant name as value"),
                );
                guidance.insert_field(FormatCorrectionField::Examples, serde_json::json!([
                    {FormatCorrectionField::Path: "", FormatCorrectionField::Value: variants.first().cloned().unwrap_or_else(|| "Variant1".to_string())},
                    {FormatCorrectionField::Path: "", FormatCorrectionField::Value: variants.get(1).cloned().unwrap_or_else(|| "Variant2".to_string())}
                ]));
                debug!(
                    "build_corrected_value_from_type_info: Final guidance with enum fields: {}",
                    serde_json::to_string_pretty(&guidance)
                        .unwrap_or_else(|_| "Failed to serialize".to_string())
                );
            } else {
                debug!(
                    "build_corrected_value_from_type_info: No enum_info found, not adding enum guidance"
                );
            }

            return guidance;
        }

        debug!(
            "build_corrected_value_from_type_info: Operation does not match mutation, returning empty object"
        );

        // Default to empty object
        debug!("build_corrected_value_from_type_info: Returning default empty object");
        serde_json::json!({})
    }
}
