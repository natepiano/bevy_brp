use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RemoveResourceParams {
    /// The fully-qualified type name of the resource to remove
    pub resource: String,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:     u16,
}

impl HasPortField for RemoveResourceParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyRemoveResource;

impl BrpToolFn for BevyRemoveResource {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<RemoveResourceParams>(ctx))
    }
}
