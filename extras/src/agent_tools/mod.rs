//! Agent-facing metadata publication for selected instant BRP methods.
//!
//! [`AgentTool`] and [`AppAgentToolExt`] publish descriptions and raw schemas for methods already
//! registered in [`RemoteMethods`](bevy_remote::RemoteMethods).
//! [`struct@crate::BrpExtrasPlugin`] installs the `brp_extras/agent_tools` endpoint that lists the
//! curated subset. The endpoint validates the complete set against live instant methods before
//! returning it, so one missing or watching backing method rejects the request without returning
//! partial records.
//!
//! See the complete
//! [agent tool registration example](https://github.com/natepiano/bevy_brp/blob/main/extras/examples/agent_tool_registration.rs).

mod catalog;
mod registration;

pub(crate) use catalog::handler as catalog_handler;
pub use registration::AgentTool;
pub use registration::AppAgentToolExt;
pub(crate) use registration::RegisteredAgentTools;
