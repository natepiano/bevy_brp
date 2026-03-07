use strum::IntoEnumIterator;

use super::ToolDef;
use super::ToolName;

pub(super) fn get_all_tool_definitions() -> Vec<ToolDef> {
    ToolName::iter().map(ToolName::to_tool_def).collect()
}
