//! Public API coverage for typed agent-tool registration and catalog schemas.

use bevy::prelude::*;
use bevy_brp_extras::AgentTool;
use bevy_brp_extras::AppAgentToolExt;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::RemoteMethodSystemId;
use bevy_remote::RemoteMethods;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

const DESCRIPTION: &str = "Multiply two signed integers";
const METHOD: &str = "example/multiply";
const NAME: &str = "example_multiply";

#[derive(Deserialize, JsonSchema)]
struct MultiplyParams {
    value:  i64,
    factor: i64,
}

#[derive(Serialize, JsonSchema)]
struct MultiplyResult {
    product: i64,
}

#[test]
fn exported_apis_publish_typed_agent_tool_metadata() -> Result<(), String> {
    let mut app = App::new();
    app.add_plugins(BrpExtrasPlugin);

    let multiply_system_id = app.world_mut().register_system(multiply);
    {
        let mut remote_methods = app.world_mut().resource_mut::<RemoteMethods>();
        remote_methods.insert(METHOD, RemoteMethodSystemId::Instant(multiply_system_id));
    }
    app.register_agent_tool(
        AgentTool::new(NAME, METHOD, DESCRIPTION)
            .params_schema_for::<MultiplyParams>()
            .result_schema_for::<MultiplyResult>(),
    );

    let catalog_method = app
        .world()
        .resource::<RemoteMethods>()
        .get("brp_extras/agent_tools")
        .copied()
        .ok_or_else(|| String::from("brp_extras/agent_tools is not registered"))?;
    let RemoteMethodSystemId::Instant(system_id) = catalog_method else {
        return Err(String::from(
            "brp_extras/agent_tools is not an instant method",
        ));
    };
    let catalog = app
        .world_mut()
        .run_system_with(system_id, None)
        .map_err(|error| error.to_string())?
        .map_err(|error| error.message)?;

    let params_schema = serde_json::to_value(schemars::schema_for!(MultiplyParams))
        .map_err(|error| error.to_string())?;
    let result_schema = serde_json::to_value(schemars::schema_for!(MultiplyResult))
        .map_err(|error| error.to_string())?;
    assert_eq!(
        catalog,
        json!({
            "version": 1,
            "tools": [{
                "name": NAME,
                "method": METHOD,
                "description": DESCRIPTION,
                "params_schema": params_schema,
                "result_schema": result_schema,
            }],
        }),
    );

    Ok(())
}

fn multiply(In(params): In<Option<Value>>) -> BrpResult {
    let params = params.ok_or_else(|| BrpError::internal("missing parameters"))?;
    let params: MultiplyParams = serde_json::from_value(params).map_err(BrpError::internal)?;

    serde_json::to_value(MultiplyResult {
        product: params.value * params.factor,
    })
    .map_err(BrpError::internal)
}
