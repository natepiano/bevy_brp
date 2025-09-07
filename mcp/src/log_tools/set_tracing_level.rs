use std::str::FromStr;

use async_trait::async_trait;
use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tracing::TracingLevel;
use crate::error::{Error, Result};
use crate::tool::ToolFn;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
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

#[async_trait]
impl ToolFn for SetTracingLevel {
    type Output = SetTracingLevelResult;
    type Params = SetTracingLevelParams;

    async fn handle_impl(&self, params: SetTracingLevelParams) -> Result<SetTracingLevelResult> {
        // Parse the tracing level
        let tracing_level = match TracingLevel::from_str(&params.level) {
            Ok(level) => level,
            Err(e) => {
                return Err(Error::invalid(
                    "tracing level",
                    format!(
                        "{}: {e}. Valid levels are: error, warn, info, debug, trace",
                        params.level
                    ),
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
}
