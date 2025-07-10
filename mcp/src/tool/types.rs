/// Metadata about a BRP request for response formatting
#[derive(Debug, Clone)]
pub struct BrpToolCallInfo {
    pub tool_name: String,
    pub port:      u16,
}

impl BrpToolCallInfo {
    pub fn new(tool_name: &str, port: u16) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            port,
        }
    }
}
