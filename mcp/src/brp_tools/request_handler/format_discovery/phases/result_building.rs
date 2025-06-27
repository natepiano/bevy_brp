//! Result building phase for the format discovery engine
//! This module handles building the final enhanced BRP result

use serde_json::Value;

use super::context::DiscoveryContext;
use super::tier_execution::DiscoveryResultData;
use crate::brp_tools::request_handler::format_discovery::detection::tier_info_to_debug_strings;
use crate::brp_tools::request_handler::format_discovery::engine::EnhancedBrpResult;
use crate::brp_tools::request_handler::format_discovery::utilities::{
    apply_corrections, get_parameter_location,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult, execute_brp_method};
use crate::error::Result;

/// Builds the final enhanced BRP result with debug information
pub async fn build_final_result(
    context: &mut DiscoveryContext,
    discovery_data: DiscoveryResultData,
) -> Result<EnhancedBrpResult> {
    // Add tier information to debug_info
    context
        .debug_info
        .extend(tier_info_to_debug_strings(&discovery_data.all_tier_info));

    if discovery_data.format_corrections.is_empty() {
        context.add_debug("Format Discovery: No corrections were possible".to_string());

        // Return the original error
        let original_error = context.initial_error.clone().unwrap_or_else(|| BrpError {
            code:    -1,
            message: "Unknown error".to_string(),
            data:    None,
        });

        Ok(EnhancedBrpResult {
            result:             BrpResult::Error(original_error),
            format_corrections: Vec::new(),
            debug_info:         context.debug_info.clone(),
        })
    } else {
        // Apply corrections and retry
        context.add_debug(format!(
            "Format Discovery: Found {} corrections, retrying request",
            discovery_data.format_corrections.len()
        ));

        // Build corrected params
        let corrected_params = build_corrected_params(context, &discovery_data.corrected_items)?;

        // Retry with corrected params
        let result =
            execute_brp_method(&context.method, Some(corrected_params), context.port).await?;

        context.add_debug(format!("Format Discovery: Retry result: {result:?}"));

        Ok(EnhancedBrpResult {
            result,
            format_corrections: discovery_data.format_corrections,
            debug_info: context.debug_info.clone(),
        })
    }
}

/// Build corrected parameters from the discovered format corrections
fn build_corrected_params(
    context: &DiscoveryContext,
    corrected_items: &[(String, Value)],
) -> Result<Value> {
    let params = context.original_params.as_ref().ok_or_else(|| {
        error_stack::report!(crate::error::Error::InvalidState(
            "No original params for correction".to_string()
        ))
    })?;

    let location = get_parameter_location(&context.method);
    Ok(apply_corrections(params, location, corrected_items))
}
