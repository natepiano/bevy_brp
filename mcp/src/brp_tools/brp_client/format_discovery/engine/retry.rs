//! Terminal retry logic for the format discovery engine
//!
//! This module implements the terminal retry state that applies corrections
//! and retries the original BRP call with corrected parameters.

use serde_json::Value;
use tracing::debug;

use super::recovery_result::FormatRecoveryResult;
use super::state::{DiscoveryEngine, Retry};
use super::types::{Correction, CorrectionInfo, Operation};
use crate::brp_tools::{BrpClientError, ResponseStatus, brp_client};
use crate::error::{Error, Result};
use crate::string_traits::JsonFieldAccess;
use crate::tool::ParameterName;

impl DiscoveryEngine<Retry> {
    /// Apply corrections and retry the original BRP call
    ///
    /// This terminal method attempts to apply corrections to the original parameters
    /// and retry the BRP call, returning either success or failure.
    pub async fn apply_corrections_and_retry(self) -> FormatRecoveryResult {
        debug!("Retry Engine: Attempting to retry operation with corrected parameters");

        // Extract CorrectionInfo from Correction::Candidate variants
        let corrections: Vec<CorrectionInfo> = self
            .context
            .corrections
            .into_iter()
            .filter_map(|correction| match correction {
                Correction::Candidate { correction_info } => Some(correction_info),
                Correction::Uncorrectable { .. } => None,
            })
            .collect();

        // All Candidate corrections should be retryable by definition
        if corrections.is_empty() {
            debug!("Retry Engine: No candidate corrections available");
            return FormatRecoveryResult::NotRecoverable { corrections };
        }

        // Build corrected parameters
        match build_corrected_params(self.operation, &self.params, &corrections) {
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
    operation: Operation,
    original_params: &Value,
    corrections: &[CorrectionInfo],
) -> Result<Option<Value>> {
    let mut params = original_params.clone();

    for correction in corrections {
        match operation {
            Operation::SpawnInsert { parameter_name } => {
                // Use the parameter_name field to determine how to build params
                match parameter_name {
                    ParameterName::Value => {
                        // For resources: update value directly
                        params
                            .insert_field(ParameterName::Value, correction.corrected_value.clone());
                    }
                    ParameterName::Components => {
                        // For components: update components[type_name]
                        if let Some(components) =
                            params.get_field_object_mut(ParameterName::Components)
                        {
                            components.insert(
                                correction.type_info.type_name().as_str().to_string(),
                                correction.corrected_value.clone(),
                            );
                        }
                    }
                    _ => {
                        return Err(Error::InvalidArgument(
                            "SpawnInsert only uses Value or Components parameters".to_string(),
                        )
                        .into());
                    }
                }
            }
            Operation::Mutate { parameter_name } => {
                // For mutations, we need the type parameter (component/resource), path, and value
                // First, set the type-specific parameter with the type name
                params.insert_field(parameter_name, correction.type_info.type_name());

                // Then set path and value from the correction
                if correction.corrected_value.is_object() {
                    if let Some(obj) = correction.corrected_value.as_object() {
                        if let (Some(path), Some(value)) = (
                            obj.get_field(ParameterName::Path),
                            obj.get_field(ParameterName::Value),
                        ) {
                            params.insert_field(ParameterName::Path, path.clone());
                            params.insert_field(ParameterName::Value, value.clone());
                        } else {
                            return Err(Error::InvalidArgument(
                                "Mutation correction missing path or value".to_string(),
                            )
                            .into());
                        }
                    }
                } else {
                    // If corrected_value is not an object, it's just the value to set
                    // Preserve the original path from the original parameters
                    if let Some(original_path) = original_params.get_field(ParameterName::Path) {
                        params.insert_field(ParameterName::Path, original_path.clone());
                    }
                    params.insert_field(ParameterName::Value, correction.corrected_value.clone());
                }
            }
        }
    }

    Ok(Some(params))
}
