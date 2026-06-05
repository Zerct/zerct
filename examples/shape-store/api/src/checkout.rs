use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::{
    catalog::product_catalog,
    http::{Response, json_value},
};

const FREE_SHIPPING_THRESHOLD_CENTS: u64 = 20_000;
const SHIPPING_CENTS: u64 = 0;
const STRIPE_API_VERSION: &str = "2026-02-25.clover";
const STRIPE_CHECKOUT_SESSIONS_URL: &str = "https://api.stripe.com/v1/checkout/sessions";

#[derive(Deserialize)]
struct CheckoutRequest {
    customer: Option<CheckoutCustomer>,
    items: Vec<CheckoutItem>,
}

#[derive(Deserialize)]
struct CheckoutCustomer {
    email: Option<String>,
}

#[derive(Deserialize)]
struct CheckoutItem {
    #[serde(rename = "productId")]
    product_id: String,
    quantity: u64,
}

struct CheckoutOrder {
    customer_email: Option<String>,
    lines: Vec<CheckoutLine>,
    subtotal_cents: u64,
}

struct CheckoutLine {
    name: String,
    price_cents: u64,
    quantity: u64,
}

#[derive(Deserialize)]
struct StripeCheckoutSession {
    url: Option<String>,
}

#[must_use]
pub(crate) fn create_order(body: &str) -> Response {
    let order = match reserved_order(body) {
        Ok(order) => order,
        Err(message) => {
            return json_value(
                "400 Bad Request",
                &serde_json::json!({"error":"invalid_order","message":message}),
            );
        }
    };

    json_value(
        "201 Created",
        &serde_json::json!({
            "ok": true,
            "orderId": new_order_id(),
            "status": "reserved",
            "totalCents": order.total_cents(),
            "message": "Order reserved for manual fulfillment"
        }),
    )
}

#[must_use]
pub(crate) fn create_checkout(body: &str, request_origin: &str) -> Response {
    let order = match checkout_order(body) {
        Ok(order) => order,
        Err(message) => {
            return json_value(
                "400 Bad Request",
                &serde_json::json!({"error":"invalid_checkout","message":message}),
            );
        }
    };

    let Ok(secret_key) = std::env::var("STRIPE_SECRET_KEY") else {
        return demo_checkout_response();
    };
    if secret_key.trim().is_empty() {
        return demo_checkout_response();
    }

    let base_url = match checkout_base_url(request_origin) {
        Ok(base_url) => base_url,
        Err(message) => {
            return json_value(
                "500 Internal Server Error",
                &serde_json::json!({"error":"stripe_not_ready","message":message}),
            );
        }
    };

    create_stripe_checkout_session(&secret_key, &base_url, &order)
}

#[must_use]
fn demo_checkout_response() -> Response {
    json_value(
        "201 Created",
        &serde_json::json!({
            "ok": true,
            "mode": "demo",
            "orderId": new_order_id(),
            "status": "stripe_demo",
            "message": "Set STRIPE_SECRET_KEY and PUBLIC_BASE_URL to enable Stripe Checkout."
        }),
    )
}

fn checkout_order(body: &str) -> Result<CheckoutOrder, String> {
    let request = serde_json::from_str::<CheckoutRequest>(body)
        .map_err(|_error| "checkout request must be JSON".to_owned())?;
    if request.items.is_empty() {
        return Err("at least one checkout item is required".to_owned());
    }

    let catalog = product_catalog()?;
    let mut lines = Vec::with_capacity(request.items.len());

    for item in request.items {
        if item.quantity == 0 {
            return Err("checkout item quantity must be greater than zero".to_owned());
        }
        let Some(product) = catalog.product(&item.product_id) else {
            return Err(format!("unknown product {}", item.product_id));
        };
        lines.push(CheckoutLine {
            name: product.name.clone(),
            price_cents: product.price_cents,
            quantity: item.quantity,
        });
    }

    let subtotal_cents = lines
        .iter()
        .map(|line| line.price_cents.saturating_mul(line.quantity))
        .sum();
    Ok(CheckoutOrder {
        customer_email: checkout_customer_email(request.customer.as_ref()),
        lines,
        subtotal_cents,
    })
}

fn reserved_order(body: &str) -> Result<CheckoutOrder, String> {
    let order = checkout_order(body)?;
    if order.customer_email.is_none() {
        return Err("customer email is required".to_owned());
    }
    Ok(order)
}

impl CheckoutOrder {
    #[must_use]
    fn total_cents(&self) -> u64 {
        self.subtotal_cents + shipping_cents_for(self.subtotal_cents)
    }
}

fn checkout_customer_email(customer: Option<&CheckoutCustomer>) -> Option<String> {
    customer
        .and_then(|customer| customer.email.as_deref())
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .map(str::to_owned)
}

fn checkout_base_url(request_origin: &str) -> Result<String, String> {
    let configured = std::env::var("PUBLIC_BASE_URL")
        .or_else(|_error| std::env::var("FRONTEND_ORIGIN"))
        .unwrap_or_else(|_error| request_origin.to_owned());
    let base_url = configured.trim().trim_end_matches('/').to_owned();
    if base_url.starts_with("https://")
        || base_url.starts_with("http://localhost")
        || base_url.starts_with("http://127.0.0.1")
    {
        Ok(base_url)
    } else {
        Err("PUBLIC_BASE_URL must be an HTTPS URL when Stripe is configured".to_owned())
    }
}

#[must_use]
fn create_stripe_checkout_session(
    secret_key: &str,
    base_url: &str,
    order: &CheckoutOrder,
) -> Response {
    let parameters = stripe_checkout_parameters(base_url, order);
    let response = reqwest::blocking::Client::new()
        .post(STRIPE_CHECKOUT_SESSIONS_URL)
        .bearer_auth(secret_key.trim())
        .header("Stripe-Version", STRIPE_API_VERSION)
        .form(&parameters)
        .send();

    let stripe_response = match response {
        Ok(response) => response,
        Err(error) => {
            return json_value(
                "502 Bad Gateway",
                &serde_json::json!({"error":"stripe_unreachable","message":error.to_string()}),
            );
        }
    };
    let status = stripe_response.status();
    let body = match stripe_response.text() {
        Ok(body) => body,
        Err(error) => {
            return json_value(
                "502 Bad Gateway",
                &serde_json::json!({"error":"stripe_response_unreadable","message":error.to_string()}),
            );
        }
    };

    if !status.is_success() {
        return json_value(
            "502 Bad Gateway",
            &serde_json::json!({"error":"stripe_checkout_failed","message":body}),
        );
    }

    match serde_json::from_str::<StripeCheckoutSession>(&body) {
        Ok(session) => stripe_checkout_response(session),
        Err(error) => json_value(
            "502 Bad Gateway",
            &serde_json::json!({"error":"stripe_response_invalid","message":error.to_string()}),
        ),
    }
}

fn stripe_checkout_parameters(base_url: &str, order: &CheckoutOrder) -> Vec<(String, String)> {
    let mut parameters = vec![
        ("mode".to_owned(), "payment".to_owned()),
        (
            "success_url".to_owned(),
            format!("{base_url}/?checkout=success"),
        ),
        (
            "cancel_url".to_owned(),
            format!("{base_url}/?checkout=cancel"),
        ),
        (
            "metadata[source]".to_owned(),
            "tovuk-example-store".to_owned(),
        ),
    ];

    if let Some(email) = &order.customer_email {
        parameters.push(("customer_email".to_owned(), email.to_owned()));
    }

    for (index, line) in order.lines.iter().enumerate() {
        push_stripe_line_item(
            &mut parameters,
            index,
            &line.name,
            line.price_cents,
            line.quantity,
        );
    }

    if should_charge_shipping(order.subtotal_cents) {
        push_stripe_line_item(
            &mut parameters,
            order.lines.len(),
            "Shipping",
            SHIPPING_CENTS,
            1,
        );
    }

    parameters
}

fn push_stripe_line_item(
    parameters: &mut Vec<(String, String)>,
    index: usize,
    name: &str,
    unit_amount: u64,
    quantity: u64,
) {
    let prefix = format!("line_items[{index}]");
    parameters.push((format!("{prefix}[price_data][currency]"), "usd".to_owned()));
    parameters.push((
        format!("{prefix}[price_data][product_data][name]"),
        name.to_owned(),
    ));
    parameters.push((
        format!("{prefix}[price_data][unit_amount]"),
        unit_amount.to_string(),
    ));
    parameters.push((format!("{prefix}[quantity]"), quantity.to_string()));
}

#[must_use]
fn should_charge_shipping(subtotal_cents: u64) -> bool {
    subtotal_cents > 0 && subtotal_cents < FREE_SHIPPING_THRESHOLD_CENTS
}

#[must_use]
fn shipping_cents_for(subtotal_cents: u64) -> u64 {
    if should_charge_shipping(subtotal_cents) {
        SHIPPING_CENTS
    } else {
        0
    }
}

#[must_use]
fn stripe_checkout_response(session: StripeCheckoutSession) -> Response {
    match session.url {
        Some(checkout_url) if !checkout_url.is_empty() => json_value(
            "201 Created",
            &serde_json::json!({"ok":true,"mode":"stripe","checkoutUrl":checkout_url}),
        ),
        _ => json_value(
            "502 Bad Gateway",
            &serde_json::json!({"error":"stripe_response_invalid","message":"Stripe did not return a checkout URL"}),
        ),
    }
}

#[must_use]
fn new_order_id() -> String {
    let order_number = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("TOV-{order_number}")
}

#[cfg(test)]
mod tests {
    use super::{checkout_order, create_order, reserved_order, stripe_checkout_parameters};

    #[test]
    fn checkout_order_uses_shared_catalog_prices() -> Result<(), Box<dyn std::error::Error>> {
        let order = checkout_order(
            r#"{"customer":{"email":"shopper@example.com"},"items":[{"productId":"shape-slide","quantity":2}]}"#,
        )?;

        assert_eq!(order.subtotal_cents, 10_000);
        assert_eq!(order.customer_email.as_deref(), Some("shopper@example.com"));

        Ok(())
    }

    #[test]
    fn reserved_order_requires_customer_email() {
        let error = reserved_order(r#"{"items":[{"productId":"shape-slide","quantity":1}]}"#).err();

        assert_eq!(error.as_deref(), Some("customer email is required"));
    }

    #[test]
    fn create_order_rejects_unknown_products() {
        let response = create_order(
            r#"{"customer":{"email":"shopper@example.com"},"items":[{"productId":"missing","quantity":1}]}"#,
        );

        assert!(response.status.starts_with("400"));
        assert!(response.body.contains(r#""error":"invalid_order""#));
        assert!(response.body.contains("unknown product missing"));
    }

    #[test]
    fn create_order_returns_server_computed_total() {
        let response = create_order(
            r#"{"customer":{"email":"shopper@example.com"},"items":[{"productId":"shape-slide","quantity":2}]}"#,
        );

        assert_eq!(response.status, "201 Created");
        assert!(response.body.contains(r#""totalCents":10000"#));
        assert!(response.body.contains(r#""status":"reserved""#));
    }

    #[test]
    fn stripe_parameters_include_checkout_lines() -> Result<(), Box<dyn std::error::Error>> {
        let order = checkout_order(r#"{"items":[{"productId":"shape-slide","quantity":1}]}"#)?;
        let parameters = stripe_checkout_parameters("https://shape-store.tovuk.app", &order);

        assert!(
            parameters
                .iter()
                .any(|(key, value)| key == "mode" && value == "payment")
        );
        assert!(parameters.iter().any(|(key, value)| {
            key == "line_items[0][price_data][product_data][name]" && value == "YS-02"
        }));
        assert!(parameters.iter().any(|(key, value)| {
            key == "line_items[0][price_data][unit_amount]" && value == "5000"
        }));

        Ok(())
    }
}
