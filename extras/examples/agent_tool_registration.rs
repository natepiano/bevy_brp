//! Register an instant BRP method and publish typed metadata for agents.
//!
//! Run this example, then use the MCP tools in this order:
//!
//! ```text
//! cargo run -p bevy_brp_extras --example agent_tool_registration
//! brp_list_agent_tools(port: 15702)
//! brp_execute(
//!     port: 15702,
//!     method: "example/multiply",
//!     params: { "value": 6, "factor": 7 }
//! )
//! ```

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_brp_extras::AgentTool;
use bevy_brp_extras::AppAgentToolExt;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::RemoteMethodSystemId;
use bevy_remote::RemoteMethods;
use bevy_remote::error_codes::INVALID_PARAMS;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

const MULTIPLY_METHOD: &str = "example/multiply";
const RUNNER_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Deserialize, JsonSchema)]
struct MultiplyParams {
    value:  i64,
    factor: i64,
}

#[derive(Serialize, JsonSchema)]
struct MultiplyResult {
    product: i64,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(ScheduleRunnerPlugin::run_loop(RUNNER_INTERVAL));
    app.add_plugins(BrpExtrasPlugin);

    let system_id = app.world_mut().register_system(multiply);
    {
        let mut remote_methods = app.world_mut().resource_mut::<RemoteMethods>();
        remote_methods.insert(MULTIPLY_METHOD, RemoteMethodSystemId::Instant(system_id));
    }

    app.register_agent_tool(
        AgentTool::new(
            "example_multiply",
            "example/multiply",
            "Multiply two signed integers",
        )
        .params_schema_for::<MultiplyParams>()
        .result_schema_for::<MultiplyResult>(),
    );

    app.run();
}

fn multiply(In(params): In<Option<Value>>) -> BrpResult {
    let params = params.ok_or_else(|| invalid_params("missing parameters"))?;
    let params: MultiplyParams = serde_json::from_value(params).map_err(invalid_params)?;
    let product = params
        .value
        .checked_mul(params.factor)
        .ok_or_else(|| invalid_params("value multiplied by factor exceeds i64"))?;

    serde_json::to_value(MultiplyResult { product }).map_err(|error| {
        BrpError::internal(format!("failed to serialize multiply result: {error}"))
    })
}

fn invalid_params(error: impl ToString) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: error.to_string(),
        data:    None,
    }
}
