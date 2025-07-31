use serde_json::{Value, json};

use super::constants::{JSONRPC_DEFAULT_ID, JSONRPC_FIELD_PARAMS};
use crate::brp_tools::brp_client::constants::{
    JSONRPC_FIELD, JSONRPC_FIELD_ID, JSONRPC_FIELD_METHOD, JSONRPC_VERSION,
};

/// Builder for constructing raw JSON-RPC 2.0 requests
///
/// This builder is used for direct HTTP communication with BRP,
/// primarily by the `check_brp` tool.
pub struct BrpJsonRpcBuilder {
    method: String,
    params: Option<Value>,
    id:     u64,
}

impl BrpJsonRpcBuilder {
    /// Create a new JSON-RPC request builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
            id:     JSONRPC_DEFAULT_ID,
        }
    }

    /// Set raw params
    pub fn params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Build the final JSON-RPC request
    pub fn build(self) -> Value {
        let mut request = json!({
            JSONRPC_FIELD: JSONRPC_VERSION,
            JSONRPC_FIELD_METHOD: self.method,
            JSONRPC_FIELD_ID: self.id
        });

        if let Some(params) = self.params {
            request[JSONRPC_FIELD_PARAMS] = params;
        } else {
            request[JSONRPC_FIELD_PARAMS] = json!(null);
        }

        request
    }
}
