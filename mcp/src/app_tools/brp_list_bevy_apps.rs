use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::support::collection_strategy::BevyAppsStrategy;
use super::support::generic_listing_handler;
use crate::response::ListBevyAppsResult;
use crate::{BrpMcpService, service};

pub async fn handle(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
) -> Result<ListBevyAppsResult, McpError> {
    service::handle_list_binaries_typed(service, context, |search_paths| async move {
        let items = generic_listing_handler::collect_all_items(&search_paths, &BevyAppsStrategy);

        Ok(ListBevyAppsResult {
            count: items.len(),
            apps:  items,
        })
    })
    .await
}
