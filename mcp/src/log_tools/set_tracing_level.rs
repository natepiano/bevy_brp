use std::str::FromStr;

use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use crate::service::{HandlerContext, LocalContext};
use super::tracing::{TracingLevel, get_trace_log_path, set_tracing_level};
use crate::tool::{HandlerResponse, HandlerResult, LocalToolFunction};

/// Result from setting the tracing level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTracingLevelResult {
    /// The new tracing level that was set
    pub level:    String,
    /// The log file where trace output is written
    pub log_file: String,
}

impl HandlerResult for SetTracingLevelResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct SetTracingLevel;

impl LocalToolFunction for SetTracingLevel {
    fn call(&self, ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_> {
        // Extract the required level parameter before the async block
        let level_str = match ctx.extract_required_string("level", "tracing level") {
            Ok(s) => s.to_string(),
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            handle_impl(&level_str).map(|result| Box::new(result) as Box<dyn HandlerResult>)
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
        level:    tracing_level.as_str().to_string(),
        log_file: log_path_str,
    })
}
