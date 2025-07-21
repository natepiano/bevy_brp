use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct MutateComponentParams {
    /// The entity ID containing the component to mutate
    pub entity:    u64,
    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value:     serde_json::Value,
    /// The fully-qualified type name of the component to mutate
    pub component: String,
    /// The path to the field within the component (e.g., 'translation.x')
    pub path:      String,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:      u16,
}

impl HasPortField for MutateComponentParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyMutateComponent;

impl BrpToolFn for BevyMutateComponent {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<MutateComponentParams>(ctx))
    }
}
