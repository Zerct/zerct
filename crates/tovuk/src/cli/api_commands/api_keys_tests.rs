use serde_json::json;

use crate::cli::args::options_for_test;

use super::api_key_create_body;

#[test]
/// Verifies API-key creation rejects an empty name.
///
/// # Panics
///
/// Panics when an empty API-key name is accepted or reports a different error.
fn api_key_create_body_requires_name() {
    let cli = options_for_test(&["api-key", "create", " "]);

    let message = api_key_create_body(&cli)
        .err()
        .map(|error| return error.message().to_owned());
    assert_eq!(message.as_deref(), Some("API key name is required."));
}

#[test]
/// Verifies API-key names are normalized before serialization.
///
/// # Panics
///
/// Panics when the serialized name does not match the normalized input.
fn api_key_create_body_uses_trimmed_name() {
    let cli = options_for_test(&["api-key", "create", " Production ", " scraper "]);

    assert_eq!(
        api_key_create_body(&cli).ok(),
        Some(json!({ "name": "Production scraper" }))
    );
}
