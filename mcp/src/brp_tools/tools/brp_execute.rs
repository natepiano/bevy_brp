//! `brp_execute` allows for executing an arbitrary BRP method - generally this is used as a
//! debugging tool for his MCP server but can also be used if (for example) a new brp method is
//! added before it's been implemented in this server code.
use async_trait::async_trait;
use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::error::Error;
use crate::tool::BrpMethod;
use crate::tool::ToolFn;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., `rpc.discover`, `world.get_components`, `world.query`)
    pub method: BrpMethod,
    /// Optional parameters for the method
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

#[async_trait]
impl ToolFn for BrpExecute {
    type Output = ExecuteResult;
    type Params = ExecuteParams;

    async fn handle_impl(&self, params: ExecuteParams) -> crate::error::Result<ExecuteResult> {
        let client = BrpClient::new(
            params.method,         // Direct use of typed BRP method
            params.port,           // Use typed port parameter
            params.params.clone(), // User-provided params (already Option<Value>)
        );

        let brp_result = client.execute_raw().await?;

        // Convert BRP result to ExecuteResult
        match brp_result {
            ResponseStatus::Success(data) => Ok(ExecuteResult::new(data)),
            ResponseStatus::Error(err) => Err(Error::tool_call_failed(err.get_message()).into()),
        }
    }
}
