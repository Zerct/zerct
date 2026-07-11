use serde_json::json;

use crate::cli::{args::options_for_test, errors::OutputFormat};

use super::{billing_checkout_body, billing_session_url};

#[test]
/// Verifies billing checkout rejects a non-numeric top-up amount.
///
/// # Panics
///
/// Panics when invalid top-up input is accepted or reports a different error.
fn billing_checkout_body_rejects_invalid_top_up_amount() {
    let cli = options_for_test(&["billing", "checkout", "--top-up-usd-cents", "twenty"]);

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| return error.message().to_owned());

    assert_eq!(
        message.as_deref(),
        Some("Billing top-up amount must be an integer number of USD cents.")
    );
}

#[test]
/// Verifies billing checkout does not mix a plan and top-up amount.
///
/// # Panics
///
/// Panics when conflicting checkout targets are accepted or report a different error.
fn billing_checkout_body_rejects_plan_with_top_up_amount() {
    let cli = options_for_test(&["billing", "checkout", "plus", "--top-up-usd-cents", "2000"]);

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| return error.message().to_owned());

    assert_eq!(
        message.as_deref(),
        Some("Billing checkout cannot include both a plan and top-up amount.")
    );
}

#[test]
/// Verifies billing checkout rejects unsupported subscription plans.
///
/// # Panics
///
/// Panics when an unsupported plan is accepted or reports a different error.
fn billing_checkout_body_rejects_unknown_plan() {
    let cli = options_for_test(&["billing", "checkout", "business"]);

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| return error.message().to_owned());

    assert_eq!(
        message.as_deref(),
        Some("Billing plan must be plus, pro, or max.")
    );
}

#[test]
/// Verifies billing checkout requires an explicit target plan.
///
/// # Panics
///
/// Panics when an omitted plan is accepted or reports a different error.
fn billing_checkout_body_requires_explicit_plan() {
    let cli = options_for_test(&["billing", "checkout"]);

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| return error.message().to_owned());

    assert_eq!(message.as_deref(), Some("Billing plan is required."));
}

#[test]
/// Verifies billing checkout serializes a subscription plan and reason.
///
/// # Panics
///
/// Panics when the valid checkout body differs from the public contract.
fn billing_checkout_body_uses_plan_and_reason() {
    let cli = options_for_test(&["billing", "checkout", "max", "scale", "usage"]);

    assert_eq!(
        billing_checkout_body(&cli).ok(),
        Some(json!({
            "target_plan": "max",
            "reason": "scale usage"
        }))
    );
}

#[test]
/// Verifies billing checkout serializes a top-up amount and reason.
///
/// # Panics
///
/// Panics when the valid checkout body differs from the public contract.
fn billing_checkout_body_uses_top_up_amount_and_reason() {
    let cli = options_for_test(&[
        "billing",
        "checkout",
        "more",
        "balance",
        "--top-up-usd-cents",
        "2000",
    ]);
    let top_up_usd_cents: u32 = 0x07d0;

    assert_eq!(
        billing_checkout_body(&cli).ok(),
        Some(json!({
            "top_up_usd_cents": top_up_usd_cents,
            "reason": "more balance"
        }))
    );
}

#[test]
/// Verifies billing responses expose their checkout URL.
///
/// # Panics
///
/// Panics when a valid checkout URL cannot be read.
fn billing_session_url_reads_checkout_url() {
    let response = json!({
        "checkout": {
            "url": "https://billing.stripe.test/session"
        }
    });

    assert_eq!(
        billing_session_url(&response, OutputFormat::Text).ok(),
        Some("https://billing.stripe.test/session")
    );
}

#[test]
/// Verifies billing responses reject a blank checkout URL.
///
/// # Panics
///
/// Panics when a blank URL is accepted or reports a different error.
fn billing_session_url_rejects_blank_url() {
    let response = json!({
        "checkout": {
            "url": " "
        }
    });
    let error_result = billing_session_url(&response, OutputFormat::Text).err();
    assert!(
        error_result.is_some(),
        "blank billing URL should be rejected"
    );
    let Some(error) = error_result else {
        return;
    };

    assert_eq!(
        error.payload().message(),
        "Tovuk billing did not return a URL."
    );
}

#[test]
/// Verifies billing responses require a checkout URL.
///
/// # Panics
///
/// Panics when a missing URL is accepted or reports a different error.
fn billing_session_url_rejects_missing_url() {
    let response = json!({});
    let error_result = billing_session_url(&response, OutputFormat::Text).err();
    assert!(
        error_result.is_some(),
        "missing billing URL should be rejected"
    );
    let Some(error) = error_result else {
        return;
    };

    assert_eq!(error.payload().code(), "billing_url_missing");
}
