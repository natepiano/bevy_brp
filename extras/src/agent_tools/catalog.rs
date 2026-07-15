use bevy::prelude::*;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::RemoteMethodSystemId;
use bevy_remote::RemoteMethods;
use bevy_remote::error_codes::INTERNAL_ERROR;
use schemars::Schema;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use super::AgentTool;
use super::RegisteredAgentTools;
use crate::constants::AGENT_TOOLS_CATALOG_VERSION;
use crate::constants::BACKING_METHOD_MISSING_REASON;
use crate::constants::BACKING_METHOD_WATCHING_REASON;

#[derive(Serialize)]
struct CatalogAgentTool<'a> {
    name:          &'a str,
    method:        &'a str,
    description:   &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params_schema: Option<&'a Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_schema: Option<&'a Schema>,
}

#[derive(Serialize)]
struct AgentToolCatalog<'a> {
    version: u32,
    tools:   Vec<CatalogAgentTool<'a>>,
}

impl<'a> From<&'a AgentTool> for CatalogAgentTool<'a> {
    fn from(agent_tool: &'a AgentTool) -> Self {
        Self {
            name:          &agent_tool.name,
            method:        &agent_tool.method,
            description:   &agent_tool.description,
            params_schema: agent_tool.params_schema.as_ref(),
            result_schema: agent_tool.result_schema.as_ref(),
        }
    }
}

/// Returns the current catalog after validating every backing method.
pub(crate) fn handler(
    In(_): In<Option<Value>>,
    registered_agent_tools: Res<RegisteredAgentTools>,
    remote_methods: Res<RemoteMethods>,
) -> BrpResult {
    let mut tools: Vec<_> = registered_agent_tools
        .0
        .iter()
        .map(CatalogAgentTool::from)
        .collect();
    tools.sort_unstable_by(|left, right| left.name.cmp(right.name));

    tools
        .iter()
        .try_for_each(|tool| validate_backing_method(tool, &remote_methods))?;

    serde_json::to_value(AgentToolCatalog {
        version: AGENT_TOOLS_CATALOG_VERSION,
        tools,
    })
    .map_err(|error| BrpError::internal(format!("failed to serialize agent tool catalog: {error}")))
}

fn validate_backing_method(
    tool: &CatalogAgentTool<'_>,
    remote_methods: &RemoteMethods,
) -> BrpResult<()> {
    match remote_methods.get(tool.method) {
        Some(RemoteMethodSystemId::Instant(_)) => Ok(()),
        Some(RemoteMethodSystemId::Watching(_)) => Err(backing_method_error(
            tool,
            BACKING_METHOD_WATCHING_REASON,
            "is registered as a watching method",
        )),
        None => Err(backing_method_error(
            tool,
            BACKING_METHOD_MISSING_REASON,
            "is not registered",
        )),
    }
}

fn backing_method_error(tool: &CatalogAgentTool<'_>, reason: &str, detail: &str) -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: format!(
            "agent tool `{}` cannot be listed: backing BRP method `{}` {detail}",
            tool.name, tool.method,
        ),
        data:    Some(json!({
            "name": tool.name,
            "method": tool.method,
            "reason": reason,
        })),
    }
}

#[cfg(test)]
mod tests {
    use bevy_remote::RemoteMethodSystemId;
    use bevy_remote::RemoteMethods;
    use schemars::json_schema;
    use serde_json::json;

    use super::*;
    use crate::AppAgentToolExt;
    use crate::BrpExtrasPlugin;
    use crate::constants::EXTRAS_COMMAND_PREFIX;
    use crate::constants::METHOD_AGENT_TOOLS;

    const DESCRIPTION: &str = "Runs a catalog test operation.";
    const INSTANT_METHOD: &str = "test/instant";
    const MISSING_METHOD: &str = "test/missing";
    const WATCHING_METHOD: &str = "test/watching";

    #[test]
    fn empty_catalog_has_exact_version_and_fields() -> Result<(), BrpError> {
        let mut app = catalog_app();

        assert_eq!(
            call_catalog(&mut app)?,
            json!({
                "version": AGENT_TOOLS_CATALOG_VERSION,
                "tools": [],
            }),
        );
        Ok(())
    }

    #[test]
    fn populated_catalog_is_sorted_and_preserves_raw_schemas() -> Result<(), BrpError> {
        let mut app = catalog_app();
        register_instant_method(&mut app, INSTANT_METHOD);
        app.register_agent_tool(AgentTool::new(
            "zeta",
            INSTANT_METHOD,
            "Omits both optional schemas.",
        ));
        app.register_agent_tool(
            AgentTool::new("alpha", INSTANT_METHOD, "Uses object and array schemas.")
                .params_schema(json_schema!({
                    "type": "object",
                    "properties": { "value": { "type": "integer" } },
                    "required": ["value"]
                }))
                .result_schema(json_schema!({
                    "type": "array",
                    "items": { "type": "string" }
                })),
        );
        app.register_agent_tool(
            AgentTool::new(
                "beta",
                INSTANT_METHOD,
                "Uses permissive and rejecting boolean schemas.",
            )
            .params_schema(json_schema!(true))
            .result_schema(json_schema!(false)),
        );
        app.register_agent_tool(
            AgentTool::new(
                "middle",
                INSTANT_METHOD,
                "Uses primitive and boolean schemas.",
            )
            .params_schema(json_schema!({ "type": "integer" }))
            .result_schema(json_schema!({ "type": "boolean" })),
        );

        assert_eq!(
            call_catalog(&mut app)?,
            json!({
                "version": AGENT_TOOLS_CATALOG_VERSION,
                "tools": [
                    {
                        "name": "alpha",
                        "method": INSTANT_METHOD,
                        "description": "Uses object and array schemas.",
                        "params_schema": {
                            "type": "object",
                            "properties": { "value": { "type": "integer" } },
                            "required": ["value"]
                        },
                        "result_schema": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    {
                        "name": "beta",
                        "method": INSTANT_METHOD,
                        "description": "Uses permissive and rejecting boolean schemas.",
                        "params_schema": true,
                        "result_schema": false
                    },
                    {
                        "name": "middle",
                        "method": INSTANT_METHOD,
                        "description": "Uses primitive and boolean schemas.",
                        "params_schema": { "type": "integer" },
                        "result_schema": { "type": "boolean" }
                    },
                    {
                        "name": "zeta",
                        "method": INSTANT_METHOD,
                        "description": "Omits both optional schemas."
                    }
                ]
            }),
        );
        Ok(())
    }

    #[test]
    fn registration_before_plugin_addition_is_visible_to_catalog() -> Result<(), BrpError> {
        let mut app = App::new();
        app.register_agent_tool(AgentTool::new("test.before", INSTANT_METHOD, DESCRIPTION));
        app.add_plugins(BrpExtrasPlugin);
        register_instant_method(&mut app, INSTANT_METHOD);

        assert_eq!(call_catalog(&mut app)?["tools"][0]["name"], "test.before");
        Ok(())
    }

    #[test]
    fn registration_after_plugin_addition_is_visible_to_catalog() -> Result<(), BrpError> {
        let mut app = catalog_app();
        register_instant_method(&mut app, INSTANT_METHOD);
        app.register_agent_tool(AgentTool::new("test.after", INSTANT_METHOD, DESCRIPTION));

        assert_eq!(call_catalog(&mut app)?["tools"][0]["name"], "test.after");
        Ok(())
    }

    #[test]
    fn missing_backing_method_returns_exact_internal_error() {
        let mut app = catalog_app();
        app.register_agent_tool(AgentTool::new("test.missing", MISSING_METHOD, DESCRIPTION));

        assert_eq!(
            call_catalog(&mut app),
            Err(BrpError {
                code:    INTERNAL_ERROR,
                message: String::from(
                    "agent tool `test.missing` cannot be listed: backing BRP method \
                     `test/missing` is not registered",
                ),
                data:    Some(json!({
                    "name": "test.missing",
                    "method": MISSING_METHOD,
                    "reason": BACKING_METHOD_MISSING_REASON,
                })),
            }),
        );
    }

    #[test]
    fn watching_backing_method_returns_exact_internal_error() {
        let mut app = catalog_app();
        register_watching_method(&mut app, WATCHING_METHOD);
        app.register_agent_tool(AgentTool::new(
            "test.watching",
            WATCHING_METHOD,
            DESCRIPTION,
        ));

        assert_eq!(
            call_catalog(&mut app),
            Err(BrpError {
                code:    INTERNAL_ERROR,
                message: String::from(
                    "agent tool `test.watching` cannot be listed: backing BRP method \
                     `test/watching` is registered as a watching method",
                ),
                data:    Some(json!({
                    "name": "test.watching",
                    "method": WATCHING_METHOD,
                    "reason": BACKING_METHOD_WATCHING_REASON,
                })),
            }),
        );
    }

    fn catalog_app() -> App {
        let mut app = App::new();
        app.add_plugins(BrpExtrasPlugin);
        app
    }

    fn call_catalog(app: &mut App) -> BrpResult {
        let catalog_method = format!("{EXTRAS_COMMAND_PREFIX}{METHOD_AGENT_TOOLS}");
        let method = app
            .world()
            .resource::<RemoteMethods>()
            .get(&catalog_method)
            .copied();
        let Some(RemoteMethodSystemId::Instant(system_id)) = method else {
            return Err(BrpError::internal(format!(
                "instant catalog method `{catalog_method}` is not registered"
            )));
        };

        app.world_mut()
            .run_system_with(system_id, None)
            .map_err(BrpError::internal)?
    }

    fn register_instant_method(app: &mut App, method: &str) {
        let system_id = app.world_mut().register_system(crate::shutdown::handler);
        app.world_mut()
            .resource_mut::<RemoteMethods>()
            .insert(method, RemoteMethodSystemId::Instant(system_id));
    }

    fn register_watching_method(app: &mut App, method: &str) {
        let system_id = app.world_mut().register_system(crate::screenshot::handler);
        app.world_mut()
            .resource_mut::<RemoteMethods>()
            .insert(method, RemoteMethodSystemId::Watching(system_id));
    }
}
