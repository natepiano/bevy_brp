use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::request_handler::format_discovery::FormatCorrectionStatus;
use crate::tool::HandlerResult;

/// Result type for BRP method calls that follows local handler patterns
#[derive(Serialize)]
pub struct BrpMethodResult {
    // Success data - the actual BRP response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    // BRP metadata - using existing field names
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub format_corrections: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_corrected:   Option<FormatCorrectionStatus>,
}

impl HandlerResult for BrpMethodResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}
