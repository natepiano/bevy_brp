//! Shared serialization utilities

use rmcp::model::{CallToolResult, Content};

use crate::support::response::JsonResponse;

/// Creates a `CallToolResult` from a `JsonResponse`
pub fn json_response_to_result(response: &JsonResponse) -> CallToolResult {
    CallToolResult::success(vec![Content::text(response.to_json_fallback())])
}
