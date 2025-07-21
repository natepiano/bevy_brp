use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SpawnParams {
    /// Object containing component data to spawn with. Keys are component types, values are
    /// component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat:
    /// [x,y,z,w], not objects with named fields.
    #[serde(default)]
    pub components: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:       u16,
}

impl HasPortField for SpawnParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevySpawn;

impl BrpToolFn for BevySpawn {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<SpawnParams>(ctx))
    }
}
