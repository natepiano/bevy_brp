use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct MutateResourceParams {
    /// The fully-qualified type name of the resource to mutate
    pub resource: String,
    /// The path to the field within the resource (e.g., 'settings.volume')
    pub path:     String,
    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value:    serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:     u16,
}

impl HasPortField for MutateResourceParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyMutateResource;

impl BrpToolFn for BevyMutateResource {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<MutateResourceParams>(ctx))
    }
}
