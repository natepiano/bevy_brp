use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RegistrySchemaParams {
    /// Include only types from these crates (e.g., [`bevy_transform`, `my_game`])
    #[serde(default)]
    pub with_crates:    Option<Vec<String>>,
    /// Exclude types from these crates (e.g., [`bevy_render`, `bevy_pbr`])
    #[serde(default)]
    pub without_crates: Option<Vec<String>>,
    /// Include only types with these reflect traits (e.g., [`Component`, `Resource`])
    #[serde(default)]
    pub with_types:     Option<Vec<String>>,
    /// Exclude types with these reflect traits (e.g., [`RenderResource`])
    #[serde(default)]
    pub without_types:  Option<Vec<String>>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:           u16,
}

impl HasPortField for RegistrySchemaParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyRegistrySchema;

impl BrpToolFn for BevyRegistrySchema {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<RegistrySchemaParams>(ctx))
    }
}
