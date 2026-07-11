use super::parse_args;

#[test]
/// Verifies account activity rejects pagination flags.
///
/// # Panics
///
/// Panics when an unsupported pagination flag is accepted.
fn account_activity_rejects_pagination_flags() {
    let parsed = parse_args(&args(&["account", "activity", "--limit", "20"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| return error.payload().code() == "unknown_argument")
    );
}

#[test]
/// Verifies the API base URL cannot be overridden through public CLI input.
///
/// # Panics
///
/// Panics when the private override is accepted.
fn api_override_is_not_public_cli_surface() {
    let parsed = parse_args(&args(&["account", "show", "--api=https://example.test"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| return error.payload().code() == "unknown_argument")
    );
}

/// Converts borrowed test arguments into owned CLI input.
fn args(values: &[&str]) -> Vec<String> {
    return values.iter().map(ToString::to_string).collect();
}

#[test]
/// Verifies unsupported output formats are rejected.
///
/// # Panics
///
/// Panics when an unsupported output format is accepted.
fn invalid_output_value_is_rejected() {
    let parsed = parse_args(&args(&["check", "--output", "text"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| return error.payload().code() == "invalid_argument")
    );
}

#[test]
/// Verifies the explicit JSON output option selects machine output.
///
/// # Panics
///
/// Panics when the option is rejected or does not select JSON.
fn output_json_enables_json_output() {
    let parsed = parse_args(&args(&["check", "--output", "json"]));

    assert!(
        parsed
            .as_ref()
            .is_ok_and(|cli| return cli.is_json() && cli.command == "check")
    );
}

#[test]
/// Verifies request listing accepts pagination flags.
///
/// # Panics
///
/// Panics when valid pagination flags are rejected or stored incorrectly.
fn request_list_accepts_pagination_flags() {
    let parsed = parse_args(&args(&[
        "request",
        "list",
        "--limit",
        "20",
        "--cursor",
        "cursor_123",
    ]));

    assert!(
        parsed
            .as_ref()
            .is_ok_and(|cli| return cli.limit == "20" && cli.cursor == "cursor_123")
    );
}

#[test]
/// Verifies retired development-port flags remain unavailable.
///
/// # Panics
///
/// Panics when a retired development flag is accepted.
fn retired_dev_port_flags_are_rejected() {
    let parsed = parse_args(&args(&[
        "dev",
        "--worker-port",
        "3001",
        "--frontend-port=5174",
    ]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| return error.payload().code() == "unknown_argument")
    );
}

#[test]
/// Verifies support creation accepts its contextual flags.
///
/// # Panics
///
/// Panics when valid support context is rejected or stored incorrectly.
fn support_create_accepts_context_flags() {
    let parsed = parse_args(&args(&[
        "support",
        "create",
        "Subject",
        "Details",
        "--request-id",
        "req_123",
        "--severity",
        "urgent",
    ]));

    assert!(
        parsed
            .as_ref()
            .is_ok_and(|cli| return cli.request_id == "req_123" && cli.severity == "urgent")
    );
}

#[test]
/// Verifies support listing rejects creation-only context flags.
///
/// # Panics
///
/// Panics when a creation-only support flag is accepted.
fn support_list_rejects_create_context_flags() {
    let parsed = parse_args(&args(&["support", "list", "--severity", "urgent"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| return error.payload().code() == "unknown_argument")
    );
}

#[test]
/// Verifies explicit tokens remain available to stateless API scripts.
///
/// # Panics
///
/// Panics when a valid token is rejected or stored incorrectly.
fn token_is_global_for_stateless_api_scripts() {
    let parsed = parse_args(&args(&["request", "list", "--token", "tvk_test"]));

    assert!(
        parsed
            .as_ref()
            .is_ok_and(|cli| return cli.command == "request" && cli.token == "tvk_test")
    );
}
