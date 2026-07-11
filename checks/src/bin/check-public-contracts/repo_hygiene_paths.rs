use std::{ffi::OsStr, path::Path};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 6] = [
    size_of_val(&is_checked_text_path),
    size_of_val(&is_forbidden_tracked_path),
    size_of_val(&is_guarded_source_path),
    size_of_val(&is_local_generated_file),
    size_of_val(&is_local_guidance_file),
    size_of_val(&is_public_text_scan_path),
];

/// Contract implementation for `is_checked_text_path`.
fn is_checked_text_path(path: &str) -> bool {
    return matches!(
        Path::new(path).extension().and_then(OsStr::to_str),
        Some(
            "css"
                | "js"
                | "jsx"
                | "json"
                | "md"
                | "mdx"
                | "mjs"
                | "py"
                | "rb"
                | "rs"
                | "sh"
                | "txt"
                | "toml"
                | "ts"
                | "tsx"
                | "yaml"
                | "yml",
        )
    );
}

/// Contract implementation for `is_forbidden_tracked_path`.
pub(super) fn is_forbidden_tracked_path(path: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    return matches!(file_name, "terraform.tfvars" | ".terraform.tfvars" | ".env")
        || is_local_guidance_file(file_name)
        || is_local_generated_file(file_name)
        || file_name.ends_with(".auto.tfvars")
        || file_name.ends_with(".auto.tfvars.json")
        || path_has_extension(file_name, "tgz")
        || path_has_extension(file_name, "key")
        || path_has_extension(file_name, "pem")
        || path_has_extension(file_name, "secret")
        || (file_name.starts_with(".env.") && file_name != ".env.example")
        || path.split('/').any(|component| {
            return component == "terraform.tfstate" || component.contains(".tfstate.");
        });
}

/// Contract implementation for `is_guarded_source_path`.
pub(super) fn is_guarded_source_path(path: &str) -> bool {
    return !path.starts_with("vendor/")
        && matches!(
            Path::new(path).extension().and_then(OsStr::to_str),
            Some(
                "css"
                    | "js"
                    | "jsx"
                    | "md"
                    | "mdx"
                    | "mjs"
                    | "py"
                    | "rb"
                    | "rs"
                    | "sh"
                    | "toml"
                    | "ts"
                    | "tsx"
                    | "yaml"
                    | "yml",
            )
        );
}

/// Contract implementation for `is_local_generated_file`.
pub(super) fn is_local_generated_file(file_name: &str) -> bool {
    return matches!(file_name, ".DS_Store") || path_has_extension(file_name, "log");
}

/// Contract implementation for `is_local_guidance_file`.
pub(super) fn is_local_guidance_file(file_name: &str) -> bool {
    return matches!(
        file_name.to_ascii_lowercase().as_str(),
        "agents.override.md"
            | "claude.md"
            | "gemini.md"
            | "publishing.md"
            | "npm-support-request.md"
    );
}

/// Contract implementation for `is_public_text_scan_path`.
pub(super) fn is_public_text_scan_path(path: &str) -> bool {
    return is_checked_text_path(path)
        && (path == "AGENTS.md"
            || path == "README.md"
            || path.starts_with(".github/")
            || path.starts_with("checks/")
            || path.starts_with("crates/")
            || path.starts_with("docs/")
            || path.starts_with("Formula/")
            || path.starts_with("packages/")
            || path.starts_with("scripts/")
            || path.starts_with("skills/"));
}

/// Contract implementation for `path_has_extension`.
pub(super) fn path_has_extension(path: &str, extension: &str) -> bool {
    return Path::new(path)
        .extension()
        .is_some_and(|actual| return actual.eq_ignore_ascii_case(extension));
}
#[cfg(test)]
#[path = "repo_hygiene_paths_tests/verification.rs"]
mod tests;
