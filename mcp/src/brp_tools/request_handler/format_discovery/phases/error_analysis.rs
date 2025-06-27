//! Error analysis phase for the format discovery engine
//! This module determines if errors are recoverable and extracts error patterns

use crate::brp_tools::request_handler::format_discovery::constants::{
    COMPONENT_FORMAT_ERROR_CODE, FORMAT_DISCOVERY_METHODS, RESOURCE_FORMAT_ERROR_CODE,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult};

/// Detect if an error is a type format error that can be fixed (component or resource)
pub const fn is_type_format_error(error: &BrpError) -> bool {
    error.code == COMPONENT_FORMAT_ERROR_CODE || error.code == RESOURCE_FORMAT_ERROR_CODE
}

/// Check if the initial result needs format discovery
pub fn needs_format_discovery<'a>(result: &'a BrpResult, method: &str) -> Option<&'a BrpError> {
    if let BrpResult::Error(error) = result {
        if FORMAT_DISCOVERY_METHODS.contains(&method) && is_type_format_error(error) {
            return Some(error);
        }
    }
    None
}
