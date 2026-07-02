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
