//! Initial attempt phase for the format discovery engine
//! This module handles the first attempt to execute a BRP method

use super::context::DiscoveryContext;
use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};
use crate::error::Result;

/// Execute the initial BRP method attempt
/// Returns the BRP result and updates the context with debug info
pub async fn execute(context: &mut DiscoveryContext) -> Result<BrpResult> {
    context.add_debug(format!(
        "Format Discovery: FUNCTION CALLED! Executing method '{}' with params: {:?}",
        context.method, context.original_params
    ));

    // Log the exact parameters being sent
    if let Some(ref params) = context.original_params {
        context.add_debug(format!(
            "Format Discovery: RAW PARAMS SENT: {}",
            serde_json::to_string_pretty(params)
                .unwrap_or_else(|_| "<serialization error>".to_string())
        ));
    }

    // Execute the BRP method
    let result = execute_brp_method(
        &context.method,
        context.original_params.clone(),
        context.port,
    )
    .await?;

    context.add_debug(format!("Format Discovery: Initial result: {result:?}"));

    // Log successful response details
    if let BrpResult::Success(ref data) = result {
        context.add_debug(format!(
            "Format Discovery: SUCCESS RESPONSE DATA: {}",
            serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| "<serialization error>".to_string())
        ));
    }

    // Store error in context if we got one
    if let BrpResult::Error(ref error) = result {
        context.set_error(error.clone());
    }

    Ok(result)
}
