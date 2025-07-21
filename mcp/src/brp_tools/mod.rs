// BRP tools module
mod brp_client;
pub mod component;
mod constants;
pub mod discovery;
pub mod dynamic;
pub mod entity;
pub mod extras;
mod format_discovery;
mod handler;
mod http_client;
mod json_rpc_builder;
pub mod resource;
mod watch_tools;

pub use brp_client::{BrpError, BrpResult, build_brp_url, execute_brp_method};
pub use format_discovery::{FORMAT_DISCOVERY_METHODS, FormatCorrection, FormatCorrectionStatus};
use json_rpc_builder::BrpJsonRpcBuilder;
pub use watch_tools::bevy_get_watch::{BevyGetWatch, GetWatchParams};
pub use watch_tools::bevy_list_watch::{BevyListWatch, ListWatchParams};
pub use watch_tools::brp_list_active::BrpListActiveWatches;
pub use watch_tools::brp_stop_watch::{BrpStopWatch, StopWatchParams};
pub use watch_tools::manager::initialize_watch_manager;
