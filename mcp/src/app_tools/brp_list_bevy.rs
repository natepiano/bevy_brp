use std::path::Path;
use std::path::PathBuf;

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::targets;
use crate::error::Result;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

/// Parameters for listing Bevy targets
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListBevyParams {
    /// Optional OS-level path to use as the search root. Overrides the default MCP workspace
    /// roots.
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub path: Option<String>,
}

/// Result from listing all Bevy targets (apps and examples)
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListBevyResult {
    /// Count of targets found
    #[to_metadata]
    count:            usize,
    /// List of all Bevy targets found (apps and examples)
    #[to_result]
    targets:          Vec<serde_json::Value>,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {count} Bevy targets")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "ListBevyParams", output = "ListBevyResult", with_context)]
pub struct ListBevy;

#[allow(
    clippy::unused_async,
    reason = "ToolFn trait requires async handler signature"
)]
async fn handle_impl(ctx: HandlerContext, params: ListBevyParams) -> Result<ListBevyResult> {
    let search_paths = params
        .path
        .as_ref()
        .map_or(ctx.roots, |path| vec![PathBuf::from(path)]);
    let mut items = targets::collect_all_bevy_targets(&search_paths);

    // When a user-specified path is provided, post-filter to only targets whose
    // manifest directory is under that path. This is needed because cargo metadata
    // resolves workspace members up to the workspace root, expanding the scope.
    if let Some(ref path) = params.path {
        let scope = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
        items.retain(|item| {
            item.get("manifest_path")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|mp| {
                    let manifest_dir = Path::new(mp).parent().unwrap_or_else(|| Path::new(mp));
                    let canonical = std::fs::canonicalize(manifest_dir)
                        .unwrap_or_else(|_| manifest_dir.to_path_buf());
                    canonical.starts_with(&scope)
                })
        });
    }

    Ok(ListBevyResult::new(items.len(), items))
}
