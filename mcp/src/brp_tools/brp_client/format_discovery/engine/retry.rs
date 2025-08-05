//! Terminal retry logic for the format discovery engine
//!
//! This module implements the terminal retry state that applies corrections
//! and retries the original BRP call with corrected parameters.

use serde_json::Value;
use tracing::debug;

use super::super::format_correction_fields::FormatCorrectionField;
use super::recovery_result::FormatRecoveryResult;
use super::types::{
    Correction, CorrectionInfo, DiscoveryEngine, Retry, can_retry_with_corrections,
};
use crate::brp_tools::{BrpClientError, ResponseStatus, brp_client};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName};

impl DiscoveryEngine<Retry> {
    /// Apply corrections and retry the original BRP call
    ///
    /// This terminal method attempts to apply corrections to the original parameters
    /// and retry the BRP call, returning either success or failure.
    pub async fn apply_corrections_and_retry(self) -> FormatRecoveryResult {
        debug!("Retry Engine: Attempting to retry operation with corrected parameters");

        // Extract CorrectionInfo from Correction::Candidate variants
        let corrections: Vec<CorrectionInfo> = self
            .state
            .corrections
            .into_iter()
            .filter_map(|correction| match correction {
                Correction::Candidate { correction_info } => Some(correction_info),
                Correction::Uncorrectable { .. } => None,
            })
            .collect();

        // Validate that we have retryable corrections
        if !can_retry_with_corrections(&corrections) {
            debug!("Retry Engine: Corrections are not retryable");
            return FormatRecoveryResult::NotRecoverable { corrections };
        }

        // Build corrected parameters
        match build_corrected_params(self.method, &self.params, &corrections) {
            Ok(corrected_params) => {
                debug!("Retry Engine: Built corrected parameters, executing retry");

                // Execute the retry asynchronously
                let client = brp_client::BrpClient::new(self.method, self.port, corrected_params);
                let retry_result = client.execute_raw().await;

                match retry_result {
                    Ok(brp_result) => match brp_result {
                        ResponseStatus::Success(value) => {
                            debug!("Retry Engine: Retry succeeded with corrected parameters");
                            FormatRecoveryResult::Recovered {
                                corrected_result: ResponseStatus::Success(value),
                                corrections,
                            }
                        }
                        ResponseStatus::Error(brp_err) => {
                            debug!(
                                "Retry Engine: Retry failed with BRP error: {}",
                                brp_err.message
                            );
                            FormatRecoveryResult::CorrectionFailed {
                                retry_error: ResponseStatus::Error(brp_err),
                                corrections,
                            }
                        }
                    },
                    Err(retry_error) => {
                        debug!("Retry Engine: Retry failed: {}", retry_error);
                        // Convert error to BrpResult::Error
                        let retry_brp_error = ResponseStatus::Error(BrpClientError {
                            code:    -1, // Generic error code
                            message: retry_error.to_string(),
                            data:    None,
                        });
                        // Return correction failed with both original and retry errors
                        FormatRecoveryResult::CorrectionFailed {
                            retry_error: retry_brp_error,
                            corrections,
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Retry Engine: Could not build corrected parameters: {}", e);
                // Return original error - we can't fix this
                FormatRecoveryResult::NotRecoverable { corrections }
            }
        }
    }
}

/// Build corrected parameters from corrections
fn build_corrected_params(
    method: BrpMethod,
    original_params: &Value,
    corrections: &[CorrectionInfo],
) -> Result<Option<Value>> {
    let mut params = original_params.clone();

    for correction in corrections {
        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
                // Update components
                if let Some(components) = ParameterName::Components.get_object_mut_from(&mut params)
                {
                    components.insert(
                        correction.type_name.clone(),
                        correction.corrected_value.clone(),
                    );
                }
            }
            BrpMethod::BevyInsertResource => {
                // Update value directly
                params[FormatCorrectionField::Value.as_ref()] = correction.corrected_value.clone();
            }
            BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource => {
                // For mutations, we need both path and value
                if correction.corrected_value.is_object() {
                    if let Some(obj) = correction.corrected_value.as_object() {
                        if let (Some(path), Some(value)) = (
                            obj.get(FormatCorrectionField::Path.as_ref()),
                            obj.get(FormatCorrectionField::Value.as_ref()),
                        ) {
                            params[FormatCorrectionField::Path.as_ref()] = path.clone();
                            params[FormatCorrectionField::Value.as_ref()] = value.clone();
                        } else {
                            return Err(Error::InvalidArgument(
                                "Mutation correction missing path or value".to_string(),
                            )
                            .into());
                        }
                    }
                }
            }
            _ => {
                // For other methods, assume simple value replacement
                params = correction.corrected_value.clone();
            }
        }
    }

    Ok(Some(params))
}
