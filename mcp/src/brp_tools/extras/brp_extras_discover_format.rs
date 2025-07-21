use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct DiscoverFormatParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:  u16,
}

impl HasPortField for DiscoverFormatParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BrpExtrasDiscoverFormat;

impl BrpToolFn for BrpExtrasDiscoverFormat {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<DiscoverFormatParams>(ctx))
    }
}
