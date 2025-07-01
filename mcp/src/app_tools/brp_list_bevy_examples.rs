use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::support::collection_strategy::BevyExamplesStrategy;
use super::support::generic_listing_handler;
use crate::BrpMcpService;

pub async fn handle(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    generic_listing_handler::handle_listing(service, context, BevyExamplesStrategy).await
}
