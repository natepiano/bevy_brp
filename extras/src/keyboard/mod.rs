//! Keyboard input simulation for BRP extras

mod constants;
mod events;
mod key_code;
mod keys;
mod typing;

pub(crate) use self::keys::process_timed_key_releases;
pub(crate) use self::keys::send_keys_handler;
pub(crate) use self::typing::process_text_typing;
pub(crate) use self::typing::type_text_handler;

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests should panic on unexpected values"
)]
#[allow(
    clippy::unwrap_used,
    reason = "tests should panic on unexpected values"
)]
#[allow(clippy::panic, reason = "tests should panic on unexpected values")]
mod tests {
    use bevy::app::App;
    use bevy::prelude::In;
    use bevy::prelude::MinimalPlugins;
    use bevy_remote::error_codes::INVALID_PARAMS;
    use serde_json::json;
    use strum::IntoEnumIterator;

    use super::constants::DEFAULT_KEY_DURATION_MS;
    use super::constants::MAX_KEY_DURATION_MS;
    use super::key_code::KeyCodeWrapper;
    use super::keys::SendKeysResponse;
    use super::keys::TimedKeyRelease;
    use super::send_keys_handler;

    #[test]
    fn test_duration_validation_exceeds_maximum() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request with duration exceeding the maximum
        let params = json!({
            "keys": ["KeyA"],
            "duration_ms": 70_000  // 70 seconds, exceeds 60 second maximum
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it returns an error
        assert!(result.is_err());

        let error = result.expect_err("Expected an error but got success");
        assert_eq!(error.code, INVALID_PARAMS);
        assert!(error.message.contains("exceeds maximum allowed duration"));
        assert!(error.message.contains(&format!("{MAX_KEY_DURATION_MS}ms")));
    }

    #[test]
    fn test_duration_validation_within_maximum() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request with duration within the maximum
        let params = json!({
            "keys": ["KeyA"],
            "duration_ms": 30_000  // 30 seconds, within 60 second maximum
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it succeeds
        assert!(result.is_ok());

        let response = result.expect("Expected success but got error");
        assert_eq!(response["success"], true);
        assert_eq!(response["duration_ms"], 30_000);
    }

    #[test]
    fn test_default_duration() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request without specifying duration_ms
        let params = json!({
            "keys": ["KeyA", "KeyB", "Space"]
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it succeeds
        assert!(result.is_ok());

        let response = result.expect("Expected success but got error");
        assert_eq!(response["success"], true);
        assert_eq!(response["duration_ms"], DEFAULT_KEY_DURATION_MS);
        assert_eq!(response["keys_sent"], json!(["KeyA", "KeyB", "Space"]));
    }

    #[test]
    fn test_zero_duration() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request with zero duration (should be valid)
        let params = json!({
            "keys": ["Enter"],
            "duration_ms": 0
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it succeeds
        assert!(result.is_ok());

        let response = result.expect("Expected success but got error");
        assert_eq!(response["success"], true);
        assert_eq!(response["duration_ms"], 0);
    }

    /// Test that all key code variants can be parsed
    #[test]
    fn test_parse_all_key_codes() {
        // Test that all keys can be successfully used in a send_keys request
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Iterate over all key code variants
        for key_wrapper in KeyCodeWrapper::iter() {
            let key = key_wrapper.to_string();
            let params = json!({
                "keys": [&key]
            });

            let result = send_keys_handler(In(Some(params)), app.world_mut());

            assert!(result.is_ok(), "Failed to parse key code: {key}");

            if let Ok(response_value) = result {
                let response: SendKeysResponse =
                    serde_json::from_value(response_value).expect("Failed to deserialize response");
                assert!(response.success);
                assert_eq!(response.keys_sent.len(), 1);
                assert_eq!(response.keys_sent[0], key);
                assert_eq!(response.duration_ms, DEFAULT_KEY_DURATION_MS);
            }
        }
    }

    /// Test invalid key codes return appropriate errors
    #[test]
    fn test_invalid_key_codes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let invalid_keys = vec![
            "InvalidKey",
            "Key1",  // Should be Digit1
            "Ctrl",  // Should be ControlLeft or ControlRight
            "Shift", // Should be ShiftLeft or ShiftRight
            "F25",   // Function keys only go up to F24
            "",
            "key a", // lowercase and space
            "KEY_A", // Wrong format
        ];

        for invalid_key in invalid_keys {
            let params = json!({
                "keys": [invalid_key]
            });

            let result = send_keys_handler(In(Some(params)), app.world_mut());

            assert!(
                result.is_err(),
                "Expected error for invalid key: {invalid_key}"
            );
        }
    }

    /// Test press-hold-release cycle with different durations
    #[test]
    fn test_press_hold_release_cycle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with default duration
        let default_params = json!({
            "keys": ["Space", "Enter"]
        });

        let default_result = send_keys_handler(In(Some(default_params)), app.world_mut());

        assert!(default_result.is_ok());
        if let Ok(response_value) = default_result {
            let response: SendKeysResponse =
                serde_json::from_value(response_value).expect("Failed to deserialize response");
            assert_eq!(response.duration_ms, DEFAULT_KEY_DURATION_MS);
            assert_eq!(response.keys_sent.len(), 2);
        }

        // Test with custom duration
        let custom_params = json!({
            "keys": ["Space", "Enter"],
            "duration_ms": 500
        });

        let custom_result = send_keys_handler(In(Some(custom_params)), app.world_mut());

        assert!(custom_result.is_ok());
        if let Ok(response_value) = custom_result {
            let response: SendKeysResponse =
                serde_json::from_value(response_value).expect("Failed to deserialize response");
            assert_eq!(response.duration_ms, 500);
            assert_eq!(response.keys_sent.len(), 2);
        }
    }

    /// Test missing parameters returns appropriate error
    #[test]
    fn test_missing_parameters() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let result = send_keys_handler(In(None), app.world_mut());

        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.message, "Missing request parameters");
        }
    }

    /// Test empty key array
    #[test]
    fn test_empty_keys() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let params = json!({
            "keys": []
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());
        if let Ok(response_value) = result {
            let response: SendKeysResponse =
                serde_json::from_value(response_value).expect("Failed to deserialize response");
            assert_eq!(response.keys_sent.len(), 0);
        }
    }

    /// Test that `TimedKeyRelease` component is always created
    #[test]
    fn test_timed_release_always_created() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with custom duration
        let params = json!({
            "keys": ["Space", "Enter"],
            "duration_ms": 500
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());

        // Check that a TimedKeyRelease component was created
        let mut query = app.world_mut().query::<&TimedKeyRelease>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1, "Expected one TimedKeyRelease component");

        // Verify the component has the correct keys
        if let Some(timed_release) = query.iter(app.world()).next() {
            assert_eq!(timed_release.keys.len(), 2);
        }
    }

    /// Test default duration creates `TimedKeyRelease` component
    #[test]
    fn test_default_duration_creates_timed_release() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with default duration (no duration_ms specified)
        let params = json!({
            "keys": ["Space"]
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());

        // Check that a TimedKeyRelease component was created with default duration
        let mut query = app.world_mut().query::<&TimedKeyRelease>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "Expected one TimedKeyRelease component with default duration"
        );
    }

    /// Test that empty key array does not create `TimedKeyRelease`
    #[test]
    fn test_empty_keys_no_timed_release() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with empty keys array
        let params = json!({
            "keys": [],
            "duration_ms": 500
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());

        // Check that no TimedKeyRelease component was created
        let mut query = app.world_mut().query::<&TimedKeyRelease>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 0,
            "Expected no TimedKeyRelease components when keys array is empty"
        );
    }
}
