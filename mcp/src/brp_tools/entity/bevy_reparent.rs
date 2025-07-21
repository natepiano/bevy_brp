use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ReparentParams {
    /// Array of entity IDs to reparent
    pub entities: Vec<u64>,
    /// The new parent entity ID (omit to remove parent)
    #[serde(default)]
    pub parent:   Option<u64>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:     u16,
}

impl HasPortField for ReparentParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyReparent;

impl BrpToolFn for BevyReparent {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<ReparentParams>(ctx))
    }
}
