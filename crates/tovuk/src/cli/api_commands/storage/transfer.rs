use std::{
    fs::{self, File},
    path::Path,
};

use reqwest::{
    blocking::{Body, Client},
    header::{CONTENT_LENGTH, CONTENT_TYPE},
};
use serde_json::Value;

use crate::cli::{
    args::CliOptions,
    errors::{Result, agent_error, internal_error},
    project::string_field,
};

pub(super) fn upload_presigned_file(
    cli: &CliOptions,
    upload: &Value,
    local_path: &Path,
    size_bytes: u64,
    content_type: &str,
) -> Result<()> {
    let url = string_field(upload, "url");
    if url.is_empty() {
        return Err(internal_error(
            "Tovuk API did not return a storage upload URL.",
        ));
    }
    let file = File::open(local_path).map_err(|error| {
        agent_error(
            "storage_file_unreadable",
            format!("Could not open local storage file: {error}"),
            "Pass a readable file path and retry the upload.",
            cli.output.json,
        )
    })?;
    let response = Client::new()
        .put(url)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_LENGTH, size_bytes)
        .body(Body::new(file))
        .send()
        .map_err(|error| {
            agent_error(
                "storage_upload_failed",
                format!("Storage upload failed: {error}"),
                "Retry the upload. If it keeps failing, create a support ticket with the service id and storage path.",
                cli.output.json,
            )
        })?;
    if response.status().is_success() {
        return Ok(());
    }
    Err(agent_error(
        "storage_upload_failed",
        format!(
            "Storage upload returned HTTP {}.",
            response.status().as_u16()
        ),
        "Retry the upload. If it keeps failing, create a support ticket with the service id and storage path.",
        cli.output.json,
    ))
}

pub(super) fn download_presigned_file(
    cli: &CliOptions,
    download: &Value,
    destination: &Path,
) -> Result<u64> {
    let url = string_field(download, "url");
    if url.is_empty() {
        return Err(internal_error(
            "Tovuk API did not return a storage download URL.",
        ));
    }
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            agent_error(
                "storage_download_failed",
                format!("Could not create storage download directory: {error}"),
                "Choose a writable destination path and retry the download.",
                cli.output.json,
            )
        })?;
    }
    let mut response = Client::new().get(url).send().map_err(|error| {
        agent_error(
            "storage_download_failed",
            format!("Storage download failed: {error}"),
            "Retry the download. If it keeps failing, create a support ticket with the service id and storage path.",
            cli.output.json,
        )
    })?;
    if !response.status().is_success() {
        return Err(agent_error(
            "storage_download_failed",
            format!(
                "Storage download returned HTTP {}.",
                response.status().as_u16()
            ),
            "Retry the download. If it keeps failing, create a support ticket with the service id and storage path.",
            cli.output.json,
        ));
    }
    let mut output = File::create(destination).map_err(|error| {
        agent_error(
            "storage_download_failed",
            format!("Could not create storage download file: {error}"),
            "Choose a writable destination path and retry the download.",
            cli.output.json,
        )
    })?;
    response.copy_to(&mut output).map_err(|error| {
        agent_error(
            "storage_download_failed",
            format!("Could not write storage download file: {error}"),
            "Choose a writable destination path and retry the download.",
            cli.output.json,
        )
    })
}
