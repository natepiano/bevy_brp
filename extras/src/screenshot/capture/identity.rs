//! Capture request identity and generation values.

use std::path::PathBuf;

use bevy_remote::BrpError;
use bevy_remote::error_codes::INVALID_PARAMS;

use crate::constants::MAX_SCREENSHOT_CAPTURE_ID_BYTES;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CaptureIdentity {
    Token(CaptureToken),
    Legacy(RequestFingerprint),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CaptureToken(String);

impl TryFrom<String> for CaptureToken {
    type Error = BrpError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(invalid_capture_id("must not be empty"));
        }
        if value.len() > MAX_SCREENSHOT_CAPTURE_ID_BYTES {
            return Err(invalid_capture_id("is too long"));
        }

        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RequestFingerprint {
    path: PathBuf,
}

impl From<PathBuf> for RequestFingerprint {
    fn from(path: PathBuf) -> Self { Self { path } }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(super) struct PathGeneration(pub(super) u64);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd)]
pub(super) struct FrameGeneration(pub(super) u64);

impl FrameGeneration {
    pub(super) const fn next(self) -> Self { Self(self.0.wrapping_add(1)) }
}

fn invalid_capture_id(detail: &str) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!("capture_id {detail}"),
        data:    None,
    }
}
