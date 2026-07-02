use serde_json::json;

use crate::cli::args::CliOptions;

use super::{billing_checkout_body, billing_session_url};

#[test]
fn billing_checkout_body_requires_explicit_plan() {
    let cli = CliOptions {
        args: vec!["checkout".to_owned()],
        ..CliOptions::default()
    };

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| error.to_string());

    assert_eq!(message.as_deref(), Some("Billing plan is required."));
}

#[test]
fn billing_checkout_body_rejects_unknown_plan() {
    let cli = CliOptions {
        args: vec!["checkout".to_owned(), "business".to_owned()],
        ..CliOptions::default()
    };

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| error.to_string());

    assert_eq!(
        message.as_deref(),
        Some("Billing plan must be plus, pro, or max.")
    );
}

#[test]
fn billing_checkout_body_uses_plan_and_reason() -> Result<(), Box<dyn std::error::Error>> {
    let cli = CliOptions {
        args: vec![
            "checkout".to_owned(),
            "max".to_owned(),
            "scale".to_owned(),
            "usage".to_owned(),
        ],
        ..CliOptions::default()
    };

    let body = billing_checkout_body(&cli)?;

    let expected = json!({
        "target_plan": "max",
        "reason": "scale usage"
    });
    if body != expected {
        return Err(format!("unexpected checkout body: {body}").into());
    }
    Ok(())
}

#[test]
fn billing_checkout_body_uses_top_up_amount_and_reason() -> Result<(), Box<dyn std::error::Error>> {
    let cli = CliOptions {
        args: vec![
            "checkout".to_owned(),
            "more".to_owned(),
            "balance".to_owned(),
        ],
        top_up_usd_cents: "2000".to_owned(),
        ..CliOptions::default()
    };

    let body = billing_checkout_body(&cli)?;

    let expected = json!({
        "top_up_usd_cents": 2000,
        "reason": "more balance"
    });
    if body != expected {
        return Err(format!("unexpected checkout body: {body}").into());
    }
    Ok(())
}

#[test]
fn billing_checkout_body_rejects_plan_with_top_up_amount() {
    let cli = CliOptions {
        args: vec!["checkout".to_owned(), "plus".to_owned()],
        top_up_usd_cents: "2000".to_owned(),
        ..CliOptions::default()
    };

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| error.to_string());

    assert_eq!(
        message.as_deref(),
        Some("Billing checkout cannot include both a plan and top-up amount.")
    );
}

#[test]
fn billing_checkout_body_rejects_invalid_top_up_amount() {
    let cli = CliOptions {
        args: vec!["checkout".to_owned()],
        top_up_usd_cents: "twenty".to_owned(),
        ..CliOptions::default()
    };

    let message = billing_checkout_body(&cli)
        .err()
        .map(|error| error.to_string());

    assert_eq!(
        message.as_deref(),
        Some("Billing top-up amount must be an integer number of USD cents.")
    );
}

#[test]
fn billing_session_url_reads_checkout_url() -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({
        "checkout": {
            "url": "https://billing.stripe.test/session"
        }
    });

    let url = billing_session_url(&response, false)?;

    if url != "https://billing.stripe.test/session" {
        return Err(format!("unexpected url: {url}").into());
    }
    Ok(())
}

#[test]
fn billing_session_url_rejects_missing_url() -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({});
    let error = match billing_session_url(&response, false) {
        Ok(url) => return Err(format!("unexpected url: {url}").into()),
        Err(error) => error,
    };

    let payload = error.payload();
    if payload.code != "billing_url_missing" {
        return Err(format!("unexpected code: {}", payload.code).into());
    }
    Ok(())
}

#[test]
fn billing_session_url_rejects_blank_url() -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({
        "checkout": {
            "url": " "
        }
    });
    let error = match billing_session_url(&response, false) {
        Ok(url) => return Err(format!("unexpected url: {url}").into()),
        Err(error) => error,
    };

    let payload = error.payload();
    if payload.message != "Tovuk billing did not return a URL." {
        return Err(format!("unexpected message: {}", payload.message).into());
    }
    Ok(())
}
