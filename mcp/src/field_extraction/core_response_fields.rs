//! Core response field names that are universal to all tool responses

use strum::{Display, EnumString, IntoStaticStr};

/// Enum representing core response field names that appear in all tool responses
#[derive(Display, EnumString, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum CoreResponseField {
    /// `status` - Response status (success/error)
    Status,
    /// `message` - Response message
    Message,
}