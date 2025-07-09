use serde_json::Value;

/// Result of parameter extraction
pub struct ExtractedParams {
    /// The method name for dynamic handlers, None for static
    pub method: Option<String>,
    /// The extracted parameters
    pub params: Option<Value>,
    /// The BRP port to use
    pub port:   u16,
}

// Helper methods were removed as they were never used
// If needed in the future, they can be re-added when actually used
