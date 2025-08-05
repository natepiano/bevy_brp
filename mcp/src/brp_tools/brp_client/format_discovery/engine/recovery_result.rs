//! Result transformation utilities for format discovery
//!
//! This module handles the transformation of format recovery results into the final
//! client response format, including error enhancement and correction metadata.

use super::types::{CorrectionInfo, FormatCorrectionStatus};
use crate::brp_tools::brp_client::types::{BrpClientError, FormatDiscoveryError, ResponseStatus};
use crate::error::Error;

/// Result of format error recovery attempt in the exception path
#[derive(Debug, Clone)]
pub enum FormatRecoveryResult {
    /// Recovery successful with corrections applied
    Recovered {
        corrected_result: ResponseStatus,
        corrections:      Vec<CorrectionInfo>,
    },
    /// Recovery not possible but guidance available
    NotRecoverable { corrections: Vec<CorrectionInfo> },
    /// Recovery attempted but correction was insufficient
    CorrectionFailed {
        retry_error: ResponseStatus,
        corrections: Vec<CorrectionInfo>,
    },
}

impl FormatRecoveryResult {
    /// Transform this recovery result into a typed result for the client
    ///
    /// This method converts the internal recovery result into the final typed
    /// response expected by the BRP client, including error enhancement and
    /// correction metadata.
    pub fn into_typed_result<R>(
        self,
        original_error: &crate::brp_tools::BrpClientError,
    ) -> crate::error::Result<R>
    where
        R: crate::brp_tools::brp_client::types::ResultStructBrpExt<
                Args = (
                    Option<serde_json::Value>,
                    Option<Vec<serde_json::Value>>,
                    Option<crate::brp_tools::brp_client::FormatCorrectionStatus>,
                ),
            >,
    {
        match self {
            Self::Recovered {
                ref corrected_result,
                ..
            } => {
                // Successfully recovered with format corrections
                // Extract the success value from corrected_result
                match corrected_result {
                    ResponseStatus::Success(value) => {
                        // Convert CorrectionInfo to JSON directly
                        R::from_brp_client_response((
                            value.clone(),
                            Some(
                                self.corrections()
                                    .iter()
                                    .map(CorrectionInfo::to_json)
                                    .collect(),
                            ),
                            Some(FormatCorrectionStatus::Succeeded),
                        ))
                    }
                    ResponseStatus::Error(err) => {
                        // Recovery succeeded but result contains error - shouldn't happen
                        Err(Error::tool_call_failed(format!(
                            "Format recovery succeeded but result contains error: {}",
                            err.message
                        ))
                        .into())
                    }
                }
            }
            Self::NotRecoverable { .. } => {
                // Format discovery couldn't fix it but has guidance
                let enhanced_error = self.create_format_discovery_error(
                    original_error,
                    "Format errors not recoverable but guidance available",
                );
                Err(enhanced_error.into())
            }
            Self::CorrectionFailed {
                ref retry_error, ..
            } => {
                // Format discovery tried but the correction failed
                let retry_error_msg = match retry_error {
                    ResponseStatus::Error(err) => &err.message,
                    ResponseStatus::Success(_) => "Unknown error",
                };
                let enhanced_error = self.create_format_discovery_error(
                    original_error,
                    &format!("Correction attempted but failed: {retry_error_msg}"),
                );
                Err(enhanced_error.into())
            }
        }
    }

    /// Get a reference to the corrections in this result
    fn corrections(&self) -> &[CorrectionInfo] {
        match self {
            Self::Recovered { corrections, .. }
            | Self::NotRecoverable { corrections }
            | Self::CorrectionFailed { corrections, .. } => corrections,
        }
    }

    /// Create an enhanced error for format discovery failures
    fn create_format_discovery_error(
        &self,
        original_error: &BrpClientError,
        reason: &str,
    ) -> Error {
        let corrections = self.corrections();

        // Build format corrections array with metadata
        // Always include the array (even if empty) to meet test expectations
        let format_corrections = Some(corrections.iter().map(CorrectionInfo::to_json).collect());

        // Build hint message from corrections
        let hint = if corrections.is_empty() {
            "No format corrections available. Check that the types have Serialize/Deserialize traits."
                .to_string()
        } else {
            corrections
                .iter()
                .filter_map(|c| {
                    if c.hint.is_empty() {
                        None
                    } else {
                        Some(format!("- {}: {}", c.type_name, c.hint))
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let format_discovery_error = FormatDiscoveryError::new(
            "not_attempted".to_string(),
            if hint.is_empty() {
                "Format discovery found issues but could not provide specific guidance.".to_string()
            } else {
                hint
            },
            format_corrections,
            Some(original_error.code),
            reason.to_string(),
            original_error.message.clone(),
        );

        Error::Structured {
            result: Box::new(format_discovery_error),
        }
    }
}

#[cfg(test)]
mod tests {

    // Note: CorrectionInfo.to_json() testing requires creating complex UnifiedTypeInfo instances.
    // The integration tests in format_discovery.md will verify the full flow.
    // The to_json() method implementation ensures identical JSON structure output.
}
