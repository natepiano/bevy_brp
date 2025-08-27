//! Terminal guidance logic for the format discovery engine
//!
//! This module implements the terminal guidance state that provides educational
//! corrections and metadata but cannot be automatically retried.

use serde_json::Value;
use tracing::debug;

use super::recovery_result::FormatRecoveryResult;
use super::state::{DiscoveryEngine, Guidance};
use super::types::{Correction, CorrectionInfo, FormatCorrectionField, Operation};
use crate::brp_tools::brp_type_schema::BrpTypeName;
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
                Correction::Uncorrectable { type_name, reason } => {
                    debug!(
                        "Guidance Engine: Found metadata for type '{}' but no correction: {}",
                        type_name.as_str(),
                        reason
                    );
                    // Create a CorrectionInfo from metadata-only result to provide guidance
                    let original_value = self
                        .context
                        .extract_value_for_type(type_name)
                        .unwrap_or(serde_json::Value::Null);
                    let corrected_value = self.build_corrected_value_from_type_name(type_name);
                    let correction_info = CorrectionInfo {
                        corrected_value,
                        hint: reason.to_string(),
                        type_name: type_name.clone(),
                        original_value,
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

    /// Build a corrected value from type name for guidance
    fn build_corrected_value_from_type_name(&self, type_name: &BrpTypeName) -> Value {
        let Some(type_info) = self.context.get_type_info(type_name) else {
            return serde_json::json!({});
        };

        debug!(
            "build_corrected_value_from_type_name: Building for type '{}' with operation '{:?}', enum_info present: {}",
            type_name.as_str(),
            self.operation,
            type_info.enum_info.is_some()
        );

        // Check if we have examples for this operation
        if let Some(example) = self
            .context
            .get_example_for_operation(type_name, self.operation)
        {
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
            if let Some(mutate_example) = self.context.get_example_for_operation(
                type_name,
                Operation::Mutate {
                    parameter_name: ParameterName::Component,
                },
            ) {
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

            if let Some(mutation_paths) = self.context.mutation_paths(type_name)
                && !mutation_paths.is_empty()
            {
                let paths: Vec<String> = mutation_paths.keys().cloned().collect();
                guidance.insert_field(
                    FormatCorrectionField::AvailablePaths,
                    serde_json::json!(paths),
                );
            }

            // Add enum-specific guidance if this is an enum
            if let Some(enum_info) = self.context.enum_info(type_name) {
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
