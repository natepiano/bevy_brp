use std::str::FromStr;

use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

use super::tracing::{TracingLevel, get_trace_log_path, set_tracing_level};
use crate::tool::{
    HandlerContext, HandlerResponse, LocalToolFn, NoMethod, NoPort, ParameterName, ToolError,
    ToolResult,
};

/// Result from setting the tracing level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTracingLevelResult {
    /// The new tracing level that was set
    pub tracing_level:    String,
    /// The log file where trace output is written
    pub tracing_log_file: String,
}

pub struct SetTracingLevel;

impl LocalToolFn for SetTracingLevel {
    type Output = SetTracingLevelResult;

    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<Self::Output> {
        // Extract the required level parameter before the async block
        let level_str = match ctx.extract_required(ParameterName::Level) {
            Ok(value) => match value.into_string() {
                Ok(s) => s,
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let result = handle_impl(&level_str).map_err(|e| ToolError::new(e.message));
            let tool_result = ToolResult { result };
            Ok(tool_result)
        })
    }
}

fn handle_impl(level_str: &str) -> Result<SetTracingLevelResult, McpError> {
    // Parse the tracing level
    let tracing_level = match TracingLevel::from_str(level_str) {
        Ok(level) => level,
        Err(e) => {
            return Err(McpError::invalid_request(
                format!(
                    "Invalid tracing level '{level_str}': {e}. Valid levels are: error, warn, info, debug, trace"
                ),
                None,
            ));
        }
    };

    // Update the tracing level
    set_tracing_level(tracing_level);

    // Get the actual trace log path
    let log_path = get_trace_log_path();
    let log_path_str = log_path.to_string_lossy().to_string();

    Ok(SetTracingLevelResult {
        tracing_level:    tracing_level.as_str().to_string(),
        tracing_log_file: log_path_str,
    })
}
