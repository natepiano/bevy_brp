//! Curated application-method discovery through `brp_extras/agent_tools`.

use async_trait::async_trait;
use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use error_stack::Report;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::brp_tools::constants::AGENT_TOOL_CATALOG_METHOD;
use crate::brp_tools::constants::AGENT_TOOL_CATALOG_USAGE;
use crate::brp_tools::constants::AGENT_TOOL_CATALOG_VERSION;
use crate::error::Error;
use crate::error::Result;
use crate::tool::ToolFn;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListAgentToolsParams {
    /// The BRP port (default: 15702).
    #[serde(default)]
    pub port: Port,
}

/// One developer-published catalog record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListedAgentTool {
    /// Agent-facing tool name.
    pub name:          String,
    /// Exact backing BRP method for `brp_execute`.
    pub method:        String,
    /// Agent-facing description of the operation.
    pub description:   String,
    /// Raw JSON Schema for the backing method's JSON-RPC parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params_schema: Option<Value>,
    /// Raw JSON Schema for the backing method's JSON-RPC result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_schema: Option<Value>,
}

/// Serialized catalog payload returned as the structured result.
#[derive(Serialize)]
pub struct ListAgentToolsPayload {
    /// How to invoke a selected backing method through MCP.
    pub usage: String,
    /// Developer-published records in catalog order.
    pub tools: Vec<ListedAgentTool>,
}

/// Curated catalog records and the supported MCP invocation workflow.
#[derive(Serialize, ResultStruct)]
pub struct ListAgentToolsResult {
    /// Catalog payload returned to the caller.
    #[to_result]
    pub catalog:          ListAgentToolsPayload,
    /// Number of catalog records returned.
    #[to_metadata]
    pub tool_count:       usize,
    /// Message template for formatting responses.
    #[to_message(message_template = "Listed {tool_count} agent tools")]
    pub message_template: String,
}

/// MCP handler that fetches the current application-owned catalog.
pub struct BrpListAgentTools;

#[async_trait]
impl ToolFn for BrpListAgentTools {
    type Output = ListAgentToolsResult;
    type Params = ListAgentToolsParams;

    async fn handle_impl(&self, params: ListAgentToolsParams) -> Result<ListAgentToolsResult> {
        let client =
            BrpClient::for_application(AGENT_TOOL_CATALOG_METHOD.to_string(), params.port, None);
        let response = client.execute_raw().await.map_err(|error| {
            Error::tool_call_failed_with_details(
                format!(
                    "Unable to fetch the agent tool catalog from port {}",
                    params.port
                ),
                serde_json::json!({
                    "stage": "catalog_fetch",
                    "method": AGENT_TOOL_CATALOG_METHOD,
                    "port": params.port,
                    "error": error.current_context().to_string(),
                }),
            )
        })?;

        interpret_catalog_response(response, params.port)
    }
}

#[derive(Deserialize)]
struct AgentToolCatalogWire {
    version: u32,
    tools:   Vec<AgentToolWire>,
}

#[derive(Deserialize)]
struct AgentToolWire {
    name:          String,
    method:        String,
    description:   String,
    params_schema: Option<Value>,
    result_schema: Option<Value>,
}

impl From<AgentToolWire> for ListedAgentTool {
    fn from(tool: AgentToolWire) -> Self {
        Self {
            name:          tool.name,
            method:        tool.method,
            description:   tool.description,
            params_schema: tool.params_schema,
            result_schema: tool.result_schema,
        }
    }
}

fn interpret_catalog_response(
    response: ResponseStatus,
    port: Port,
) -> Result<ListAgentToolsResult> {
    let value = match response {
        ResponseStatus::Success(Some(value)) => value,
        ResponseStatus::Success(None) => {
            return Err(catalog_decode_error(
                port,
                "brp_extras/agent_tools returned no result",
            ));
        },
        ResponseStatus::Error(error) => {
            return Err(catalog_brp_error(
                port,
                error.code,
                error.message,
                error.data,
            ));
        },
    };

    let catalog = serde_json::from_value::<AgentToolCatalogWire>(value)
        .map_err(|error| catalog_decode_error(port, error))?;
    if catalog.version != AGENT_TOOL_CATALOG_VERSION {
        return Err(catalog_version_error(port, catalog.version));
    }

    let tools: Vec<_> = catalog
        .tools
        .into_iter()
        .map(ListedAgentTool::from)
        .collect();
    let tool_count = tools.len();
    Ok(ListAgentToolsResult::new(
        ListAgentToolsPayload {
            usage: AGENT_TOOL_CATALOG_USAGE.to_string(),
            tools,
        },
        tool_count,
    ))
}

fn catalog_decode_error(port: Port, error: impl ToString) -> Report<Error> {
    Error::tool_call_failed_with_details(
        format!("Unable to decode the agent tool catalog from port {port}"),
        serde_json::json!({
            "stage": "catalog_decode",
            "method": AGENT_TOOL_CATALOG_METHOD,
            "port": port,
            "error": error.to_string(),
        }),
    )
    .into()
}

fn catalog_version_error(port: Port, version: u32) -> Report<Error> {
    Error::tool_call_failed_with_details(
        format!("Unsupported agent tool catalog version {version} from port {port}"),
        serde_json::json!({
            "stage": "catalog_version",
            "method": AGENT_TOOL_CATALOG_METHOD,
            "port": port,
            "version": version,
            "supported_version": AGENT_TOOL_CATALOG_VERSION,
        }),
    )
    .into()
}

fn catalog_brp_error(port: Port, code: i32, message: String, data: Option<Value>) -> Report<Error> {
    Error::tool_call_failed_with_details(
        format!("Agent tool catalog request failed on port {port}: {message}"),
        serde_json::json!({
            "stage": "catalog_request",
            "method": AGENT_TOOL_CATALOG_METHOD,
            "port": port,
            "code": code,
            "data": data,
        }),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use rmcp::model::CallToolRequestParams;
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::json;

    use super::AGENT_TOOL_CATALOG_METHOD;
    use super::AGENT_TOOL_CATALOG_USAGE;
    use super::AGENT_TOOL_CATALOG_VERSION;
    use super::BrpListAgentTools;
    use super::ListAgentToolsParams;
    use super::ListAgentToolsResult;
    use super::ListedAgentTool;
    use super::interpret_catalog_response;
    use crate::brp_tools;
    use crate::brp_tools::JSON_RPC_ERROR_METHOD_NOT_FOUND;
    use crate::brp_tools::Port;
    use crate::brp_tools::ResponseStatus;
    use crate::error::Error;
    use crate::error::Result;
    use crate::tool::FieldPlacement;
    use crate::tool::HasFieldPlacement;
    use crate::tool::ToolFn;
    use crate::tool::ToolName;

    const INTERNAL_ERROR: i32 = -32_603;
    const TEST_DESCRIPTION_ALPHA: &str = "First catalog record";
    const TEST_DESCRIPTION_BETA: &str = "Second catalog record";
    const TEST_METHOD_ALPHA: &str = "test/alpha";
    const TEST_METHOD_BETA: &str = "test/beta";
    const TEST_NAME_ALPHA: &str = "test_alpha";
    const TEST_NAME_BETA: &str = "test_beta";
    const TEST_PORT: Port = Port(21_234);

    struct StaticListAgentTools;

    #[async_trait]
    impl ToolFn for StaticListAgentTools {
        type Output = ListAgentToolsResult;
        type Params = ListAgentToolsParams;

        async fn handle_impl(&self, _: ListAgentToolsParams) -> Result<ListAgentToolsResult> {
            interpret_catalog_response(populated_response(), TEST_PORT)
        }
    }

    fn success_response(value: Value) -> ResponseStatus { ResponseStatus::Success(Some(value)) }

    fn populated_response() -> ResponseStatus {
        success_response(json!({
            "version": AGENT_TOOL_CATALOG_VERSION,
            "tools": [
                {
                    "name": TEST_NAME_BETA,
                    "method": TEST_METHOD_BETA,
                    "description": TEST_DESCRIPTION_BETA,
                    "params_schema": ["array", 7, true],
                    "result_schema": "primitive"
                },
                {
                    "name": TEST_NAME_ALPHA,
                    "method": TEST_METHOD_ALPHA,
                    "description": TEST_DESCRIPTION_ALPHA,
                    "params_schema": {"type": "object"},
                    "result_schema": false
                }
            ]
        }))
    }

    fn response_status(value: Value) -> serde_json::Result<ResponseStatus> {
        serde_json::from_value(value)
    }

    fn assert_catalog_brp_error(
        response: ResponseStatus,
        expected_code: i32,
        expected_data: Option<Value>,
    ) {
        let result = interpret_catalog_response(response, TEST_PORT);
        assert!(result.is_err());
        if let Err(report) = result {
            assert!(matches!(report.current_context(), Error::ToolCall { .. }));
            let Error::ToolCall { message, details } = report.current_context() else {
                return;
            };
            assert!(message.contains(&TEST_PORT.to_string()));
            assert_eq!(
                details.as_ref().and_then(|value| value.get("code")),
                Some(&json!(expected_code))
            );
            assert_eq!(
                details.as_ref().and_then(|value| value.get("data")),
                Some(&expected_data.unwrap_or(Value::Null))
            );
        }
    }

    #[test]
    fn params_default_and_accept_an_explicit_port() -> serde_json::Result<()> {
        let defaults = serde_json::from_value::<ListAgentToolsParams>(json!({}))?;
        let explicit = serde_json::from_value::<ListAgentToolsParams>(json!({
            "port": TEST_PORT.0
        }))?;

        assert_eq!(defaults.port, Port::default());
        assert_eq!(explicit.port, TEST_PORT);
        Ok(())
    }

    #[test]
    fn required_wire_fields_cannot_be_omitted() {
        for document in [
            json!({"tools": []}),
            json!({"version": AGENT_TOOL_CATALOG_VERSION}),
            json!({"version": AGENT_TOOL_CATALOG_VERSION, "tools": [{
                "method": TEST_METHOD_ALPHA,
                "description": TEST_DESCRIPTION_ALPHA
            }]}),
            json!({"version": AGENT_TOOL_CATALOG_VERSION, "tools": [{
                "name": TEST_NAME_ALPHA,
                "description": TEST_DESCRIPTION_ALPHA
            }]}),
            json!({"version": AGENT_TOOL_CATALOG_VERSION, "tools": [{
                "name": TEST_NAME_ALPHA,
                "method": TEST_METHOD_ALPHA
            }]}),
        ] {
            assert!(interpret_catalog_response(success_response(document), TEST_PORT).is_err());
        }
    }

    #[test]
    fn empty_catalog_is_valid_and_includes_usage() -> Result<()> {
        let result = interpret_catalog_response(
            success_response(json!({
                "version": AGENT_TOOL_CATALOG_VERSION,
                "tools": []
            })),
            TEST_PORT,
        )?;

        assert!(result.catalog.tools.is_empty());
        assert_eq!(result.catalog.usage, AGENT_TOOL_CATALOG_USAGE);
        assert_eq!(result.tool_count, 0);
        Ok(())
    }

    #[test]
    fn populated_catalog_preserves_order_fields_and_raw_schema_values() -> Result<()> {
        let result = interpret_catalog_response(populated_response(), TEST_PORT)?;

        assert_eq!(
            result.catalog.tools,
            vec![
                ListedAgentTool {
                    name:          TEST_NAME_BETA.to_string(),
                    method:        TEST_METHOD_BETA.to_string(),
                    description:   TEST_DESCRIPTION_BETA.to_string(),
                    params_schema: Some(json!(["array", 7, true])),
                    result_schema: Some(json!("primitive")),
                },
                ListedAgentTool {
                    name:          TEST_NAME_ALPHA.to_string(),
                    method:        TEST_METHOD_ALPHA.to_string(),
                    description:   TEST_DESCRIPTION_ALPHA.to_string(),
                    params_schema: Some(json!({"type": "object"})),
                    result_schema: Some(json!(false)),
                },
            ]
        );
        assert_eq!(result.tool_count, 2);
        Ok(())
    }

    #[test]
    fn omitted_optional_schemas_remain_omitted()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let result = interpret_catalog_response(
            success_response(json!({
                "version": AGENT_TOOL_CATALOG_VERSION,
                "tools": [{
                    "name": TEST_NAME_ALPHA,
                    "method": TEST_METHOD_ALPHA,
                    "description": TEST_DESCRIPTION_ALPHA
                }]
            })),
            TEST_PORT,
        )?;
        let serialized = serde_json::to_value(&result.catalog.tools[0])?;

        assert_eq!(result.catalog.tools[0].params_schema, None);
        assert_eq!(result.catalog.tools[0].result_schema, None);
        assert!(serialized.get("params_schema").is_none());
        assert!(serialized.get("result_schema").is_none());
        Ok(())
    }

    #[test]
    fn unsupported_versions_and_malformed_responses_identify_stage_and_port() {
        let cases = [
            (
                success_response(json!({"version": 2, "tools": []})),
                "catalog_version",
            ),
            (
                success_response(json!({"version": "one", "tools": []})),
                "catalog_decode",
            ),
            (ResponseStatus::Success(None), "catalog_decode"),
        ];

        for (response, expected_stage) in cases {
            let result = interpret_catalog_response(response, TEST_PORT);
            assert!(result.is_err());
            if let Err(report) = result {
                assert!(matches!(report.current_context(), Error::ToolCall { .. }));
                let Error::ToolCall { message, details } = report.current_context() else {
                    continue;
                };
                assert!(message.contains(&TEST_PORT.to_string()));
                assert_eq!(
                    details.as_ref().and_then(|value| value.get("stage")),
                    Some(&json!(expected_stage))
                );
                assert_eq!(
                    details.as_ref().and_then(|value| value.get("port")),
                    Some(&json!(TEST_PORT))
                );
            }
        }
    }

    #[test]
    fn method_not_found_preserves_code_and_plugin_guidance() -> serde_json::Result<()> {
        let message =
            brp_tools::method_not_found_message(AGENT_TOOL_CATALOG_METHOD, "Method not found");
        let response = response_status(json!({
            "Error": {
                "code": JSON_RPC_ERROR_METHOD_NOT_FOUND,
                "message": message,
                "data": null
            }
        }))?;

        let result = interpret_catalog_response(response, TEST_PORT);
        assert!(result.is_err());
        if let Err(report) = result {
            assert!(matches!(report.current_context(), Error::ToolCall { .. }));
            let Error::ToolCall { message, details } = report.current_context() else {
                return Ok(());
            };
            assert!(message.contains("BrpExtrasPlugin"));
            assert_eq!(
                details.as_ref().and_then(|value| value.get("code")),
                Some(&json!(JSON_RPC_ERROR_METHOD_NOT_FOUND))
            );
        }
        Ok(())
    }

    #[test]
    fn missing_backing_method_error_preserves_exact_data() -> serde_json::Result<()> {
        let data = json!({
            "name": TEST_NAME_ALPHA,
            "method": TEST_METHOD_ALPHA,
            "reason": "backing_method_missing"
        });
        let response = response_status(json!({
            "Error": {
                "code": INTERNAL_ERROR,
                "message": "backing method is not registered",
                "data": data
            }
        }))?;

        assert_catalog_brp_error(response, INTERNAL_ERROR, Some(data));
        Ok(())
    }

    #[test]
    fn watching_backing_method_error_preserves_exact_data() -> serde_json::Result<()> {
        let data = json!({
            "name": TEST_NAME_BETA,
            "method": TEST_METHOD_BETA,
            "reason": "backing_method_watching"
        });
        let response = response_status(json!({
            "Error": {
                "code": INTERNAL_ERROR,
                "message": "backing method is registered as watching",
                "data": data
            }
        }))?;

        assert_catalog_brp_error(response, INTERNAL_ERROR, Some(data));
        Ok(())
    }

    #[test]
    fn catalog_is_the_only_result_field() {
        let placements = ListAgentToolsResult::field_placements();
        let result_placements: Vec<_> = placements
            .iter()
            .filter(|field| matches!(field.placement, FieldPlacement::Result))
            .collect();

        assert_eq!(result_placements.len(), 1);
        assert_eq!(result_placements[0].field_name, "catalog");
    }

    #[tokio::test]
    async fn structured_result_contains_exact_usage_and_catalog_records() {
        let mut definition = crate::tool::get_all_tool_definitions()
            .into_iter()
            .find(|definition| definition.tool_name == ToolName::BrpListAgentTools);
        assert!(definition.is_some());
        if let Some(definition) = definition.as_mut() {
            definition.handler = Arc::new(StaticListAgentTools);
            let response = definition
                .call_tool(
                    CallToolRequestParams::new("brp_list_agent_tools").with_arguments(Map::new()),
                )
                .await;
            assert!(response.is_ok());
            if let Ok(response) = response {
                let result = response
                    .structured_content
                    .as_ref()
                    .and_then(|content| content.get("result"));
                assert_eq!(
                    result,
                    Some(&json!({
                        "usage": AGENT_TOOL_CATALOG_USAGE,
                        "tools": [
                            {
                                "name": TEST_NAME_BETA,
                                "method": TEST_METHOD_BETA,
                                "description": TEST_DESCRIPTION_BETA,
                                "params_schema": ["array", 7, true],
                                "result_schema": "primitive"
                            },
                            {
                                "name": TEST_NAME_ALPHA,
                                "method": TEST_METHOD_ALPHA,
                                "description": TEST_DESCRIPTION_ALPHA,
                                "params_schema": {"type": "object"},
                                "result_schema": false
                            }
                        ]
                    }))
                );
            }
        }
    }

    #[test]
    fn handler_type_remains_the_fixed_local_tool() {
        let handler = BrpListAgentTools;
        assert_eq!(
            std::any::type_name_of_val(&handler),
            std::any::type_name::<BrpListAgentTools>()
        );
    }
}
