//! Shared BRP request parsing and response serialization.

use bevy::prelude::warn;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::constants::MISSING_REQUEST_PARAMETERS_MESSAGE;

/// Whether `parse_request` should accept `None` params by treating them as an empty object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyParamsPolicy {
    Allow,
    Reject,
}

/// Parse BRP request parameters into a strongly typed request struct.
pub(crate) fn parse_request<T: serde::de::DeserializeOwned>(
    params: Option<Value>,
    empty_params_policy: EmptyParamsPolicy,
) -> Result<T, BrpError> {
    let params = match (params, empty_params_policy) {
        (Some(params), _) => params,
        (None, EmptyParamsPolicy::Allow) => Value::Object(Map::default()),
        (None, EmptyParamsPolicy::Reject) => {
            return Err(invalid_params(
                MISSING_REQUEST_PARAMETERS_MESSAGE.to_string(),
            ));
        },
    };

    serde_json::from_value(params)
        .map_err(|e| invalid_params(format!("Failed to parse parameters: {e}")))
}

/// Build an `INVALID_PARAMS` error carrying `message`.
pub(crate) const fn invalid_params(message: String) -> BrpError {
    BrpError {
        code: INVALID_PARAMS,
        message,
        data: None,
    }
}

/// Serialize a BRP response with consistent error handling and logging.
pub(crate) fn serialize_response<T: Serialize>(response: T, handler_name: &str) -> BrpResult {
    serde_json::to_value(response).map_err(|e| {
        warn!("Failed to serialize {handler_name} response: {e}");
        BrpError {
            code:    INTERNAL_ERROR,
            message: format!("Failed to serialize response: {e}"),
            data:    None,
        }
    })
}
