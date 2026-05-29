use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use reqwest::{
    Method,
    blocking::{Body, Client},
    header::{CONTENT_LENGTH, CONTENT_TYPE},
};
use serde_json::{Value, json};

use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, internal_error, print_json},
    project::{encode_component, string_field},
};
use super::{common::app_route, http::api_request};

pub(crate) fn storage_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => storage_list(cli),
        "upload" | "put" => storage_upload(cli),
        "download" | "get" => storage_download(cli),
        "delete" | "rm" => storage_delete(cli),
        "url" => storage_download_url(cli),
        _ => Err(agent_error(
            "unknown_storage_command",
            "Unknown storage command.",
            "Use `tovuk storage list`, `storage upload`, `storage download`, `storage delete`, or `storage url`.",
            cli.output.json,
        )),
    }
}

fn storage_list(cli: &CliOptions) -> Result<()> {
    let response = storage_list_response(cli)?;
    if cli.output.json {
        return print_json(&response);
    }

    let empty_objects: &[Value] = &[];
    let objects = response
        .get("objects")
        .and_then(Value::as_array)
        .map_or(empty_objects, Vec::as_slice);
    if objects.is_empty() {
        println!("no storage objects");
        return Ok(());
    }

    for object in objects {
        let path = string_field(object, "path");
        let size = object
            .get("sizeBytes")
            .and_then(Value::as_u64)
            .map_or_else(String::new, |value| value.to_string());
        let status = string_field(object, "status");
        let public_url = object
            .get("publicUrl")
            .and_then(Value::as_str)
            .unwrap_or("");
        if public_url.is_empty() {
            println!("{path}\t{size}\t{status}");
        } else {
            println!("{path}\t{size}\t{status}\t{public_url}");
        }
    }
    Ok(())
}

fn storage_upload(cli: &CliOptions) -> Result<()> {
    let local_path = required_storage_arg(
        cli,
        1,
        "missing_storage_file",
        "Local file path is required.",
        "Use `tovuk storage upload --app <app> ./file.png uploads/file.png --public --json`.",
    )?;
    let local_path = PathBuf::from(local_path);
    let remote_path =
        optional_storage_arg(cli, 2).map_or_else(|| default_remote_path(&local_path), Ok)?;
    let metadata = fs::metadata(&local_path).map_err(|error| {
        agent_error(
            "storage_file_unreadable",
            format!("Could not read local storage file: {error}"),
            "Pass a readable file path and retry the upload.",
            cli.output.json,
        )
    })?;
    if !metadata.is_file() {
        return Err(agent_error(
            "storage_file_unreadable",
            "Storage upload source must be a file.",
            "Pass a regular file path and retry the upload.",
            cli.output.json,
        ));
    }

    let content_type = storage_content_type(cli, &local_path);
    let token = read_or_login_token(cli)?;
    let upload = api_request(
        cli,
        Method::POST,
        &app_route(cli, "storage/upload-url")?,
        Some(&token),
        Some(json!({
            "path": remote_path,
            "contentType": content_type,
            "sizeBytes": metadata.len(),
            "publicRead": cli.storage.public_read,
        })),
    )?;
    upload_presigned_file(cli, &upload, &local_path, metadata.len(), &content_type)?;
    let completed = api_request(
        cli,
        Method::POST,
        &app_route(cli, "storage/complete")?,
        Some(&token),
        Some(json!({ "path": string_field(&upload, "path") })),
    )?;

    if cli.output.json {
        return print_json(&completed);
    }
    print_storage_object_summary(&completed);
    Ok(())
}

fn storage_download(cli: &CliOptions) -> Result<()> {
    let remote_path = required_storage_arg(
        cli,
        1,
        "missing_storage_path",
        "Storage path is required.",
        "Use `tovuk storage download --app <app> uploads/file.png ./file.png --json`.",
    )?;
    let destination = optional_storage_arg(cli, 2).map_or_else(
        || default_download_path(&remote_path),
        |value| Ok(PathBuf::from(value)),
    )?;
    let download = storage_download_url_response(cli, &remote_path)?;
    let bytes = download_presigned_file(cli, &download, &destination)?;

    if cli.output.json {
        return print_json(&json!({
            "path": remote_path,
            "file": destination.display().to_string(),
            "bytes": bytes,
        }));
    }
    println!("downloaded {remote_path} to {}", destination.display());
    Ok(())
}

fn storage_delete(cli: &CliOptions) -> Result<()> {
    let remote_path = required_storage_arg(
        cli,
        1,
        "missing_storage_path",
        "Storage path is required.",
        "Use `tovuk storage delete --app <app> uploads/file.png --json`.",
    )?;
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::DELETE,
        &app_route(
            cli,
            &format!("storage?path={}", encode_component(&remote_path)),
        )?,
        Some(&token),
        None,
    )?;
    if cli.output.json {
        return print_json(&response);
    }
    println!(
        "{} {}",
        if response
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "deleted"
        } else {
            "not-found"
        },
        string_field(&response, "path")
    );
    Ok(())
}

fn storage_download_url(cli: &CliOptions) -> Result<()> {
    let remote_path = required_storage_arg(
        cli,
        1,
        "missing_storage_path",
        "Storage path is required.",
        "Use `tovuk storage url --app <app> uploads/file.png --json`.",
    )?;
    let response = storage_download_url_response(cli, &remote_path)?;
    if cli.output.json {
        return print_json(&response);
    }
    println!("{}", string_field(&response, "url"));
    Ok(())
}

fn storage_list_response(cli: &CliOptions) -> Result<Value> {
    let token = read_or_login_token(cli)?;
    api_request(
        cli,
        Method::GET,
        &app_route(cli, "storage")?,
        Some(&token),
        None,
    )
}

fn storage_download_url_response(cli: &CliOptions, remote_path: &str) -> Result<Value> {
    let token = read_or_login_token(cli)?;
    api_request(
        cli,
        Method::POST,
        &app_route(cli, "storage/download-url")?,
        Some(&token),
        Some(json!({ "path": remote_path })),
    )
}

fn upload_presigned_file(
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
                "Retry the upload. If it keeps failing, create a support ticket with the app id and storage path.",
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
        "Retry the upload. If it keeps failing, create a support ticket with the app id and storage path.",
        cli.output.json,
    ))
}

fn download_presigned_file(cli: &CliOptions, download: &Value, destination: &Path) -> Result<u64> {
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
            "Retry the download. If it keeps failing, create a support ticket with the app id and storage path.",
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
            "Retry the download. If it keeps failing, create a support ticket with the app id and storage path.",
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

fn print_storage_object_summary(object: &Value) {
    let path = string_field(object, "path");
    let size = object
        .get("sizeBytes")
        .and_then(Value::as_u64)
        .map_or_else(String::new, |value| value.to_string());
    let public_url = object
        .get("publicUrl")
        .and_then(Value::as_str)
        .unwrap_or("");
    if public_url.is_empty() {
        println!("uploaded {path} {size}");
    } else {
        println!("uploaded {path} {size} {public_url}");
    }
}

fn required_storage_arg(
    cli: &CliOptions,
    index: usize,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    optional_storage_arg(cli, index).ok_or_else(|| {
        agent_error(
            code,
            message.to_owned(),
            instruction.to_owned(),
            cli.output.json,
        )
    })
}

fn optional_storage_arg(cli: &CliOptions, index: usize) -> Option<String> {
    cli.args
        .get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
}

fn default_remote_path(local_path: &Path) -> Result<String> {
    local_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| internal_error("Could not infer storage path from local file name."))
}

fn default_download_path(remote_path: &str) -> Result<PathBuf> {
    Path::new(remote_path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| internal_error("Could not infer download file name from storage path."))
}

fn storage_content_type(cli: &CliOptions, local_path: &Path) -> String {
    if !cli.content_type.is_empty() {
        return cli.content_type.clone();
    }
    match local_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("avif") => "image/avif",
        Some("css") => "text/css",
        Some("gif") => "image/gif",
        Some("html" | "htm") => "text/html",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("js" | "mjs") => "text/javascript",
        Some("json") => "application/json",
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("txt" | "md") => "text/plain",
        Some("wasm") => "application/wasm",
        Some("wav") => "audio/wav",
        Some("webm") => "video/webm",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
    .to_owned()
}
