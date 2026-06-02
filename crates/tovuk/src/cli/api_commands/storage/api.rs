use reqwest::Method;
use serde_json::{Value, json};

use super::super::{common::service_route, http::api_request};
use crate::cli::{
    args::CliOptions, auth::read_or_login_token, errors::Result, project::encode_component,
};

pub(super) fn list_response(cli: &CliOptions) -> Result<Value> {
    let route = service_route(cli, "storage")?;
    let token = read_or_login_token(cli)?;
    api_request(cli, Method::GET, &route, Some(&token), None)
}

pub(super) fn upload_url_response(
    cli: &CliOptions,
    token: &str,
    path: &str,
    content_type: &str,
    size_bytes: u64,
    public_read: bool,
) -> Result<Value> {
    api_request(
        cli,
        Method::POST,
        &service_route(cli, "storage/upload-url")?,
        Some(token),
        Some(json!({
            "path": path,
            "contentType": content_type,
            "sizeBytes": size_bytes,
            "publicRead": public_read,
        })),
    )
}

pub(super) fn complete_upload_response(cli: &CliOptions, token: &str, path: &str) -> Result<Value> {
    api_request(
        cli,
        Method::POST,
        &service_route(cli, "storage/complete")?,
        Some(token),
        Some(json!({ "path": path })),
    )
}

pub(super) fn multipart_create_response(
    cli: &CliOptions,
    token: &str,
    path: &str,
    content_type: &str,
    size_bytes: u64,
    part_size_bytes: u64,
    public_read: bool,
) -> Result<Value> {
    api_request(
        cli,
        Method::POST,
        &service_route(cli, "storage/multipart/create")?,
        Some(token),
        Some(json!({
            "path": path,
            "contentType": content_type,
            "sizeBytes": size_bytes,
            "partSizeBytes": part_size_bytes,
            "publicRead": public_read,
        })),
    )
}

pub(super) fn multipart_complete_response(
    cli: &CliOptions,
    token: &str,
    path: &str,
    upload_id: &str,
    parts: &[Value],
) -> Result<Value> {
    api_request(
        cli,
        Method::POST,
        &service_route(cli, "storage/multipart/complete")?,
        Some(token),
        Some(json!({
            "path": path,
            "uploadId": upload_id,
            "parts": parts,
        })),
    )
}

pub(super) fn multipart_abort_response(
    cli: &CliOptions,
    token: &str,
    path: &str,
    upload_id: &str,
) -> Result<Value> {
    api_request(
        cli,
        Method::POST,
        &service_route(cli, "storage/multipart/abort")?,
        Some(token),
        Some(json!({
            "path": path,
            "uploadId": upload_id,
        })),
    )
}

pub(super) fn download_url_response(cli: &CliOptions, remote_path: &str) -> Result<Value> {
    let route = service_route(cli, "storage/download-url")?;
    let token = read_or_login_token(cli)?;
    api_request(
        cli,
        Method::POST,
        &route,
        Some(&token),
        Some(json!({ "path": remote_path })),
    )
}

pub(super) fn delete_response(cli: &CliOptions, token: &str, remote_path: &str) -> Result<Value> {
    api_request(
        cli,
        Method::DELETE,
        &service_route(
            cli,
            &format!("storage?path={}", encode_component(remote_path)),
        )?,
        Some(token),
        None,
    )
}
