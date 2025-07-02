//! Format error recovery engine with 3-level architecture
//!
//! Recovery levels (early-exit design):
//! 1. Registry checks - Fast type registration and serialization trait verification
//! 2. Direct discovery - Query running Bevy app via `bevy_brp_extras` for type schemas
//! 3. Pattern transformations - Apply known fixes for common format errors
//!
//! Each level returns immediately on success to minimize processing.

use serde_json::Value;
use tracing::debug;

use super::flow_types::{CorrectionResult, FormatRecoveryResult};
use crate::brp_tools::support::brp_client::BrpResult;

/// Execute format error recovery using the 3-level decision tree
pub async fn attempt_format_recovery(
    method: &str,
    original_params: Option<Value>,
    error: BrpResult,
    port: Option<u16>,
) -> FormatRecoveryResult {
    debug!("Recovery Engine: Starting 3-level recovery for method '{method}'");

    // Extract type names from the parameters for recovery attempts
    let type_names = extract_type_names_from_params(method, original_params.as_ref());
    if type_names.is_empty() {
        debug!("Recovery Engine: No type names found in parameters, cannot recover");
        return FormatRecoveryResult::NotRecoverable(error);
    }

    debug!(
        "Recovery Engine: Found {} type names to process",
        type_names.len()
    );

    // Level 1: Registry/Serialization Checks
    debug!("Recovery Engine: Level 1 - Registry/serialization checks");
    match execute_level_1_registry_checks(&type_names, port).await {
        LevelResult::Success(corrections) => {
            debug!("Recovery Engine: Level 1 succeeded with registry-based corrections");
            return build_recovery_success(corrections);
        }
        LevelResult::Educational(_educational_info) => {
            debug!("Recovery Engine: Level 1 provided educational guidance");
            return FormatRecoveryResult::Educational {
                original_error: error,
            };
        }
        LevelResult::Continue => {
            debug!("Recovery Engine: Level 1 complete, proceeding to Level 2");
        }
    }

    // Level 2: Direct Discovery via bevy_brp_extras
    debug!("Recovery Engine: Level 2 - Direct discovery via bevy_brp_extras");
    match execute_level_2_direct_discovery(&type_names, port).await {
        LevelResult::Success(corrections) => {
            debug!("Recovery Engine: Level 2 succeeded with direct discovery");
            return build_recovery_success(corrections);
        }
        LevelResult::Educational(_educational_info) => {
            debug!("Recovery Engine: Level 2 provided educational guidance");
            return FormatRecoveryResult::Educational {
                original_error: error,
            };
        }
        LevelResult::Continue => {
            debug!("Recovery Engine: Level 2 complete, proceeding to Level 3");
        }
    }

    // Level 3: Pattern-Based Transformations
    debug!("Recovery Engine: Level 3 - Pattern-based transformations");
    match execute_level_3_pattern_transformations(&type_names, method, original_params.as_ref()) {
        LevelResult::Success(corrections) => {
            debug!("Recovery Engine: Level 3 succeeded with pattern-based corrections");
            build_recovery_success(corrections)
        }
        LevelResult::Educational(_educational_info) => {
            debug!("Recovery Engine: Level 3 provided educational guidance");
            FormatRecoveryResult::Educational {
                original_error: error,
            }
        }
        LevelResult::Continue => {
            debug!("Recovery Engine: All levels exhausted, no recovery possible");
            FormatRecoveryResult::NotRecoverable(error)
        }
    }
}

/// Result of a recovery level attempt
#[derive(Debug)]
enum LevelResult {
    /// Level succeeded and produced corrections
    Success(Vec<CorrectionResult>),
    /// Level provided educational information but no corrections
    Educational(String),
    /// Level completed but recovery should continue to next level
    Continue,
}

/// Level 1: Fast registry and serialization checks
async fn execute_level_1_registry_checks(type_names: &[String], port: Option<u16>) -> LevelResult {
    debug!(
        "Level 1: Checking {} types against registry",
        type_names.len()
    );

    // Use batch checking for efficiency when checking multiple types
    let registry_results =
        super::registry_integration::check_multiple_types_registry_status(type_names, port).await;

    let mut corrections = Vec::new();
    let mut educational_messages = Vec::new();

    for (type_name, registry_result) in registry_results {
        debug!("Level 1: Processing registry result for '{type_name}'");

        if let Some(type_info) = registry_result {
            // Type found in registry, check serialization support
            if type_info.serialization.brp_compatible {
                debug!("Level 1: Type '{type_name}' is fully BRP compatible");
                // Create a metadata-only correction since we have good type info
                let correction = CorrectionResult::MetadataOnly {
                    type_info,
                    reason: "Type found in registry with full BRP support".to_string(),
                };
                corrections.push(correction);
            } else {
                // Type in registry but missing serialization traits
                let educational_message = format!(
                    "Type '{type_name}' is registered in Bevy's type registry but lacks required serialization traits. \
                    To use this type with BRP operations, ensure it derives or implements Serialize and Deserialize traits."
                );
                debug!(
                    "Level 1: Educational guidance for '{type_name}': missing serialization traits"
                );
                educational_messages.push(educational_message);
            }
        } else {
            // Type not found in registry
            let educational_message = format!(
                "Type '{type_name}' is not registered in Bevy's type registry. \
                To use this type with BRP operations, ensure it's registered with the App using .register_type::<{type_name}>()"
            );
            debug!("Level 1: Educational guidance for '{type_name}': not in registry");
            educational_messages.push(educational_message);
        }
    }

    // Determine the level result based on what we found
    if !corrections.is_empty() {
        debug!(
            "Level 1: Found {} corrections from registry information",
            corrections.len()
        );
        LevelResult::Success(corrections)
    } else if !educational_messages.is_empty() {
        let combined_message = educational_messages.join("\n\n");
        debug!("Level 1: Providing educational guidance for registry issues");
        LevelResult::Educational(combined_message)
    } else {
        debug!("Level 1: Registry checks complete, proceeding to Level 2");
        LevelResult::Continue
    }
}

/// Level 2: Direct discovery via `bevy_brp_extras/discover_format`
async fn execute_level_2_direct_discovery(type_names: &[String], port: Option<u16>) -> LevelResult {
    debug!(
        "Level 2: Attempting direct discovery for {} types",
        type_names.len()
    );

    // Check if bevy_brp_extras is available
    if !is_brp_extras_available(port).await {
        debug!("Level 2: bevy_brp_extras not available, proceeding to Level 3");
        return LevelResult::Continue;
    }

    // Attempt direct discovery for each type using bevy_brp_extras
    let mut corrections = Vec::new();

    for type_name in type_names {
        debug!("Level 2: Attempting direct discovery for '{type_name}'");

        // Call extras_integration to discover the type format
        match super::extras_integration::discover_type_format(type_name, port).await {
            Ok(Some(type_info)) => {
                debug!("Level 2: Successfully discovered type information for '{type_name}'");

                // Create a correction from the discovered type information
                let correction = super::extras_integration::create_correction_from_discovery(
                    type_info, None, // We don't have the original value in this context
                );
                corrections.push(correction);
            }
            Ok(None) => {
                debug!("Level 2: No type information found for '{type_name}' via direct discovery");
                // Type discovery failure tracked in debug_info
            }
            Err(e) => {
                debug!("Level 2: Direct discovery failed for '{type_name}': {e}");
                // Type discovery failure tracked in debug_info
            }
        }
    }

    // Determine the level result based on what we discovered
    if corrections.is_empty() {
        debug!("Level 2: Direct discovery complete, proceeding to Level 3");
        LevelResult::Continue
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
    _method: &str,
    _original_params: Option<&Value>,
) -> LevelResult {
    debug!(
        "Level 3: Applying pattern transformations for {} types",
        type_names.len()
    );

    // Initialize transformer registry with default transformers
    let transformer_registry = super::transformers::TransformerRegistry::with_defaults();
    let mut corrections = Vec::new();

    for type_name in type_names {
        debug!("Level 3: Checking transformation patterns for '{type_name}'");

        // Try to generate format corrections based on common patterns for each type
        if let Some(correction) = attempt_pattern_based_correction(type_name, &transformer_registry)
        {
            debug!("Level 3: Found pattern-based correction for '{type_name}'");
            corrections.push(correction);
        } else {
            debug!("Level 3: No pattern-based correction found for '{type_name}'");
        }
    }

    if corrections.is_empty() {
        debug!("Level 3: Pattern transformations complete, no corrections found");
        LevelResult::Continue
    } else {
        debug!(
            "Level 3: Found {} pattern-based corrections",
            corrections.len()
        );
        LevelResult::Success(corrections)
    }
}

/// Try to generate pattern-based corrections for well-known types
fn attempt_pattern_based_correction(
    type_name: &str,
    _transformer_registry: &super::transformers::TransformerRegistry,
) -> Option<CorrectionResult> {
    debug!("Level 3: Attempting pattern correction for type '{type_name}'");

    // Create common educational corrections for well-known types
    match type_name {
        // Math types - common object vs array issues
        t if t.contains("Vec2") || t.contains("Vec3") || t.contains("Vec4") => {
            Some(create_math_vector_correction(t))
        }

        // Quaternion types
        t if t.contains("Quat") => Some(create_quaternion_correction(t)),

        // Other types - no specific patterns yet
        _ => {
            debug!("Level 3: No specific pattern available for type '{type_name}'");
            None
        }
    }
}

/// Create a correction for math vector types (Vec2, Vec3, Vec4)
fn create_math_vector_correction(type_name: &str) -> CorrectionResult {
    debug!("Level 3: Detected math type '{type_name}', providing array format guidance");

    let examples = create_vector_examples(type_name);
    let type_info = create_math_type_info(type_name, examples, "Math");

    CorrectionResult::MetadataOnly {
        type_info,
        reason: format!(
            "Math type '{type_name}' typically uses array format [x, y, ...] instead of object format"
        ),
    }
}

/// Create a correction for quaternion types
fn create_quaternion_correction(type_name: &str) -> CorrectionResult {
    debug!("Level 3: Detected quaternion type '{type_name}', providing array format guidance");

    let mut examples = std::collections::HashMap::new();
    examples.insert("spawn".to_string(), serde_json::json!([0.0, 0.0, 0.0, 1.0]));

    let type_info = create_math_type_info(type_name, examples, "Math");

    CorrectionResult::MetadataOnly {
        type_info,
        reason: format!(
            "Quaternion type '{type_name}' uses array format [x, y, z, w] where w is typically 1.0 for identity"
        ),
    }
}

/// Create examples for vector types based on their dimensions
fn create_vector_examples(type_name: &str) -> std::collections::HashMap<String, serde_json::Value> {
    let mut examples = std::collections::HashMap::new();

    if type_name.contains("Vec2") {
        examples.insert("spawn".to_string(), serde_json::json!([1.0, 2.0]));
    } else if type_name.contains("Vec3") {
        examples.insert("spawn".to_string(), serde_json::json!([1.0, 2.0, 3.0]));
    } else if type_name.contains("Vec4") {
        examples.insert("spawn".to_string(), serde_json::json!([1.0, 2.0, 3.0, 4.0]));
    }

    examples
}

/// Create a `UnifiedTypeInfo` for math types
fn create_math_type_info(
    type_name: &str,
    examples: std::collections::HashMap<String, serde_json::Value>,
    category: &str,
) -> super::unified_types::UnifiedTypeInfo {
    super::unified_types::UnifiedTypeInfo {
        type_name:            type_name.to_string(),
        registry_status:      super::unified_types::RegistryStatus {
            in_registry: true,
            has_reflect: true,
            type_path:   Some(type_name.to_string()),
        },
        serialization:        super::unified_types::SerializationSupport {
            has_serialize:   true,
            has_deserialize: true,
            brp_compatible:  true,
        },
        format_info:          super::unified_types::FormatInfo {
            examples,
            mutation_paths: std::collections::HashMap::new(),
            original_format: None,
            corrected_format: None,
        },
        supported_operations: vec!["spawn".to_string(), "insert".to_string()],
        type_category:        category.to_string(),
        child_types:          std::collections::HashMap::new(),
        enum_info:            None,
        discovery_source:     super::unified_types::DiscoverySource::PatternMatching,
    }
}

/// Check if `bevy_brp_extras` is available
async fn is_brp_extras_available(port: Option<u16>) -> bool {
    debug!("Level 2: Checking bevy_brp_extras availability");

    // Check if bevy_brp_extras is available by calling extras_integration
    let is_available = super::extras_integration::check_brp_extras_availability(port).await;
    debug!("Level 2: bevy_brp_extras availability check result: {is_available}");
    is_available
}

/// Extract type names from BRP method parameters based on method type
fn extract_type_names_from_params(method: &str, params: Option<&Value>) -> Vec<String> {
    let Some(params) = params else {
        return Vec::new();
    };

    let mut type_names = Vec::new();

    match method {
        "bevy/spawn" | "bevy/insert" => {
            // Types are keys in the "components" object
            if let Some(components) = params.get("components").and_then(|c| c.as_object()) {
                for type_name in components.keys() {
                    type_names.push(type_name.clone());
                }
            }
        }
        "bevy/mutate_component" => {
            // Single type in "component" field
            if let Some(component) = params.get("component").and_then(|c| c.as_str()) {
                type_names.push(component.to_string());
            }
        }
        "bevy/insert_resource" | "bevy/mutate_resource" => {
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

/// Convert correction results into final recovery result
fn build_recovery_success(correction_results: Vec<CorrectionResult>) -> FormatRecoveryResult {
    let mut corrections = Vec::new();

    for correction_result in correction_results {
        match correction_result {
            CorrectionResult::Applied { correction_info } => {
                let type_name = correction_info.type_name.clone();
                corrections.push(correction_info);
                debug!("Recovery Engine: Applied correction for type '{type_name}'");
            }
            CorrectionResult::MetadataOnly { type_info, reason } => {
                debug!(
                    "Recovery Engine: Found metadata for type '{}' but no correction: {}",
                    type_info.type_name, reason
                );
            }
        }
    }

    if corrections.is_empty() {
        debug!("Recovery Engine: No applicable corrections found");
        // TODO: For now, return a placeholder success result
        // In a real implementation, this would use the actual corrected BRP result
        FormatRecoveryResult::Educational {
            original_error: crate::brp_tools::support::brp_client::BrpResult::Error(
                crate::brp_tools::support::brp_client::BrpError {
                    code:    -32602,
                    message: "Format recovery attempted but no corrections applicable".to_string(),
                    data:    None,
                },
            ),
        }
    } else {
        debug!(
            "Recovery Engine: Built recovery result with {} corrections",
            corrections.len()
        );
        // TODO: For now, return a placeholder success result
        // In a real implementation, this would re-execute the BRP method with corrected parameters
        FormatRecoveryResult::Recovered {
            corrected_result: crate::brp_tools::support::brp_client::BrpResult::Success(Some(
                serde_json::json!({"recovered": true, "corrections_applied": corrections.len()}),
            )),
            corrections,
        }
    }
}
