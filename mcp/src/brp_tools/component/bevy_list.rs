use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ListParams {
    /// Optional entity ID to list components for - to list all types, do not pass entity parameter
    #[serde(default)]
    pub entity: Option<u64>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:   u16,
}

impl HasPortField for ListParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyList;

impl BrpToolFn for BevyList {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<ListParams>(ctx))
    }
}
