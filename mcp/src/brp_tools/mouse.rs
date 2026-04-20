//! Mouse input wrappers used by the `brp_extras/*_mouse` tools.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Mouse button for BRP operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum MouseButtonWrapper {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button (wheel click)
    Middle,
    /// Back navigation button
    Back,
    /// Forward navigation button
    Forward,
}

/// Scroll unit for BRP scroll operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum ScrollUnitWrapper {
    /// Line-based scrolling
    Line,
    /// Pixel-based scrolling
    Pixel,
}
