use serde_json::json;

use crate::cli::args::CliOptions;

use super::{request_input, request_input_source};

#[test]
fn request_create_requires_input_json() {
    let cli = CliOptions {
        command: "request".to_owned(),
        args: vec!["create".to_owned(), "github".to_owned()],
        ..CliOptions::default()
    };

    let message = request_input_source(&cli)
        .err()
        .map(|error| error.to_string());
    assert_eq!(message.as_deref(), Some("Request input JSON is required."));
}

#[test]
fn request_input_accepts_json_object_and_applies_limit_flag() {
    let cli = CliOptions {
        limit: "25".to_owned(),
        ..CliOptions::default()
    };

    assert_eq!(
        request_input(&cli, r#"{"query":"coffee shops"}"#).ok(),
        Some(json!({
            "query": "coffee shops",
            "limit": 25
        }))
    );
}

#[test]
fn request_input_rejects_non_object_json() {
    let message = request_input(&CliOptions::default(), "[]")
        .err()
        .map(|error| error.to_string());
    assert_eq!(
        message.as_deref(),
        Some("Request input must be a JSON object.")
    );
}
