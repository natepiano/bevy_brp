mod brp_client;
mod brp_execute;
mod constants;
mod format_discovery;
pub mod handler;
mod http_client;
mod json_rpc_builder;
pub mod tools;
mod watch_tools;

// Public exports
pub use brp_client::{BrpError, BrpResult, build_brp_url, execute_brp_method};
// Export special case tools that don't follow the standard pattern
pub use brp_execute::{BrpExecute, ExecuteParams};
pub use constants::{
    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE, BRP_PORT_ENV_VAR, FormatCorrectionField,
    JSON_RPC_ERROR_METHOD_NOT_FOUND, VALID_PORT_RANGE, default_port,
};
pub use format_discovery::{FormatCorrection, FormatCorrectionStatus};
use json_rpc_builder::BrpJsonRpcBuilder;
pub use watch_tools::WatchInfo;
// Export parameter structs from the parameters module

// Export watch tools
pub use watch_tools::bevy_get_watch::{BevyGetWatch, GetWatchParams};
pub use watch_tools::bevy_list_watch::{BevyListWatch, ListWatchParams};
pub use watch_tools::brp_list_active::BrpListActiveWatches;
pub use watch_tools::brp_stop_watch::{BrpStopWatch, StopWatchParams};
pub use watch_tools::manager::initialize_watch_manager;
