//! Screenshot wire request decoding and immutable request identity.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::Entity;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde_json::Value;

use super::capture;
use super::capture::CaptureIdentity;
use super::capture::CaptureToken;
use super::capture::RequestFingerprint;
use crate::constants::SCREENSHOT_ZERO_PADDING;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ScreenshotScope {
    Full,
    Entity {
        entity:  Entity,
        camera:  Option<Entity>,
        padding: u32,
    },
}

#[derive(Clone, Debug)]
pub(super) struct ScreenshotRequest {
    path:        PathBuf,
    fingerprint: RequestFingerprint,
    identity:    CaptureIdentity,
    scope:       ScreenshotScope,
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

        let scope = ScreenshotScope::try_from(&raw)?;
        let raw_path = raw.path.ok_or_else(missing_path_error)?;
        let path = absolute_path(&raw_path)?;
        let fingerprint =
            capture::request_fingerprint(path.clone(), raw.camera, raw.entity, raw.padding);
        let identity = match raw.capture_id {
            Some(capture_id) => CaptureIdentity::Token(CaptureToken::try_from(capture_id)?),
            None => CaptureIdentity::Legacy(fingerprint.clone()),
        };
        Ok(Self {
            path,
            fingerprint,
            identity,
            scope,
        })
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        PathBuf,
        RequestFingerprint,
        CaptureIdentity,
        ScreenshotScope,
    ) {
        (self.path, self.fingerprint, self.identity, self.scope)
    }

    pub(super) const fn fingerprint(&self) -> &RequestFingerprint { &self.fingerprint }

    pub(super) const fn identity(&self) -> &CaptureIdentity { &self.identity }

    pub(super) fn path(&self) -> &Path { &self.path }

    pub(super) const fn scope(&self) -> &ScreenshotScope { &self.scope }

    #[cfg(test)]
    pub(super) fn with_fingerprint(mut self, fingerprint: RequestFingerprint) -> Self {
        self.fingerprint = fingerprint;
        self
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScreenshotRequest {
    camera:     Option<u64>,
    capture_id: Option<String>,
    entity:     Option<u64>,
    padding:    Option<u32>,
    path:       Option<String>,
}

impl TryFrom<&RawScreenshotRequest> for ScreenshotScope {
    type Error = BrpError;

    fn try_from(raw: &RawScreenshotRequest) -> Result<Self, Self::Error> {
        match raw.entity {
            Some(entity) => Ok(Self::Entity {
                entity:  decode_entity_id(entity, "entity")?,
                camera:  raw
                    .camera
                    .map(|camera| decode_entity_id(camera, "camera"))
                    .transpose()?,
                padding: raw.padding.unwrap_or(SCREENSHOT_ZERO_PADDING),
            }),
            None if raw.camera.is_some() => Err(entity_scope_field_error("camera")),
            None if raw.padding.is_some() => Err(entity_scope_field_error("padding")),
            None => Ok(Self::Full),
        }
    }
}

fn decode_entity_id(bits: u64, field: &str) -> BrpResult<Entity> {
    Entity::try_from_bits(bits).ok_or_else(|| BrpError {
        code:    INVALID_PARAMS,
        message: format!("Invalid '{field}' entity ID: {bits}"),
        data:    None,
    })
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

fn entity_scope_field_error(field: &str) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!("'{field}' requires an 'entity' screenshot scope"),
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

    #[test]
    fn full_and_entity_fields_convert_to_typed_scopes() {
        let full = ScreenshotRequest::from_params(Some(json!({ "path": "shot.png" })));
        let entity = ScreenshotRequest::from_params(Some(json!({
            "camera": 9,
            "entity": 7,
            "path": "entity.png"
        })));

        assert!(matches!(full, Ok(request) if request.scope == ScreenshotScope::Full));
        assert!(matches!(
            entity,
            Ok(request)
                if request.scope == ScreenshotScope::Entity {
                    entity: Entity::from_bits(7),
                    camera: Some(Entity::from_bits(9)),
                    padding: SCREENSHOT_ZERO_PADDING,
                }
        ));
    }

    #[test]
    fn entity_scope_preserves_explicit_padding() {
        let request = ScreenshotRequest::from_params(Some(json!({
            "entity": 7,
            "padding": 12,
            "path": "entity.png"
        })));

        assert!(matches!(
            request,
            Ok(request)
                if request.scope == ScreenshotScope::Entity {
                    entity: Entity::from_bits(7),
                    camera: None,
                    padding: 12,
                }
        ));
    }

    #[test]
    fn camera_and_padding_require_entity_scope() {
        let camera = ScreenshotRequest::from_params(Some(json!({
            "camera": 9,
            "path": "shot.png"
        })));
        let padding = ScreenshotRequest::from_params(Some(json!({
            "padding": 1,
            "path": "shot.png"
        })));

        assert!(camera.is_err());
        assert!(padding.is_err());
    }

    #[test]
    fn invalid_entity_and_camera_bit_patterns_are_field_specific_errors() {
        let invalid_bits = 0;
        let entity = ScreenshotRequest::from_params(Some(json!({
            "entity": invalid_bits,
            "path": "entity.png"
        })));
        let camera = ScreenshotRequest::from_params(Some(json!({
            "camera": invalid_bits,
            "entity": 7,
            "path": "entity.png"
        })));

        assert!(matches!(
            entity,
            Err(error)
                if error.code == INVALID_PARAMS
                    && error.message.contains("'entity'")
                    && error.message.contains(&invalid_bits.to_string())
        ));
        assert!(matches!(
            camera,
            Err(error)
                if error.code == INVALID_PARAMS
                    && error.message.contains("'camera'")
                    && error.message.contains(&invalid_bits.to_string())
        ));
    }

    #[test]
    fn extras_request_rejects_name_selectors() {
        let request = ScreenshotRequest::from_params(Some(json!({
            "name": "Player",
            "path": "shot.png"
        })));

        assert!(request.is_err());
    }

    #[test]
    fn raw_scope_fields_participate_in_the_fingerprint() {
        let omitted = ScreenshotRequest::from_params(Some(json!({
            "entity": 7,
            "path": "shot.png"
        })));
        let explicit_zero = ScreenshotRequest::from_params(Some(json!({
            "entity": 7,
            "padding": 0,
            "path": "shot.png"
        })));

        assert!(matches!(
            (omitted, explicit_zero),
            (Ok(omitted), Ok(explicit_zero)) if omitted.fingerprint != explicit_zero.fingerprint
        ));
    }
}
