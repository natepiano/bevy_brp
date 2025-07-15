use rmcp::Error as McpError;
use serde_json::{Value, json};

use super::{HandlerContext, HasCallInfo};
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
    pub fn extract_params_from_definition(&self) -> Result<Option<serde_json::Value>, McpError> {
        // Get the tool definition
        let tool_def = self.tool_def()?;

        // Build params from parameter definitions
        let mut params_obj = serde_json::Map::new();
        let mut has_params = false;

        for param in tool_def.parameters() {
            // Extract parameter value based on type
            let value = match param.param_type() {
                ParamType::Number => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        serde_json::Value::as_u64,
                    )?
                    .map(|v| json!(v)),
                ParamType::String => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| v.as_str().map(std::string::ToString::to_string),
                    )?
                    .map(|s| json!(s)),
                ParamType::Boolean => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        serde_json::Value::as_bool,
                    )?
                    .map(|b| json!(b)),
                ParamType::StringArray => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| {
                            v.as_array().and_then(|arr| {
                                arr.iter()
                                    .map(|item| item.as_str())
                                    .collect::<Option<Vec<_>>>()
                                    .map(|strings| {
                                        strings
                                            .into_iter()
                                            .map(String::from)
                                            .collect::<Vec<String>>()
                                    })
                            })
                        },
                    )?
                    .map(|array| json!(array)),
                ParamType::NumberArray => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| {
                            v.as_array().and_then(|arr| {
                                arr.iter()
                                    .map(serde_json::Value::as_u64)
                                    .collect::<Option<Vec<_>>>()
                            })
                        },
                    )?
                    .map(|array| json!(array)),
                ParamType::Any => self.extract_typed_param(
                    param.name(),
                    param.description(),
                    param.required(),
                    |v| Some(v.clone()),
                )?,
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
