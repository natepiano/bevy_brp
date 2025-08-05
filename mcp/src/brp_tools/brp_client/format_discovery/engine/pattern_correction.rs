//! `PatternCorrection` state implementation
//!
//! This module implements the `PatternCorrection` state for the discovery engine.
//! This state applies pattern-based corrections when extras discovery is unavailable
//! or fails. This is the terminal state of the discovery process.

use serde_json::Value;
use tracing::debug;

use super::super::detection::ErrorPattern;
use super::super::transformers;
use super::state::{DiscoveryEngine, Guidance, PatternCorrection, Retry};
use super::types::{
    BrpTypeName, Correction, CorrectionInfo, CorrectionMethod, EnumInfo, EnumVariant,
    TransformationResult, TypeCategory, are_corrections_retryable,
};
use super::unified_types::UnifiedTypeInfo;
use crate::tool::BrpMethod;

impl DiscoveryEngine<PatternCorrection> {
    /// Try to apply pattern-based corrections (terminal state)
    ///
    /// This method implements Level 3: Pattern-Based Transformations from the old engine.
    /// It processes types using transformer registry and pattern matching to generate
    /// corrections when possible.
    ///
    /// Returns `Either<Retry, Guidance>` based on correction evaluation.
    pub fn try_pattern_corrections(
        self,
    ) -> either::Either<DiscoveryEngine<Retry>, DiscoveryEngine<Guidance>> {
        debug!(
            "PatternCorrection: Applying pattern transformations for {} types",
            self.state.type_names().len()
        );

        // Execute Level 3: Pattern-Based Transformations
        let corrections = self
            .execute_level_3_pattern_transformations()
            .unwrap_or_default();

        debug!(
            "PatternCorrection: Found {} corrections from pattern transformations",
            corrections.len()
        );

        // Extract the discovery context for terminal state creation
        let discovery_context = self.state.into_inner();

        // Evaluate whether corrections are retryable or guidance-only
        if are_corrections_retryable(&corrections) {
            debug!("PatternCorrection: Corrections are retryable, creating Retry state");
            let retry_state = Retry::new(discovery_context, corrections);
            let retry_engine = DiscoveryEngine {
                method:         self.method,
                port:           self.port,
                params:         self.params,
                original_error: self.original_error,
                state:          retry_state,
            };
            either::Either::Left(retry_engine)
        } else {
            debug!("PatternCorrection: Corrections are guidance-only, creating Guidance state");
            let guidance_state = Guidance::new(discovery_context, corrections);
            let guidance_engine = DiscoveryEngine {
                method:         self.method,
                port:           self.port,
                params:         self.params,
                original_error: self.original_error,
                state:          guidance_state,
            };
            either::Either::Right(guidance_engine)
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
            let type_name = type_info.type_name.as_str();

            debug!("Level 3: Checking transformation patterns for '{type_name}'");

            // Try to generate format corrections using the transformer registry
            if let Some(correction) = self.attempt_pattern_based_correction(
                type_name,
                transformer_registry,
                mutation_path,
                type_info,
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
        type_info: &UnifiedTypeInfo,
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

        // Step 2: Get original value from type_info
        let Some(original_value) = type_info.original_value.clone() else {
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

        // Step 3.5: Try UnifiedTypeInfo's transform_value() first
        if let Some(corrected_value) = type_info.transform_value(&original_value) {
            debug!(
                "Level 3: Successfully transformed value using UnifiedTypeInfo.transform_value()"
            );

            let correction_info = CorrectionInfo {
                type_name:         type_name.into(),
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
                target_type:       type_name.into(),
                corrected_format:  Some(corrected_value),
                type_info:         type_info.clone(),
                correction_method: CorrectionMethod::ObjectToArray,
            };

            return Some(Correction::Candidate { correction_info });
        }

        // Step 4: Try transformation with type information
        if let Some(transformation_result) = transformer_registry.transform_with_type_info(
            &original_value,
            &error_pattern,
            &self.original_error,
            type_info,
        ) {
            debug!("Level 3: Successfully transformed value for type '{type_name}'");
            let correction_result = Self::transform_result_to_correction(
                transformation_result,
                type_name,
                type_info,
                original_value.clone(),
            );

            return Some(correction_result);
        }

        // Step 5: Fall back to old pattern-based approach for well-known types
        debug!(
            "Level 3: No transformer could handle the error pattern, falling back to pattern matching"
        );
        Self::fallback_pattern_based_correction(type_name)
    }

    /// Handle mutation-specific errors for invalid paths
    fn handle_mutation_specific_errors(
        &self,
        mutation_path: Option<&str>,
        error_pattern: &ErrorPattern,
        type_name: &str,
        type_info: &UnifiedTypeInfo,
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

                // Use the registry type_info to provide better guidance
                let hint = {
                    let mutation_paths = type_info.get_mutation_paths();

                    if mutation_paths.is_empty() {
                        // No mutation paths available from registry
                        format!(
                            "Invalid mutation path '{attempted_path}' for type '{type_name}'. \
                            The field '{field_name}' does not exist. \
                            Use bevy_brp_extras/discover_format to find valid mutation paths."
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
                };

                let corrected_value = serde_json::json!({}); // Simple guidance placeholder

                let correction_info = CorrectionInfo {
                    type_name: type_name.into(),
                    original_value: serde_json::json!({}),
                    corrected_value,
                    hint,
                    target_type: type_name.into(),
                    corrected_format: None,
                    type_info: type_info.clone(),
                    correction_method: CorrectionMethod::DirectReplacement,
                };

                Some(Correction::Uncorrectable {
                    type_info: type_info.clone(),
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
    fn create_basic_type_info(
        type_name: impl Into<BrpTypeName>,
        original_value: Option<Value>,
    ) -> UnifiedTypeInfo {
        UnifiedTypeInfo::for_pattern_matching(type_name, original_value)
    }

    /// Convert transformer output to `Correction`
    fn transform_result_to_correction(
        result: TransformationResult,
        type_name: &str,
        type_info: &UnifiedTypeInfo,
        original_value: Value,
    ) -> Correction {
        let TransformationResult {
            corrected_value,
            hint: description,
        } = result;

        // Create correction info
        let correction_info = CorrectionInfo {
            type_name: type_name.into(),
            original_value,
            corrected_value,
            hint: description,
            target_type: type_name.into(),
            corrected_format: None,
            type_info: type_info.clone(),
            correction_method: CorrectionMethod::DirectReplacement,
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

                let type_info = UnifiedTypeInfo::for_math_type(t, None);

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
