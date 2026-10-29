use std::path::Path;

use serde_json::Value;

use crate::app_tools::constants::WORKSPACE_DEV_FEATURES_KEY;
use crate::app_tools::constants::WORKSPACE_DEV_FEATURES_LABEL;
use crate::app_tools::constants::WORKSPACE_DEV_TABLE;
use crate::error::Error;
use crate::error::Result;

/// Features a workspace asks every non-release launch build to enable, read from
/// `[workspace.metadata.dev] features` in its root `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevFeatures {
    /// The workspace sets no `[workspace.metadata.dev] features` key.
    Unset,
    /// The features the key lists, in order; may be empty.
    Listed(Vec<String>),
    /// The key is present but is not an array of strings; the text says what was found.
    Malformed(String),
}

impl DevFeatures {
    /// Parses `cargo metadata`'s `workspace_metadata` value.
    pub(crate) fn from_workspace_metadata(value: &Value) -> Self {
        let Some(features) = value
            .get(WORKSPACE_DEV_TABLE)
            .and_then(|dev| dev.get(WORKSPACE_DEV_FEATURES_KEY))
        else {
            return Self::Unset;
        };

        let malformed =
            || Self::Malformed(format!("expected an array of strings, found {features}"));
        let Some(values) = features.as_array() else {
            return malformed();
        };

        values
            .iter()
            .map(|value| value.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .map_or_else(malformed, Self::Listed)
    }

    /// The features a non-release build adds: empty for `Unset`, the list for
    /// `Listed`, an error naming `workspace_manifest` and the key for `Malformed`.
    pub(crate) fn for_launch(&self, workspace_manifest: &Path) -> Result<&[String]> {
        match self {
            Self::Unset => Ok(&[]),
            Self::Listed(features) => Ok(features),
            Self::Malformed(found) => Err(Error::InvalidArgument(format!(
                "{}: {WORKSPACE_DEV_FEATURES_LABEL}: {found}",
                workspace_manifest.display()
            ))
            .into()),
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "tests should panic on unexpected values"
)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::DevFeatures;
    use crate::app_tools::constants::WORKSPACE_DEV_FEATURES_LABEL;
    use crate::error::Error;

    #[test]
    fn dev_features_absent_key_is_unset() {
        for metadata in [json!(null), json!({}), json!({"dev": {"other": true}})] {
            let features = DevFeatures::from_workspace_metadata(&metadata);
            assert_eq!(features, DevFeatures::Unset);
            assert_eq!(
                features
                    .for_launch(Path::new("/workspace/Cargo.toml"))
                    .expect("absent dev features must be valid"),
                Vec::<String>::new()
            );
        }
    }

    #[test]
    fn dev_features_preserve_list_order() {
        let features = DevFeatures::from_workspace_metadata(&json!({
            "dev": {"features": ["bevy/dynamic_linking", "app/dev", "bevy/dynamic_linking"]}
        }));
        let expected = ["bevy/dynamic_linking", "app/dev", "bevy/dynamic_linking"];
        assert_eq!(
            features,
            DevFeatures::Listed(expected.map(String::from).to_vec())
        );
        assert_eq!(
            features
                .for_launch(Path::new("/workspace/Cargo.toml"))
                .expect("listed dev features must be valid"),
            expected
        );
    }

    #[test]
    fn dev_features_empty_list_adds_nothing() {
        let features = DevFeatures::from_workspace_metadata(&json!({"dev": {"features": []}}));
        assert_eq!(features, DevFeatures::Listed(Vec::new()));
        assert_eq!(
            features
                .for_launch(Path::new("/workspace/Cargo.toml"))
                .expect("empty dev features must be valid"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn dev_features_malformed_values_name_manifest_key_and_value() {
        for value in [
            json!("bevy/dynamic_linking"),
            json!(42),
            json!(true),
            json!(null),
            json!({"feature": "bevy/dynamic_linking"}),
            json!(["bevy/dynamic_linking", 42]),
        ] {
            let features =
                DevFeatures::from_workspace_metadata(&json!({"dev": {"features": value}}));
            assert!(matches!(features, DevFeatures::Malformed(_)));
            let error = features
                .for_launch(Path::new("/workspace/Cargo.toml"))
                .expect_err("malformed dev features must fail at launch");
            assert!(matches!(error.current_context(), Error::InvalidArgument(_)));
            let message = error.to_string();
            assert!(message.contains("/workspace/Cargo.toml"), "{message}");
            assert!(message.contains(WORKSPACE_DEV_FEATURES_LABEL), "{message}");
            assert!(
                message.contains("expected an array of strings"),
                "{message}"
            );
            assert!(message.contains(&value.to_string()), "{message}");
        }
    }
}
