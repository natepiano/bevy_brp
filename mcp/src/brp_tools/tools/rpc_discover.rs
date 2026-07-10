//! `rpc.discover` tool - Discover available BRP methods

use bevy::remote::schemas::open_rpc::OpenRpcDocument;
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
use crate::error::Result;
use crate::tool::BrpMethod;

/// Parameters for the `rpc.discover` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `rpc.discover` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct RpcDiscoverResult {
    /// The raw BRP response containing method discovery information
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of methods discovered
    #[to_metadata(result_operation = "count_methods")]
    pub method_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Discovered {method_count} methods")]
    pub message_template: String,
}

pub(super) async fn discover_method_names(port: Port) -> Result<Vec<String>> {
    let client = BrpClient::new(BrpMethod::RpcDiscover, port, None);
    let response = match client.execute_raw().await {
        Ok(response) => response,
        Err(error) => {
            return Err(Error::tool_call_failed_with_details(
                format!("Failed to discover BRP methods on port {port}"),
                serde_json::json!({
                    "stage": "discovery",
                    "port": port,
                    "error": error.current_context().to_string(),
                }),
            )
            .into());
        },
    };

    let value = match response {
        ResponseStatus::Success(Some(value)) => value,
        ResponseStatus::Success(None) => {
            return Err(discovery_decode_error(
                port,
                "rpc.discover returned no result",
            ));
        },
        ResponseStatus::Error(error) => {
            return Err(Error::tool_call_failed_with_details(
                format!("rpc.discover failed on port {port}: {}", error.message),
                serde_json::json!({
                    "stage": "discovery",
                    "port": port,
                    "code": error.code,
                    "data": error.data,
                }),
            )
            .into());
        },
    };

    decode_method_names(value, port)
}

fn decode_method_names(value: Value, port: Port) -> Result<Vec<String>> {
    let document = serde_json::from_value::<OpenRpcDocument>(value)
        .map_err(|error| discovery_decode_error(port, error))?;
    let methods: Vec<String> = document
        .methods
        .into_iter()
        .map(|method| method.name)
        .collect();

    if methods.iter().any(String::is_empty) {
        return Err(discovery_decode_error(
            port,
            "rpc.discover returned an empty method name",
        ));
    }

    Ok(methods)
}

fn discovery_decode_error(port: Port, error: impl ToString) -> error_stack::Report<Error> {
    Error::tool_call_failed_with_details(
        format!("Unable to decode rpc.discover response from port {port}"),
        serde_json::json!({
            "stage": "discovery",
            "port": port,
            "error": error.to_string(),
        }),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Port;
    use super::decode_method_names;

    const TEST_PORT: Port = Port(15_702);

    #[test]
    fn decodes_application_method_names() -> Result<(), Box<dyn std::error::Error>> {
        let methods = decode_method_names(
            json!({
                "openrpc": "1.3.2",
                "info": {
                    "title": "Bevy Remote Protocol",
                    "version": "0.19.0"
                },
                "methods": [
                    {"name": "rpc.discover", "params": []},
                    {"name": "test/multiply", "params": []}
                ],
                "servers": null
            }),
            TEST_PORT,
        )?;

        assert_eq!(
            methods,
            vec![String::from("rpc.discover"), String::from("test/multiply")]
        );
        Ok(())
    }

    #[test]
    fn rejects_malformed_discovery_documents() {
        assert!(decode_method_names(json!({"methods": "invalid"}), TEST_PORT).is_err());
    }

    #[test]
    fn rejects_empty_method_names() {
        let result = decode_method_names(
            json!({
                "openrpc": "1.3.2",
                "info": {
                    "title": "Bevy Remote Protocol",
                    "version": "0.19.0"
                },
                "methods": [{"name": "", "params": []}],
                "servers": null
            }),
            TEST_PORT,
        );

        assert!(result.is_err());
    }
}
