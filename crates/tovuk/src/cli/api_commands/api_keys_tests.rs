use serde_json::json;

use crate::cli::args::CliOptions;

use super::api_key_create_body;

#[test]
fn api_key_create_body_uses_trimmed_name() {
    let cli = CliOptions {
        command: "api-key".to_owned(),
        args: vec![
            "create".to_owned(),
            " Production ".to_owned(),
            " scraper ".to_owned(),
        ],
        ..CliOptions::default()
    };

    assert_eq!(
        api_key_create_body(&cli).ok(),
        Some(json!({ "name": "Production scraper" }))
    );
}

#[test]
fn api_key_create_body_requires_name() {
    let cli = CliOptions {
        command: "api-key".to_owned(),
        args: vec!["create".to_owned(), " ".to_owned()],
        ..CliOptions::default()
    };

    let message = api_key_create_body(&cli)
        .err()
        .map(|error| error.to_string());
    assert_eq!(message.as_deref(), Some("API key name is required."));
}
