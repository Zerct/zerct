use serde_json::json;

use super::{CliOptions, required_positive_number_field};

/// Valid protocol duration used by authentication tests.
const LOGIN_EXPIRY_SECONDS: u64 = 0x0384;
/// Invalid zero duration used by authentication tests.
const ZERO_INTERVAL_SECONDS: u64 = 0x0000;

#[test]
/// Verifies login timing reads the public camel-case protocol field.
///
/// # Panics
///
/// Panics when a valid timing field is not accepted.
fn login_timing_accepts_camel_case_protocol_field() {
    let value = required_positive_number_field(
        &CliOptions::default(),
        &json!({"expiresInSeconds": LOGIN_EXPIRY_SECONDS}),
        "expiresInSeconds",
        "login expiry seconds",
    );

    assert_eq!(value.ok(), Some(0x0384));
}

#[test]
/// Verifies login timing rejects zero-valued protocol fields.
///
/// # Panics
///
/// Panics when zero is accepted or reports a different protocol error.
fn login_timing_requires_positive_number() {
    let cli = CliOptions::default();
    let message = required_positive_number_field(
        &cli,
        &json!({"intervalSeconds": ZERO_INTERVAL_SECONDS}),
        "intervalSeconds",
        "login poll interval seconds",
    )
    .err()
    .map(|error| return error.message().to_owned());

    assert_eq!(
        message.as_deref(),
        Some("Tovuk login did not return valid login poll interval seconds.")
    );
}
