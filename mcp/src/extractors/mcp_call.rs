//! Extractor for MCP tool call arguments

use rmcp::Error as McpError;
use rmcp::model::CallToolRequestParam;
use serde_json::Value;

use crate::brp_tools::constants::DEFAULT_BRP_PORT;
use crate::constants::PARAM_BINARY_PATH;
use crate::error::{Error, report_to_mcp_error};

/// Extractor for data from MCP tool call arguments
pub struct McpCallExtractor<'a> {
    params:  Option<&'a Value>,
    request: Option<&'a CallToolRequestParam>,
}

impl<'a> McpCallExtractor<'a> {
    /// Create a new extractor from MCP tool call parameters
    pub const fn new(params: &'a Value) -> Self {
        Self {
            params:  Some(params),
            request: None,
        }
    }

    /// Create an extractor from a `CallToolRequestParam`
    pub const fn from_request(request: &'a CallToolRequestParam) -> Self {
        Self {
            params:  None,
            request: Some(request),
        }
    }

    /// Get a field value from either params or request
    fn get_field(&self, field_name: &str) -> Option<&'a Value> {
        self.params.map_or_else(
            || {
                self.request.and_then(|request| {
                    request
                        .arguments
                        .as_ref()
                        .and_then(|args| args.get(field_name))
                })
            },
            |params| params.get(field_name),
        )
    }

    /// Extract entity ID from MCP tool call parameters
    pub fn entity_id(&self) -> Option<u64> {
        self.get_field("entity").and_then(Value::as_u64)
    }

    /// Extract resource name from MCP tool call parameters
    pub fn resource_name(&self) -> Option<&str> {
        self.get_field("resource").and_then(|v| v.as_str())
    }

    // pub const fn query_params(&self) -> Option<&Value> {
    //     match self.params {
    //         Some(params) => Some(params),
    //         None => {
    //             // For request, we need to return a Value::Object from the JsonObject
    //             // This is not ideal as we can't return a reference to a temporary
    //             None
    //         }
    //     }
    // }

    /// Extract a specific field from the context parameters
    pub fn field(&self, field_name: &str) -> Option<&Value> {
        self.get_field(field_name)
    }

    /// Extract an optional number parameter with default
    pub fn optional_number(&self, field_name: &str, default: u64) -> u64 {
        self.get_field(field_name)
            .and_then(Value::as_u64)
            .unwrap_or(default)
    }

    /// Extract an optional string array parameter
    pub fn optional_string_array(&self, field_name: &str) -> Option<Vec<String>> {
        self.get_field(field_name)?.as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
    }

    // Result-based extractors for required fields

    /// Extract a required string parameter with error handling
    pub fn get_required_string(
        &self,
        field_name: &str,
        field_description: &str,
    ) -> Result<&str, McpError> {
        self.get_field(field_name)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Missing {field_description} parameter"
                    )))
                    .attach_printable(format!("Field name: {field_name}"))
                    .attach_printable("Expected: string value"),
                )
            })
    }

    /// Extract a required u64 parameter with error handling
    pub fn get_required_u64(
        &self,
        field_name: &str,
        field_description: &str,
    ) -> Result<u64, McpError> {
        self.get_field(field_name)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(Error::ParameterExtraction(format!(
                        "Missing {field_description} parameter"
                    )))
                    .attach_printable(format!("Field name: {field_name}"))
                    .attach_printable("Expected: u64 number"),
                )
            })
    }

    /// Extract a required u32 parameter with error handling
    pub fn get_required_u32(
        &self,
        field_name: &str,
        field_description: &str,
    ) -> Result<u32, McpError> {
        let value = self.get_required_u64(field_name, field_description)?;
        u32::try_from(value).map_err(|_| {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(format!(
                    "Invalid {field_description} value"
                )))
                .attach_printable(format!("Field name: {field_name}"))
                .attach_printable("Value too large for u32"),
            )
        })
    }

    // Result-based extractors for optional fields

    /// Extract an optional string parameter with a default value
    pub fn get_optional_string<'b>(&'b self, param_name: &str, default: &'b str) -> &'b str {
        self.get_field(param_name)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    /// Extract an optional bool parameter with a default value
    pub fn get_optional_bool(&self, param_name: &str, default: bool) -> bool {
        self.get_field(param_name)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default)
    }

    /// Extract an optional u16 parameter with validation for port numbers
    pub fn get_optional_u16(&self, param_name: &str) -> Result<Option<u16>, McpError> {
        match self.get_field(param_name) {
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

    /// Extract an optional u32 parameter with a default value
    pub fn get_optional_u32(&self, param_name: &str, default: u32) -> Result<u32, McpError> {
        let value = self
            .get_field(param_name)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| u64::from(default));

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

    /// Extract an optional path parameter
    /// Returns None if not provided or empty string
    pub fn get_optional_path(&self) -> Option<String> {
        let path = self.get_optional_string(PARAM_BINARY_PATH, "");
        if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        }
    }

    // Specialized common extractors

    /// Extract port parameter with default value
    pub fn get_port(&self) -> Result<u16, McpError> {
        Ok(self.get_optional_u16("port")?.unwrap_or(DEFAULT_BRP_PORT))
    }

    /// Extract entity ID parameter
    pub fn get_entity_id(&self) -> Result<u64, McpError> {
        self.get_required_u64("entity", "entity ID")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_extract_field_from_context() {
        let params = json!({"components": ["Transform"], "entity": 42});
        let extractor = McpCallExtractor::new(&params);

        let result = extractor.field("components");
        assert_eq!(result, Some(&json!(["Transform"])));

        let result = extractor.field("entity");
        assert_eq!(result, Some(&json!(42)));

        let result = extractor.field("missing");
        assert_eq!(result, None);
    }
}
