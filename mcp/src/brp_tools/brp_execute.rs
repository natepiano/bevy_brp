//! `brp_execute` allows for executing an arbitrary BRP method - generally this is used as a
//! debugging tool for his MCP server but can also be used if (for example) a new brp method is
//! added before it's been implemented in this server code.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::format_discovery::execute_brp_method_with_format_discovery;
use crate::brp_tools::handler::{BrpMethodResult, HasPortField, convert_to_brp_method_result};
use crate::constants::default_port;
use crate::tool::{HandlerContext, HandlerResponse, UnifiedToolFn};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')
    pub method: String,
    /// Optional parameters for the method, as a JSON object or array
    pub params: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:   u16,
}

impl HasPortField for ExecuteParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BrpExecute;

impl UnifiedToolFn for BrpExecute {
    type Output = BrpMethodResult;
    type CallInfoData = crate::response::LocalWithPortCallInfo;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)> {
        let ctx = ctx.clone();

        Box::pin(async move {
            // Extract typed parameters
            let params = ctx.extract_typed_params::<ExecuteParams>()?;
            let port = params.port;

            // For brp_execute, use method from parameters (user input)
            let enhanced_result = execute_brp_method_with_format_discovery(
                &params.method, // User-provided method name from ExecuteParams
                params.params,  // User-provided params (already Option<Value>)
                port,           // Use typed port parameter
            )
            .await?;

            // Convert result using existing conversion function
            let result = convert_to_brp_method_result(enhanced_result, &ctx)?;
            Ok((crate::response::LocalWithPortCallInfo { port }, result))
        })
    }
}
