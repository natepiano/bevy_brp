//! `brp_execute` allows for executing an arbitrary BRP method - generally this is used as a
//! debugging tool for his MCP server but can also be used if (for example) a new brp method is
//! added before it's been implemented in this server code.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::format_discovery;
use crate::brp_tools::handler::{BrpMethodResult, HasPortField, convert_to_brp_method_result};
use crate::brp_tools::{default_port, deserialize_port};
use crate::error::Error;
use crate::response::LocalWithPortCallInfo;
use crate::tool::{BrpMethod, HandlerContext, HandlerResponse, ToolFn};

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')
    #[to_metadata]
    #[to_call_info(as = "brp_method")]
    pub method: String,
    /// Optional parameters for the method, as a JSON object or array
    #[to_metadata(skip_if_none)]
    pub params: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port:   u16,
}

impl HasPortField for ExecuteParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BrpExecute;

impl ToolFn for BrpExecute {
    type Output = BrpMethodResult;
    type CallInfoData = LocalWithPortCallInfo;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)> {
        let ctx = ctx.clone();

        Box::pin(async move {
            // Extract typed parameters
            let params = ctx.extract_parameter_values::<ExecuteParams>()?;
            let port = params.port;

            // For brp_execute, parse user input to BrpMethod
            let brp_method = BrpMethod::from_str(&params.method).ok_or_else(|| {
                Error::InvalidArgument(format!("Unknown BRP method: {}", params.method))
            })?;

            let enhanced_result = format_discovery::execute_brp_method_with_format_discovery(
                brp_method,    // Parsed BRP method
                params.params, // User-provided params (already Option<Value>)
                port,          // Use typed port parameter
            )
            .await?;

            // Convert result using existing conversion function
            let result = convert_to_brp_method_result(enhanced_result, &ctx)?;
            Ok((LocalWithPortCallInfo { port }, result))
        })
    }
}
