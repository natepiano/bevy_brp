use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::brp_tools::request_handler::{FormatCorrection, FormatCorrectionStatus};
use crate::response::{FormatterConfig, ResponseBuilder, ResponseFormatter};
use crate::service::{HandlerContext, HasCallInfo};

/// V2 formatter that handles both local and BRP results uniformly
pub fn format_tool_call_result_v2<T>(
    result: Result<serde_json::Value, McpError>,
    handler_context: &HandlerContext<T>,
    formatter_config: FormatterConfig,
) -> Result<CallToolResult, McpError>
where
    HandlerContext<T>: HasCallInfo,
{
    match result {
        Ok(value) => {
            // Check if this is an error response
            let is_error = value
                .get("status")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "error");

            if is_error {
                // Handle error response
                let message = value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");

                let error_response =
                    ResponseBuilder::error(handler_context.call_info()).message(message);

                // Add error fields as metadata
                let error_response = if let serde_json::Value::Object(map) = &value {
                    map.iter()
                        .filter(|(key, val)| {
                            let k = key.as_str();
                            k != "status" && k != "message" && !val.is_null()
                        })
                        .try_fold(error_response, |builder, (key, val)| {
                            builder.add_field(key, val)
                        })
                        .unwrap_or_else(|_| {
                            // If adding fields failed, just return the basic error response
                            ResponseBuilder::error(handler_context.call_info()).message(message)
                        })
                } else {
                    error_response
                };

                Ok(error_response.build().to_call_tool_result())
            } else {
                // Handle success response
                let formatter = ResponseFormatter::new(formatter_config);

                // Check if this is a BRP result with format correction information
                let (format_corrections, format_corrected) = extract_format_correction_info(&value);

                // For V2, the entire value contains the structured result
                // Use format_success_with_corrections to handle format correction messaging
                Ok(formatter.format_success_with_corrections(
                    &value,
                    handler_context,
                    format_corrections.as_deref(),
                    format_corrected.as_ref(),
                ))
            }
        }
        Err(e) => Err(e),
    }
}

/// Extract format correction information from V2 BRP result JSON
fn extract_format_correction_info(
    value: &serde_json::Value,
) -> (
    Option<Vec<FormatCorrection>>,
    Option<FormatCorrectionStatus>,
) {
    let format_corrected = value
        .get("format_corrected")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let format_corrections = value
        .get("format_corrections")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|correction_json| {
                    // Convert JSON back to FormatCorrection struct
                    let component = correction_json
                        .get("component")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let original_format = correction_json
                        .get("original_format")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let corrected_format = correction_json
                        .get("corrected_format")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let hint = correction_json
                        .get("hint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let supported_operations = correction_json
                        .get("supported_operations")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });

                    let mutation_paths = correction_json
                        .get("mutation_paths")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });

                    let type_category = correction_json
                        .get("type_category")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    Some(FormatCorrection {
                        component,
                        original_format,
                        corrected_format,
                        hint,
                        supported_operations,
                        mutation_paths,
                        type_category,
                    })
                })
                .collect()
        });

    (format_corrections, format_corrected)
}
