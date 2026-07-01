use crate::cli::args::CliOptions;

use super::{ACCOUNT_ACTIVITY_PATH, account_command};

#[test]
fn account_activity_uses_consolidated_account_route() {
    assert_eq!(ACCOUNT_ACTIVITY_PATH, "/v1/account/activity");
    let retired_path = format!("/v1/{}", "activity");
    assert_ne!(ACCOUNT_ACTIVITY_PATH, retired_path);
}

#[test]
fn account_update_command_is_not_public_surface() {
    let cli = CliOptions {
        command: "account".to_owned(),
        args: vec!["update".to_owned()],
        ..CliOptions::default()
    };

    let message = account_command(&cli).err().map(|error| error.to_string());
    assert_eq!(message.as_deref(), Some("Unknown account command."));
}
