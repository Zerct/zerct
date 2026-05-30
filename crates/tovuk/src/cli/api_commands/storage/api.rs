use reqwest::Method;
use serde_json::{Value, json};

use super::super::{common::app_route, http::api_request};
use crate::cli::{
    args::CliOptions, auth::read_or_login_token, errors::Result, project::encode_component,
};

pub(super) fn list_response(cli: &CliOptions) -> Result<Value> {
    let token = read_or_login_token(cli)?;
    api_request(
        cli,
        Method::GET,
        &app_route(cli, "storage")?,
        Some(&token),
        None,
    )
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
        &app_route(cli, "storage/upload-url")?,
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
        &app_route(cli, "storage/complete")?,
        Some(token),
        Some(json!({ "path": path })),
    )
}

pub(super) fn download_url_response(cli: &CliOptions, remote_path: &str) -> Result<Value> {
    let token = read_or_login_token(cli)?;
    api_request(
        cli,
        Method::POST,
        &app_route(cli, "storage/download-url")?,
        Some(&token),
        Some(json!({ "path": remote_path })),
    )
}

pub(super) fn delete_response(cli: &CliOptions, token: &str, remote_path: &str) -> Result<Value> {
    api_request(
        cli,
        Method::DELETE,
        &app_route(
            cli,
            &format!("storage?path={}", encode_component(remote_path)),
        )?,
        Some(token),
        None,
    )
}
