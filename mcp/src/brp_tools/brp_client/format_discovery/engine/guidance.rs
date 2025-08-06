//! Terminal guidance logic for the format discovery engine
//!
//! This module implements the terminal guidance state that provides educational
//! corrections and metadata but cannot be automatically retried.

use serde_json::Value;
use tracing::debug;

use super::super::format_correction_fields::FormatCorrectionField;
use super::recovery_result::FormatRecoveryResult;
use super::state::{DiscoveryEngine, Guidance};
use super::types::{Correction, CorrectionInfo, CorrectionMethod};
use super::unified_types::UnifiedTypeInfo;
use crate::tool::BrpMethod;

impl DiscoveryEngine<Guidance> {
    /// Provide guidance based on educational corrections
    ///
    /// This terminal method processes educational corrections and metadata
    /// to provide guidance to the user, always returning `NotRecoverable`.
    pub fn provide_guidance(self) -> FormatRecoveryResult {
        debug!("Guidance Engine: Processing educational corrections and metadata");

        let mut corrections = Vec::new();

        for correction_result in self.context.corrections {
            match correction_result {
                Correction::Candidate { correction_info } => {
                    // Include guidance-only candidates (with metadata/hints but no retry values)
                    corrections.push(correction_info);
                }
                Correction::Uncorrectable { type_info, reason } => {
                    debug!(
                        "Guidance Engine: Found metadata for type '{}' but no correction: {}",
                        type_info.type_name.as_str(),
                        reason
                    );
                    // Create a CorrectionInfo from metadata-only result to provide guidance
                    let correction_info = CorrectionInfo {
                        corrected_value: build_corrected_value_from_type_info(
                            &type_info,
                            self.method,
                        ),
                        hint: reason,
                        corrected_format: None,
                        type_info,
                        correction_method: CorrectionMethod::DirectReplacement,
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
}

/// Build a corrected value from type info for guidance
fn build_corrected_value_from_type_info(type_info: &UnifiedTypeInfo, method: BrpMethod) -> Value {
    debug!(
        "build_corrected_value_from_type_info: Building for type '{}' with method '{}', enum_info present: {}",
        type_info.type_name.as_str(),
        method.as_str(),
        type_info.enum_info.is_some()
    );

    // Check if we have examples for this method
    if let Some(example) = type_info.format_info.examples.get(method.as_str()) {
        debug!("build_corrected_value_from_type_info: Found example for method, returning it");
        return example.clone();
    }

    // For mutations, provide mutation path guidance
    debug!(
        "build_corrected_value_from_type_info: Checking mutation method match - method: {:?}",
        method
    );
    if matches!(
        method,
        BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
    ) {
        debug!(
            "build_corrected_value_from_type_info: Method matches mutation, proceeding with guidance"
        );
        // Check if we have a mutate example
        debug!(
            "build_corrected_value_from_type_info: Checking for mutate example, examples keys: {:?}",
            type_info.format_info.examples.keys().collect::<Vec<_>>()
        );
        if let Some(mutate_example) = type_info.format_info.examples.get("mutate") {
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
            FormatCorrectionField::Hint.as_ref(): "Use appropriate path and value for mutation"
        });

        if !type_info.format_info.mutation_paths.is_empty() {
            let paths: Vec<String> = type_info
                .format_info
                .mutation_paths
                .keys()
                .cloned()
                .collect();
            guidance[FormatCorrectionField::AvailablePaths.as_ref()] = serde_json::json!(paths);
        }

        // Add enum-specific guidance if this is an enum
        if let Some(enum_info) = &type_info.enum_info {
            let variants: Vec<String> = enum_info.variants.iter().map(|v| v.name.clone()).collect();
            debug!(
                "build_corrected_value_from_type_info: Adding enum guidance with {} variants: {:?}",
                variants.len(),
                variants
            );
            guidance[FormatCorrectionField::ValidValues.as_ref()] = serde_json::json!(variants);
            guidance[FormatCorrectionField::Hint.as_ref()] =
                serde_json::json!("Use empty path with variant name as value");
            guidance[FormatCorrectionField::Examples.as_ref()] = serde_json::json!([
                {FormatCorrectionField::Path.as_ref(): "", FormatCorrectionField::Value.as_ref(): variants.first().cloned().unwrap_or_else(|| "Variant1".to_string())},
                {FormatCorrectionField::Path.as_ref(): "", FormatCorrectionField::Value.as_ref(): variants.get(1).cloned().unwrap_or_else(|| "Variant2".to_string())}
            ]);
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
        "build_corrected_value_from_type_info: Method does not match mutation, returning empty object"
    );

    // Default to empty object
    debug!("build_corrected_value_from_type_info: Returning default empty object");
    serde_json::json!({})
}
