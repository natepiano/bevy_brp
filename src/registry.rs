use rmcp::model::{CallToolRequestParam, CallToolResult, ListToolsResult};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use crate::app_tools::brp_extras_shutdown;
use crate::brp_tools::{brp_set_debug_mode, brp_status, watch};
// Imports removed - using fully qualified paths in match statement to avoid naming conflicts
use crate::error::{Error, report_to_mcp_error};
use crate::{BrpMcpService, tool_definitions, tool_generator};

pub fn register_tools() -> ListToolsResult {
    let mut tools = vec![];

    // Generate tools from declarative definitions
    for def in tool_definitions::get_all_tools() {
        tools.push(tool_generator::generate_tool_registration(&def));
    }

    // Add remaining tools that don't follow simple request/response
    tools.extend(vec![
        // Core BRP tools (with custom logic)
        brp_status::register_tool(),
        // bevy_brp_extras tools
        brp_extras_shutdown::register_tool(),
        // Streaming/watch tools (custom logic)
        watch::bevy_get_watch::register_tool(),
        watch::bevy_list_watch::register_tool(),
        watch::brp_stop_watch::register_tool(),
        watch::brp_list_active::register_tool(),
        // Debug tools
        brp_set_debug_mode::register_tool(),
    ]);

    // Sort all tools alphabetically by name for consistent ordering
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    ListToolsResult {
        next_cursor: None,
        tools,
    }
}

pub async fn handle_tool_call(
    service: &BrpMcpService,
    request: CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    // Check if this is one of the declaratively defined tools
    let all_tools = tool_definitions::get_all_tools();
    if let Some(def) = all_tools.iter().find(|d| d.name == request.name) {
        return tool_generator::generate_tool_handler(def, service, request, context).await;
    }

    // Handle remaining tools
    match request.name.as_ref() {
        // Core BRP tools (with custom logic)
        name if name == crate::tools::TOOL_BRP_STATUS => {
            brp_status::handle(service, request, context).await
        }

        // bevy_brp_extras tools
        name if name == crate::tools::TOOL_BRP_EXTRAS_SHUTDOWN => {
            brp_extras_shutdown::handle(service, request, context).await
        }

        // Streaming/watch tools (custom logic)
        name if name == crate::tools::TOOL_BEVY_GET_WATCH => {
            watch::bevy_get_watch::handle(service, request, context).await
        }
        name if name == crate::tools::TOOL_BEVY_LIST_WATCH => {
            watch::bevy_list_watch::handle(service, request, context).await
        }
        name if name == crate::tools::TOOL_BRP_STOP_WATCH => {
            watch::brp_stop_watch::handle(service, request, context).await
        }
        name if name == crate::tools::TOOL_BRP_LIST_ACTIVE_WATCHES => {
            watch::brp_list_active::handle(service, request, context).await
        }

        // Debug tools
        name if name == crate::tools::TOOL_BRP_SET_DEBUG_MODE => {
            brp_set_debug_mode::handle_set_debug_mode(service, request, context).await
        }

        _ => {
            let tool_name = &request.name;
            Err(report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(format!(
                    "unknown tool: {tool_name}"
                )))
                .attach_printable("Tool not found in registry"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_are_registered_in_alphabetical_order() {
        let result = register_tools();

        // Check that tools are sorted alphabetically
        let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_ref()).collect();
        let mut sorted_names = tool_names.clone();
        sorted_names.sort_unstable();

        assert_eq!(
            tool_names, sorted_names,
            "Tools are not in alphabetical order. Expected: {sorted_names:?}, Got: {tool_names:?}"
        );

        // Also verify we have a reasonable number of tools registered
        let len = result.tools.len();
        assert!(len > 20, "Expected at least 20 tools, got {len}");
    }

    #[test]
    fn test_tool_names_have_proper_prefixes() {
        let result = register_tools();

        for tool in &result.tools {
            let name = &tool.name;
            assert!(
                name.starts_with("bevy_")
                    || name.starts_with("brp_")
                    || name.starts_with("brp_extras_"),
                "Tool '{name}' does not have a proper prefix (bevy_, brp_, or brp_extras_)"
            );
        }
    }
}
