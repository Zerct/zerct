use std::{
    error::Error,
    io::{Read, Write},
    net::TcpListener,
    thread::{self, JoinHandle},
};

use reqwest::Method;

use crate::cli::args::CliOptions;

use super::api_request;

type TestServer = JoinHandle<Result<(), String>>;

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

#[test]
fn successful_api_response_must_be_valid_json() -> Result<(), Box<dyn Error>> {
    let (api_url, server) = serve_once("200 OK", "not-json")?;
    let cli = CliOptions {
        api_url,
        ..CliOptions::default()
    };

    let error = match api_request(&cli, Method::GET, "/v1/status", None, None) {
        Ok(_response) => return Err("request unexpectedly succeeded".into()),
        Err(error) => error,
    };
    join_server(server)?;
    let payload = error.payload();

    if payload.code != "internal_error" {
        return Err(format!("unexpected code: {}", payload.code).into());
    }
    if !payload.message.contains("invalid JSON") {
        return Err(format!("unexpected message: {}", payload.message).into());
    }
    Ok(())
}

#[test]
fn malformed_error_response_still_reports_http_status() -> Result<(), Box<dyn Error>> {
    let (api_url, server) = serve_once("503 Service Unavailable", "<html>down</html>")?;
    let cli = CliOptions {
        api_url,
        ..CliOptions::default()
    };

    let error = match api_request(&cli, Method::GET, "/v1/status", None, None) {
        Ok(_response) => return Err("request unexpectedly succeeded".into()),
        Err(error) => error,
    };
    join_server(server)?;
    let payload = error.payload();

    if payload.code != "api_error" {
        return Err(format!("unexpected code: {}", payload.code).into());
    }
    if payload.message != "Tovuk API returned HTTP 503." {
        return Err(format!("unexpected message: {}", payload.message).into());
    }
    Ok(())
}

fn serve_once(status: &str, body: &str) -> Result<(String, TestServer), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let api_url = format!("http://{}", listener.local_addr()?);
    let status = status.to_owned();
    let body = body.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let mut request = [0_u8; 1024];
        let _request_size = stream
            .read(&mut request)
            .map_err(|error| error.to_string())?;
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(())
    });
    Ok((api_url, server))
}

fn join_server(server: TestServer) -> Result<(), Box<dyn Error>> {
    match server.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("test server panicked".into()),
    }
}
