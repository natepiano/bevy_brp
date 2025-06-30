//! Tier execution phase for the format discovery engine
//! This module handles the tiered approach to format discovery

use serde_json::{Map, Value};

use super::context::DiscoveryContext;
use crate::brp_tools::request_handler::format_discovery::constants::{
    TIER_DETERMINISTIC, TIER_DIRECT_DISCOVERY, TIER_GENERIC_FALLBACK, TIER_SERIALIZATION,
};
use crate::brp_tools::request_handler::format_discovery::detection::{
    TierInfo, TierManager, analyze_error_pattern, check_type_serialization,
    detect_format_quality_issues, is_high_quality_format,
};
use crate::brp_tools::request_handler::format_discovery::engine::{
    FormatCorrection, ParameterLocation,
};
use crate::brp_tools::request_handler::format_discovery::transformers::TransformerRegistry;
use crate::brp_tools::request_handler::format_discovery::utilities::{
    extract_type_items, get_parameter_location,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult, execute_brp_method};
use crate::error::{Error, Result};
use crate::tools::{BRP_METHOD_EXTRAS_DISCOVER_FORMAT, BRP_METHOD_INSERT, BRP_METHOD_SPAWN};

/// Data needed for building discovery result
pub struct DiscoveryResultData {
    pub format_corrections: Vec<FormatCorrection>,
    pub corrected_items:    Vec<(String, Value)>,
    pub all_tier_info:      Vec<TierInfo>,
}

/// Runs the discovery tiers to find correct format
/// This is the main entry point for attempting format discovery after an error
pub async fn run_discovery_tiers(context: &mut DiscoveryContext) -> Result<DiscoveryResultData> {
    let error = context
        .initial_error
        .as_ref()
        .ok_or_else(|| {
            error_stack::report!(Error::InvalidState(
                "No initial error for format discovery".to_string()
            ))
        })?
        .clone();

    // Phase 1: Extraction
    let (_location, type_items) = extract_discovery_context(context)?;

    // Phase 2: Processing
    let (format_corrections, corrected_items, all_tier_info) = process_type_items_for_corrections(
        &type_items,
        &context.method,
        context.port,
        &error,
        &mut context.debug_info,
    )
    .await?;

    Ok(DiscoveryResultData {
        format_corrections,
        corrected_items,
        all_tier_info,
    })
}

/// Extract parameter location and type items from params
fn extract_discovery_context(
    context: &mut DiscoveryContext,
) -> Result<(ParameterLocation, Vec<(String, Value)>)> {
    context.add_debug(format!(
        "Format Discovery: Attempting discovery for method '{}'",
        context.method
    ));

    // Get parameter location based on method
    let location = get_parameter_location(&context.method);
    context.add_debug(format!(
        "Format Discovery: Parameter location: {location:?}"
    ));

    // Extract type items based on location
    let params = context.original_params.as_ref().ok_or_else(|| {
        error_stack::report!(Error::InvalidState(
            "No params for format discovery".to_string()
        ))
    })?;

    let type_items = extract_type_items(params, location);

    if type_items.is_empty() {
        context.add_debug("Format Discovery: No type items found in params".to_string());
        return Err(error_stack::report!(Error::InvalidState(
            "No type items found for format discovery".to_string(),
        )));
    }

    context.add_debug(format!(
        "Format Discovery: Found {} type items to check",
        type_items.len()
    ));

    Ok((location, type_items))
}

/// Process type items and generate corrections
async fn process_type_items_for_corrections(
    type_items: &[(String, Value)],
    method: &str,
    port: Option<u16>,
    original_error: &BrpError,
    debug_info: &mut Vec<String>,
) -> Result<(Vec<FormatCorrection>, Vec<(String, Value)>, Vec<TierInfo>)> {
    let mut format_corrections = Vec::new();
    let mut corrected_items = Vec::new();
    let mut all_tier_info = Vec::new();

    // Process each type item
    for (type_name, type_value) in type_items {
        let (discovery_result, tier_info) = process_single_type_item(
            type_name,
            type_value,
            method,
            port,
            original_error,
            debug_info,
        )
        .await?;

        all_tier_info.extend(tier_info);

        match discovery_result {
            Some((final_format, hint)) => {
                format_corrections.push(FormatCorrection {
                    component: type_name.clone(),
                    original_format: type_value.clone(),
                    corrected_format: final_format.clone(),
                    hint,
                });
                corrected_items.push((type_name.clone(), final_format));
            }
            None => {
                // Keep original format if no alternative found
                corrected_items.push((type_name.clone(), type_value.clone()));
            }
        }
    }

    Ok((format_corrections, corrected_items, all_tier_info))
}

/// Process a single type item (component or resource) for format discovery
async fn process_single_type_item(
    type_name: &str,
    type_value: &Value,
    method: &str,
    port: Option<u16>,
    original_error: &BrpError,
    debug_info: &mut Vec<String>,
) -> Result<(Option<(Value, String)>, Vec<TierInfo>)> {
    debug_info.push(format!(
        "Format Discovery: Checking type '{type_name}' with value: {type_value:?}"
    ));

    let (discovery_result, mut tier_info) =
        tiered_type_format_discovery(type_name, type_value, method, original_error, port).await;

    // Add type context to tier info
    for info in &mut tier_info {
        info.action = format!("[{}] {}", type_name, info.action);
    }

    if let Some((corrected_value, hint)) = discovery_result {
        debug_info.push(format!(
            "Format Discovery: Found alternative for '{type_name}': {corrected_value:?}"
        ));

        // For spawn, validate the format by testing; for insert, just trust it
        let final_format = if method == BRP_METHOD_SPAWN {
            match test_component_format_with_spawn(type_name, &corrected_value, port).await {
                Ok(validated_format) => validated_format,
                Err(_) => return Ok((None, tier_info)), // Skip this type if validation fails
            }
        } else {
            corrected_value
        };

        Ok((Some((final_format, hint)), tier_info))
    } else {
        debug_info.push(format!(
            "Format Discovery: No alternative found for '{type_name}'"
        ));
        Ok((None, tier_info))
    }
}

/// Tiered format discovery dispatcher
/// Uses intelligent pattern matching with fallback to generic approaches
async fn tiered_type_format_discovery(
    type_name: &str,
    original_value: &Value,
    method: &str,
    error: &BrpError,
    port: Option<u16>,
) -> (Option<(Value, String)>, Vec<TierInfo>) {
    let mut tier_manager = TierManager::new();

    // ========== TIER 1: Serialization Diagnostics ==========
    let error_analysis = analyze_error_pattern(error);
    if method == BRP_METHOD_INSERT || method == BRP_METHOD_SPAWN {
        tier_manager.start_tier(
            TIER_SERIALIZATION,
            "Serialization Diagnostics",
            format!("Checking serialization support for type: {type_name}"),
        );

        match check_type_serialization(type_name, port).await {
            Ok(serialization_check) => {
                tier_manager.complete_tier(true, serialization_check.diagnostic_message.clone());

                if serialization_check
                    .diagnostic_message
                    .contains("cannot be used with BRP")
                {
                    return (
                        Some((
                            original_value.clone(),
                            serialization_check.diagnostic_message,
                        )),
                        tier_manager.into_vec(),
                    );
                }

                // Don't return early - continue to Tier 2 for format discovery
                // Even if type has serialization support, we may need format correction
            }
            Err(e) => {
                tier_manager.complete_tier(
                    false,
                    Error::FormatDiscovery(format!(
                        "failed to query serialization info for {type_name}: {e}"
                    ))
                    .to_string(),
                );
            }
        }
    }

    // ========== TIER 2: Direct Discovery ==========
    if let Some(result) =
        try_direct_discovery(type_name, original_value, port, &mut tier_manager).await
    {
        return (Some(result), tier_manager.into_vec());
    }

    // ========== TIERS 3 & 4: Smart Format Discovery ==========
    tier_manager.start_tier(
        TIER_DETERMINISTIC, // Still report as Tier 3 for compatibility
        "Smart Format Discovery",
        "Applying pattern matching and transformation logic".to_string(),
    );

    let smart_result =
        apply_transformer_based_discovery(original_value, error, error_analysis.pattern.as_ref());

    if let Some((corrected_value, hint)) = smart_result {
        // Determine which tier actually succeeded based on the hint
        if hint.contains("pattern") || hint.contains("AccessError") || hint.contains("MissingField")
        {
            tier_manager.complete_tier(true, format!("Applied pattern fix: {hint}"));
        } else {
            // This was a generic transformation
            tier_manager.complete_tier(false, "Pattern matching failed".to_string());
            tier_manager.start_tier(
                TIER_GENERIC_FALLBACK,
                "Generic Fallback",
                "Trying generic format alternatives".to_string(),
            );
            tier_manager.complete_tier(true, format!("Found generic alternative: {hint}"));
        }
        return (Some((corrected_value, hint)), tier_manager.into_vec());
    }

    tier_manager.complete_tier(false, "No format discovery succeeded".to_string());
    (None, tier_manager.into_vec())
}

/// Try direct discovery using `bevy_brp_extras/discover_format`
async fn try_direct_discovery(
    type_name: &str,
    original_value: &Value,
    port: Option<u16>,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    tier_manager.start_tier(
        TIER_DIRECT_DISCOVERY,
        "Direct Discovery",
        format!("Querying bevy_brp_extras for {type_name}"),
    );

    let discovery_response = execute_discovery_request(type_name, port).await;

    match discovery_response {
        Some(data) => process_discovery_response(&data, type_name, original_value, tier_manager),
        None => {
            tier_manager.complete_tier(false, "Direct discovery unavailable or failed".to_string());
            None
        }
    }
}

/// Execute the discovery request to bevy_brp_extras
async fn execute_discovery_request(type_name: &str, port: Option<u16>) -> Option<Value> {
    let discover_params = serde_json::json!({
        "types": [type_name]
    });

    let result = execute_brp_method(
        BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
        Some(discover_params),
        port,
    )
    .await
    .ok();

    match result {
        Some(BrpResult::Success(Some(data))) => Some(data),
        _ => None,
    }
}

/// Process the discovery response and extract format information
fn process_discovery_response(
    data: &Value,
    type_name: &str,
    original_value: &Value,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    // Try new TypeDiscoveryResponse format first
    if let Some(result) = process_new_format(data, type_name, original_value, tier_manager) {
        return Some(result);
    }

    // Fall back to legacy format
    process_legacy_format(data, type_name, tier_manager)
}

/// Process new TypeDiscoveryResponse format
fn process_new_format(
    data: &Value,
    type_name: &str,
    original_value: &Value,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    let type_info = data.get("type_info")?.as_object()?;
    let type_response = type_info.get(type_name)?;

    // Try to extract spawn example
    if let Some(result) = extract_spawn_example(type_response, type_name, tier_manager) {
        return Some(result);
    }

    // Handle type in registry but no spawn example
    handle_registry_type_without_spawn(type_response, type_name, original_value, tier_manager)
}

/// Extract and validate spawn example from type response
fn extract_spawn_example(
    type_response: &Value,
    type_name: &str,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    let example_values = type_response.get("example_values")?.as_object()?;
    let spawn_example = example_values.get("spawn")?;

    if is_high_quality_format(spawn_example) {
        tier_manager.complete_tier(
            true,
            format!("Direct discovery successful: found high-quality format for {type_name}"),
        );

        let hint = build_discovery_hint(type_response);
        Some((spawn_example.clone(), hint))
    } else {
        handle_low_quality_format(spawn_example, tier_manager);
        None
    }
}

/// Build hint message from type response information
fn build_discovery_hint(type_response: &Value) -> String {
    let mut hint_parts = vec!["Direct discovery from bevy_brp_extras".to_string()];

    if let Some(operations) = type_response
        .get("supported_operations")
        .and_then(|so| so.as_array())
    {
        let ops: Vec<String> = operations
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();
        if !ops.is_empty() {
            hint_parts.push(format!("Supports: {}", ops.join(", ")));
        }
    }

    hint_parts.join(". ")
}

/// Handle low quality format detection
fn handle_low_quality_format(spawn_example: &Value, tier_manager: &mut TierManager) {
    if let Some(quality_issue) = detect_format_quality_issues(spawn_example) {
        tier_manager.complete_tier(
            false,
            format!("Direct discovery returned low-quality format: {quality_issue}"),
        );
    } else {
        tier_manager.complete_tier(
            false,
            "Direct discovery returned low-quality format".to_string(),
        );
    }
}

/// Handle type in registry but without spawn example
fn handle_registry_type_without_spawn(
    type_response: &Value,
    type_name: &str,
    original_value: &Value,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    if !type_response
        .get("in_registry")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return None;
    }

    let has_serialize = type_response
        .get("has_serialize")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_deserialize = type_response
        .get("has_deserialize")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let reason = if !has_serialize || !has_deserialize {
        "Type lacks Serialize/Deserialize traits required for BRP operations"
    } else {
        "Type format could not be determined"
    };

    tier_manager.complete_tier(true, format!("Type found but cannot be used: {reason}"));

    Some((
        original_value.clone(),
        format!("Type '{type_name}' is registered but {reason}"),
    ))
}

/// Process legacy format for backward compatibility
fn process_legacy_format(
    data: &Value,
    type_name: &str,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    let formats = data.get("formats")?.as_object()?;
    let format_info = formats.get(type_name)?;
    let spawn_format = format_info.get("spawn_format")?.get("example")?;

    tier_manager.complete_tier(
        true,
        format!("Direct discovery successful: found format for {type_name} (legacy format)"),
    );

    let hint = "Direct discovery from bevy_brp_extras".to_string();
    Some((spawn_format.clone(), hint))
}

/// Test a component format by spawning a test entity
async fn test_component_format_with_spawn(
    type_name: &str,
    component_value: &Value,
    port: Option<u16>,
) -> Result<Value> {
    let mut test_components = Map::new();
    test_components.insert(type_name.to_string(), component_value.clone());

    let test_params = serde_json::json!({
        "components": test_components
    });

    match execute_brp_method(BRP_METHOD_SPAWN, Some(test_params), port).await? {
        BrpResult::Success(Some(data)) => {
            // Immediately clean up the test entity
            if let Some(entity) = data.get("entity").and_then(serde_json::Value::as_u64) {
                let destroy_params = serde_json::json!({
                    "entity": entity
                });
                _ = execute_brp_method(
                    crate::tools::BRP_METHOD_DESTROY,
                    Some(destroy_params),
                    port,
                )
                .await;
            }
            Ok(component_value.clone())
        }
        BrpResult::Success(None) | BrpResult::Error(_) => Err(error_stack::report!(
            Error::FormatDiscovery("Test spawn failed with corrected format".to_string(),)
        )),
    }
}

/// New transformer-based format discovery that replaces the old transformations.rs logic
/// Uses the clean trait-based transformer system for maintainable format fixes
fn apply_transformer_based_discovery(
    original_value: &Value,
    error: &BrpError,
    error_pattern: Option<&super::super::detection::ErrorPattern>,
) -> Option<(Value, String)> {
    // First try deterministic pattern matching using new transformer system (Tier 3)
    if let Some(pattern) = error_pattern {
        let registry = TransformerRegistry::with_defaults();
        if let Some(result) = registry.transform(original_value, pattern, error) {
            return Some(result);
        }
    }

    // If pattern matching didn't work, no transformation found
    None
}
