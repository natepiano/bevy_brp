use rmcp::Error as McpError;
use rmcp::model::CallToolRequestParam;
use serde_json::Value;

use crate::constants::PARAM_WORKSPACE;
use crate::error::{Error, report_to_mcp_error};

// Value-based extraction functions (lower-level)

/// Generic function to extract a required numeric value from a JSON value
pub fn extract_required_numeric<T>(
    arguments: &Value,
    field_name: &str,
    field_description: &str,
) -> Result<T, McpError>
where
    T: TryFrom<u64>,
    T::Error: std::fmt::Display,
{
    arguments[field_name]
        .as_u64()
        .ok_or_else(|| {
            error_stack::Report::new(Error::ParameterExtraction(format!(
                "Missing {field_description} parameter"
            )))
            .attach_printable(format!("Field name: {field_name}"))
            .attach_printable(format!("Expected: {} number", std::any::type_name::<T>()))
        })
        .map_err(|report| report_to_mcp_error(&report))?
        .try_into()
        .map_err(|e| {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(format!(
                    "Invalid {field_description} value"
                )))
                .attach_printable(format!("Field name: {field_name}"))
                .attach_printable(format!("Conversion error: {e}")),
            )
        })
}

/// Extract a required u32 from a JSON value
pub fn extract_required_u32(
    arguments: &Value,
    field_name: &str,
    field_description: &str,
) -> std::result::Result<u32, McpError> {
    extract_required_numeric::<u32>(arguments, field_name, field_description)
}

/// Extract a required u64 from a JSON value
pub fn extract_required_u64(
    arguments: &Value,
    field_name: &str,
    field_description: &str,
) -> std::result::Result<u64, McpError> {
    extract_required_numeric::<u64>(arguments, field_name, field_description)
}

/// Generic function to extract an optional numeric value from a JSON value
pub fn extract_optional_numeric<T>(arguments: &Value, field_name: &str, default: T) -> T
where
    T: TryFrom<u64>,
    T::Error: std::fmt::Display,
{
    arguments[field_name]
        .as_u64()
        .and_then(|v| T::try_from(v).ok())
        .unwrap_or(default)
}

/// Extract an optional u16 with a default value
pub fn extract_optional_u16(arguments: &Value, field_name: &str, default_value: u16) -> u16 {
    extract_optional_numeric::<u16>(arguments, field_name, default_value)
}

/// Extract an optional array of strings from a Value
pub fn extract_optional_string_array(arguments: &Value, field_name: &str) -> Option<Vec<String>> {
    arguments[field_name].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>()
    })
}

// CallToolRequestParam-based extraction functions (higher-level)

/// Extract a required string parameter from the request
pub fn extract_required_string<'a>(
    request: &'a CallToolRequestParam,
    param_name: &str,
) -> std::result::Result<&'a str, McpError> {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(format!(
                    "Missing required parameter: {param_name}"
                )))
                .attach_printable(format!("Parameter name: {param_name}"))
                .attach_printable("Expected: string value"),
            )
        })
}

/// Extract an optional string parameter from the request with a default value
pub fn extract_optional_string<'a>(
    request: &'a CallToolRequestParam,
    param_name: &str,
    default: &'a str,
) -> &'a str {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
        .and_then(|v| v.as_str())
        .unwrap_or(default)
}

/// Extract an optional number parameter from the request with a default value
pub fn extract_optional_number(
    request: &CallToolRequestParam,
    param_name: &str,
    default: u64,
) -> std::result::Result<u64, McpError> {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
        .map_or(Ok(default), |v| {
            v.as_u64().ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Invalid parameter '{param_name}'"
                    )))
                    .attach_printable(format!("Parameter name: {param_name}"))
                    .attach_printable("Expected: number value"),
                )
            })
        })
}

/// Extract an optional u32 parameter from the request with a default value
pub fn extract_optional_u32(
    request: &CallToolRequestParam,
    param_name: &str,
    default: u32,
) -> std::result::Result<u32, McpError> {
    let value = extract_optional_number(request, param_name, u64::from(default))?;
    u32::try_from(value).map_err(|_| {
        report_to_mcp_error(
            &error_stack::Report::new(Error::ParameterExtraction(format!(
                "Invalid parameter '{param_name}'"
            )))
            .attach_printable(format!("Parameter name: {param_name}"))
            .attach_printable("Value too large for u32"),
        )
    })
}

/// Extract an optional u16 parameter from the request
/// Returns None if not provided, Some(u16) if provided and valid
pub fn extract_optional_u16_from_request(
    request: &CallToolRequestParam,
    param_name: &str,
) -> Result<Option<u16>, McpError> {
    match request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
    {
        Some(v) => {
            let value = v.as_u64().ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Invalid parameter '{param_name}'"
                    )))
                    .attach_printable(format!("Parameter name: {param_name}"))
                    .attach_printable("Expected: number value"),
                )
            })?;
            let port = u16::try_from(value).map_err(|_| {
                report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Invalid parameter '{param_name}'"
                    )))
                    .attach_printable(format!("Parameter name: {param_name}"))
                    .attach_printable("Value too large for u16"),
                )
            })?;

            // Validate port range (1024-65535 for non-privileged ports)
            if port < 1024 {
                return Err(report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Invalid parameter '{param_name}'"
                    )))
                    .attach_printable(format!("Parameter name: {param_name}"))
                    .attach_printable("Port must be >= 1024 (non-privileged ports only)"),
                ));
            }

            Ok(Some(port))
        }
        None => Ok(None),
    }
}

/// Extract a required number parameter from the request
pub fn extract_required_number(
    request: &CallToolRequestParam,
    param_name: &str,
) -> std::result::Result<u64, McpError> {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(format!(
                    "Missing required parameter: {param_name}"
                )))
                .attach_printable(format!("Parameter name: {param_name}"))
                .attach_printable("Expected: number value"),
            )
        })
}

/// Extract any value parameter from the request (for generic JSON values)
pub fn extract_any_value<'a>(
    request: &'a CallToolRequestParam,
    param_name: &str,
) -> Option<&'a serde_json::Value> {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
}

/// Extract an optional string array parameter from the request
pub fn extract_optional_string_array_from_request(
    request: &CallToolRequestParam,
    param_name: &str,
) -> std::result::Result<Option<Vec<String>>, McpError> {
    match request
        .arguments
        .as_ref()
        .and_then(|args| args.get(param_name))
    {
        Some(v) => {
            if let Some(arr) = v.as_array() {
                let mut result = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    if let Some(s) = item.as_str() {
                        result.push(s.to_string());
                    } else {
                        return Err(report_to_mcp_error(
                            &error_stack::Report::new(Error::ParameterExtraction(format!(
                                "Invalid item in '{param_name}' array"
                            )))
                            .attach_printable(format!("Parameter name: {param_name}"))
                            .attach_printable(format!("Array index: {index}"))
                            .attach_printable("Expected: string value"),
                        ));
                    }
                }
                Ok(Some(result))
            } else {
                Err(report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Invalid parameter '{param_name}'"
                    )))
                    .attach_printable(format!("Parameter name: {param_name}"))
                    .attach_printable("Expected: array value"),
                ))
            }
        }
        None => Ok(None),
    }
}

/// Extract an optional workspace parameter from the request
/// Returns None if not provided or empty string
pub fn extract_optional_workspace(request: &CallToolRequestParam) -> Option<String> {
    let workspace = extract_optional_string(request, PARAM_WORKSPACE, "");
    if workspace.is_empty() {
        None
    } else {
        Some(workspace.to_string())
    }
}
