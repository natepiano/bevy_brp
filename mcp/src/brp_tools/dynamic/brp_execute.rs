use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::format_discovery::execute_brp_method_with_format_discovery;
use crate::brp_tools::handler::{BrpMethodResult, HasPortField, convert_to_brp_method_result};
use crate::constants::default_port;
use crate::tool::{HandlerContext, HandlerResponse, HasPort, LocalToolFnWithPort, NoMethod};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')
    pub method: String,
    /// Optional parameters for the method, as a JSON object or array
    pub params: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:   u16,
}

impl HasPortField for ExecuteParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BrpExecute;

impl LocalToolFnWithPort for BrpExecute {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<Self::Output> {
        let ctx = ctx.clone();

        Box::pin(async move {
            // Extract typed parameters
            let params = ctx.extract_typed_params::<ExecuteParams>()?;

            // For brp_execute, use method from parameters (user input)
            let enhanced_result = execute_brp_method_with_format_discovery(
                &params.method, // User-provided method name from ExecuteParams
                params.params,  // User-provided params (already Option<Value>)
                params.port,    // Use typed port parameter
            )
            .await?;

            // Convert result using existing conversion function
            convert_to_brp_method_result(enhanced_result, &ctx)
        })
    }
}
