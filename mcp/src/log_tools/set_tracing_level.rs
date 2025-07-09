use std::str::FromStr;

use rmcp::Error as McpError;

use crate::response::TracingLevelResult;
use crate::support::params;
use crate::support::tracing::{TracingLevel, set_tracing_level};

/// Handle the `brp_set_tracing_level` tool request
pub fn handle(request: &rmcp::model::CallToolRequestParam) -> Result<TracingLevelResult, McpError> {
    // Extract the required level parameter
    let level_str = params::extract_required_string(request, "level")?;

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

    Ok(TracingLevelResult {
        level:    tracing_level.as_str().to_string(),
        log_file: "bevy_brp_mcp_trace.log (in temp directory)".to_string(),
    })
}
