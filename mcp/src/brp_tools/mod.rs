// BRP tools module
mod brp_client;
mod constants;
mod http_client;
mod json_rpc_builder;
mod request_handler;
mod watch_tools;

pub use brp_client::{BrpError, BrpResult, build_brp_url, execute_brp_method};
use json_rpc_builder::BrpJsonRpcBuilder;
pub use request_handler::{
    BrpMethodHandler, FORMAT_DISCOVERY_METHODS, FormatCorrection, FormatCorrectionStatus,
};
pub use watch_tools::bevy_get_watch::BevyGetWatch;
pub use watch_tools::bevy_list_watch::BevyListWatch;
pub use watch_tools::brp_list_active::BrpListActiveWatches;
pub use watch_tools::brp_stop_watch::BrpStopWatch;
pub use watch_tools::manager::initialize_watch_manager;
