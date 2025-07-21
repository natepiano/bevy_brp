use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SendKeysParams {
    /// Array of key code names to send
    pub keys:        Vec<String>,
    /// Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1
    /// minute)
    #[serde(default)]
    pub duration_ms: Option<u32>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:        u16,
}

impl HasPortField for SendKeysParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BrpExtrasSendKeys;

impl BrpToolFn for BrpExtrasSendKeys {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<SendKeysParams>(ctx))
    }
}
