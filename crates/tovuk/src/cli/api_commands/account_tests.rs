use serde_json::json;

use crate::cli::args::CliOptions;

use super::{ACCOUNT_ACTIVITY_PATH, account_update_body};

#[test]
fn account_update_prefers_explicit_flags() {
    let mut cli = CliOptions {
        command: "account".to_owned(),
        args: vec!["update".to_owned(), "ignored".to_owned()],
        ..CliOptions::default()
    };
    cli.account.handle = "tovuk-team".to_owned();
    cli.account.display_name = "Tovuk Team".to_owned();

    assert_eq!(
        account_update_body(&cli).ok(),
        Some(json!({ "handle": "tovuk-team", "displayName": "Tovuk Team" }))
    );
}

#[test]
fn account_update_accepts_positional_handle_and_name() {
    let cli = CliOptions {
        command: "account".to_owned(),
        args: vec![
            "update".to_owned(),
            "tovuk-team".to_owned(),
            "Tovuk".to_owned(),
            "Team".to_owned(),
        ],
        ..CliOptions::default()
    };

    assert_eq!(
        account_update_body(&cli).ok(),
        Some(json!({ "handle": "tovuk-team", "displayName": "Tovuk Team" }))
    );
}

#[test]
fn account_update_requires_handle() {
    let cli = CliOptions {
        command: "account".to_owned(),
        args: vec!["update".to_owned()],
        ..CliOptions::default()
    };

    let message = account_update_body(&cli)
        .err()
        .map(|error| error.to_string());
    assert_eq!(message.as_deref(), Some("Account handle is required."));
}

#[test]
fn account_activity_uses_consolidated_account_route() {
    assert_eq!(ACCOUNT_ACTIVITY_PATH, "/v1/account/activity");
    let retired_path = format!("/v1/{}", "activity");
    assert_ne!(ACCOUNT_ACTIVITY_PATH, retired_path);
}
