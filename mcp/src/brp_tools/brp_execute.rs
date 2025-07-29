//! `brp_execute` allows for executing an arbitrary BRP method - generally this is used as a
//! debugging tool for his MCP server but can also be used if (for example) a new brp method is
//! added before it's been implemented in this server code.
use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::format_discovery;
use crate::brp_tools::Port;
use crate::brp_tools::handler::{HasPortField, format_correction_to_json};
use crate::error::Error;
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')
    pub method: String,
    /// Optional parameters for the method, as a JSON object or array
    #[to_metadata(skip_if_none)]
    pub params: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:   Port,
}

impl HasPortField for ExecuteParams {
    fn port(&self) -> Port {
        self.port
    }
}

/// Result type for the dynamic BRP execute tool
#[derive(Serialize, ResultStruct)]
pub struct ExecuteResult {
    /// The raw BRP response data
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Format corrections applied during the BRP call
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<Value>>,

    /// Whether format corrections were applied
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,

    /// Message template for formatting responses
    #[to_message(message_template = "Executed method {method}")]
    message_template: String,
}

// ExecuteResult uses format discovery, so it needs the 3-parameter FromBrpValue implementation
impl crate::brp_tools::handler::FromBrpValue for ExecuteResult {
    type Args = (
        Option<Value>,
        Option<Vec<Value>>,
        Option<crate::brp_tools::FormatCorrectionStatus>,
    );

    fn from_brp_value(args: Self::Args) -> crate::error::Result<Self> {
        // Call the existing macro-generated method
        Self::from_brp_value(args.0, args.1, args.2)
    }
}

pub struct BrpExecute;

impl ToolFn for BrpExecute {
    type Output = ExecuteResult;
    type Params = ExecuteParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            // Extract typed parameters
            let params: ExecuteParams = ctx.extract_parameter_values()?;
            let port = params.port;

            // For brp_execute, parse user input to BrpMethod
            let Some(brp_method) = BrpMethod::from_str(&params.method) else {
                return Ok(ToolResult {
                    result: Err(Error::InvalidArgument(format!(
                        "Unknown BRP method: {}",
                        params.method
                    ))
                    .into()),
                    params: Some(params),
                });
            };

            let enhanced_result = match format_discovery::execute_brp_method_with_format_discovery(
                brp_method,            // Parsed BRP method
                params.params.clone(), // User-provided params (already Option<Value>)
                port,                  // Use typed port parameter
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ToolResult {
                        result: Err(e),
                        params: Some(params),
                    });
                }
            };

            // Convert enhanced result to ExecuteResult using FromBrpValue trait
            let result = match enhanced_result.result {
                super::brp_client::BrpResult::Success(data) => {
                    let format_corrections = if enhanced_result.format_corrections.is_empty() {
                        None
                    } else {
                        Some(
                            enhanced_result
                                .format_corrections
                                .iter()
                                .map(format_correction_to_json)
                                .collect(),
                        )
                    };

                    ExecuteResult::from_brp_value(
                        data,
                        format_corrections,
                        Some(enhanced_result.format_corrected),
                    )
                }
                super::brp_client::BrpResult::Error(err) => {
                    // For now, keep error handling as-is
                    return Ok(ToolResult {
                        result: Err(Error::tool_call_failed(err.message).into()),
                        params: Some(params),
                    });
                }
            };

            Ok(ToolResult {
                result,
                params: Some(params),
            })
        })
    }
}
