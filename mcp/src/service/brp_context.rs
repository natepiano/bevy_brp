use rmcp::Error as McpError;
use serde_json::{Value, json};

use super::{HandlerContext, HasCallInfo};
use crate::error::Error;
use crate::response::CallInfo;
use crate::tool::ParamType;

/// Data type for BRP handler contexts (carries extracted request data)
#[derive(Clone)]
pub struct BrpContext {
    pub method: String,
    pub port:   u16,
}

impl HasCallInfo for HandlerContext<BrpContext> {
    fn call_info(&self) -> CallInfo {
        self.call_info()
    }
}

impl HandlerContext<BrpContext> {
    pub fn brp_method(&self) -> &str {
        &self.handler_data.method
    }

    pub fn call_info(&self) -> CallInfo {
        CallInfo::brp(
            self.request.name.to_string(),
            self.handler_data.method.clone(),
            self.handler_data.port,
        )
    }

    /// Extract brp method parameters from tool definition
    #[allow(clippy::too_many_lines)]
    pub fn extract_params_from_definition(&self) -> Result<Option<serde_json::Value>, McpError> {
        // Get the tool definition
        let tool_def = self.tool_def()?;

        // Build params from parameter definitions
        let mut params_obj = serde_json::Map::new();
        let mut has_params = false;

        for param in tool_def.parameters() {
            // Extract parameter value based on type
            let value = match param.param_type() {
                ParamType::Number => {
                    if param.required() {
                        // Remove special handling for entity parameter
                        Some(json!(
                            self.extract_required_u64(param.name(), param.description())?
                        ))
                    } else {
                        self.extract_optional_named_field(param.name())
                            .and_then(serde_json::Value::as_u64)
                            .map(|v| json!(v))
                    }
                }
                ParamType::String => {
                    if param.required() {
                        Some(json!(self.extract_required_string(
                            param.name(),
                            param.description()
                        )?))
                    } else {
                        self.extract_optional_named_field(param.name())
                            .and_then(|v| v.as_str())
                            .map(|s| json!(s))
                    }
                }
                ParamType::Boolean => {
                    if param.required() {
                        // For required boolean, we need to check if it exists and is a bool
                        let value = self
                            .extract_optional_named_field(param.name())
                            .and_then(serde_json::Value::as_bool)
                            .ok_or_else(|| {
                                crate::error::report_to_mcp_error(
                                    &error_stack::Report::new(
                                        crate::error::Error::InvalidArgument(format!(
                                            "Missing {} parameter",
                                            param.description()
                                        )),
                                    )
                                    .attach_printable(format!("Field name: {}", param.name()))
                                    .attach_printable("Expected: boolean value"),
                                )
                            })?;
                        Some(json!(value))
                    } else {
                        self.extract_optional_named_field(param.name())
                            .and_then(serde_json::Value::as_bool)
                            .map(|v| json!(v))
                    }
                }
                ParamType::StringArray => {
                    if param.required() {
                        let array = self
                            .extract_optional_string_array(param.name())
                            .ok_or_else(|| {
                                crate::error::report_to_mcp_error(
                                    &error_stack::Report::new(
                                        crate::error::Error::InvalidArgument(format!(
                                            "Missing {} parameter",
                                            param.description()
                                        )),
                                    )
                                    .attach_printable(format!("Field name: {}", param.name()))
                                    .attach_printable("Expected: array of strings"),
                                )
                            })?;
                        Some(json!(array))
                    } else {
                        self.extract_optional_string_array(param.name())
                            .map(|v| json!(v))
                    }
                }
                ParamType::NumberArray => {
                    if param.required() {
                        let array = self
                            .extract_optional_named_field(param.name())
                            .and_then(|v| v.as_array())
                            .and_then(|arr| {
                                arr.iter()
                                    .map(serde_json::Value::as_u64)
                                    .collect::<Option<Vec<_>>>()
                            })
                            .ok_or_else(|| {
                                crate::error::report_to_mcp_error(
                                    &error_stack::Report::new(
                                        crate::error::Error::InvalidArgument(format!(
                                            "Missing {} parameter",
                                            param.description()
                                        )),
                                    )
                                    .attach_printable(format!("Field name: {}", param.name()))
                                    .attach_printable("Expected: array of numbers"),
                                )
                            })?;
                        Some(json!(array))
                    } else {
                        self.extract_optional_named_field(param.name())
                            .and_then(|v| v.as_array())
                            .and_then(|arr| {
                                arr.iter()
                                    .map(serde_json::Value::as_u64)
                                    .collect::<Option<Vec<_>>>()
                            })
                            .map(|v| json!(v))
                    }
                }
                ParamType::Any => {
                    if param.required() {
                        let value =
                            self.extract_optional_named_field(param.name())
                                .ok_or_else(|| {
                                    crate::error::report_to_mcp_error(
                                        &error_stack::Report::new(Error::InvalidArgument(format!(
                                            "Missing {} parameter",
                                            param.description()
                                        )))
                                        .attach_printable(format!("Field name: {}", param.name()))
                                        .attach_printable("Expected: JSON value"),
                                    )
                                })?;
                        Some(value.clone())
                    } else {
                        self.extract_optional_named_field(param.name()).cloned()
                    }
                }
            };

            // Add to params if value exists
            if let Some(val) = value {
                params_obj.insert(param.name().to_string(), val);
                has_params = true;
            }
        }

        // Return params
        let params = if has_params {
            Some(Value::Object(params_obj))
        } else {
            None
        };

        Ok(params)
    }
}
