use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

const HEADER_DELIMITER_LENGTH: usize = 4;
const LISTEN_BACKLOG: i32 = 1024;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MIN_WORKER_THREADS: usize = 64;
const MAX_WORKER_THREADS: usize = 128;
const READ_CHUNK_BYTES: usize = 4096;
const WORKER_THREAD_STACK_BYTES: usize = 256 * 1024;
const SHIPPING_CENTS: u64 = 0;
const FREE_SHIPPING_THRESHOLD_CENTS: u64 = 20_000;
const STRIPE_API_VERSION: &str = "2026-02-25.clover";
const STRIPE_CHECKOUT_SESSIONS_URL: &str = "https://api.stripe.com/v1/checkout/sessions";

const PRODUCTS_JSON: &str = r#"{"products":[
{"id":"shape-slide","name":"YS-02","categories":["NEW","FOOTWEAR","SLIDES","MENS","WOMENS"],"priceCents":5000,"inventory":24,"image":"/products/shape-capsule.svg"},
{"id":"shape-square","name":"SL-03","categories":["NEW","FOOTWEAR","MENS","WOMENS"],"priceCents":3500,"inventory":12,"image":"/products/shape-square.svg"},
{"id":"shape-circle","name":"SG-03","categories":["NEW","MENS"],"priceCents":2000,"inventory":30,"image":"/products/shape-circle.svg"},
{"id":"shape-triangle","name":"SL-01","categories":["NEW","FOOTWEAR","SLIDES"],"priceCents":2400,"inventory":19,"image":"/products/shape-triangle.svg"},
{"id":"shape-diamond","name":"TS-07","categories":["NEW","MENS","WOMENS"],"priceCents":4500,"inventory":16,"image":"/products/shape-diamond.svg"},
{"id":"shape-rectangle","name":"LS-03","categories":["NEW","MENS","WOMENS"],"priceCents":4000,"inventory":18,"image":"/products/shape-rectangle.svg"},
{"id":"shape-arch","name":"JC-07","categories":["NEW","WOMENS"],"priceCents":5600,"inventory":10,"image":"/products/shape-arch.svg"},
{"id":"shape-pentagon","name":"JC-09","categories":["NEW","ACCESSORIES"],"priceCents":3000,"inventory":20,"image":"/products/shape-pentagon.svg"},
{"id":"shape-oval","name":"BB-02","categories":["NEW","ACCESSORIES","MENS","WOMENS"],"priceCents":1800,"inventory":22,"image":"/products/shape-oval.svg"},
{"id":"shape-cross","name":"WD-01","categories":["NEW","ACCESSORIES","MENS","WOMENS"],"priceCents":1200,"inventory":36,"image":"/products/shape-cross.svg"},
{"id":"shape-star","name":"WP-01","categories":["ACCESSORIES","WOMENS"],"priceCents":1800,"inventory":28,"image":"/products/shape-star.svg"},
{"id":"shape-hexagon","name":"WB-04","categories":["MENS","WOMENS"],"priceCents":2600,"inventory":18,"image":"/products/shape-hexagon.svg"},
{"id":"shape-crescent","name":"BB-01","categories":["ACCESSORIES","WOMENS"],"priceCents":2000,"inventory":16,"image":"/products/shape-crescent.svg"},
{"id":"shape-ring","name":"HD-04","categories":["MENS","ACCESSORIES"],"priceCents":3000,"inventory":14,"image":"/products/shape-ring.svg"},
{"id":"shape-slash","name":"TS-03","categories":["MENS","WOMENS"],"priceCents":2000,"inventory":34,"image":"/products/shape-slash.svg"},
{"id":"shape-trapezoid","name":"PT-05","categories":["MENS"],"priceCents":3200,"inventory":15,"image":"/products/shape-trapezoid.svg"},
{"id":"shape-droplet","name":"LG-14","categories":["WOMENS","ACCESSORIES"],"priceCents":2800,"inventory":15,"image":"/products/shape-droplet.svg"},
{"id":"shape-chevron","name":"TT-06","categories":["NEW","MENS","WOMENS"],"priceCents":2200,"inventory":21,"image":"/products/shape-chevron.svg"},
{"id":"shape-barbell","name":"HD-10","categories":["NEW","ACCESSORIES","MENS"],"priceCents":3000,"inventory":17,"image":"/products/shape-barbell.svg"},
{"id":"shape-semicircle","name":"YS-01","categories":["NEW","FOOTWEAR","SLIDES"],"priceCents":2000,"inventory":33,"image":"/products/shape-semicircle.svg"},
{"id":"shape-bolt","name":"BD-10","categories":["NEW","WOMENS"],"priceCents":2800,"inventory":12,"image":"/products/shape-bolt.svg"},
{"id":"shape-parallelogram","name":"WJ-02","categories":["NEW","MENS","WOMENS"],"priceCents":4200,"inventory":11,"image":"/products/shape-parallelogram.svg"},
{"id":"shape-frame","name":"BX-01","categories":["NEW","ACCESSORIES"],"priceCents":1800,"inventory":26,"image":"/products/shape-frame.svg"},
{"id":"shape-hourglass","name":"WH-01","categories":["NEW","WOMENS"],"priceCents":2400,"inventory":20,"image":"/products/shape-hourglass.svg"},
{"id":"shape-pillar","name":"HD-01","categories":["MENS","WOMENS"],"priceCents":2600,"inventory":24,"image":"/products/shape-pillar.svg"},
{"id":"shape-step","name":"PT-04","categories":["MENS"],"priceCents":3200,"inventory":15,"image":"/products/shape-step.svg"},
{"id":"shape-shield","name":"BT-01","categories":["FOOTWEAR","MENS"],"priceCents":3600,"inventory":9,"image":"/products/shape-shield.svg"},
{"id":"shape-fan","name":"JC-05","categories":["WOMENS"],"priceCents":5200,"inventory":8,"image":"/products/shape-fan.svg"},
{"id":"shape-wave","name":"TS-01","categories":["MENS","WOMENS"],"priceCents":2200,"inventory":19,"image":"/products/shape-wave.svg"},
{"id":"shape-kite","name":"TS-02","categories":["MENS","WOMENS"],"priceCents":2400,"inventory":21,"image":"/products/shape-kite.svg"},
{"id":"shape-notch","name":"TS-04","categories":["MENS","WOMENS"],"priceCents":2000,"inventory":18,"image":"/products/shape-notch.svg"},
{"id":"shape-double","name":"BD-03","categories":["ACCESSORIES"],"priceCents":1800,"inventory":30,"image":"/products/shape-double.svg"},
{"id":"shape-cube","name":"TT-02","categories":["NEW","MENS","WOMENS"],"priceCents":2400,"inventory":18,"image":"/products/shape-cube.svg"},
{"id":"shape-stack","name":"TT-04","categories":["NEW","MENS","WOMENS"],"priceCents":2600,"inventory":22,"image":"/products/shape-stack.svg"},
{"id":"shape-ticket","name":"BD-04","categories":["ACCESSORIES"],"priceCents":1800,"inventory":30,"image":"/products/shape-ticket.svg"},
{"id":"shape-door","name":"HD-02","categories":["MENS","WOMENS"],"priceCents":2600,"inventory":24,"image":"/products/shape-door.svg"},
{"id":"shape-window","name":"HD-03","categories":["MENS","WOMENS"],"priceCents":2600,"inventory":24,"image":"/products/shape-window.svg"},
{"id":"shape-ribbon","name":"WB-01","categories":["MENS","WOMENS"],"priceCents":2600,"inventory":16,"image":"/products/shape-ribbon.svg"},
{"id":"shape-bracket","name":"BR-09","categories":["ACCESSORIES"],"priceCents":2200,"inventory":13,"image":"/products/shape-bracket.svg"},
{"id":"shape-flag","name":"SH-01","categories":["MENS"],"priceCents":2400,"inventory":20,"image":"/products/shape-flag.svg"},
{"id":"shape-wedge","name":"SH-06","categories":["MENS"],"priceCents":2400,"inventory":20,"image":"/products/shape-wedge.svg"},
{"id":"shape-ladder","name":"PT-03","categories":["MENS"],"priceCents":3200,"inventory":15,"image":"/products/shape-ladder.svg"},
{"id":"shape-tunnel","name":"SP-01","categories":["FOOTWEAR","SLIDES","MENS","WOMENS"],"priceCents":3000,"inventory":27,"image":"/products/shape-tunnel.svg"},
{"id":"shape-keyhole","name":"SP-06","categories":["FOOTWEAR","SLIDES","MENS","WOMENS"],"priceCents":3000,"inventory":17,"image":"/products/shape-keyhole.svg"},
{"id":"shape-pin","name":"LG-01","categories":["WOMENS"],"priceCents":2800,"inventory":18,"image":"/products/shape-pin.svg"},
{"id":"shape-moon","name":"LG-04","categories":["WOMENS"],"priceCents":2800,"inventory":18,"image":"/products/shape-moon.svg"},
{"id":"shape-ellipse-cut","name":"LG-13","categories":["WOMENS"],"priceCents":2800,"inventory":18,"image":"/products/shape-ellipse-cut.svg"},
{"id":"shape-scallop","name":"UW-02","categories":["WOMENS"],"priceCents":1800,"inventory":12,"image":"/products/shape-scallop.svg"},
{"id":"shape-spark","name":"SK-01","categories":["MENS","WOMENS"],"priceCents":2000,"inventory":22,"image":"/products/shape-spark.svg"},
{"id":"shape-crown","name":"WP-02","categories":["ACCESSORIES","WOMENS"],"priceCents":1800,"inventory":11,"image":"/products/shape-crown.svg"}
]}"#;

struct Request {
    body: String,
    method: String,
    origin: String,
    path: String,
}

struct Response {
    body: String,
    status: &'static str,
}

#[derive(Deserialize)]
struct ProductCatalog {
    products: Vec<CatalogProduct>,
}

#[derive(Deserialize)]
struct CatalogProduct {
    id: String,
    name: String,
    #[serde(rename = "priceCents")]
    price_cents: u64,
}

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

fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let listener = bind_listener(port)?;
    let worker_count = thread::available_parallelism().map_or(MIN_WORKER_THREADS, |count| {
        count.get().clamp(MIN_WORKER_THREADS, MAX_WORKER_THREADS)
    });
    let mut workers = Vec::with_capacity(worker_count);

    for _index in 0..worker_count {
        let worker_listener = listener.try_clone()?;
        let worker = thread::Builder::new()
            .stack_size(WORKER_THREAD_STACK_BYTES)
            .spawn(move || accept_loop(&worker_listener))?;
        workers.push(worker);
    }

    for worker in workers {
        match worker.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(_panic) => return Err(std::io::Error::other("request worker thread failed")),
        }
    }

    Ok(())
}

fn bind_listener(port: u16) -> std::io::Result<TcpListener> {
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&SockAddr::from(address))?;
    socket.listen(LISTEN_BACKLOG)?;
    Ok(socket.into())
}

fn accept_loop(listener: &TcpListener) -> std::io::Result<()> {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle(stream) {
                    eprintln!("request failed: {error}");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let request = read_request(&mut stream)?;
    let cors_origin = allowed_origin(&request.origin);

    if request.method == "OPTIONS" {
        return write_response(
            &mut stream,
            &Response {
                status: "204 No Content",
                body: String::new(),
            },
            &cors_origin,
        );
    }

    let response = route(&request);
    write_response(&mut stream, &response, &cors_origin)
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; READ_CHUNK_BYTES];

    loop {
        let size = stream.read(&mut buffer)?;
        if size == 0 {
            break;
        }

        bytes.extend_from_slice(&buffer[..size]);

        if request_is_complete(&bytes) || bytes.len() >= MAX_REQUEST_BYTES {
            break;
        }
    }

    Ok(parse_request(&bytes))
}

#[must_use]
fn request_is_complete(bytes: &[u8]) -> bool {
    let Some(header_end) = header_end(bytes) else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..header_end]);
    let expected_body_bytes = content_length(&head);
    bytes.len() >= header_end + HEADER_DELIMITER_LENGTH + expected_body_bytes
}

#[must_use]
fn parse_request(bytes: &[u8]) -> Request {
    let header_end = header_end(bytes).unwrap_or(bytes.len());
    let head = String::from_utf8_lossy(&bytes[..header_end]);
    let body_start = (header_end + HEADER_DELIMITER_LENGTH).min(bytes.len());
    let body = String::from_utf8_lossy(&bytes[body_start..]).into_owned();
    let mut request_line_parts = head.lines().next().unwrap_or_default().split_whitespace();
    let method = request_line_parts.next().unwrap_or_default().to_owned();
    let raw_path = request_line_parts.next().unwrap_or("/");
    let path = raw_path.split('?').next().unwrap_or("/").to_owned();
    let origin = header_value(&head, "Origin").unwrap_or("*").to_owned();

    Request {
        body,
        method,
        origin,
        path,
    }
}

#[must_use]
fn route(request: &Request) -> Response {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/healthz" | "/api/healthz") => json("200 OK", r#"{"ok":true}"#),
        ("GET", "/api/products") => json("200 OK", PRODUCTS_JSON),
        ("POST", "/api/orders") => create_order(&request.body),
        ("POST", "/api/checkout") => create_checkout(&request.body, &request.origin),
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

#[must_use]
fn create_order(body: &str) -> Response {
    if !(body.contains(r#""email""#) && body.contains(r#""items""#)) {
        return json(
            "400 Bad Request",
            r#"{"error":"invalid_order","message":"customer email and items are required"}"#,
        );
    }

    json(
        "201 Created",
        &format!(
            r#"{{"ok":true,"orderId":"{}","status":"reserved","message":"Order reserved for manual fulfillment"}}"#,
            new_order_id()
        ),
    )
}

#[must_use]
fn create_checkout(body: &str, request_origin: &str) -> Response {
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
        return demo_checkout_response(&order);
    };
    if secret_key.trim().is_empty() {
        return demo_checkout_response(&order);
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
fn demo_checkout_response(_order: &CheckoutOrder) -> Response {
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

    let catalog = serde_json::from_str::<ProductCatalog>(PRODUCTS_JSON)
        .map_err(|_error| "product catalog is unavailable".to_owned())?;
    let mut lines = Vec::with_capacity(request.items.len());

    for item in request.items {
        if item.quantity == 0 {
            return Err("checkout item quantity must be greater than zero".to_owned());
        }
        let Some(product) = catalog
            .products
            .iter()
            .find(|catalog_product| catalog_product.id == item.product_id)
        else {
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

#[must_use]
fn json(status: &'static str, body: &str) -> Response {
    Response {
        body: body.to_owned(),
        status,
    }
}

#[must_use]
fn json_value(status: &'static str, body: &serde_json::Value) -> Response {
    json(status, &body.to_string())
}

#[must_use]
fn header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(HEADER_DELIMITER_LENGTH)
        .position(|window| window == b"\r\n\r\n")
}

#[must_use]
fn content_length(head: &str) -> usize {
    header_value(head, "Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

#[must_use]
fn header_value<'a>(head: &'a str, expected_name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case(expected_name) {
            Some(value.trim())
        } else {
            None
        }
    })
}

#[must_use]
fn allowed_origin(request_origin: &str) -> String {
    let configured =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_error| request_origin.to_owned());
    if configured == "*" || configured == request_origin {
        configured
    } else {
        "null".to_owned()
    }
}

fn write_response(
    stream: &mut TcpStream,
    response: &Response,
    origin: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\naccess-control-allow-origin: {origin}\r\naccess-control-allow-methods: GET, POST, OPTIONS\r\naccess-control-allow-headers: content-type, authorization\r\nconnection: close\r\n\r\n{}",
        response.status,
        response.body.len(),
        response.body
    )
}
