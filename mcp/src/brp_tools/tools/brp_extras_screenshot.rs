//! `brp_extras/screenshot` MCP composite.

use async_trait::async_trait;
use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use error_stack::Report;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::world_find_entities_by_name;
use super::world_find_entities_by_name::NameMatchMode;
use super::world_find_entities_by_name::NamedEntity;
use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::error::Error;
use crate::error::Result;
use crate::tool;
use crate::tool::BrpMethod;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

/// Parameters for the terminal `brp_extras/screenshot` tool.
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ScreenshotParams {
    /// Canonical Bevy entity ID to capture.
    pub entity:  Option<u64>,
    /// Unique, case-sensitive exact Bevy `Name` to resolve before capture.
    pub name:    Option<String>,
    /// Camera entity ID. Captures its viewport, or selects it for an entity crop.
    pub camera:  Option<u64>,
    /// Physical pixels to add around an entity crop. Defaults to zero.
    pub padding: Option<u32>,
    /// File path where the complete PNG should be published.
    pub path:    String,
    /// The BRP port (default: 15702).
    #[serde(default)]
    pub port:    Port,
}

/// Result returned after the complete PNG has been published.
#[derive(Serialize, ResultStruct)]
pub struct ScreenshotResult {
    /// The terminal BRP response containing the final PNG metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result:           Option<Value>,
    /// Canonical entity ID used for an entity or name capture.
    #[to_metadata(skip_if_none)]
    pub entity:           Option<u64>,
    /// Exact Bevy `Name` used to resolve the entity ID.
    #[to_metadata(skip_if_none)]
    pub name:             Option<String>,
    /// Message template for formatting responses.
    #[to_message(message_template = "Screenshot saved to {path}")]
    pub message_template: String,
}

/// Local MCP handler that optionally resolves a name before calling extras.
pub struct BrpExtrasScreenshot;

#[async_trait]
impl ToolFn for BrpExtrasScreenshot {
    type Output = ScreenshotResult;
    type Params = ScreenshotParams;

    fn call(
        &self,
        context: HandlerContext,
    ) -> HandlerResult<'_, ToolResult<Self::Output, Self::Params>> {
        tool::call_with_typed_params(context, |_, params: ScreenshotParams| async move {
            take_screenshot(params).await
        })
    }

    async fn handle_impl(&self, params: ScreenshotParams) -> Result<ScreenshotResult> {
        take_screenshot(params).await
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ScreenshotScope {
    Full {
        camera: Option<u64>,
    },
    Entity {
        entity:  u64,
        camera:  Option<u64>,
        padding: u32,
    },
    ExactName {
        name:    String,
        camera:  Option<u64>,
        padding: u32,
    },
}

struct ScreenshotRequest {
    path:  String,
    port:  Port,
    scope: ScreenshotScope,
}

impl TryFrom<ScreenshotParams> for ScreenshotRequest {
    type Error = Report<Error>;

    fn try_from(params: ScreenshotParams) -> Result<Self> {
        let ScreenshotParams {
            entity,
            name,
            camera,
            padding,
            path,
            port,
        } = params;

        let scope = match (entity, name) {
            (Some(_), Some(_)) => {
                return Err(selector_error(
                    "`entity` and `name` are mutually exclusive; provide only one screenshot selector",
                ));
            },
            (Some(entity), None) => ScreenshotScope::Entity {
                entity,
                camera,
                padding: padding.unwrap_or_default(),
            },
            (None, Some(name)) => ScreenshotScope::ExactName {
                name,
                camera,
                padding: padding.unwrap_or_default(),
            },
            (None, None) if padding.is_some() => {
                return Err(selector_error(
                    "`padding` requires an `entity` or `name` screenshot selector",
                ));
            },
            (None, None) => ScreenshotScope::Full { camera },
        };

        Ok(Self { path, port, scope })
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ResolvedScope {
    Full {
        camera: Option<u64>,
    },
    Entity {
        entity:  u64,
        name:    Option<String>,
        camera:  Option<u64>,
        padding: u32,
    },
}

impl ResolvedScope {
    fn extras_params(&self, path: String) -> Result<Value> {
        let (entity, camera, padding) = match self {
            Self::Full { camera } => (None, *camera, None),
            Self::Entity {
                entity,
                camera,
                padding,
                ..
            } => (Some(*entity), *camera, Some(*padding)),
        };
        let params = ExtrasScreenshotParams {
            camera,
            entity,
            padding,
            path,
        };

        serde_json::to_value(params).map_err(|error| {
            Error::InvalidState(format!(
                "Failed to serialize screenshot parameters: {error}"
            ))
            .into()
        })
    }

    fn metadata(self) -> (Option<u64>, Option<String>) {
        match self {
            Self::Full { .. } => (None, None),
            Self::Entity { entity, name, .. } => (Some(entity), name),
        }
    }
}

#[derive(Serialize)]
struct ExtrasScreenshotParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    camera:  Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity:  Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<u32>,
    path:    String,
}

async fn take_screenshot(params: ScreenshotParams) -> Result<ScreenshotResult> {
    let request = ScreenshotRequest::try_from(params)?;
    let ScreenshotRequest { path, port, scope } = request;
    let resolved_scope = resolve_scope(scope, port).await?;
    let extras_params = resolved_scope.extras_params(path)?;
    let client = BrpClient::new(BrpMethod::BrpExtrasScreenshot, port, Some(extras_params));
    let response = client.execute_raw().await?;

    screenshot_result(response, resolved_scope, port)
}

async fn resolve_scope(scope: ScreenshotScope, port: Port) -> Result<ResolvedScope> {
    match scope {
        ScreenshotScope::Full { camera } => Ok(ResolvedScope::Full { camera }),
        ScreenshotScope::Entity {
            entity,
            camera,
            padding,
        } => Ok(ResolvedScope::Entity {
            entity,
            name: None,
            camera,
            padding,
        }),
        ScreenshotScope::ExactName {
            name,
            camera,
            padding,
        } => {
            let entities = world_find_entities_by_name::find_entities_by_name(
                &name,
                NameMatchMode::Exact,
                port,
            )
            .await?;
            resolve_exact_name(name, camera, padding, entities, port)
        },
    }
}

fn resolve_exact_name(
    requested_name: String,
    camera: Option<u64>,
    padding: u32,
    mut entities: Vec<NamedEntity>,
    port: Port,
) -> Result<ResolvedScope> {
    match entities.len() {
        0 => Err(Error::tool_call_failed(format!(
            "No entity named `{requested_name}` was found on port {port}. Use `world_find_entities_by_name` to discover candidates or retry with `entity`"
        ))
        .into()),
        1 => {
            let named_entity = entities.pop().ok_or_else(|| {
                Error::InvalidState(
                    "A unique screenshot name match disappeared during resolution".to_string(),
                )
            })?;
            Ok(ResolvedScope::Entity {
                entity: named_entity.entity,
                name: Some(named_entity.name),
                camera,
                padding,
            })
        },
        _ => {
            let matching_ids = entities
                .iter()
                .map(|named_entity| named_entity.entity)
                .collect::<Vec<_>>();
            Err(Error::tool_call_failed(format!(
                "Name `{requested_name}` matched multiple entity IDs {matching_ids:?} on port {port}. Retry with `entity` or use `world_find_entities_by_name` for generic discovery"
            ))
            .into())
        },
    }
}

fn screenshot_result(
    response: ResponseStatus,
    resolved_scope: ResolvedScope,
    port: Port,
) -> Result<ScreenshotResult> {
    match response {
        ResponseStatus::Success(result) => {
            let (entity, name) = resolved_scope.metadata();
            Ok(ScreenshotResult::new(result, entity, name))
        },
        ResponseStatus::Error(error) => Err(screenshot_brp_error(
            error.code,
            error.message,
            error.data,
            port,
        )),
    }
}

fn screenshot_brp_error(
    code: i32,
    message: String,
    data: Option<Value>,
    port: Port,
) -> Report<Error> {
    Error::tool_call_failed_with_details(
        format!(
            "{} failed on port {port}: {message}",
            BrpMethod::BrpExtrasScreenshot.as_str(),
        ),
        serde_json::json!({
            "method": BrpMethod::BrpExtrasScreenshot.as_str(),
            "port": port,
            "code": code,
            "data": data,
        }),
    )
    .into()
}

fn selector_error(message: impl Into<String>) -> Report<Error> {
    Error::tool_call_failed(message).into()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::tool::ToolName;

    const TEST_BRP_ERROR_CODE: i32 = -32_602;
    const TEST_CAMERA: u64 = 11;
    const TEST_ENTITY_HIGH: u64 = 42;
    const TEST_ENTITY_LOW: u64 = 7;
    const TEST_ERROR_MESSAGE: &str = "capture failed";
    const TEST_HEIGHT: u64 = 180;
    const TEST_NAME: &str = "NatesList";
    const TEST_PADDING: u32 = 1;
    const TEST_PATH: &str = "/tmp/nates-list.png";
    const TEST_PORT: Port = Port(15_702);
    const TEST_STATUS_COMPLETED: &str = "completed";
    const TEST_STATUS_FAILED: &str = "failed";
    const TEST_WIDTH: u64 = 320;

    fn params() -> ScreenshotParams {
        ScreenshotParams {
            entity:  None,
            name:    None,
            camera:  None,
            padding: None,
            path:    TEST_PATH.to_string(),
            port:    TEST_PORT,
        }
    }

    #[tokio::test]
    async fn request_modes_convert_to_typed_scopes_and_extras_payloads()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let full = ScreenshotRequest::try_from(params())?;
        assert_eq!(full.scope, ScreenshotScope::Full { camera: None });
        let full_extras =
            ResolvedScope::Full { camera: None }.extras_params(TEST_PATH.to_string())?;
        assert_eq!(
            full_extras,
            json!({
                "path": TEST_PATH,
            })
        );

        let camera = ScreenshotRequest::try_from(ScreenshotParams {
            camera: Some(TEST_CAMERA),
            ..params()
        })?;
        assert_eq!(
            camera.scope,
            ScreenshotScope::Full {
                camera: Some(TEST_CAMERA),
            }
        );
        let camera_extras = resolve_scope(camera.scope, TEST_PORT)
            .await?
            .extras_params(TEST_PATH.to_string())?;
        assert_eq!(
            camera_extras,
            json!({
                "camera": TEST_CAMERA,
                "path": TEST_PATH,
            })
        );

        let entity = ScreenshotRequest::try_from(ScreenshotParams {
            entity: Some(TEST_ENTITY_LOW),
            ..params()
        })?;
        assert_eq!(
            entity.scope,
            ScreenshotScope::Entity {
                entity:  TEST_ENTITY_LOW,
                camera:  None,
                padding: u32::default(),
            }
        );
        let entity_extras = resolve_scope(entity.scope, TEST_PORT)
            .await?
            .extras_params(TEST_PATH.to_string())?;
        assert_eq!(
            entity_extras,
            json!({
                "entity": TEST_ENTITY_LOW,
                "padding": u32::default(),
                "path": TEST_PATH,
            })
        );

        let camera_entity = ScreenshotRequest::try_from(ScreenshotParams {
            entity: Some(TEST_ENTITY_LOW),
            camera: Some(TEST_CAMERA),
            ..params()
        })?;
        assert_eq!(
            camera_entity.scope,
            ScreenshotScope::Entity {
                entity:  TEST_ENTITY_LOW,
                camera:  Some(TEST_CAMERA),
                padding: u32::default(),
            }
        );
        let camera_entity_extras = resolve_scope(camera_entity.scope, TEST_PORT)
            .await?
            .extras_params(TEST_PATH.to_string())?;
        assert_eq!(
            camera_entity_extras,
            json!({
                "camera": TEST_CAMERA,
                "entity": TEST_ENTITY_LOW,
                "padding": u32::default(),
                "path": TEST_PATH,
            })
        );
        assert!(camera_entity_extras.get("name").is_none());
        Ok(())
    }

    #[test]
    fn public_params_wire_and_schema_include_screenshot_selectors()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let public_wire = serde_json::to_value(ScreenshotParams {
            entity:  Some(TEST_ENTITY_LOW),
            name:    None,
            camera:  Some(TEST_CAMERA),
            padding: Some(TEST_PADDING),
            path:    TEST_PATH.to_string(),
            port:    TEST_PORT,
        })?;
        assert_eq!(
            public_wire,
            json!({
                "camera": TEST_CAMERA,
                "entity": TEST_ENTITY_LOW,
                "name": null,
                "padding": TEST_PADDING,
                "path": TEST_PATH,
                "port": *TEST_PORT,
            })
        );
        let public_schema = serde_json::to_value(schemars::schema_for!(ScreenshotParams))?;
        assert!(public_schema.pointer("/properties/entity").is_some());
        assert!(public_schema.pointer("/properties/name").is_some());
        assert!(public_schema.pointer("/properties/camera").is_some());
        assert!(public_schema.pointer("/properties/padding").is_some());
        assert!(public_schema.pointer("/properties/path").is_some());
        assert!(public_schema.pointer("/properties/port").is_some());
        Ok(())
    }

    #[test]
    fn exact_name_defaults_to_zero_padding() -> core::result::Result<(), Box<dyn std::error::Error>>
    {
        let request = ScreenshotRequest::try_from(ScreenshotParams {
            name: Some(TEST_NAME.to_string()),
            ..params()
        })?;

        assert_eq!(
            request.scope,
            ScreenshotScope::ExactName {
                name:    TEST_NAME.to_string(),
                camera:  None,
                padding: u32::default(),
            }
        );
        Ok(())
    }

    #[test]
    fn unique_exact_name_resolves_to_an_extras_entity_request()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let resolved_scope = resolve_exact_name(
            TEST_NAME.to_string(),
            Some(TEST_CAMERA),
            u32::default(),
            vec![NamedEntity {
                entity: TEST_ENTITY_LOW,
                name:   TEST_NAME.to_string(),
            }],
            TEST_PORT,
        )?;
        let extras_params = resolved_scope.extras_params(TEST_PATH.to_string())?;

        assert_eq!(
            extras_params,
            json!({
                "camera": TEST_CAMERA,
                "entity": TEST_ENTITY_LOW,
                "padding": u32::default(),
                "path": TEST_PATH,
            })
        );
        assert!(extras_params.get("name").is_none());
        Ok(())
    }

    #[test]
    fn no_exact_name_match_returns_actionable_error() {
        let result = resolve_exact_name(
            TEST_NAME.to_string(),
            None,
            u32::default(),
            Vec::new(),
            TEST_PORT,
        );

        assert!(matches!(
            result.as_ref().map_err(error_stack::Report::current_context),
            Err(Error::ToolCall { message, .. })
                if message.contains(TEST_NAME)
                    && message.contains("world_find_entities_by_name")
                    && message.contains("entity")
        ));
    }

    #[test]
    fn duplicate_exact_name_error_preserves_sorted_ids() {
        let result = resolve_exact_name(
            TEST_NAME.to_string(),
            None,
            u32::default(),
            vec![
                NamedEntity {
                    entity: TEST_ENTITY_LOW,
                    name:   TEST_NAME.to_string(),
                },
                NamedEntity {
                    entity: TEST_ENTITY_HIGH,
                    name:   TEST_NAME.to_string(),
                },
            ],
            TEST_PORT,
        );

        assert!(matches!(
            result.as_ref().map_err(error_stack::Report::current_context),
            Err(Error::ToolCall { message, .. })
                if message.contains(&format!("[{TEST_ENTITY_LOW}, {TEST_ENTITY_HIGH}]"))
                    && message.contains("Retry with `entity`")
                    && message.contains("world_find_entities_by_name")
        ));
    }

    #[test]
    fn entity_and_name_are_mutually_exclusive() {
        let result = ScreenshotRequest::try_from(ScreenshotParams {
            entity: Some(TEST_ENTITY_LOW),
            name: Some(TEST_NAME.to_string()),
            ..params()
        });

        assert!(matches!(
            result.as_ref().map_err(error_stack::Report::current_context),
            Err(Error::ToolCall { message, .. }) if message.contains("mutually exclusive")
        ));
    }

    #[test]
    fn padding_is_invalid_without_an_entity_or_name() {
        let padding = ScreenshotRequest::try_from(ScreenshotParams {
            padding: Some(TEST_PADDING),
            ..params()
        });

        assert!(matches!(
            padding.as_ref().map_err(error_stack::Report::current_context),
            Err(Error::ToolCall { message, .. }) if message.contains("`padding` requires")
        ));
    }

    #[test]
    fn terminal_extras_result_is_preserved_with_resolved_metadata()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let terminal_result = json!({
            "status": TEST_STATUS_COMPLETED,
            "path": TEST_PATH,
            "width": TEST_WIDTH,
            "height": TEST_HEIGHT,
        });
        let screenshot_result = screenshot_result(
            ResponseStatus::Success(Some(terminal_result.clone())),
            ResolvedScope::Entity {
                entity:  TEST_ENTITY_LOW,
                name:    Some(TEST_NAME.to_string()),
                camera:  None,
                padding: u32::default(),
            },
            TEST_PORT,
        )?;

        assert_eq!(screenshot_result.result, Some(terminal_result));
        assert_eq!(screenshot_result.entity, Some(TEST_ENTITY_LOW));
        assert_eq!(screenshot_result.name.as_deref(), Some(TEST_NAME));
        Ok(())
    }

    #[test]
    fn terminal_extras_error_preserves_code_and_data() {
        let result = Err::<ScreenshotResult, _>(screenshot_brp_error(
            TEST_BRP_ERROR_CODE,
            TEST_ERROR_MESSAGE.to_string(),
            Some(json!({"status": TEST_STATUS_FAILED})),
            TEST_PORT,
        ));

        assert!(matches!(
            result.as_ref().map_err(error_stack::Report::current_context),
            Err(Error::ToolCall {
                details: Some(Value::Object(details)),
                ..
            }) if details.get("code") == Some(&json!(TEST_BRP_ERROR_CODE))
                && details.get("data") == Some(&json!({"status": TEST_STATUS_FAILED}))
        ));
    }

    #[test]
    fn registry_contains_one_screenshot_tool_and_no_entity_variant() {
        let screenshot_tools = tool::get_all_tool_definitions()
            .into_iter()
            .filter(|definition| definition.tool_name.to_string().contains("screenshot"))
            .map(|definition| definition.tool_name.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            screenshot_tools,
            vec![ToolName::BrpExtrasScreenshot.to_string()]
        );
        assert!(
            screenshot_tools
                .iter()
                .all(|tool_name| !tool_name.contains("screenshot_entity"))
        );
    }
}
