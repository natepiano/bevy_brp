//! Agent-facing metadata publication for selected BRP methods.

mod catalog;
mod registration;

pub(crate) use catalog::handler as catalog_handler;
pub use registration::AgentTool;
pub use registration::AppAgentToolExt;
pub(crate) use registration::RegisteredAgentTools;
