use super::parse_args;

fn args(values: &[&str]) -> Vec<String> {
    values
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}

#[test]
fn output_json_enables_json_output() {
    let parsed = parse_args(&args(&["check", "--output", "json"]));

    assert!(
        parsed
            .as_ref()
            .is_ok_and(|cli| cli.output.json && cli.command == "check")
    );
}

#[test]
fn invalid_output_value_is_rejected() {
    let parsed = parse_args(&args(&["check", "--output", "text"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| error.payload().code == "invalid_argument")
    );
}

#[test]
fn api_override_is_not_public_cli_surface() {
    let parsed = parse_args(&args(&["account", "show", "--api=https://example.test"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| error.payload().code == "unknown_argument")
    );
}

#[test]
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
            .is_err_and(|error| error.payload().code == "unknown_argument")
    );
}

#[test]
fn account_activity_rejects_pagination_flags() {
    let parsed = parse_args(&args(&["account", "activity", "--limit", "20"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| error.payload().code == "unknown_argument")
    );
}

#[test]
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
            .is_ok_and(|cli| cli.limit == "20" && cli.cursor == "cursor_123")
    );
}

#[test]
fn token_is_global_for_stateless_api_scripts() {
    let parsed = parse_args(&args(&["request", "list", "--token", "tvk_test"]));

    assert!(
        parsed
            .as_ref()
            .is_ok_and(|cli| cli.command == "request" && cli.token == "tvk_test")
    );
}

#[test]
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
            .is_ok_and(|cli| { cli.request_id == "req_123" && cli.severity == "urgent" })
    );
}

#[test]
fn support_list_rejects_create_context_flags() {
    let parsed = parse_args(&args(&["support", "list", "--severity", "urgent"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| error.payload().code == "unknown_argument")
    );
}
