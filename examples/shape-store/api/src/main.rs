mod catalog;
mod checkout;
mod http;
mod server;

use http::{Request, Response, json};

fn main() -> std::io::Result<()> {
    server::run(route)
}

#[must_use]
fn route(request: &Request) -> Response {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/healthz" | "/api/healthz") => json("200 OK", r#"{"ok":true}"#),
        ("GET", "/api/products") => json("200 OK", catalog::PRODUCTS_JSON),
        ("POST", "/api/orders") => checkout::create_order(&request.body),
        ("POST", "/api/checkout") => checkout::create_checkout(&request.body, &request.origin),
        ("POST", path) if path.starts_with("/.tovuk/queues/") => {
            json("200 OK", r#"{"ok":true,"event":"queue"}"#)
        }
        ("POST", path) if path.starts_with("/.tovuk/cron/") => {
            json("200 OK", r#"{"ok":true,"event":"cron"}"#)
        }
        ("POST", path) if path.starts_with("/.tovuk/state/") && path.ends_with("/alarm") => {
            json("200 OK", r#"{"ok":true,"event":"state-alarm"}"#)
        }
        _ => json("404 Not Found", r#"{"error":"not_found"}"#),
    }
}
