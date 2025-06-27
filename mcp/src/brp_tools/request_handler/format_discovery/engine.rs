//! Orchestration and retry logic for format discovery

use serde_json::Value;

use super::constants::FORMAT_DISCOVERY_METHODS;
use crate::brp_tools::support::brp_client::BrpResult;
use crate::error::Result;

/// Location of type items in method parameters
#[derive(Debug, Clone, Copy)]
pub enum ParameterLocation {
    /// Type items are in a "components" object (spawn, insert)
    Components,
    /// Single type value in "value" field (`mutate_component`)
    ComponentValue,
    /// Single type value in "value" field (`insert_resource`, `mutate_resource`)
    ResourceValue,
}

/// Format correction information for a type (component or resource)
#[derive(Debug, Clone)]
pub struct FormatCorrection {
    pub component:        String, // Keep field name for API compatibility
    pub original_format:  Value,
    pub corrected_format: Value,
    pub hint:             String,
}

/// Enhanced response with format corrections
#[derive(Debug, Clone)]
pub struct EnhancedBrpResult {
    pub result:             BrpResult,
    pub format_corrections: Vec<FormatCorrection>,
    pub debug_info:         Vec<String>,
}

/// Execute a BRP method with automatic format discovery
pub async fn execute_brp_method_with_format_discovery(
    method: &str,
    params: Option<Value>,
    port: Option<u16>,
    initial_debug_info: Vec<String>,
) -> Result<EnhancedBrpResult> {
    use crate::brp_tools::request_handler::format_discovery::phases::context::DiscoveryContext;
    use crate::brp_tools::request_handler::format_discovery::phases::{
        error_analysis, initial_attempt, result_building, tier_execution,
    };

    // Initialize the discovery context
    let mut context = DiscoveryContext::new(method, params, port, initial_debug_info);

    // Phase 1: Execute initial attempt
    let initial_result = initial_attempt::execute(&mut context).await?;

    // Phase 2: Check if error analysis indicates recovery is possible
    if let Some(error) = error_analysis::needs_format_discovery(&initial_result, method) {
        context.add_debug(format!(
            "Format Discovery: Got error code {}, checking if method '{}' supports format discovery",
            error.code, method
        ));

        context.add_debug(format!(
            "Format Discovery: Method '{method}' is in FORMAT_DISCOVERY_METHODS"
        ));

        context.add_debug(
            "Format Discovery: Error is type format error, attempting discovery".to_string(),
        );

        // Phase 3: Run tiered discovery to find corrections
        let discovery_data = tier_execution::run_discovery_tiers(&mut context).await?;

        // Phase 4: Build final result with corrections
        return result_building::build_final_result(&mut context, discovery_data).await;
    }

    // Log appropriate message based on the result
    if let BrpResult::Error(ref error) = initial_result {
        if FORMAT_DISCOVERY_METHODS.contains(&method) {
            context.add_debug(format!(
                "Format Discovery: Error is NOT a type format error (code: {})",
                error.code
            ));
        } else {
            context.add_debug(format!(
                "Format Discovery: Method '{method}' is NOT in FORMAT_DISCOVERY_METHODS"
            ));
        }
    } else {
        context.add_debug(
            "Format Discovery: Initial request succeeded, no discovery needed".to_string(),
        );
    }

    // Return original result if no format discovery needed/possible
    context.add_debug(format!(
        "Format Discovery: Returning original result with {} debug messages",
        context.debug_info.len()
    ));

    Ok(EnhancedBrpResult {
        result:             initial_result,
        format_corrections: Vec::new(),
        debug_info:         context.debug_info,
    })
}
