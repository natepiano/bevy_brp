use std::str::FromStr;

use bevy_brp_mcp_macros::ResultFieldPlacement;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tracing::{TracingLevel, get_trace_log_path, set_tracing_level};
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

#[derive(Deserialize, JsonSchema, ResultFieldPlacement)]
pub struct SetTracingLevelParams {
    /// Tracing level to set (error, warn, info, debug, trace)
    #[to_metadata]
    pub level: String,
}

/// Result from setting the tracing level
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
pub struct SetTracingLevelResult {
    /// The new tracing level that was set
    #[to_metadata]
    pub tracing_level:    String,
    /// The log file where trace output is written
    #[to_metadata]
    pub tracing_log_file: String,
}

pub struct SetTracingLevel;

impl ToolFn for SetTracingLevel {
    type Output = SetTracingLevelResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            let params: SetTracingLevelParams = ctx.extract_parameter_values()?;
            Ok(ToolResult::from_result(
                handle_impl(&params.level),
                LocalCallInfo,
            ))
        })
    }
}

fn handle_impl(level_str: &str) -> Result<SetTracingLevelResult> {
    // Parse the tracing level
    let tracing_level = match TracingLevel::from_str(level_str) {
        Ok(level) => level,
        Err(e) => {
            return Err(Error::invalid(
                "tracing level",
                format!("{level_str}: {e}. Valid levels are: error, warn, info, debug, trace"),
            )
            .into());
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
