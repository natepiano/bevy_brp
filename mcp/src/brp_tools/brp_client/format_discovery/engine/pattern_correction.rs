//! `PatternCorrection` state implementation
//!
//! This module implements the `PatternCorrection` state for the discovery engine.
//! This state applies pattern-based corrections when extras discovery is unavailable
//! or fails. This is the terminal state of the discovery process.

use serde_json::Value;
use tracing::debug;

use super::super::detection::ErrorPattern;
use super::super::format_correction_fields::FormatCorrectionField;
use super::super::transformers;
use super::super::types::{
    Correction, CorrectionInfo, CorrectionMethod, CorrectionSource, EnumInfo, EnumVariant,
    TransformationResult, TypeCategory,
};
use super::super::unified_types::UnifiedTypeInfo;
use super::recovery_result::FormatRecoveryResult;
use super::types::{DiscoveryEngine, PatternCorrection};
use crate::brp_tools::{BrpClientError, ResponseStatus, brp_client};
use crate::error::Result;
use crate::tool::{BrpMethod, ParameterName};

impl DiscoveryEngine<PatternCorrection> {
    /// Apply pattern-based corrections (terminal state)
    ///
    /// This method implements Level 3: Pattern-Based Transformations from the old engine.
    /// It processes types using transformer registry and pattern matching to generate
    /// corrections when possible.
    ///
    /// Returns `Result<FormatRecoveryResult>` as this is a terminal state.
    pub async fn apply_pattern_corrections(self) -> Result<FormatRecoveryResult> {
        debug!(
            "PatternCorrection: Applying pattern transformations for {} types",
            self.state.type_names().len()
        );

        // Execute Level 3: Pattern-Based Transformations
        if let Some(corrections) = self.execute_level_3_pattern_transformations() {
            debug!("PatternCorrection: Level 3 succeeded with pattern-based corrections");
            Ok(self.build_recovery_result(corrections).await)
        } else {
            debug!("PatternCorrection: All levels exhausted, no recovery possible");
            Ok(FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            })
        }
    }

    /// Level 3: Pattern-based transformations
    fn execute_level_3_pattern_transformations(&self) -> Option<Vec<Correction>> {
        let type_names = self.state.type_names();
        debug!(
            "Level 3: Applying pattern transformations for {} types",
            type_names.len()
        );

        let transformer_registry = transformers::transformer_registry();
        let mut corrections = Vec::new();

        // For mutation methods, extract the path
        let mutation_path = if matches!(
            self.method,
            BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
        ) {
            self.params.get("path").and_then(|p| p.as_str())
        } else {
            None
        };

        // Process each type
        for type_info in self.state.types() {
            let type_name = &type_info.type_name;

            debug!("Level 3: Checking transformation patterns for '{type_name}'");

            // Try to generate format corrections using the transformer registry
            if let Some(correction) = self.attempt_pattern_based_correction(
                type_name,
                transformer_registry,
                mutation_path,
                Some(type_info),
            ) {
                debug!("Level 3: Found pattern-based correction for '{type_name}'");
                corrections.push(correction);
            } else {
                debug!("Level 3: No pattern-based correction found for '{type_name}'");

                // Handle uncorrectable types with discovered info
                corrections.push(Correction::Uncorrectable {
                    type_info: type_info.clone(),
                    reason: format!(
                        "Format discovery attempted pattern-based correction for type '{type_name}' but no applicable transformer could handle the error pattern."
                    ),
                });
            }
        }

        if corrections.is_empty() {
            debug!("Level 3: No pattern-based corrections found");
            None
        } else {
            debug!(
                "Level 3: Found {} pattern-based corrections",
                corrections.len()
            );
            Some(corrections)
        }
    }

    /// Attempt pattern-based correction for a specific type
    fn attempt_pattern_based_correction(
        &self,
        type_name: &str,
        transformer_registry: &transformers::TransformerRegistry,
        mutation_path: Option<&str>,
        type_info: Option<&UnifiedTypeInfo>,
    ) -> Option<Correction> {
        debug!("Level 3: Attempting pattern correction for type '{type_name}'");

        // Step 1: Analyze the error pattern
        let error_analysis = super::super::detection::analyze_error_pattern(&self.original_error);
        let Some(error_pattern) = error_analysis.pattern else {
            debug!("Level 3: No recognizable error pattern found for type '{type_name}'");
            return None;
        };

        debug!("Level 3: Identified error pattern: {error_pattern:?}");

        // Step 1.5: Handle mutation-specific errors
        if let Some(result) = self.handle_mutation_specific_errors(
            mutation_path,
            &error_pattern,
            type_name,
            type_info,
        ) {
            return Some(result);
        }

        // Step 2: Get original value from type_info if available
        let original_value = type_info.and_then(|info| info.original_value.clone());

        let Some(original_value) = original_value else {
            debug!("Level 3: No original value available for transformation");
            // For enum types, we might be able to return enhanced format info
            if matches!(
                error_pattern,
                super::super::detection::ErrorPattern::EnumUnitVariantMutation { .. }
                    | super::super::detection::ErrorPattern::EnumUnitVariantAccessError { .. }
            ) {
                return Some(Self::create_enhanced_enum_guidance(
                    type_name,
                    &error_pattern,
                ));
            }
            return None;
        };

        // Step 3: Use type info from registry or create basic one as fallback
        let type_info_owned = Self::create_basic_type_info(type_name, Some(original_value.clone()));
        let type_info_ref = type_info.unwrap_or(&type_info_owned);

        // Step 3.5: Try UnifiedTypeInfo's transform_value() first if available
        if let Some(type_info) = type_info {
            if let Some(corrected_value) = type_info.transform_value(&original_value) {
                debug!(
                    "Level 3: Successfully transformed value using UnifiedTypeInfo.transform_value()"
                );

                let correction_info = CorrectionInfo {
                    type_name:         type_name.to_string(),
                    original_value:    original_value.clone(),
                    corrected_value:   corrected_value.clone(),
                    hint:              format!(
                        "Transformed {} format for type '{}'",
                        if original_value.is_object() {
                            "object"
                        } else {
                            "value"
                        },
                        type_name
                    ),
                    target_type:       type_name.to_string(),
                    corrected_format:  Some(corrected_value),
                    type_info:         Some(type_info.clone()),
                    correction_method: CorrectionMethod::ObjectToArray,
                    correction_source: CorrectionSource::PatternMatching,
                };

                return Some(Correction::Candidate { correction_info });
            }
        }

        // Step 4: Try transformation with type information
        if let Some(transformation_result) = transformer_registry.transform_with_type_info(
            &original_value,
            &error_pattern,
            &self.original_error,
            type_info_ref,
        ) {
            debug!("Level 3: Successfully transformed value for type '{type_name}'");
            let mut correction_result =
                Self::transform_result_to_correction(transformation_result, type_name);

            // Add the original value to the correction info
            if let Correction::Candidate {
                ref mut correction_info,
            } = correction_result
            {
                correction_info.original_value = original_value.clone();
                correction_info.type_info = Some(type_info_ref.clone());
            }

            return Some(correction_result);
        }

        // Step 5: Fall back to error-only transformation
        if let Some(transformation_result) = transformer_registry.transform_legacy(
            &original_value,
            &error_pattern,
            &self.original_error,
        ) {
            debug!("Level 3: Successfully applied fallback transformation for type '{type_name}'");
            let mut correction_result =
                Self::transform_result_to_correction(transformation_result, type_name);

            // Add the original value to the correction info
            if let Correction::Candidate {
                ref mut correction_info,
            } = correction_result
            {
                correction_info.original_value = original_value.clone();
            }

            return Some(correction_result);
        }

        // Step 6: Fall back to old pattern-based approach for well-known types
        debug!(
            "Level 3: No transformer could handle the error pattern, falling back to pattern matching"
        );
        Self::fallback_pattern_based_correction(type_name)
    }

    /// Build a recovery result from corrections
    #[allow(clippy::too_many_lines)]
    async fn build_recovery_result(
        &self,
        correction_results: Vec<Correction>,
    ) -> FormatRecoveryResult {
        let mut corrections = Vec::new();
        let mut has_applied_corrections = false;

        for correction_result in correction_results {
            match correction_result {
                Correction::Candidate { correction_info } => {
                    let type_name = correction_info.type_name.clone();
                    corrections.push(correction_info);
                    has_applied_corrections = true;
                    debug!("Recovery Engine: Applied correction for type '{type_name}'");
                }
                Correction::Uncorrectable { type_info, reason } => {
                    debug!(
                        "Recovery Engine: Found metadata for type '{}' but no correction: {}",
                        type_info.type_name, reason
                    );
                    // Create a CorrectionInfo from metadata-only result to provide guidance
                    let correction_info = CorrectionInfo {
                        type_name:         type_info.type_name.clone(),
                        original_value:    type_info
                            .original_value
                            .clone()
                            .unwrap_or_else(|| serde_json::json!({})),
                        corrected_value:   build_corrected_value_from_type_info(
                            &type_info,
                            self.method,
                        ),
                        hint:              reason,
                        target_type:       type_info.type_name.clone(),
                        corrected_format:  None,
                        type_info:         Some(type_info),
                        correction_method: CorrectionMethod::DirectReplacement,
                        correction_source: CorrectionSource::PatternMatching,
                    };
                    corrections.push(correction_info);
                }
            }
        }

        if corrections.is_empty() {
            debug!("Recovery Engine: No corrections found, returning original error");
            return FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            };
        }

        // Check if we can actually apply the corrections (i.e., we have fixable corrections)
        if has_applied_corrections && can_retry_with_corrections(&corrections) {
            debug!("Recovery Engine: Attempting to retry operation with corrected parameters");

            // Build corrected parameters
            if let Some(corrected_params) =
                build_corrected_params(self.method, &self.params, &corrections)
            {
                debug!("Recovery Engine: Built corrected parameters, executing retry");

                // Execute the retry asynchronously
                let client =
                    brp_client::BrpClient::new(self.method, self.port, Some(corrected_params));
                let retry_result = client.execute_raw().await;

                match retry_result {
                    Ok(brp_result) => match brp_result {
                        ResponseStatus::Success(value) => {
                            debug!("Recovery Engine: Retry succeeded with corrected parameters");
                            FormatRecoveryResult::Recovered {
                                corrected_result: ResponseStatus::Success(value),
                                corrections,
                            }
                        }
                        ResponseStatus::Error(brp_err) => {
                            debug!(
                                "Recovery Engine: Retry failed with corrected parameters: {}",
                                brp_err.message
                            );
                            FormatRecoveryResult::CorrectionFailed {
                                retry_error: ResponseStatus::Error(brp_err),
                                corrections,
                            }
                        }
                    },
                    Err(client_err) => {
                        debug!(
                            "Recovery Engine: Retry failed with client error: {}",
                            client_err
                        );
                        FormatRecoveryResult::CorrectionFailed {
                            retry_error: ResponseStatus::Error(BrpClientError {
                                code:    -32603,
                                message: format!("Client error during retry: {client_err}"),
                                data:    None,
                            }),
                            corrections,
                        }
                    }
                }
            } else {
                debug!("Recovery Engine: Failed to build corrected parameters");
                FormatRecoveryResult::CorrectionFailed {
                    retry_error: ResponseStatus::Error(BrpClientError {
                        code:    -32602,
                        message:
                            "Parameter correction failed: could not build corrected parameters"
                                .to_string(),
                        data:    None,
                    }),
                    corrections,
                }
            }
        } else {
            debug!("Recovery Engine: Corrections available but not retryable, providing guidance");
            FormatRecoveryResult::NotRecoverable { corrections }
        }
    }

    /// Handle mutation-specific errors for invalid paths
    fn handle_mutation_specific_errors(
        &self,
        mutation_path: Option<&str>,
        error_pattern: &ErrorPattern,
        type_name: &str,
        type_info: Option<&UnifiedTypeInfo>,
    ) -> Option<Correction> {
        if !matches!(
            self.method,
            BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
        ) {
            return None;
        }

        let attempted_path = mutation_path?;

        match error_pattern {
            ErrorPattern::MissingField { field_name, .. }
            | ErrorPattern::AccessError {
                access: field_name, ..
            } => {
                debug!(
                    "Level 3: Mutation path error - invalid path '{attempted_path}' (field: '{field_name}')"
                );

                // Use the registry type_info if available to provide better guidance
                let hint = type_info.map_or_else(
                    || {
                        // No registry info available at all
                        format!(
                            "Invalid mutation path '{attempted_path}' for type '{type_name}'. \
                            The field '{field_name}' does not exist. \
                            Use bevy_brp_extras/discover_format to find valid mutation paths."
                        )
                    },
                    |registry_info| {
                        let mutation_paths = registry_info.get_mutation_paths();

                        if mutation_paths.is_empty() {
                            // No mutation paths available from registry
                            format!(
                                "Invalid mutation path '{attempted_path}' for type '{type_name}'. \
                                The field '{field_name}' does not exist."
                            )
                        } else {
                            // We have valid paths from registry or discovery
                            let paths_list: Vec<String> = mutation_paths
                                .iter()
                                .map(|(path, desc)| format!("{path} - {desc}"))
                                .collect();

                            format!(
                                "Invalid mutation path '{attempted_path}' for type '{type_name}'. \
                                The field '{field_name}' does not exist. Valid paths: {}",
                                paths_list.join(", ")
                            )
                        }
                    },
                );

                let corrected_value = type_info.map_or_else(
                    || serde_json::json!({}),
                    |info| build_corrected_value_from_type_info(info, self.method),
                );

                let type_info_ref = type_info.map_or_else(
                    || Self::create_basic_type_info(type_name, None),
                    Clone::clone,
                );

                let correction_info = CorrectionInfo {
                    type_name: type_name.to_string(),
                    original_value: serde_json::json!({}),
                    corrected_value,
                    hint,
                    target_type: type_name.to_string(),
                    corrected_format: None,
                    type_info: Some(type_info_ref),
                    correction_method: CorrectionMethod::DirectReplacement,
                    correction_source: CorrectionSource::PatternMatching,
                };

                Some(Correction::Uncorrectable {
                    type_info: correction_info.type_info.clone()?,
                    reason:    correction_info.hint,
                })
            }
            _ => None,
        }
    }

    /// Create enhanced enum guidance from error patterns
    fn create_enhanced_enum_guidance(type_name: &str, error_pattern: &ErrorPattern) -> Correction {
        debug!("Level 3: Creating enhanced enum guidance for type '{type_name}'");
        debug!("Level 3: Error pattern: {:?}", error_pattern);

        let mut type_info = Self::create_basic_type_info(type_name, None);
        type_info.type_category = TypeCategory::Enum;

        // Extract variant information from the error pattern
        let valid_values = match error_pattern {
            ErrorPattern::EnumUnitVariantMutation {
                expected_variant_type,
                actual_variant_type: _,
            }
            | ErrorPattern::EnumUnitVariantAccessError {
                expected_variant_type,
                actual_variant_type: _,
                ..
            } => {
                vec![expected_variant_type.clone()]
            }
            _ => {
                // General enum guidance
                Vec::new()
            }
        };

        // Create basic enum info for Level 3 fallback
        let variants: Vec<EnumVariant> = valid_values
            .into_iter()
            .map(|name| EnumVariant {
                name,
                variant_type: "Unit".to_string(),
            })
            .collect();

        if variants.is_empty() {
            debug!("Level 3: No variants extracted from error pattern");
        } else {
            debug!(
                "Level 3: Setting enum_info with {} variants: {:?}",
                variants.len(),
                variants
            );
            type_info.enum_info = Some(EnumInfo { variants });
        }
        type_info.supported_operations = vec![
            "spawn".to_string(),
            "insert".to_string(),
            "mutate".to_string(),
        ];

        Correction::Uncorrectable {
            type_info,
            reason: "Enhanced enum guidance with variant information and usage examples"
                .to_string(),
        }
    }

    /// Create basic type info for pattern matching
    fn create_basic_type_info(type_name: &str, original_value: Option<Value>) -> UnifiedTypeInfo {
        UnifiedTypeInfo::for_pattern_matching(type_name.to_string(), original_value)
    }

    /// Convert transformer output to `Correction`
    fn transform_result_to_correction(result: TransformationResult, type_name: &str) -> Correction {
        let TransformationResult {
            corrected_value,
            hint: description,
        } = result;

        // Create correction info
        let correction_info = CorrectionInfo {
            type_name: type_name.to_string(),
            original_value: serde_json::Value::Null, // Will be filled by caller if available
            corrected_value,
            hint: description,
            target_type: type_name.to_string(),
            corrected_format: None,
            type_info: None,
            correction_method: CorrectionMethod::DirectReplacement,
            correction_source: CorrectionSource::PatternMatching,
        };

        Correction::Candidate { correction_info }
    }

    /// Fallback to the original pattern-based correction for well-known types
    fn fallback_pattern_based_correction(type_name: &str) -> Option<Correction> {
        match type_name {
            // Math types - common object vs array issues
            t if t.contains("Vec2")
                || t.contains("Vec3")
                || t.contains("Vec4")
                || t.contains("Quat") =>
            {
                debug!("Level 3: Detected math type '{t}', providing array format guidance");

                let type_info = UnifiedTypeInfo::for_math_type(t.to_string(), None);

                let reason = if t.contains("Quat") {
                    format!(
                        "Quaternion type '{t}' uses array format [x, y, z, w] where w is typically 1.0 for identity"
                    )
                } else {
                    format!(
                        "Math type '{t}' typically uses array format [x, y, ...] instead of object format"
                    )
                };

                Some(Correction::Uncorrectable { type_info, reason })
            }

            // Other types - no specific patterns yet
            _ => {
                debug!("Level 3: No specific pattern available for type '{type_name}'");
                None
            }
        }
    }
}

/// Check if corrections can be applied for a retry
fn can_retry_with_corrections(corrections: &[CorrectionInfo]) -> bool {
    // Only retry if we have corrections with actual values
    if corrections.is_empty() {
        return false;
    }

    // Check if all corrections have valid corrected values
    for correction in corrections {
        // Skip if the corrected value is just a placeholder or metadata
        if correction.corrected_value.is_null()
            || (correction.corrected_value.is_object()
                && correction.corrected_value.as_object().is_some_and(|o| {
                    o.contains_key(FormatCorrectionField::Hint.as_ref())
                        || o.contains_key(FormatCorrectionField::Examples.as_ref())
                        || o.contains_key(FormatCorrectionField::ValidValues.as_ref())
                }))
        {
            return false;
        }
    }

    true
}

/// Build corrected value from type info for display purposes
fn build_corrected_value_from_type_info(type_info: &UnifiedTypeInfo, method: BrpMethod) -> Value {
    debug!(
        "build_corrected_value_from_type_info: Building for type '{}' with method '{}', enum_info present: {}",
        type_info.type_name,
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
                serde_json::to_string_pretty(mutate_example)
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

/// Build corrected parameters from corrections
#[allow(clippy::unnecessary_wraps)]
fn build_corrected_params(
    method: BrpMethod,
    original_params: &Value,
    corrections: &[CorrectionInfo],
) -> Option<Value> {
    let mut params = original_params.clone();

    for correction in corrections {
        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
                // Update components
                if let Some(components) = ParameterName::Components.get_object_mut_from(&mut params)
                {
                    components.insert(
                        correction.type_name.clone(),
                        correction.corrected_value.clone(),
                    );
                }
            }
            BrpMethod::BevyInsertResource => {
                // Update value directly
                if let Some(value) = ParameterName::Value.get_mut_from(&mut params) {
                    *value = correction.corrected_value.clone();
                }
            }
            BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource => {
                // Update value directly for mutation
                if let Some(value) = ParameterName::Value.get_mut_from(&mut params) {
                    *value = correction.corrected_value.clone();
                }
            }
            _ => {
                // Other methods - no specific parameter handling yet
                debug!(
                    "build_corrected_params: No specific handling for method {:?}",
                    method
                );
            }
        }
    }

    Some(params)
}
