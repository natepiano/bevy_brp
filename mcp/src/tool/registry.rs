use strum::IntoEnumIterator;

use super::ToolDef;
use super::ToolName;

/// Visibility facade implementation for whole-registry tool definition assembly.
///
/// This is kept separate from `ToolName` so callers can depend on the `tool`
/// subsystem boundary rather than on enum-owned registry construction.
pub(super) fn get_all_tool_definitions() -> Vec<ToolDef> {
    ToolName::iter().map(ToolName::to_tool_def).collect()
}
