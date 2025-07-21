use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GetParams {
    /// The entity ID to get component data from
    pub entity:     u64,
    /// Array of component types to retrieve. Each component must be a fully-qualified type name
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:       u16,
}

impl HasPortField for GetParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyGet;

impl BrpToolFn for BevyGet {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<GetParams>(ctx))
    }
}
