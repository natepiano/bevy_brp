//! Tier execution phase for the format discovery engine
//! This module handles the tiered approach to format discovery

use serde_json::Value;

use super::context::DiscoveryContext;
use crate::brp_tools::request_handler::format_discovery::constants::{
    TIER_DETERMINISTIC, TIER_DIRECT_DISCOVERY, TIER_GENERIC_FALLBACK, TIER_SERIALIZATION,
};
use crate::brp_tools::request_handler::format_discovery::detection::{
    TierInfo, TierManager, analyze_error_pattern, check_type_serialization,
};
use crate::brp_tools::request_handler::format_discovery::engine::{
    FormatCorrection, ParameterLocation,
};
use crate::brp_tools::request_handler::format_discovery::support::{
    extract_type_items, get_parameter_location,
};
use crate::brp_tools::request_handler::format_discovery::transformers::TransformerRegistry;
use crate::brp_tools::support::brp_client::{BrpError, BrpResult, execute_brp_method};
use crate::error::{Error, Result};
use crate::tools::{BRP_METHOD_EXTRAS_DISCOVER_FORMAT, BRP_METHOD_INSERT, BRP_METHOD_SPAWN};

/// Facts discovered about a type from Tier 2 (BRP discovery)
#[derive(Debug, Clone)]
pub struct DiscoveredFacts {
    pub spawn_example:        Option<Value>,
    pub supported_operations: Option<Vec<String>>,
    pub mutation_paths:       Option<Vec<String>>,
    pub type_category:        Option<String>,
    pub in_registry:          Option<bool>,
    pub has_serialize:        Option<bool>,
    pub has_deserialize:      Option<bool>,
    pub legacy_format:        Option<Value>,
}

impl DiscoveredFacts {
    /// Create a new empty facts instance for a type
    pub const fn new() -> Self {
        Self {
            spawn_example:        None,
            supported_operations: None,
            mutation_paths:       None,
            type_category:        None,
            in_registry:          None,
            has_serialize:        None,
            has_deserialize:      None,
            legacy_format:        None,
        }
    }
}

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
        let (discovery_result, tier_info, facts) = process_single_type_item(
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
                    supported_operations: facts.supported_operations.clone(),
                    mutation_paths: facts.mutation_paths.clone(),
                    type_category: facts.type_category.clone(),
                });
                corrected_items.push((type_name.clone(), final_format));
            }
            None => {
                // Keep original format if no alternative found
                corrected_items.push((type_name.clone(), type_value.clone()));
            }
        }

        // Facts are now embedded in FormatCorrection, no need to store separately
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
) -> Result<(Option<(Value, String)>, Vec<TierInfo>, DiscoveredFacts)> {
    log_type_processing_start(type_name, type_value, debug_info);

    let (discovery_result, tier_info, facts) =
        perform_type_discovery(type_name, type_value, method, original_error, port).await;

    let enriched_tier_info = enrich_tier_info_with_type_context(tier_info, type_name);

    let result = build_discovery_result(discovery_result, type_name, debug_info);

    Ok((result, enriched_tier_info, facts))
}

/// Log the start of type processing for debugging
fn log_type_processing_start(type_name: &str, type_value: &Value, debug_info: &mut Vec<String>) {
    debug_info.push(format!(
        "Format Discovery: Checking type '{type_name}' with value: {type_value:?}"
    ));
}

/// Perform the actual type discovery using the tiered approach
async fn perform_type_discovery(
    type_name: &str,
    type_value: &Value,
    method: &str,
    original_error: &BrpError,
    port: Option<u16>,
) -> (Option<(Value, String)>, Vec<TierInfo>, DiscoveredFacts) {
    tiered_type_format_discovery(type_name, type_value, method, original_error, port).await
}

/// Add type context to tier info for better debugging
fn enrich_tier_info_with_type_context(
    mut tier_info: Vec<TierInfo>,
    type_name: &str,
) -> Vec<TierInfo> {
    for info in &mut tier_info {
        info.action = format!("[{type_name}] {}", info.action);
    }
    tier_info
}

/// Build the final discovery result and log the outcome
fn build_discovery_result(
    discovery_result: Option<(Value, String)>,
    type_name: &str,
    debug_info: &mut Vec<String>,
) -> Option<(Value, String)> {
    if let Some((corrected_value, hint)) = discovery_result {
        debug_info.push(format!(
            "Format Discovery: Found alternative for '{type_name}': {corrected_value:?}"
        ));
        Some((corrected_value, hint))
    } else {
        debug_info.push(format!(
            "Format Discovery: No alternative found for '{type_name}'"
        ));
        None
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
) -> (Option<(Value, String)>, Vec<TierInfo>, DiscoveredFacts) {
    let mut tier_manager = TierManager::new();
    let mut facts = DiscoveredFacts::new();
    let error_analysis = analyze_error_pattern(error);

    // ========== TIER 1: Serialization Diagnostics ==========
    if let Some((value, message)) = execute_tier1_serialization_check(
        type_name,
        original_value,
        method,
        port,
        &mut tier_manager,
    )
    .await
    {
        return (Some((value, message)), tier_manager.into_vec(), facts);
    }

    // ========== TIER 2: Direct Discovery ==========
    gather_direct_discovery_facts(
        type_name,
        original_value,
        port,
        &mut tier_manager,
        &mut facts,
    )
    .await;

    // ========== TIERS 3 & 4: Smart Format Discovery ==========
    if let Some((value, hint)) = execute_smart_format_discovery(
        original_value,
        error,
        error_analysis.pattern.as_ref(),
        &facts,
        &mut tier_manager,
    ) {
        return (Some((value, hint)), tier_manager.into_vec(), facts);
    }

    tier_manager.complete_tier(false, "No format discovery succeeded".to_string());
    (None, tier_manager.into_vec(), facts)
}

/// Execute Tier 1 serialization diagnostics check
async fn execute_tier1_serialization_check(
    type_name: &str,
    original_value: &Value,
    method: &str,
    port: Option<u16>,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    if method != BRP_METHOD_INSERT && method != BRP_METHOD_SPAWN {
        return None;
    }

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
                return Some((
                    original_value.clone(),
                    serialization_check.diagnostic_message,
                ));
            }
            // Continue to Tier 2 for format discovery
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

    None
}

/// Execute Tiers 3 & 4: Smart format discovery and generic fallback
fn execute_smart_format_discovery(
    original_value: &Value,
    error: &BrpError,
    error_pattern: Option<&super::super::detection::ErrorPattern>,
    facts: &DiscoveredFacts,
    tier_manager: &mut TierManager,
) -> Option<(Value, String)> {
    tier_manager.start_tier(
        TIER_DETERMINISTIC,
        "Smart Format Discovery",
        "Applying pattern matching and transformation logic".to_string(),
    );

    let smart_result =
        apply_transformer_based_discovery(original_value, error, error_pattern, Some(facts));

    if let Some((corrected_value, hint)) = smart_result {
        let result = handle_smart_discovery_result(corrected_value, hint, tier_manager);
        Some(result)
    } else {
        tier_manager.complete_tier(false, "No format discovery succeeded".to_string());
        None
    }
}

/// Handle the result from smart format discovery
fn handle_smart_discovery_result(
    corrected_value: Value,
    hint: String,
    tier_manager: &mut TierManager,
) -> (Value, String) {
    if hint.contains("pattern") || hint.contains("AccessError") || hint.contains("MissingField") {
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

    (corrected_value, hint)
}

/// Gather facts from direct discovery and populate the facts structure
async fn gather_direct_discovery_facts(
    type_name: &str,
    original_value: &Value,
    port: Option<u16>,
    tier_manager: &mut TierManager,
    facts: &mut DiscoveredFacts,
) {
    tier_manager.start_tier(
        TIER_DIRECT_DISCOVERY,
        "Direct Discovery",
        format!("Querying bevy_brp_extras for {type_name}"),
    );

    let discovery_response = execute_discovery_request(type_name, port).await;

    if let Some(data) = discovery_response {
        // Gather facts and populate the facts structure
        populate_facts_from_discovery(&data, type_name, original_value, facts);
        tier_manager.complete_tier(true, "Direct discovery facts gathered".to_string());
    } else {
        tier_manager.complete_tier(false, "Direct discovery unavailable or failed".to_string());
    }
}

/// Execute the discovery request to `bevy_brp_extras`
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

/// Populate facts structure from discovery response data
fn populate_facts_from_discovery(
    data: &Value,
    type_name: &str,
    _original_value: &Value,
    facts: &mut DiscoveredFacts,
) {
    // Try new TypeDiscoveryResponse format first
    if let Some(type_info) = data.get("type_info").and_then(|ti| ti.as_object()) {
        if let Some(type_response) = type_info.get(type_name) {
            extract_facts_from_type_response(type_response, facts);
        }
    }

    // Also try legacy format
    extract_legacy_format(data, type_name, facts);
}

/// Extract facts from the new `TypeDiscoveryResponse` format
fn extract_facts_from_type_response(type_response: &Value, facts: &mut DiscoveredFacts) {
    extract_spawn_example(type_response, facts);
    extract_supported_operations(type_response, facts);
    extract_mutation_paths(type_response, facts);
    extract_registry_and_serialization_info(type_response, facts);
    extract_type_category(type_response, facts);
}

/// Extract spawn example from type response
fn extract_spawn_example(type_response: &Value, facts: &mut DiscoveredFacts) {
    if let Some(example_values) = type_response
        .get("example_values")
        .and_then(|ev| ev.as_object())
    {
        if let Some(spawn_example) = example_values.get("spawn") {
            facts.spawn_example = Some(spawn_example.clone());
        }
    }
}

/// Extract supported operations from type response
fn extract_supported_operations(type_response: &Value, facts: &mut DiscoveredFacts) {
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
            facts.supported_operations = Some(ops);
        }
    }
}

/// Extract mutation paths from type response
fn extract_mutation_paths(type_response: &Value, facts: &mut DiscoveredFacts) {
    if let Some(mutation_info) = type_response.get("mutation_info") {
        if let Some(paths) = mutation_info
            .get("available_paths")
            .and_then(|ap| ap.as_array())
        {
            let mutation_paths: Vec<String> = paths
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect();
            if !mutation_paths.is_empty() {
                facts.mutation_paths = Some(mutation_paths);
            }
        }
    }
}

/// Extract registry and serialization information
fn extract_registry_and_serialization_info(type_response: &Value, facts: &mut DiscoveredFacts) {
    if let Some(in_registry) = type_response
        .get("in_registry")
        .and_then(serde_json::Value::as_bool)
    {
        facts.in_registry = Some(in_registry);
    }

    if let Some(has_serialize) = type_response
        .get("has_serialize")
        .and_then(serde_json::Value::as_bool)
    {
        facts.has_serialize = Some(has_serialize);
    }

    if let Some(has_deserialize) = type_response
        .get("has_deserialize")
        .and_then(serde_json::Value::as_bool)
    {
        facts.has_deserialize = Some(has_deserialize);
    }
}

/// Extract type category from type response
fn extract_type_category(type_response: &Value, facts: &mut DiscoveredFacts) {
    if let Some(category) = type_response
        .get("type_category")
        .and_then(|tc| tc.as_str())
    {
        facts.type_category = Some(category.to_string());
    }
}

/// Extract legacy format if available and not already populated
fn extract_legacy_format(data: &Value, type_name: &str, facts: &mut DiscoveredFacts) {
    if let Some(formats) = data.get("formats").and_then(|f| f.as_object()) {
        if let Some(format_info) = formats.get(type_name) {
            if let Some(spawn_format) = format_info
                .get("spawn_format")
                .and_then(|sf| sf.get("example"))
            {
                // Only set if we don't already have a spawn example from new format
                if facts.spawn_example.is_none() {
                    facts.legacy_format = Some(spawn_format.clone());
                }
            }
        }
    }
}

/// New transformer-based format discovery that replaces the old transformations.rs logic
/// Uses the clean trait-based transformer system for maintainable format fixes
///
/// With optional discovered facts, this can create rich responses with educational content
fn apply_transformer_based_discovery(
    original_value: &Value,
    error: &BrpError,
    error_pattern: Option<&super::super::detection::ErrorPattern>,
    facts: Option<&DiscoveredFacts>,
) -> Option<(Value, String)> {
    // First try deterministic pattern matching (Tier 3)
    if let Some(result) = try_pattern_based_transformation(original_value, error, error_pattern) {
        return Some(result);
    }

    // If pattern matching didn't work but we have facts, try educational responses
    if let Some(discovered_facts) = facts {
        return create_fact_based_response(original_value, discovered_facts);
    }

    // No transformation found
    None
}

/// Try pattern-based transformation using the transformer registry
fn try_pattern_based_transformation(
    original_value: &Value,
    error: &BrpError,
    error_pattern: Option<&super::super::detection::ErrorPattern>,
) -> Option<(Value, String)> {
    error_pattern.and_then(|pattern| {
        let registry = TransformerRegistry::with_defaults();
        registry.transform(original_value, pattern, error)
    })
}

/// Create a response based on discovered facts
fn create_fact_based_response(
    original_value: &Value,
    facts: &DiscoveredFacts,
) -> Option<(Value, String)> {
    // Check for spawn examples
    if let Some(spawn_example) = &facts.spawn_example {
        return Some((
            spawn_example.clone(),
            create_rich_educational_hint(facts, "Using correct format from type discovery"),
        ));
    }

    // Check for legacy format
    if let Some(legacy_format) = &facts.legacy_format {
        return Some((
            legacy_format.clone(),
            create_rich_educational_hint(facts, "Using legacy format from type discovery"),
        ));
    }

    // Check for registry/serialization issues
    create_educational_response_for_issues(original_value, facts)
}

/// Create educational response for registry or serialization issues
fn create_educational_response_for_issues(
    original_value: &Value,
    facts: &DiscoveredFacts,
) -> Option<(Value, String)> {
    if facts.in_registry == Some(false) {
        // Type not in registry
        Some((
            original_value.clone(),
            create_rich_educational_hint(
                facts,
                "Type not found in BRP registry - this type may not be accessible via BRP",
            ),
        ))
    } else if facts.has_serialize == Some(false) || facts.has_deserialize == Some(false) {
        // Serialization issues
        Some((
            original_value.clone(),
            create_rich_educational_hint(
                facts,
                "Type serialization not supported - format cannot be corrected",
            ),
        ))
    } else {
        None
    }
}

/// Create a rich educational hint that includes relevant discovered facts
fn create_rich_educational_hint(facts: &DiscoveredFacts, base_message: &str) -> String {
    use std::fmt::Write;

    let mut hint = base_message.to_string();

    // Add supported operations if available
    if let Some(operations) = &facts.supported_operations {
        write!(hint, " | Supported operations: {}", operations.join(", ")).unwrap();
    }

    // Add mutation paths if available
    if let Some(paths) = &facts.mutation_paths {
        if !paths.is_empty() {
            write!(hint, " | Available mutation paths: {}", paths.join(", ")).unwrap();
        }
    }

    // Add type category if available
    if let Some(category) = &facts.type_category {
        write!(hint, " | Type category: {category}").unwrap();
    }

    // Add registry status if explicitly known
    if let Some(in_registry) = facts.in_registry {
        write!(hint, " | In BRP registry: {in_registry}").unwrap();
    }

    // Add serialization info if available
    match (facts.has_serialize, facts.has_deserialize) {
        (Some(ser), Some(deser)) => {
            write!(
                hint,
                " | Serialization support: serialize={ser}, deserialize={deser}"
            )
            .unwrap();
        }
        (Some(ser), None) => {
            write!(hint, " | Serialize support: {ser}").unwrap();
        }
        (None, Some(deser)) => {
            write!(hint, " | Deserialize support: {deser}").unwrap();
        }
        _ => {}
    }

    hint
}
