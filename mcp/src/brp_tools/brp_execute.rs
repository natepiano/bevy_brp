//! `brp_execute` allows for executing an arbitrary BRP method - generally this is used as a
//! debugging tool for his MCP server but can also be used if (for example) a new brp method is
//! added before it's been implemented in this server code.
use bevy_brp_mcp_macros::ParamStruct;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::format_discovery;
use crate::brp_tools::Port;
use crate::brp_tools::handler::{BrpMethodResult, HasPortField, convert_to_brp_method_result};
use crate::error::Error;
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')
    #[to_metadata]
    #[to_call_info(as = "brp_method")]
    pub method: String,
    /// Optional parameters for the method, as a JSON object or array
    #[to_metadata(skip_if_none)]
    pub params: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(default)]
    #[to_call_info]
    pub port:   Port,
}

impl HasPortField for ExecuteParams {
    fn port(&self) -> Port {
        self.port
    }
}

pub struct BrpExecute;

impl ToolFn for BrpExecute {
    type Output = BrpMethodResult;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output>> {
        Box::pin(async move {
            // Extract typed parameters
            let params: ExecuteParams = ctx.extract_parameter_values()?;
            let port = params.port;

            // For brp_execute, parse user input to BrpMethod
            let Some(brp_method) = BrpMethod::from_str(&params.method) else {
                return Ok(ToolResult::with_port(
                    Err(
                        Error::InvalidArgument(format!("Unknown BRP method: {}", params.method))
                            .into(),
                    ),
                    port,
                ));
            };

            let enhanced_result = match format_discovery::execute_brp_method_with_format_discovery(
                brp_method,    // Parsed BRP method
                params.params, // User-provided params (already Option<Value>)
                port,          // Use typed port parameter
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ToolResult::with_port(Err(e), port));
                }
            };

            // Convert result using existing conversion function
            let result = convert_to_brp_method_result(enhanced_result, &params.method);
            Ok(ToolResult::with_port(result, port))
        })
    }
}
