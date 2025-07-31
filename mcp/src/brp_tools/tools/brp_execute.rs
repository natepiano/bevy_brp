//! `brp_execute` allows for executing an arbitrary BRP method - generally this is used as a
//! debugging tool for his MCP server but can also be used if (for example) a new brp method is
//! added before it's been implemented in this server code.
use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{BrpClient, BrpClientResult, Port};
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

/// Result type for the dynamic BRP execute tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ExecuteResult {
    /// The raw BRP response data
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Executed method {method}")]
    message_template: String,
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

            let client = BrpClient::new(
                brp_method,            // Parsed BRP method
                port,                  // Use typed port parameter
                params.params.clone(), // User-provided params (already Option<Value>)
            );
            let brp_result = match client.execute_direct().await {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ToolResult {
                        result: Err(e),
                        params: Some(params),
                    });
                }
            };

            // Convert BRP result to ToolResult
            match brp_result {
                BrpClientResult::Success(data) => Ok(ToolResult {
                    result: Ok(ExecuteResult::new(data)),
                    params: Some(params),
                }),
                BrpClientResult::Error(err) => Ok(ToolResult {
                    result: Err(Error::tool_call_failed(err.message).into()),
                    params: Some(params),
                }),
            }
        })
    }
}
