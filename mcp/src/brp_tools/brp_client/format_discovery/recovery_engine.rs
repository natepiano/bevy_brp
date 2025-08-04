//! Format error recovery engine with 3-level architecture
//!
//! Recovery levels (early-exit design):
//! 1. Registry checks - Fast type registration and serialization trait verification
//! 2. Direct discovery - Query running Bevy app via `bevy_brp_extras` for type schemas
//! 3. Pattern transformations - Apply known fixes for common format errors
//!
//! Each level returns immediately on success to minimize processing.

use std::collections::HashMap;

use serde_json::Value;
use tracing::debug;

use super::detection::ErrorPattern;
use super::engine::LevelResult;
use super::format_correction_fields::FormatCorrectionField;
use super::transformers::TransformerRegistry;
use super::types::{
    Correction, CorrectionInfo, CorrectionMethod, EnumInfo, EnumVariant, TransformationResult,
    TypeCategory,
};
use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::brp_client::BrpClientError;
use crate::tool::{BrpMethod, JsonFieldAccess, ParameterName};

/// Level 3: Apply pattern-based transformations for known errors
pub fn execute_level_3_pattern_transformations(
    type_names: &[String],
    method: BrpMethod,
    original_params: &Value,
    original_error: &BrpClientError,
    type_infos: &HashMap<String, UnifiedTypeInfo>,
) -> LevelResult {
    debug!(
        "Level 3: Applying pattern transformations for {} types",
        type_names.len()
    );

    // Initialize transformer registry with default transformers
    let transformer_registry = TransformerRegistry::with_defaults();
    let mut corrections = Vec::new();

    // Extract original values from parameters for transformation
    let original_values = extract_type_values_from_params(method, original_params);

    // For mutation methods, also extract the path that was attempted
    let mutation_path = if matches!(
        method,
        BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
    ) {
        original_params
            .get(FormatCorrectionField::Path.as_ref())
            .and_then(|v| v.as_str())
    } else {
        None
    };

    // No discovery context needed anymore

    for type_name in type_names {
        debug!("Level 3: Checking transformation patterns for '{type_name}'");

        // Get the type info from previous levels (may be None if type wasn't in registry)
        let type_info = type_infos.get(type_name);

        // Try to generate format corrections using the transformer registry
        if let Some(correction) = attempt_pattern_based_correction(
            type_name,
            &transformer_registry,
            original_values,
            original_error,
            method,
            mutation_path,
            type_info,
        ) {
            debug!("Level 3: Found pattern-based correction for '{type_name}'");
            corrections.push(correction);
        } else {
            debug!("Level 3: No pattern-based correction found for '{type_name}'");

            // Create a failure correction to provide feedback
            let failure_correction = if let Some(existing_type_info) = type_info.cloned() {
                // Type was discovered but couldn't be corrected
                Correction::Uncorrectable {
                    type_info: existing_type_info,
                    reason:    format!(
                        "Format discovery attempted pattern-based correction for type '{type_name}' but no applicable transformer could handle the error pattern. This may indicate a limitation in the current transformation logic or an unsupported format combination."
                    ),
                }
            } else {
                // Type was never discovered - don't create synthetic type info
                // This will be handled by the original BRP error message
                debug!(
                    "Level 3: Type '{type_name}' was never discovered, skipping synthetic correction"
                );
                continue;
            };
            corrections.push(failure_correction);
        }
    }

    if corrections.is_empty() {
        debug!("Level 3: Pattern transformations complete, no corrections found");
        LevelResult::Continue(HashMap::new())
    } else {
        debug!(
            "Level 3: Found {} pattern-based corrections",
            corrections.len()
        );
        LevelResult::Success(corrections)
    }
}

/// Handle mutation-specific errors
fn handle_mutation_specific_errors(
    method: BrpMethod,
    mutation_path: Option<&str>,
    error_pattern: &ErrorPattern,
    type_name: &str,
    type_info: Option<&UnifiedTypeInfo>,
) -> Option<Correction> {
    if !matches!(
        method,
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
                            Valid paths:\n{}",
                            paths_list.join("\n")
                        )
                    }
                },
            );

            // Use the existing type_info if available, or create a new one
            let final_type_info = type_info
                .cloned()
                .unwrap_or_else(|| UnifiedTypeInfo::for_pattern_matching(type_name.to_string()));

            Some(Correction::Uncorrectable {
                type_info: final_type_info,
                reason:    hint,
            })
        }
        _ => None,
    }
}

/// Try to generate pattern-based corrections for well-known types
fn attempt_pattern_based_correction(
    type_name: &str,
    transformer_registry: &TransformerRegistry,
    original_value: Option<&Value>,
    error: &BrpClientError,
    method: BrpMethod,
    mutation_path: Option<&str>,
    type_info: Option<&UnifiedTypeInfo>,
) -> Option<Correction> {
    debug!("Level 3: Attempting pattern correction for type '{type_name}'");

    // Step 1: Analyze the error pattern
    let error_analysis = super::detection::analyze_error_pattern(error);
    let Some(error_pattern) = error_analysis.pattern else {
        debug!("Level 3: No recognizable error pattern found for type '{type_name}'");
        return None;
    };

    debug!("Level 3: Identified error pattern: {error_pattern:?}");

    // Step 1.5: Handle mutation-specific errors
    if let Some(result) =
        handle_mutation_specific_errors(method, mutation_path, &error_pattern, type_name, type_info)
    {
        return Some(result);
    }

    // Step 2: Apply transformation if we have an original value
    let Some(original_value) = original_value else {
        debug!("Level 3: No original value available for transformation");
        // For enum types, we might be able to return enhanced format info
        if matches!(
            error_pattern,
            ErrorPattern::EnumUnitVariantMutation { .. }
                | ErrorPattern::EnumUnitVariantAccessError { .. }
        ) {
            return Some(create_enhanced_enum_guidance(type_name, &error_pattern));
        }
        return None;
    };

    // Step 3: Use type info from registry or create basic one as fallback
    let type_info_owned = create_basic_type_info(type_name);
    let type_info_ref = type_info.unwrap_or(&type_info_owned);

    // Step 3.5: Try UnifiedTypeInfo's transform_value() first if available
    if let Some(type_info) = type_info {
        if let Some(corrected_value) = type_info.transform_value(original_value) {
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
            };

            return Some(Correction::Candidate { correction_info });
        }
    }

    // Step 4: Try transformation with type information
    if let Some(transformation_result) = transformer_registry.transform_with_type_info(
        original_value,
        &error_pattern,
        error,
        type_info_ref,
    ) {
        debug!("Level 3: Successfully transformed value for type '{type_name}'");
        let mut correction_result =
            transform_result_to_correction(transformation_result, type_name);

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
    if let Some(transformation_result) =
        transformer_registry.transform_legacy(original_value, &error_pattern, error)
    {
        debug!("Level 3: Successfully applied fallback transformation for type '{type_name}'");
        let mut correction_result =
            transform_result_to_correction(transformation_result, type_name);

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
    fallback_pattern_based_correction(type_name)
}

/// Extract original values from BRP method parameters for transformer use
fn extract_type_values_from_params(method: BrpMethod, params: &Value) -> Option<&Value> {
    match method {
        BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
            // Return the components object containing type values
            ParameterName::Components.get_from(params)
        }
        BrpMethod::BevyMutateComponent
        | BrpMethod::BevyInsertResource
        | BrpMethod::BevyMutateResource => {
            // Return the value field
            params.get(FormatCorrectionField::Value.as_ref())
        }
        _ => {
            // For other methods, we don't currently support value extraction
            None
        }
    }
}

/// Convert transformer output to `CorrectionResult`
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
    };

    Correction::Candidate { correction_info }
}

/// Create basic type info for transformer use
fn create_basic_type_info(type_name: &str) -> UnifiedTypeInfo {
    UnifiedTypeInfo::for_pattern_matching(type_name.to_string())
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

            let type_info = UnifiedTypeInfo::for_math_type(t.to_string());

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

/// Create enhanced guidance for enum types when we can't transform but can provide format info
fn create_enhanced_enum_guidance(type_name: &str, error_pattern: &ErrorPattern) -> Correction {
    debug!("Level 3: Creating enhanced enum guidance for type '{type_name}'");

    let mut type_info = create_basic_type_info(type_name);
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

    if !variants.is_empty() {
        type_info.enum_info = Some(EnumInfo { variants });
    }
    type_info.supported_operations = vec![
        "spawn".to_string(),
        "insert".to_string(),
        "mutate".to_string(),
    ];

    Correction::Uncorrectable {
        type_info,
        reason: "Enhanced enum guidance with variant information and usage examples".to_string(),
    }
}
