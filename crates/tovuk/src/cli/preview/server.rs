use super::super::{
    errors::{Result, agent_error, internal_error},
    project::ensure_directory,
};
use reqwest::{Method, StatusCode, blocking::Client};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
};

pub(super) fn serve_static(root: &Path, port: u16, api_proxy_port: Option<u16>) -> Result<()> {
    ensure_directory(root)?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
        agent_error(
            "preview_failed",
            "Could not start preview server.",
            error.to_string(),
            false,
        )
    })?;
    println!("preview http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        let stream = stream.map_err(|error| internal_error(error.to_string()))?;
        handle_static_request(stream, root, port, api_proxy_port)?;
    }
    Ok(())
}

fn handle_static_request(
    mut stream: TcpStream,
    root: &Path,
    port: u16,
    api_proxy_port: Option<u16>,
) -> Result<()> {
    let mut buffer = [0_u8; 8192];
    let size = stream
        .read(&mut buffer)
        .map_err(|error| internal_error(error.to_string()))?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut request_line = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = request_line.next().unwrap_or("GET");
    let path = request_line.next().unwrap_or("/");
    let pathname = path.split('?').next().unwrap_or("/");
    if api_proxy_port.is_some_and(|_| pathname == "/api" || pathname.starts_with("/api/")) {
        return proxy_to_backend(&mut stream, method, path, api_proxy_port.unwrap_or(port));
    }
    let target = static_target(root, pathname);
    if target.as_os_str().is_empty() {
        write_http_response(
            &mut stream,
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            b"not found",
        )?;
        return Ok(());
    }
    let body = fs::read(&target).map_err(|error| internal_error(error.to_string()))?;
    write_http_response(&mut stream, StatusCode::OK, content_type(&target), &body)
}

fn proxy_to_backend(stream: &mut TcpStream, method: &str, path: &str, port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{port}{path}");
    let method = Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET);
    let response = Client::new().request(method, url).send();
    match response {
        Ok(response) => {
            let status = response.status();
            let body = response
                .bytes()
                .map_err(|error| internal_error(error.to_string()))?;
            write_http_response(stream, status, "application/octet-stream", &body)
        }
        Err(_error) => write_http_response(
            stream,
            StatusCode::BAD_GATEWAY,
            "text/plain; charset=utf-8",
            b"worker unavailable",
        ),
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: StatusCode,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {} {}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("OK"),
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .and_then(|()| stream.write_all(body))
        .map_err(|error| internal_error(error.to_string()))
}

fn static_target(root: &Path, pathname: &str) -> PathBuf {
    let safe_path = pathname.trim_start_matches('/');
    let candidate = normalize_path(&root.join(if safe_path.is_empty() {
        "index.html"
    } else {
        safe_path
    }));
    let root = normalize_path(root);
    if !candidate.starts_with(&root) {
        return PathBuf::new();
    }
    if candidate.is_file() {
        return candidate;
    }
    let index = root.join("index.html");
    if index.is_file() {
        index
    } else {
        PathBuf::new()
    }
}

fn content_type(file: &Path) -> &'static str {
    match file
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}
