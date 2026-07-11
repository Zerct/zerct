use crate::cli::args::options_for_test;

use crate::cli::ExecuteCommand as _;

use super::{ACCOUNT_ACTIVITY_PATH, AccountCommand};

#[test]
/// Verifies account activity uses the consolidated public route.
///
/// # Panics
///
/// Panics when the route contract changes.
fn account_activity_uses_consolidated_account_route() {
    assert_eq!(ACCOUNT_ACTIVITY_PATH, "/v1/account/activity");
    let retired_path = format!("/v1/{}", "activity");
    assert_ne!(ACCOUNT_ACTIVITY_PATH, retired_path);
}

#[test]
/// Verifies the retired account-update command is not public.
///
/// # Panics
///
/// Panics when the retired command becomes accepted.
fn account_update_command_is_not_public_surface() {
    let cli = options_for_test(&["account", "update"]);

    let message = AccountCommand
        .execute(&cli)
        .err()
        .map(|error| return error.message().to_owned());
    assert_eq!(message.as_deref(), Some("Unknown account command."));
}
