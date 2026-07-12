//! Screenshot wire request decoding and immutable request identity.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde_json::Value;

use super::capture::CaptureIdentity;
use super::capture::CaptureToken;
use super::capture::RequestFingerprint;

#[derive(Clone, Debug)]
pub(super) struct ScreenshotRequest {
    path:        PathBuf,
    fingerprint: RequestFingerprint,
    identity:    CaptureIdentity,
}

impl ScreenshotRequest {
    pub(super) fn from_params(params: Option<Value>) -> BrpResult<Self> {
        let value = params.ok_or_else(missing_path_error)?;
        let raw =
            serde_json::from_value::<RawScreenshotRequest>(value).map_err(|error| BrpError {
                code:    INVALID_PARAMS,
                message: format!("Invalid screenshot request: {error}"),
                data:    None,
            })?;

        let raw_path = raw.path.ok_or_else(missing_path_error)?;
        let path = absolute_path(&raw_path)?;
        let fingerprint = RequestFingerprint::from(path.clone());
        let identity = match raw.capture_id {
            Some(capture_id) => CaptureIdentity::Token(CaptureToken::try_from(capture_id)?),
            None => CaptureIdentity::Legacy(fingerprint.clone()),
        };

        Ok(Self {
            path,
            fingerprint,
            identity,
        })
    }

    pub(super) fn into_parts(self) -> (PathBuf, RequestFingerprint, CaptureIdentity) {
        (self.path, self.fingerprint, self.identity)
    }

    pub(super) const fn fingerprint(&self) -> &RequestFingerprint { &self.fingerprint }

    pub(super) const fn identity(&self) -> &CaptureIdentity { &self.identity }

    #[cfg(test)]
    pub(super) fn with_fingerprint(mut self, fingerprint: RequestFingerprint) -> Self {
        self.fingerprint = fingerprint;
        self
    }
}

#[derive(Deserialize)]
struct RawScreenshotRequest {
    capture_id: Option<String>,
    path:       Option<String>,
}

fn absolute_path(path: &str) -> BrpResult<PathBuf> {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| BrpError {
                code:    INTERNAL_ERROR,
                message: format!("Failed to get current directory: {error}"),
                data:    None,
            })?
            .join(path)
    };

    Ok(normalize_path(&absolute))
}

fn missing_path_error() -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: "Missing 'path' parameter".to_string(),
        data:    None,
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {},
            Component::ParentDir => {
                normalized.pop();
            },
            Component::Normal(segment) => normalized.push(segment),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn equivalent_paths_have_the_same_legacy_identity() {
        let first = ScreenshotRequest::from_params(Some(json!({ "path": "images/../shot.png" })));
        let second = ScreenshotRequest::from_params(Some(json!({ "path": "shot.png" })));

        assert!(matches!(
            (first, second),
            (Ok(first), Ok(second)) if first.identity == second.identity
        ));
    }

    #[test]
    fn capture_id_must_be_nonempty_and_bounded() {
        let empty = ScreenshotRequest::from_params(Some(json!({
            "capture_id": "",
            "path": "shot.png"
        })));
        let oversized = ScreenshotRequest::from_params(Some(json!({
            "capture_id": "x".repeat(crate::constants::MAX_SCREENSHOT_CAPTURE_ID_BYTES + 1),
            "path": "shot.png"
        })));

        assert!(empty.is_err());
        assert!(oversized.is_err());
    }
}
