//! Named gamepad inputs accepted by the `brp_extras/*_gamepad*` tools.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// A named Bevy gamepad button.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum GamepadButtonWrapper {
    /// South face button.
    South,
    /// East face button.
    East,
    /// North face button.
    North,
    /// West face button.
    West,
    /// C button.
    C,
    /// Z button.
    Z,
    /// Left shoulder button.
    LeftTrigger,
    /// Left analog trigger button.
    LeftTrigger2,
    /// Right shoulder button.
    RightTrigger,
    /// Right analog trigger button.
    RightTrigger2,
    /// Select button.
    Select,
    /// Start button.
    Start,
    /// Mode button.
    Mode,
    /// Left stick button.
    LeftThumb,
    /// Right stick button.
    RightThumb,
    /// D-pad up button.
    DPadUp,
    /// D-pad down button.
    DPadDown,
    /// D-pad left button.
    DPadLeft,
    /// D-pad right button.
    DPadRight,
}

/// A named Bevy gamepad axis.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum GamepadAxisWrapper {
    /// Left stick horizontal axis.
    LeftStickX,
    /// Left stick vertical axis.
    LeftStickY,
    /// Left trigger axis.
    LeftZ,
    /// Right stick horizontal axis.
    RightStickX,
    /// Right stick vertical axis.
    RightStickY,
    /// Right trigger axis.
    RightZ,
}
