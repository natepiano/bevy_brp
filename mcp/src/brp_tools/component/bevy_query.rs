use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::handler::{BrpMethodResult, HasPortField, execute_static_brp_call};
use crate::constants::default_port;
use crate::tool::{BrpToolFn, HandlerContext, HandlerResponse, HasMethod, HasPort};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve. Properties: components (array), option
    /// (array), has (array)
    pub data:   serde_json::Value,
    /// Object specifying which entities to query. Properties: with (array), without (array)
    #[serde(default)]
    pub filter: Option<serde_json::Value>,
    /// If true, returns error on unknown component types (default: false)
    #[serde(default)]
    pub strict: Option<bool>,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port")]
    pub port:   u16,
}

impl HasPortField for QueryParams {
    fn port(&self) -> u16 {
        self.port
    }
}

pub struct BevyQuery;

impl BrpToolFn for BevyQuery {
    type Output = BrpMethodResult;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(execute_static_brp_call::<QueryParams>(ctx))
    }
}
