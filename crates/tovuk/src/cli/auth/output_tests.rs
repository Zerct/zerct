use serde_json::json;

use super::LoggedInMessage;

#[test]
/// Verifies text login output does not invent an absent identity.
///
/// # Panics
///
/// Panics when a missing email produces a different message.
fn logged_in_message_does_not_invent_user_when_email_is_missing() {
    assert_eq!(String::from(LoggedInMessage::from(&json!({}))), "logged in");
}

#[test]
/// Verifies text login output includes the authenticated email.
///
/// # Panics
///
/// Panics when a present email produces a different message.
fn logged_in_message_uses_email_when_present() {
    assert_eq!(
        String::from(LoggedInMessage::from(&json!({"email": "ada@example.com"}))),
        "logged in as ada@example.com"
    );
}
