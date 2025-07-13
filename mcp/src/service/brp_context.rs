use rmcp::Error as McpError;
use serde_json::Value;

use super::{HandlerContext, HasCallInfo};
use crate::constants::PARAM_ENTITY;
use crate::response::CallInfo;

/// Data type for BRP handler contexts (carries extracted request data)
#[derive(Clone)]
pub struct BrpContext {
    pub method: String,
    pub port:   u16,
}

impl HasCallInfo for HandlerContext<BrpContext> {
    fn call_info(&self) -> CallInfo {
        self.call_info()
    }
}

impl HandlerContext<BrpContext> {
    pub fn brp_method(&self) -> &str {
        &self.handler_data.method
    }

    pub fn call_info(&self) -> CallInfo {
        CallInfo::brp(
            self.request.name.to_string(),
            self.handler_data.method.clone(),
            self.handler_data.port,
        )
    }

    // BRP-specific extraction methods

    /// Extract entity ID from MCP tool call parameters
    pub fn entity_id(&self) -> Option<u64> {
        self.extract_optional_named_field(PARAM_ENTITY)
            .and_then(Value::as_u64)
    }

    /// Extract entity ID parameter (required)
    pub fn get_entity_id(&self) -> Result<u64, McpError> {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        self.entity_id().ok_or_else(|| {
            report_to_mcp_error(
                &error_stack::Report::new(ServiceError::InvalidArgument(
                    "Missing entity ID parameter".to_string(),
                ))
                .attach_printable("Field name: entity")
                .attach_printable("Expected: u64 number"),
            )
        })
    }
}
