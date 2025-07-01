//! Result building phase for the format discovery engine
//! This module handles building the final enhanced BRP result

use serde_json::Value;
use tracing::trace;

use super::context::DiscoveryContext;
use super::tier_execution::DiscoveryResultData;
use crate::brp_tools::request_handler::format_discovery::detection::tier_info_to_debug_strings;
use crate::brp_tools::request_handler::format_discovery::engine::EnhancedBrpResult;
use crate::brp_tools::request_handler::format_discovery::path_suggestions::enhance_type_mismatch_error;
use crate::brp_tools::request_handler::format_discovery::support::{
    apply_corrections, get_parameter_location,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult, execute_brp_method};
use crate::error::Result;

/// Builds the final enhanced BRP result with debug information
pub async fn build_final_result(
    context: &DiscoveryContext,
    discovery_data: DiscoveryResultData,
) -> Result<EnhancedBrpResult> {
    // Log tier information to tracing
    for debug_line in tier_info_to_debug_strings(&discovery_data.all_tier_info) {
        trace!("Tier info: {}", debug_line);
    }

    if discovery_data.format_corrections.is_empty() {
        DiscoveryContext::add_debug("Format Discovery: No corrections were possible".to_string());

        // Return the original error, enhanced with path suggestions if applicable
        let original_error = context.initial_error.clone().unwrap_or_else(|| BrpError {
            code:    -1,
            message: "Unknown error".to_string(),
            data:    None,
        });

        // Try to enhance the error with path suggestions
        let enhanced_error = enhance_type_mismatch_error_with_context(&original_error, context)
            .await
            .unwrap_or(original_error);

        Ok(EnhancedBrpResult {
            result:             BrpResult::Error(enhanced_error),
            format_corrections: Vec::new(),
        })
    } else {
        // Apply corrections and retry
        let corrections_with_metadata = discovery_data
            .format_corrections
            .iter()
            .filter(|correction| correction.has_rich_metadata())
            .count();

        DiscoveryContext::add_debug(format!(
            "Format Discovery: Found {} corrections ({} with rich metadata), retrying request",
            discovery_data.format_corrections.len(),
            corrections_with_metadata
        ));

        // Build corrected params
        let corrected_params = build_corrected_params(context, &discovery_data.corrected_items)?;

        // Retry with corrected params
        let result =
            execute_brp_method(&context.method, Some(corrected_params), context.port).await?;

        DiscoveryContext::add_debug(format!("Format Discovery: Retry result: {result:?}"));

        Ok(EnhancedBrpResult {
            result,
            format_corrections: discovery_data.format_corrections,
        })
    }
}

/// Enhance error with path suggestions using context information
async fn enhance_type_mismatch_error_with_context(
    original_error: &BrpError,
    context: &DiscoveryContext,
) -> Result<BrpError> {
    // Extract component type from the original parameters
    let component_type = extract_component_type_from_context(context);

    Ok(enhance_type_mismatch_error(original_error, component_type.as_deref(), context.port).await)
}

/// Extract component type from discovery context
fn extract_component_type_from_context(context: &DiscoveryContext) -> Option<String> {
    context.original_params.as_ref().and_then(|params| {
        match context.method.as_str() {
            "bevy/mutate_component" => {
                // Extract from "component" field
                params
                    .get("component")
                    .and_then(Value::as_str)
                    .map(String::from)
            }
            "bevy/insert" | "bevy/spawn" => {
                // Extract from "components" object keys
                params
                    .get("components")
                    .and_then(Value::as_object)
                    .and_then(|components| components.keys().next().cloned())
            }
            "bevy/mutate_resource" | "bevy/insert_resource" => {
                // Extract from "resource" field
                params
                    .get("resource")
                    .and_then(Value::as_str)
                    .map(String::from)
            }
            _ => None,
        }
    })
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
