mod api;
mod args;
mod output;
mod transfer;

use std::{fs, path::PathBuf};

use serde_json::json;

use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, print_json},
};
use api::{
    complete_upload_response, delete_response, download_url_response, list_response,
    upload_url_response,
};
use args::{
    default_download_path, default_remote_path, required_storage_arg, storage_content_type,
};
use output::{print_delete_result, print_list_result, print_upload_result};
use transfer::{download_presigned_file, upload_presigned_file};

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
        "Use `tovuk storage upload --app <app> ./file.png uploads/file.png --public --json`.",
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

fn storage_download(cli: &CliOptions) -> Result<()> {
    let remote_path = required_storage_arg(
        cli,
        1,
        "missing_storage_path",
        "Storage path is required.",
        "Use `tovuk storage download --app <app> uploads/file.png ./file.png --json`.",
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
        "Use `tovuk storage delete --app <app> uploads/file.png --json`.",
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
        "Use `tovuk storage url --app <app> uploads/file.png --json`.",
    )?;
    let response = download_url_response(cli, &remote_path)?;
    if cli.output.json {
        return print_json(&response);
    }
    println!("{}", super::super::project::string_field(&response, "url"));
    Ok(())
}
