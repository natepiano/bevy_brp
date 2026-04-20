mod brp_client;
mod brp_type_guide;
mod constants;
mod mouse;
mod port;
mod tools;
mod watch_tools;

// Public exports
//
// We export `JSON_RPC_ERROR_METHOD_NOT_FOUND` so that the `brp_shutdown` tool can determine if
// `brp_mcp_extras` is available
pub use brp_client::BrpClient;
pub use brp_client::BrpToolConfig;
pub use brp_client::FormatCorrectionStatus;
pub use brp_client::JSON_RPC_ERROR_METHOD_NOT_FOUND;
pub use brp_client::ResponseStatus;
pub use brp_client::ResultStructBrpExt;
//
// Export brp_type_guide tools
pub use brp_type_guide::AllTypeGuidesParams;
pub use brp_type_guide::BrpAllTypeGuides;
pub use brp_type_guide::BrpTypeGuide;
pub use brp_type_guide::BrpTypeName;
pub use brp_type_guide::TypeGuideParams;
pub use constants::BRP_EXTRAS_PORT_ENV_VAR;
pub use constants::MAX_VALID_PORT;
pub use port::Port;
//
// Export all tool parameter and result structs via the tools facade
pub use tools::BrpExecute;
pub use tools::ClickMouseParams;
pub use tools::ClickMouseResult;
pub use tools::DespawnEntityParams;
pub use tools::DespawnEntityResult;
pub use tools::DoubleClickMouseParams;
pub use tools::DoubleClickMouseResult;
pub use tools::DoubleTapGestureParams;
pub use tools::DoubleTapGestureResult;
pub use tools::DragMouseParams;
pub use tools::DragMouseResult;
pub use tools::ExecuteParams;
pub use tools::GetComponentsParams;
pub use tools::GetComponentsResult;
pub use tools::GetDiagnosticsParams;
pub use tools::GetDiagnosticsResult;
pub use tools::GetResourcesParams;
pub use tools::GetResourcesResult;
pub use tools::InsertComponentsParams;
pub use tools::InsertComponentsResult;
pub use tools::InsertResourcesParams;
pub use tools::InsertResourcesResult;
pub use tools::ListComponentsParams;
pub use tools::ListComponentsResult;
pub use tools::ListResourcesParams;
pub use tools::ListResourcesResult;
pub use tools::MoveMouseParams;
pub use tools::MoveMouseResult;
pub use tools::MutateComponentsParams;
pub use tools::MutateComponentsResult;
pub use tools::MutateResourcesParams;
pub use tools::MutateResourcesResult;
pub use tools::PinchGestureParams;
pub use tools::PinchGestureResult;
pub use tools::QueryParams;
pub use tools::QueryResult;
pub use tools::RegistrySchemaParams;
pub use tools::RegistrySchemaResult;
pub use tools::RemoveComponentsParams;
pub use tools::RemoveComponentsResult;
pub use tools::RemoveResourcesParams;
pub use tools::RemoveResourcesResult;
pub use tools::ReparentEntitiesParams;
pub use tools::ReparentEntitiesResult;
pub use tools::RotationGestureParams;
pub use tools::RotationGestureResult;
pub use tools::RpcDiscoverParams;
pub use tools::RpcDiscoverResult;
pub use tools::ScreenshotParams;
pub use tools::ScreenshotResult;
pub use tools::ScrollMouseParams;
pub use tools::ScrollMouseResult;
pub use tools::SendKeysParams;
pub use tools::SendKeysResult;
pub use tools::SendMouseButtonParams;
pub use tools::SendMouseButtonResult;
pub use tools::SetWindowTitleParams;
pub use tools::SetWindowTitleResult;
pub use tools::SpawnEntityParams;
pub use tools::SpawnEntityResult;
pub use tools::TriggerEventParams;
pub use tools::TriggerEventResult;
pub use tools::TypeTextParams;
pub use tools::TypeTextResult;
//
// Export watch tools
pub use watch_tools::BevyListWatch;
pub use watch_tools::BrpListActiveWatches;
pub use watch_tools::BrpStopWatch;
pub use watch_tools::GetComponentsWatchParams;
pub use watch_tools::ListComponentsWatchParams;
pub use watch_tools::StopWatchParams;
pub use watch_tools::WorldGetComponentsWatch;
