use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tracing::{TracingLevel, get_trace_log_path, set_tracing_level};
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResponse, LocalCallInfo, ToolFn, WithCallInfo};

#[derive(Deserialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct SetTracingLevelParams {
    /// Tracing level to set (error, warn, info, debug, trace)
    #[to_metadata]
    pub level: String,
}

/// Result from setting the tracing level
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
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
        ctx: &HandlerContext,
    ) -> HandlerResponse<(Self::CallInfoData, crate::error::Result<Self::Output>)> {
        // Extract typed parameters
        let params: SetTracingLevelParams = match ctx.extract_parameter_values() {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Ok(Err(e).with_call_info(LocalCallInfo)) }),
        };

        Box::pin(async move { Ok(handle_impl(&params.level).with_call_info(LocalCallInfo)) })
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
