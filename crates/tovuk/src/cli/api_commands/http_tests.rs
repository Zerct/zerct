use std::{error::Error, net::TcpListener};

use reqwest::Method;

use crate::cli::args::CliOptions;

use super::api_request;

#[test]
fn api_unreachable_points_agents_to_status_docs() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let api_url = format!("http://{}", listener.local_addr()?);
    drop(listener);
    let cli = CliOptions {
        api_url,
        ..CliOptions::default()
    };

    let error = match api_request(&cli, Method::GET, "/v1/status", None, None) {
        Ok(_response) => return Err("request unexpectedly succeeded".into()),
        Err(error) => error,
    };
    let payload = error.payload();

    if payload.code != "api_unreachable" {
        return Err(format!("unexpected code: {}", payload.code).into());
    }
    if payload.docs_url.as_deref() != Some("https://docs.tovuk.com/status") {
        return Err(format!("unexpected docs url: {:?}", payload.docs_url).into());
    }
    Ok(())
}
