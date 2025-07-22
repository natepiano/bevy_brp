//! Format error recovery engine with 3-level architecture
//!
//! Recovery levels (early-exit design):
//! 1. Registry checks - Fast type registration and serialization trait verification
//! 2. Direct discovery - Query running Bevy app via `bevy_brp_extras` for type schemas
//! 3. Pattern transformations - Apply known fixes for common format errors
//!
//! Each level returns immediately on success to minimize processing.

use std::collections::HashMap;
use std::fmt::Write;

use serde_json::Value;
use tracing::debug;

use super::detection::ErrorPattern;
use super::extras_integration;
use super::flow_types::{CorrectionResult, FormatRecoveryResult};
use super::unified_types::{
    CorrectionInfo, CorrectionMethod, TransformationResult, TypeCategory, UnifiedTypeInfo,
};
use crate::brp_tools::FormatCorrectionField;
use crate::brp_tools::brp_client::{self, BrpError, BrpResult};
use crate::tool::BrpMethod;

/// Execute format error recovery using the 3-level decision tree with pre-fetched type infos
pub async fn attempt_format_recovery_with_type_infos(
    method: &str,
    original_params: Option<Value>,
    error: BrpResult,
    registry_type_info: HashMap<String, UnifiedTypeInfo>,
    port: u16,
) -> FormatRecoveryResult {
    debug!("Recovery Engine: FUNCTION ENTRY - attempt_format_recovery_with_type_infos called");
    debug!(
        "Recovery Engine: Starting multi-level recovery for method '{method}' with {} pre-fetched type info(s)",
        registry_type_info.len()
    );

    // Extract type names from the parameters for recovery attempts
    let type_names = extract_type_names_from_params(method, original_params.as_ref());
    if type_names.is_empty() {
        debug!("Recovery Engine: No type names found in parameters, cannot recover");
        return FormatRecoveryResult::NotRecoverable {
            original_error: error,
            corrections:    Vec::new(),
        };
    }

    debug!(
        "Recovery Engine: Found {} type names to process",
        type_names.len()
    );

    // Level 2: Direct Discovery via bevy_brp_extras
    debug!("Recovery Engine: beginning level 2 direct discovery");
    let level_2_type_infos = match execute_level_2_direct_discovery(
        &type_names,
        method,
        &registry_type_info,
        original_params.as_ref(),
        port,
    )
    .await
    {
        LevelResult::Success(corrections) => {
            debug!("Recovery Engine: Level 2 succeeded with direct discovery");
            return build_recovery_success(
                corrections,
                method,
                original_params.as_ref(),
                &error,
                port,
            )
            .await;
        }
        LevelResult::Continue(type_infos) => {
            debug!(
                "Recovery Engine: Level 2 complete, proceeding to Level 3 with {} type infos",
                type_infos.len()
            );
            type_infos
        }
    };

    // Level 3: Pattern-Based Transformations
    debug!("Recovery Engine: Level 3 - Pattern-based transformations");

    // Extract the BrpError from the error result to pass to Level 3
    let brp_error = match &error {
        BrpResult::Error(brp_error) => brp_error,
        BrpResult::Success(_) => {
            // This shouldn't happen as we only call recovery on errors
            debug!("Recovery Engine: Warning - Level 3 called with success result");
            return FormatRecoveryResult::NotRecoverable {
                original_error: error,
                corrections:    Vec::new(),
            };
        }
    };

    match execute_level_3_pattern_transformations(
        &type_names,
        method,
        original_params.as_ref(),
        brp_error,
        &level_2_type_infos,
    ) {
        LevelResult::Success(corrections) => {
            debug!("Recovery Engine: Level 3 succeeded with pattern-based corrections");
            build_recovery_success(corrections, method, original_params.as_ref(), &error, port)
                .await
        }
        LevelResult::Continue(_) => {
            debug!("Recovery Engine: All levels exhausted, no recovery possible");
            FormatRecoveryResult::NotRecoverable {
                original_error: error,
                corrections:    Vec::new(),
            }
        }
    }
}

/// Result of a recovery level attempt
#[derive(Debug)]
enum LevelResult {
    /// Level succeeded and produced corrections
    Success(Vec<CorrectionResult>),
    /// Level completed but recovery should continue to next level
    Continue(std::collections::HashMap<String, UnifiedTypeInfo>),
}

/// Level 2: Direct discovery via `bevy_brp_extras/discover_format`
async fn execute_level_2_direct_discovery(
    type_names: &[String],
    method: &str,
    registry_type_info: &HashMap<String, UnifiedTypeInfo>,
    original_params: Option<&Value>,
    port: u16,
) -> LevelResult {
    debug!(
        "Level 2: Attempting direct discovery for {} types",
        type_names.len()
    );

    // Start with type infos from Level 1
    let mut enhanced_type_info = registry_type_info.clone();

    // Attempt direct discovery for each type using bevy_brp_extras
    let mut corrections = Vec::new();

    for type_name in type_names {
        debug!("Level 2: Attempting direct discovery for '{type_name}'");

        // Call extras_integration to discover the type format
        match extras_integration::discover_type_format(type_name, port).await {
            Ok(Some(mut discovered_info)) => {
                debug!("Level 2: Successfully discovered type information for '{type_name}'");

                // Merge with existing type info from Level 1 if available
                if let Some(existing_info) = registry_type_info.get(type_name) {
                    // Preserve registry information but enhance with discovery data
                    discovered_info.registry_status = existing_info.registry_status.clone();
                    if discovered_info.type_category == TypeCategory::Unknown
                        && existing_info.type_category != TypeCategory::Unknown
                    {
                        discovered_info
                            .type_category
                            .clone_from(&existing_info.type_category);
                    }
                }

                // Update the enhanced type infos
                enhanced_type_info.insert(type_name.clone(), discovered_info.clone());

                // Check if this is a mutation method and we have mutation paths
                if (method == BrpMethod::BevyMutateComponent.as_str()
                    || method == BrpMethod::BevyMutateResource.as_str())
                    && discovered_info.supports_mutation()
                {
                    debug!(
                        "Level 2: Type '{}' supports mutation with {} paths",
                        type_name,
                        discovered_info.get_mutation_paths().len()
                    );

                    // Create a mutation-specific correction with available paths
                    let mut hint =
                        format!("Type '{type_name}' supports mutation. Available paths:\n");
                    for (path, description) in discovered_info.get_mutation_paths() {
                        let _ = writeln!(hint, "  {path} - {description}");
                    }

                    let correction = CorrectionResult::CannotCorrect {
                        type_info: discovered_info,
                        reason:    hint,
                    };
                    corrections.push(correction);
                } else {
                    // Extract the original value for this component
                    let original_component_value = original_params
                        .and_then(|params| extract_component_value(method, params, type_name));

                    // Create a correction from the discovered type information with original value
                    let correction = super::extras_integration::create_correction_from_discovery(
                        discovered_info,
                        original_component_value,
                    );
                    corrections.push(correction);
                }
            }
            Ok(None) => {
                debug!("Level 2: No type information found for '{type_name}' via direct discovery");
                // Keep the registry info from Level 1
            }
            Err(e) => {
                debug!("Level 2: Direct discovery failed for '{type_name}': {e}");
                // Keep the registry info from Level 1
            }
        }
    }

    // Determine the level result based on what we discovered
    if corrections.is_empty() {
        debug!(
            "Level 2: Direct discovery complete, proceeding to Level 3 with {} type infos",
            enhanced_type_info.len()
        );
        LevelResult::Continue(enhanced_type_info)
    } else {
        debug!(
            "Level 2: Found {} corrections from direct discovery",
            corrections.len()
        );
        LevelResult::Success(corrections)
    }
}

/// Level 3: Apply pattern-based transformations for known errors
fn execute_level_3_pattern_transformations(
    type_names: &[String],
    method: &str,
    original_params: Option<&Value>,
    original_error: &BrpError,
    type_infos: &HashMap<String, UnifiedTypeInfo>,
) -> LevelResult {
    debug!(
        "Level 3: Applying pattern transformations for {} types",
        type_names.len()
    );

    // Initialize transformer registry with default transformers
    let transformer_registry = super::transformers::TransformerRegistry::with_defaults();
    let mut corrections = Vec::new();

    // Extract original values from parameters for transformation
    let original_values = extract_type_values_from_params(method, original_params);

    // For mutation methods, also extract the path that was attempted
    let mutation_path = if method == BrpMethod::BevyMutateComponent.as_str()
        || method == BrpMethod::BevyMutateResource.as_str()
    {
        original_params
            .and_then(|p| p.get(FormatCorrectionField::Path.as_ref()))
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

            // Create a failure correction to provide structured feedback
            let failure_correction = if let Some(existing_type_info) = type_info.cloned() {
                // Type was discovered but couldn't be corrected
                CorrectionResult::CannotCorrect {
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
    method: &str,
    mutation_path: Option<&str>,
    error_pattern: &ErrorPattern,
    type_name: &str,
    type_info: Option<&UnifiedTypeInfo>,
) -> Option<CorrectionResult> {
    if method != BrpMethod::BevyMutateComponent.as_str()
        && method != BrpMethod::BevyMutateResource.as_str()
    {
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
            let final_type_info = type_info.cloned().unwrap_or_else(|| {
                super::unified_types::UnifiedTypeInfo::new(
                    type_name.to_string(),
                    super::unified_types::DiscoverySource::PatternMatching,
                )
            });

            Some(CorrectionResult::CannotCorrect {
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
    transformer_registry: &super::transformers::TransformerRegistry,
    original_value: Option<&Value>,
    error: &BrpError,
    method: &str,
    mutation_path: Option<&str>,
    type_info: Option<&UnifiedTypeInfo>,
) -> Option<CorrectionResult> {
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

            return Some(CorrectionResult::Corrected { correction_info });
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
        if let CorrectionResult::Corrected {
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
        if let CorrectionResult::Corrected {
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
fn extract_type_values_from_params<'a>(
    method: &str,
    params: Option<&'a Value>,
) -> Option<&'a Value> {
    let params = params?;

    match method {
        m if m == BrpMethod::BevySpawn.as_str() || m == BrpMethod::BevyInsert.as_str() => {
            // Return the components object containing type values
            params.get("components")
        }
        m if m == BrpMethod::BevyMutateComponent.as_str()
            || m == BrpMethod::BevyInsertResource.as_str()
            || m == BrpMethod::BevyMutateResource.as_str() =>
        {
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
fn transform_result_to_correction(
    result: TransformationResult,
    type_name: &str,
) -> CorrectionResult {
    let TransformationResult {
        corrected_value,
        hint: description,
    } = result;

    // Create correction info
    let correction_info = super::unified_types::CorrectionInfo {
        type_name: type_name.to_string(),
        original_value: serde_json::Value::Null, // Will be filled by caller if available
        corrected_value,
        hint: description,
        target_type: type_name.to_string(),
        corrected_format: None,
        type_info: None,
        correction_method: super::unified_types::CorrectionMethod::DirectReplacement,
    };

    CorrectionResult::Corrected { correction_info }
}

/// Create basic type info for transformer use
fn create_basic_type_info(type_name: &str) -> super::unified_types::UnifiedTypeInfo {
    super::unified_types::UnifiedTypeInfo::new(
        type_name.to_string(),
        super::unified_types::DiscoverySource::PatternMatching,
    )
}

/// Fallback to the original pattern-based correction for well-known types
fn fallback_pattern_based_correction(type_name: &str) -> Option<CorrectionResult> {
    match type_name {
        // Math types - common object vs array issues
        t if t.contains("Vec2")
            || t.contains("Vec3")
            || t.contains("Vec4")
            || t.contains("Quat") =>
        {
            debug!("Level 3: Detected math type '{t}', providing array format guidance");

            let mut type_info = UnifiedTypeInfo::new(
                t.to_string(),
                super::unified_types::DiscoverySource::PatternMatching,
            );

            // Set type category
            type_info.type_category = TypeCategory::MathType;

            // Ensure examples are generated
            type_info.ensure_examples();

            let reason = if t.contains("Quat") {
                format!(
                    "Quaternion type '{t}' uses array format [x, y, z, w] where w is typically 1.0 for identity"
                )
            } else {
                format!(
                    "Math type '{t}' typically uses array format [x, y, ...] instead of object format"
                )
            };

            Some(CorrectionResult::CannotCorrect { type_info, reason })
        }

        // Other types - no specific patterns yet
        _ => {
            debug!("Level 3: No specific pattern available for type '{type_name}'");
            None
        }
    }
}

/// Create enhanced guidance for enum types when we can't transform but can provide format info
fn create_enhanced_enum_guidance(
    type_name: &str,
    error_pattern: &ErrorPattern,
) -> CorrectionResult {
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
    let variants: Vec<super::unified_types::EnumVariant> = valid_values
        .into_iter()
        .map(|name| super::unified_types::EnumVariant {
            name,
            variant_type: "Unit".to_string(),
        })
        .collect();

    if !variants.is_empty() {
        type_info.enum_info = Some(super::unified_types::EnumInfo { variants });
    }
    type_info.supported_operations = vec![
        "spawn".to_string(),
        "insert".to_string(),
        "mutate".to_string(),
    ];

    CorrectionResult::CannotCorrect {
        type_info,
        reason: "Enhanced enum guidance with variant information and usage examples".to_string(),
    }
}

/// Extract type names from BRP method parameters based on method type
fn extract_type_names_from_params(method: &str, params: Option<&Value>) -> Vec<String> {
    let Some(params) = params else {
        return Vec::new();
    };

    let mut type_names = Vec::new();

    match method {
        m if m == BrpMethod::BevySpawn.as_str() || m == BrpMethod::BevyInsert.as_str() => {
            // Types are keys in the "components" object
            if let Some(components) = params.get("components").and_then(|c| c.as_object()) {
                for type_name in components.keys() {
                    type_names.push(type_name.clone());
                }
            }
        }
        m if m == BrpMethod::BevyMutateComponent.as_str() => {
            // Single type in "component" field
            if let Some(component) = params
                .get(FormatCorrectionField::Component.as_ref())
                .and_then(|c| c.as_str())
            {
                type_names.push(component.to_string());
            }
        }
        m if m == BrpMethod::BevyInsertResource.as_str()
            || m == BrpMethod::BevyMutateResource.as_str() =>
        {
            // Single type in "resource" field
            if let Some(resource) = params.get("resource").and_then(|r| r.as_str()) {
                type_names.push(resource.to_string());
            }
        }
        _ => {
            // For other methods, we don't currently support type extraction
        }
    }

    type_names
}

/// Check if corrections can be applied for a retry
fn can_retry_with_corrections(
    _method: &str,
    corrections: &[CorrectionInfo],
    original_params: Option<&Value>,
) -> bool {
    // Only retry if we have original params and corrections with actual values
    if original_params.is_none() || corrections.is_empty() {
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

/// Extract component value from parameters based on method
fn extract_component_value(method: &str, params: &Value, type_name: &str) -> Option<Value> {
    match method {
        m if m == BrpMethod::BevySpawn.as_str() || m == BrpMethod::BevyInsert.as_str() => params
            .get("components")
            .and_then(|c| c.get(type_name))
            .cloned(),
        m if m == BrpMethod::BevyInsertResource.as_str()
            || m == BrpMethod::BevyMutateComponent.as_str()
            || m == BrpMethod::BevyMutateResource.as_str() =>
        {
            params.get(FormatCorrectionField::Value.as_ref()).cloned()
        }
        _ => None,
    }
}

/// Build a corrected value from type info for guidance
fn build_corrected_value_from_type_info(type_info: &UnifiedTypeInfo, method: &str) -> Value {
    // Clone the type info so we can call ensure_examples on it
    let mut type_info_copy = type_info.clone();
    type_info_copy.ensure_examples();

    // Check if we have examples for this method
    if let Some(example) = type_info_copy.format_info.examples.get(method) {
        return example.clone();
    }

    // For mutations, provide mutation path guidance
    if method == BrpMethod::BevyMutateComponent.as_str()
        || method == BrpMethod::BevyMutateResource.as_str()
    {
        // Check if we have a mutate example after ensure_examples
        if let Some(mutate_example) = type_info_copy.format_info.examples.get("mutate") {
            return mutate_example.clone();
        }

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
            guidance[FormatCorrectionField::ValidValues.as_ref()] = serde_json::json!(variants);
            guidance[FormatCorrectionField::Hint.as_ref()] =
                serde_json::json!("Use empty path with variant name as value");
            guidance[FormatCorrectionField::Examples.as_ref()] = serde_json::json!([
                {FormatCorrectionField::Path.as_ref(): "", FormatCorrectionField::Value.as_ref(): variants.first().cloned().unwrap_or_else(|| "Variant1".to_string())},
                {FormatCorrectionField::Path.as_ref(): "", FormatCorrectionField::Value.as_ref(): variants.get(1).cloned().unwrap_or_else(|| "Variant2".to_string())}
            ]);
        }

        return guidance;
    }

    // Default to empty object
    serde_json::json!({})
}

/// Build corrected parameters from corrections
fn build_corrected_params(
    method: &str,
    original_params: Option<&Value>,
    corrections: &[CorrectionInfo],
) -> Result<Option<Value>, String> {
    let mut params = original_params
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    for correction in corrections {
        match method {
            m if m == BrpMethod::BevySpawn.as_str() || m == BrpMethod::BevyInsert.as_str() => {
                // Update components
                if let Some(components) =
                    params.get_mut("components").and_then(|c| c.as_object_mut())
                {
                    components.insert(
                        correction.type_name.clone(),
                        correction.corrected_value.clone(),
                    );
                }
            }
            m if m == BrpMethod::BevyInsertResource.as_str() => {
                // Update value directly
                params[FormatCorrectionField::Value.as_ref()] = correction.corrected_value.clone();
            }
            m if m == BrpMethod::BevyMutateComponent.as_str()
                || m == BrpMethod::BevyMutateResource.as_str() =>
            {
                // For mutations, we need both path and value
                if correction.corrected_value.is_object() {
                    if let Some(obj) = correction.corrected_value.as_object() {
                        if let (Some(path), Some(value)) = (
                            obj.get(FormatCorrectionField::Path.as_ref()),
                            obj.get(FormatCorrectionField::Value.as_ref()),
                        ) {
                            params[FormatCorrectionField::Path.as_ref()] = path.clone();
                            params[FormatCorrectionField::Value.as_ref()] = value.clone();
                        } else {
                            return Err("Mutation correction missing path or value".to_string());
                        }
                    }
                } else {
                    // Simple value correction
                    params[FormatCorrectionField::Value.as_ref()] =
                        correction.corrected_value.clone();
                }
            }
            _ => return Err(format!("Unsupported method for corrections: {method}")),
        }
    }

    Ok(Some(params))
}

/// Convert correction results into final recovery result
async fn build_recovery_success(
    correction_results: Vec<CorrectionResult>,
    method: &str,
    original_params: Option<&Value>,
    original_error: &BrpResult,
    port: u16,
) -> FormatRecoveryResult {
    let mut corrections = Vec::new();
    let mut has_applied_corrections = false;

    for correction_result in correction_results {
        match correction_result {
            CorrectionResult::Corrected { correction_info } => {
                let type_name = correction_info.type_name.clone();
                corrections.push(correction_info);
                has_applied_corrections = true;
                debug!("Recovery Engine: Applied correction for type '{type_name}'");
            }
            CorrectionResult::CannotCorrect { type_info, reason } => {
                debug!(
                    "Recovery Engine: Found metadata for type '{}' but no correction: {}",
                    type_info.type_name, reason
                );
                // Create a CorrectionInfo from metadata-only result to provide guidance
                let correction_info = CorrectionInfo {
                    type_name:         type_info.type_name.clone(),
                    original_value:    original_params
                        .and_then(|p| extract_component_value(method, p, &type_info.type_name))
                        .unwrap_or_else(|| serde_json::json!({})),
                    corrected_value:   build_corrected_value_from_type_info(&type_info, method),
                    hint:              reason,
                    target_type:       type_info.type_name.clone(),
                    corrected_format:  None,
                    type_info:         Some(type_info),
                    correction_method: CorrectionMethod::DirectReplacement,
                };
                corrections.push(correction_info);
            }
        }
    }

    if corrections.is_empty() {
        debug!("Recovery Engine: No corrections found, returning original error");
        return FormatRecoveryResult::NotRecoverable {
            original_error: original_error.clone(),
            corrections:    Vec::new(),
        };
    }

    // Check if we can actually apply the corrections (i.e., we have fixable corrections)
    if has_applied_corrections && can_retry_with_corrections(method, &corrections, original_params)
    {
        debug!("Recovery Engine: Attempting to retry operation with corrected parameters");

        // Build corrected parameters
        match build_corrected_params(method, original_params, &corrections) {
            Ok(corrected_params) => {
                debug!("Recovery Engine: Built corrected parameters, executing retry");

                // Execute the retry asynchronously
                let retry_result =
                    brp_client::execute_brp_method(method, corrected_params, port).await;

                match retry_result {
                    Ok(success_result) => {
                        debug!("Recovery Engine: Retry succeeded with corrected parameters");
                        FormatRecoveryResult::Recovered {
                            corrected_result: success_result,
                            corrections,
                        }
                    }
                    Err(retry_error) => {
                        debug!("Recovery Engine: Retry failed: {}", retry_error);
                        // Convert error to BrpResult::Error
                        let retry_brp_error = BrpResult::Error(BrpError {
                            code:    -1, // Generic error code
                            message: retry_error.to_string(),
                            data:    None,
                        });
                        // Return correction failed with both original and retry errors
                        FormatRecoveryResult::CorrectionFailed {
                            original_error: original_error.clone(),
                            retry_error: retry_brp_error,
                            corrections,
                        }
                    }
                }
            }
            Err(e) => {
                debug!(
                    "Recovery Engine: Could not build corrected parameters: {}",
                    e
                );
                // Return original error - we can't fix this
                FormatRecoveryResult::NotRecoverable {
                    original_error: original_error.clone(),
                    corrections:    Vec::new(),
                }
            }
        }
    } else {
        debug!("Recovery Engine: No fixable corrections, returning error with guidance");
        // We have corrections but they're not fixable (like the enum case)
        // Return the original error - the handler will add format_corrections to it
        FormatRecoveryResult::NotRecoverable {
            original_error: original_error.clone(),
            corrections,
        }
    }
}
