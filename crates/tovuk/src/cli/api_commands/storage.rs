mod api;
mod args;
mod output;
mod transfer;

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;

use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, print_json},
};
use api::{
    complete_upload_response, delete_response, download_url_response, list_response,
    multipart_abort_response, multipart_complete_response, multipart_create_response,
    upload_url_response,
};
use args::{
    default_download_path, default_remote_path, required_storage_arg, storage_content_type,
};
use output::{print_delete_result, print_list_result, print_upload_result};
use transfer::{download_presigned_file, upload_multipart_file, upload_presigned_file};

const MULTIPART_UPLOAD_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;
const DEFAULT_MULTIPART_PART_BYTES: u64 = 100 * 1024 * 1024;

pub(crate) fn storage_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => storage_list(cli),
        "upload" => storage_upload(cli),
        "download" => storage_download(cli),
        "delete" => storage_delete(cli),
        "url" => storage_download_url(cli),
        _ => Err(agent_error(
            "unknown_storage_command",
            "Unknown storage command.",
            "Use `tovuk storage list`, `tovuk storage upload`, `tovuk storage download`, `tovuk storage delete`, or `tovuk storage url`.",
            cli.output.json,
        )),
    }
}

fn storage_list(cli: &CliOptions) -> Result<()> {
    let response = list_response(cli)?;
    if cli.output.json {
        return print_json(&response);
    }
    print_list_result(&response);
    Ok(())
}

fn storage_upload(cli: &CliOptions) -> Result<()> {
    let local_path = PathBuf::from(required_storage_arg(
        cli,
        1,
        "missing_storage_file",
        "Local file path is required.",
        "Use `tovuk storage upload --service <service> ./file.png uploads/file.png --public --json`.",
    )?);
    let remote_path =
        args::optional_storage_arg(cli, 2).map_or_else(|| default_remote_path(&local_path), Ok)?;
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
    if metadata.len() > MULTIPART_UPLOAD_THRESHOLD_BYTES {
        return storage_multipart_upload(
            cli,
            &token,
            &local_path,
            &remote_path,
            &content_type,
            metadata.len(),
        );
    }
    let upload = upload_url_response(
        cli,
        &token,
        &remote_path,
        &content_type,
        metadata.len(),
        cli.storage.public_read,
    )?;
    upload_presigned_file(cli, &upload, &local_path, metadata.len(), &content_type)?;
    let completed = complete_upload_response(
        cli,
        &token,
        &super::super::project::string_field(&upload, "path"),
    )?;

    if cli.output.json {
        return print_json(&completed);
    }
    print_upload_result(&completed);
    Ok(())
}

fn storage_multipart_upload(
    cli: &CliOptions,
    token: &str,
    local_path: &Path,
    remote_path: &str,
    content_type: &str,
    size_bytes: u64,
) -> Result<()> {
    let upload = multipart_create_response(
        cli,
        token,
        remote_path,
        content_type,
        size_bytes,
        DEFAULT_MULTIPART_PART_BYTES,
        cli.storage.public_read,
    )?;
    let path = super::super::project::string_field(&upload, "path");
    let upload_id = super::super::project::string_field(&upload, "uploadId");
    match upload_multipart_file(cli, &upload, local_path) {
        Ok(parts) => {
            let completed = multipart_complete_response(cli, token, &path, &upload_id, &parts)?;
            if cli.output.json {
                return print_json(&completed);
            }
            print_upload_result(&completed);
            Ok(())
        }
        Err(error) => {
            if !path.is_empty() && !upload_id.is_empty() {
                let _abort_result = multipart_abort_response(cli, token, &path, &upload_id);
            }
            Err(error)
        }
    }
}

fn storage_download(cli: &CliOptions) -> Result<()> {
    let remote_path = required_storage_arg(
        cli,
        1,
        "missing_storage_path",
        "Storage path is required.",
        "Use `tovuk storage download --service <service> uploads/file.png ./file.png --json`.",
    )?;
    let destination = args::optional_storage_arg(cli, 2).map_or_else(
        || default_download_path(&remote_path),
        |value| Ok(PathBuf::from(value)),
    )?;
    let download = download_url_response(cli, &remote_path)?;
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
        "Use `tovuk storage delete --service <service> uploads/file.png --json`.",
    )?;
    let token = read_or_login_token(cli)?;
    let response = delete_response(cli, &token, &remote_path)?;
    if cli.output.json {
        return print_json(&response);
    }
    print_delete_result(&response);
    Ok(())
}

fn storage_download_url(cli: &CliOptions) -> Result<()> {
    let remote_path = required_storage_arg(
        cli,
        1,
        "missing_storage_path",
        "Storage path is required.",
        "Use `tovuk storage url --service <service> uploads/file.png --json`.",
    )?;
    let response = download_url_response(cli, &remote_path)?;
    if cli.output.json {
        return print_json(&response);
    }
    println!("{}", super::super::project::string_field(&response, "url"));
    Ok(())
}
