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

use super::rpc_discover;
use crate::brp_tools;
use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::error::Error;
use crate::error::Result;
use crate::tool::ToolFn;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ExecuteParams {
    /// The BRP method to execute (e.g., `rpc.discover`, `world.get_components`, `world.query`)
    pub method: String,
    /// Optional parameters for the method
    #[to_metadata(skip_if_none)]
    pub params: Option<Value>,
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

    async fn handle_impl(&self, params: ExecuteParams) -> Result<ExecuteResult> {
        let method_names = rpc_discover::discover_method_names(params.port).await?;
        if !method_is_registered(&method_names, &params.method) {
            let mut available_methods = method_names;
            available_methods.sort_unstable();
            let message = format!(
                "BRP method `{}` is not registered on port {}",
                params.method, params.port
            );
            return Err(Error::tool_call_failed_with_details(
                brp_tools::method_not_found_message(&params.method, &message),
                serde_json::json!({
                    "stage": "discovery",
                    "method": params.method,
                    "port": params.port,
                    "available_methods": available_methods,
                }),
            )
            .into());
        }

        let brp_client =
            BrpClient::for_application(params.method.clone(), params.port, params.params.clone());

        let brp_result = brp_client.execute_raw().await?;

        match brp_result {
            ResponseStatus::Success(data) => Ok(ExecuteResult::new(data)),
            ResponseStatus::Error(error) => Err(Error::tool_call_failed_with_details(
                error.get_message(),
                serde_json::json!({
                    "stage": "execution",
                    "method": params.method,
                    "port": params.port,
                    "code": error.code,
                    "data": error.data,
                }),
            )
            .into()),
        }
    }
}

fn method_is_registered(method_names: &[String], requested_method: &str) -> bool {
    method_names.iter().any(|method| method == requested_method)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ExecuteParams;
    use super::method_is_registered;

    #[test]
    fn execute_params_accept_application_method_names() -> serde_json::Result<()> {
        let params = serde_json::from_value::<ExecuteParams>(json!({
            "port": 15_702,
            "method": "test/multiply",
            "params": {"value": 6, "factor": 7}
        }))?;

        assert_eq!(params.method, "test/multiply");
        assert_eq!(params.params, Some(json!({"value": 6, "factor": 7})));
        Ok(())
    }

    #[test]
    fn registration_requires_an_exact_method_name() {
        let methods = vec![String::from("rpc.discover"), String::from("test/multiply")];

        assert!(method_is_registered(&methods, "test/multiply"));
        assert!(!method_is_registered(&methods, "test/multiply_more"));
    }
}
