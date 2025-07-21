use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port: u16,
}

impl HasPortField for RpcDiscoverParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyRpcDiscover;

impl BrpToolFn for BevyRpcDiscover {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<RpcDiscoverParams>(ctx))
    }
}
