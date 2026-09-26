//! BRP JSON-RPC response, status, and error types.

use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::constants::BRP_ERROR_ACCESS_ERROR;
use super::constants::BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE;
use super::constants::JSON_RPC_ERROR_INTERNAL_ERROR;
use super::constants::JSON_RPC_ERROR_INVALID_PARAMS;
use super::constants::RESOURCE_NOT_IN_WORLD_MESSAGES;
use super::constants::RESOURCE_NOT_INITIALIZED_MESSAGES;
use crate::error::Result;

/// Configuration trait for BRP tools to control enhanced error handling
pub trait BrpToolConfig {
    /// Whether this tool should use enhanced error handling with `type_guide` embedding
    const ADD_TYPE_GUIDE_TO_ERROR: bool = false;
}

/// Extension trait for `ResultStruct` types that handle BRP responses
pub trait ResultStructBrpExt: Sized {
    type Args;

    /// Construct from BRP client response
    fn from_brp_client_response(response: Self::Args) -> Result<Self>;
}

/// Error information from BRP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrpClientError {
    pub code:    i32,
    pub message: String,
    pub data:    Option<Value>,
}

impl BrpClientError {
    /// Get the error code
    pub const fn get_code(&self) -> i32 { self.code }

    /// Get the error message
    pub fn get_message(&self) -> &str { &self.message }

    /// Return true when a BRP JSON-RPC error can trigger format discovery.
    ///
    /// `bevy_remote` can report `BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE` for a
    /// `Component` missing `Serialize`/`Deserialize`; `Resource` errors usually
    /// arrive as JSON-RPC format codes. `BRP_ERROR_ACCESS_ERROR` counts as a format
    /// error when a mutation path fails to reach a field, and does not count when its
    /// message reports a resource absence (see `resource_absence`), because no format
    /// correction initializes or inserts a missing resource.
    pub fn is_format_error(&self) -> bool {
        match self.code {
            JSON_RPC_ERROR_INVALID_PARAMS
            | JSON_RPC_ERROR_INTERNAL_ERROR
            | BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE => true,
            BRP_ERROR_ACCESS_ERROR => self.resource_absence().is_none(),
            _ => false,
        }
    }

    /// Return the resource absence `bevy_remote` reports with `BRP_ERROR_ACCESS_ERROR`, matching
    /// the message against `RESOURCE_NOT_INITIALIZED_MESSAGES` and
    /// `RESOURCE_NOT_IN_WORLD_MESSAGES`. Any other code returns `None`.
    pub(super) fn resource_absence(&self) -> Option<ResourceAbsence> {
        if self.code != BRP_ERROR_ACCESS_ERROR {
            return None;
        }
        let message_matches = |messages: &[&str]| {
            messages
                .iter()
                .any(|message| self.message.contains(message))
        };
        if message_matches(RESOURCE_NOT_INITIALIZED_MESSAGES) {
            Some(ResourceAbsence::NotInitialized)
        } else if message_matches(RESOURCE_NOT_IN_WORLD_MESSAGES) {
            Some(ResourceAbsence::NotInWorld)
        } else {
            None
        }
    }
}

impl Display for BrpClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.message) }
}

/// Why `bevy_remote` could not reach a resource, each case calling for a different fix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResourceAbsence {
    /// The resource's component was never registered because the app never initialized the
    /// resource. BRP cannot insert it; the app must add the plugin that owns it or call
    /// `init_resource`/`insert_resource`.
    NotInitialized,
    /// The resource's component is registered but no entity holds the resource. Inserting it
    /// with `world_insert_resources` fixes it.
    NotInWorld,
}

/// Raw BRP JSON-RPC response structure
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct BrpClientCallJsonResponse {
    pub jsonrpc: String,
    pub id:      u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<JsonRpcError>,
}

/// Raw BRP error structure from JSON-RPC response
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JsonRpcError {
    pub code:    i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:    Option<Value>,
}

/// Status of a BRP operation - determines `status` field in the `ToolCallJsonResponse`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Successful operation with optional data
    Success(Option<Value>),
    /// Error with code, message and optional data
    Error(BrpClientError),
}

/// Status of format correction attempts
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCorrectionStatus {
    /// Format discovery was not enabled for this request
    NotApplicable,
    /// No format correction was attempted
    NotAttempted,
    /// Format correction was applied and the operation succeeded
    Succeeded,
}

#[cfg(test)]
mod tests {
    use super::BRP_ERROR_ACCESS_ERROR;
    use super::BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE;
    use super::BrpClientError;
    use super::JSON_RPC_ERROR_INVALID_PARAMS;
    use super::ResourceAbsence;
    use crate::brp_tools::JSON_RPC_ERROR_METHOD_NOT_FOUND;

    #[test]
    fn test_brp_client_error_display() {
        let error = BrpClientError {
            code:    JSON_RPC_ERROR_INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data:    None,
        };
        assert_eq!(error.to_string(), "Invalid params");
    }

    #[test]
    fn test_brp_client_error_is_format_error() {
        let format_error = BrpClientError {
            code:    JSON_RPC_ERROR_INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data:    None,
        };
        assert!(format_error.is_format_error());

        let unknown_component_error = BrpClientError {
            code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
            message: "Unknown component type".to_string(),
            data:    None,
        };
        assert!(unknown_component_error.is_format_error());

        let non_format_error = BrpClientError {
            code:    JSON_RPC_ERROR_METHOD_NOT_FOUND,
            message: "Method not found".to_string(),
            data:    None,
        };
        assert!(!non_format_error.is_format_error());
    }

    #[test]
    fn test_missing_resource_entity_is_not_in_world() {
        let error = BrpClientError {
            code:    BRP_ERROR_ACCESS_ERROR,
            message: "Resource entity does not exist.".to_string(),
            data:    None,
        };
        assert!(!error.is_format_error());
        assert_eq!(error.resource_absence(), Some(ResourceAbsence::NotInWorld));
    }

    #[test]
    fn test_unregistered_resource_on_insert_is_not_initialized() {
        let error = BrpClientError {
            code:    BRP_ERROR_ACCESS_ERROR,
            message: "Resource is not registered: `extras_plugin::KeyboardInputHistory`"
                .to_string(),
            data:    None,
        };
        assert!(!error.is_format_error());
        assert_eq!(
            error.resource_absence(),
            Some(ResourceAbsence::NotInitialized)
        );
    }

    #[test]
    fn test_unregistered_resource_on_access_is_not_initialized() {
        let error = BrpClientError {
            code:    BRP_ERROR_ACCESS_ERROR,
            message: "Resource not registered: `extras_plugin::KeyboardInputHistory`".to_string(),
            data:    None,
        };
        assert!(!error.is_format_error());
        assert_eq!(
            error.resource_absence(),
            Some(ResourceAbsence::NotInitialized)
        );
    }

    #[test]
    fn test_path_access_error_is_format_error() {
        let error = BrpClientError {
            code:    BRP_ERROR_ACCESS_ERROR,
            message: "Error accessing element with `.red` access(offset 3): Expected variant field access to access Struct variant, found a Tuple variant instead.".to_string(),
            data:    None,
        };
        assert!(error.is_format_error());
        assert_eq!(error.resource_absence(), None);
    }

    #[test]
    fn test_resource_absence_needs_access_error_code() {
        let error = BrpClientError {
            code:    JSON_RPC_ERROR_METHOD_NOT_FOUND,
            message: "Resource entity does not exist.".to_string(),
            data:    None,
        };
        assert_eq!(error.resource_absence(), None);
    }
}
