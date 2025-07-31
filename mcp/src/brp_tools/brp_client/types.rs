//! Common types for BRP tools

use crate::error::Result;

/// Execution mode for BRP tool responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecuteMode {
    /// Use format discovery for enhanced error handling
    WithFormatDiscovery,
    /// Standard processing without format discovery
    Standard,
}

/// Extension trait for `ResultStruct` types that handle BRP responses
pub trait ResultStructBrpExt: Sized {
    type Args;

    /// Determine the execution mode for this tool
    fn brp_tool_execute_mode() -> ExecuteMode;

    /// Construct from BRP client response
    fn from_brp_client_response(args: Self::Args) -> Result<Self>;
}
