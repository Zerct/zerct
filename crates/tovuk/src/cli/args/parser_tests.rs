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
fn output_text_overrides_json_flag() {
    let parsed = parse_args(&args(&["check", "--json", "--output=text"]));

    assert!(parsed.as_ref().is_ok_and(|cli| !cli.output.json));
}

#[test]
fn invalid_output_value_is_rejected() {
    let parsed = parse_args(&args(&["check", "--output", "yaml"]));

    assert!(
        parsed
            .as_ref()
            .is_err_and(|error| error.payload().code == "invalid_argument")
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
