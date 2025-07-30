use std::str::FromStr;

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tracing::TracingLevel;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SetTracingLevelParams {
    /// Tracing level to set (error, warn, info, debug, trace)
    pub level: String,
}

/// Result from setting the tracing level
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct SetTracingLevelResult {
    /// The new tracing level that was set
    #[to_metadata]
    tracing_level:    String,
    /// The log file where trace output is written
    #[to_metadata]
    tracing_log_file: String,
    /// Message template for formatting responses
    #[to_message(message_template = "Set tracing level to {tracing_level}")]
    message_template: String,
}

pub struct SetTracingLevel;

impl ToolFn for SetTracingLevel {
    type Output = SetTracingLevelResult;
    type Params = SetTracingLevelParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            let params: SetTracingLevelParams = ctx.extract_parameter_values()?;

            let result = handle_impl(&params.level);
            Ok(ToolResult {
                result,
                params: Some(params),
            })
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
    TracingLevel::set_tracing_level(tracing_level);

    // Get the actual trace log path
    let log_path = TracingLevel::get_trace_log_path();
    let log_path_str = log_path.to_string_lossy().to_string();

    Ok(SetTracingLevelResult::new(
        tracing_level.as_str().to_string(),
        log_path_str,
    ))
}
