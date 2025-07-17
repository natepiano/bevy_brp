use serde_json::{Value, json};

use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use super::types::BrpMethodResult;
use crate::brp_tools::support::brp_client::BrpResult;
use crate::error;
use crate::service::{BrpContext, HandlerContext};
use crate::tool::{BrpToolFnV2, HandlerResponse, HandlerResult};

/// BRP method handler V2 that implements `BrpToolFnV2` and returns `HandlerResponse`
pub struct BrpMethodHandlerV2;

/// Convert a `FormatCorrection` to JSON representation with metadata
fn format_correction_to_json(correction: &FormatCorrection) -> Value {
    let mut correction_json = json!({
        "component": correction.component,
        "original_format": correction.original_format,
        "corrected_format": correction.corrected_format,
        "hint": correction.hint
    });

    // Add rich metadata fields if available
    if let Some(obj) = correction_json.as_object_mut() {
        if let Some(ops) = &correction.supported_operations {
            obj.insert("supported_operations".to_string(), json!(ops));
        }
        if let Some(paths) = &correction.mutation_paths {
            obj.insert("mutation_paths".to_string(), json!(paths));
        }
        if let Some(cat) = &correction.type_category {
            obj.insert("type_category".to_string(), json!(cat));
        }
    }

    correction_json
}

impl BrpToolFnV2 for BrpMethodHandlerV2 {
    fn call(&self, ctx: &HandlerContext<BrpContext>) -> HandlerResponse<'_> {
        let ctx = ctx.clone();

        Box::pin(async move {
            // Extract parameters for the call based on the tool definition
            let params = ctx.extract_params_from_definition()?;
            let method_name = ctx.brp_method();

            // Execute with format discovery (reuse existing function)
            let enhanced_result =
                execute_brp_method_with_format_discovery(method_name, params, ctx.port())
                    .await
                    .map_err(|err| error::report_to_mcp_error(&err))?;

            // Convert to BrpMethodResult
            let result = convert_to_brp_method_result(enhanced_result, &ctx);

            Ok(Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

/// Convert `EnhancedBrpResult` to `BrpMethodResult`
fn convert_to_brp_method_result(
    enhanced_result: EnhancedBrpResult,
    ctx: &HandlerContext<BrpContext>,
) -> BrpMethodResult {
    match enhanced_result.result {
        BrpResult::Success(data) => BrpMethodResult {
            status:             None, // No status field for success
            message:            None,
            code:               None,
            error_data:         None,
            result:             data, // Direct BRP response data
            format_corrections: enhanced_result
                .format_corrections
                .iter()
                .map(format_correction_to_json)
                .collect(),
            format_corrected:   Some(enhanced_result.format_corrected),
        },
        BrpResult::Error(ref err) => {
            // Process error enhancements (reuse logic from current handler)
            let enhanced_message = enhance_error_message(err, &enhanced_result, ctx);
            let error_data = enhance_error_data(err.data.clone(), &enhanced_result, ctx);

            BrpMethodResult {
                status: Some("error".to_string()),
                message: Some(enhanced_message),
                code: Some(err.code),
                error_data,
                result: None,
                format_corrections: enhanced_result
                    .format_corrections
                    .iter()
                    .map(format_correction_to_json)
                    .collect(),
                format_corrected: Some(enhanced_result.format_corrected),
            }
        }
    }
}

/// Enhance error message with format discovery insights
fn enhance_error_message(
    err: &crate::brp_tools::support::brp_client::BrpError,
    enhanced_result: &EnhancedBrpResult,
    _ctx: &HandlerContext<BrpContext>,
) -> String {
    // Check if the enhanced result has a different error message
    if let BrpResult::Error(enhanced_error) = &enhanced_result.result {
        if enhanced_error.message != err.message {
            return enhanced_error.message.clone();
        }
    }

    // Check format corrections for educational hints
    if let Some(correction) = enhanced_result
        .format_corrections
        .iter()
        .find(|c| c.hint.contains("cannot be used with BRP"))
    {
        return correction.hint.clone();
    }

    // Use original message
    err.message.clone()
}

/// Enhance error data with format corrections
fn enhance_error_data(
    original_data: Option<Value>,
    enhanced_result: &EnhancedBrpResult,
    ctx: &HandlerContext<BrpContext>,
) -> Option<Value> {
    use super::format_discovery::FORMAT_DISCOVERY_METHODS;
    use crate::constants::JSON_FIELD_FORMAT_CORRECTIONS;

    // Only add format corrections for methods that support format discovery
    if !FORMAT_DISCOVERY_METHODS.contains(&ctx.brp_method())
        || enhanced_result.format_corrections.is_empty()
    {
        return original_data;
    }

    let mut data_obj = original_data.unwrap_or_else(|| serde_json::json!({}));

    if let Value::Object(map) = &mut data_obj {
        // Add format corrections as JSON (manually convert like the current handler)
        let corrections: Vec<Value> = enhanced_result
            .format_corrections
            .iter()
            .map(format_correction_to_json)
            .collect();

        map.insert(
            JSON_FIELD_FORMAT_CORRECTIONS.to_string(),
            Value::Array(corrections),
        );

        // Add rich metadata from first correction
        if let Some(first_correction) = enhanced_result.format_corrections.first() {
            if let Some(ops) = &first_correction.supported_operations {
                map.insert("supported_operations".to_string(), serde_json::json!(ops));
            }
        }

        // Add format corrected status
        map.insert(
            "format_corrected".to_string(),
            serde_json::to_value(&enhanced_result.format_corrected).unwrap_or(Value::Null),
        );
    }

    Some(data_obj)
}
