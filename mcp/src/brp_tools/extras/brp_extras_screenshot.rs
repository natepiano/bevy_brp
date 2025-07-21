use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ScreenshotParams {
    /// File path where the screenshot should be saved
    pub path: String,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port: u16,
}

impl HasPortField for ScreenshotParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BrpExtrasScreenshot;

impl BrpToolFn for BrpExtrasScreenshot {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<ScreenshotParams>(ctx))
    }
}
