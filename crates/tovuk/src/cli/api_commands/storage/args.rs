use std::path::{Path, PathBuf};

use crate::cli::{
    args::CliOptions,
    errors::{Result, agent_error, internal_error},
};

pub(super) fn required_storage_arg(
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

pub(super) fn optional_storage_arg(cli: &CliOptions, index: usize) -> Option<String> {
    cli.args
        .get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn default_remote_path(local_path: &Path) -> Result<String> {
    local_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| internal_error("Could not infer storage path from local file name."))
}

pub(super) fn default_download_path(remote_path: &str) -> Result<PathBuf> {
    Path::new(remote_path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| internal_error("Could not infer download file name from storage path."))
}

pub(super) fn storage_content_type(cli: &CliOptions, local_path: &Path) -> String {
    if !cli.storage.content_type.is_empty() {
        return cli.storage.content_type.clone();
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
