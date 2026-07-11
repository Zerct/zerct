use std::{ffi::OsStr, path::Path};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000a] = [
    size_of_val(&has_forbidden_artifact_extension),
    size_of_val(&is_checked_text_path),
    size_of_val(&is_forbidden_directory_component),
    size_of_val(&is_forbidden_tracked_path),
    size_of_val(&is_guarded_source_path),
    size_of_val(&is_local_generated_file),
    size_of_val(&is_local_guidance_file),
    size_of_val(&is_public_repository_scan_path),
    size_of_val(&is_public_text_scan_path),
    size_of_val(&is_sensitive_local_path),
];

/// Return whether a file extension denotes a generated binary or archive.
fn has_forbidden_artifact_extension(file_name: &str) -> bool {
    let extension = Path::new(file_name)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    return matches!(
        extension.as_str(),
        "a" | "crate"
            | "db"
            | "dll"
            | "dmg"
            | "dylib"
            | "egg"
            | "egg-info"
            | "exe"
            | "gz"
            | "jks"
            | "key"
            | "keystore"
            | "o"
            | "p12"
            | "p8"
            | "pem"
            | "pfx"
            | "profdata"
            | "profraw"
            | "pyc"
            | "pyo"
            | "secret"
            | "so"
            | "sqlite"
            | "sqlite3"
            | "tar"
            | "tgz"
            | "wasm"
            | "whl"
            | "zip"
    );
}

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

/// Return whether a path component belongs only to local or generated state.
fn is_forbidden_directory_component(component: &str) -> bool {
    return matches!(
        component,
        ".agents"
            | ".aws"
            | ".cache"
            | ".cargo-home"
            | ".claude"
            | ".codex"
            | ".continue"
            | ".cursor"
            | ".direnv"
            | ".eggs"
            | ".hypothesis"
            | ".idea"
            | ".mintlify"
            | ".pytest_cache"
            | ".roo"
            | ".ruff_cache"
            | ".ssh"
            | ".terraform"
            | ".tovuk"
            | ".venv"
            | ".vscode"
            | ".windsurf"
            | "__pycache__"
            | "htmlcov"
            | "node_modules"
            | "pip-wheel-metadata"
            | "playwright-report"
            | "target"
            | "test-results"
    ) || component.starts_with(".aider");
}

/// Contract implementation for `is_forbidden_tracked_path`.
pub(super) fn is_forbidden_tracked_path(path: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    return path.starts_with("vendor/")
        || path.starts_with("coverage/")
        || path.starts_with("docs/fonts/")
        || path.starts_with("docs/output/")
        || path.starts_with("output/")
        || path.starts_with("packages/tovuk/dist/")
        || path.starts_with("packages/tovuk-py/build/")
        || path.starts_with("packages/tovuk-py/dist/")
        || path.split('/').any(is_forbidden_directory_component)
        || is_sensitive_local_path(path, file_name)
        || is_local_guidance_file(file_name)
        || is_local_generated_file(file_name)
        || has_forbidden_artifact_extension(file_name);
}

/// Contract implementation for `is_guarded_source_path`.
pub(super) fn is_guarded_source_path(path: &str) -> bool {
    return matches!(
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
    let extension = Path::new(file_name)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    return matches!(
        file_name,
        ".coverage" | ".DS_Store" | "CACHEDIR.TAG" | "Thumbs.db" | "coverage.xml"
    ) || file_name.ends_with('~')
        || matches!(
            extension.as_str(),
            "bak" | "log" | "orig" | "rej" | "swo" | "swp" | "tmp"
        );
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

/// Return whether a tracked repository file must receive leakage scanning.
pub(super) fn is_public_repository_scan_path(path: &str) -> bool {
    return Path::new(path).is_file();
}

/// Contract implementation for `is_public_text_scan_path`.
pub(super) fn is_public_text_scan_path(path: &str) -> bool {
    return is_checked_text_path(path)
        && (path == "AGENTS.md"
            || path == "README.md"
            || path == "SECURITY.md"
            || path.starts_with(".github/")
            || path.starts_with("checks/")
            || path.starts_with("crates/")
            || path.starts_with("docs/")
            || path.starts_with("Formula/")
            || path.starts_with("packages/")
            || path.starts_with("scripts/")
            || path.starts_with("skills/"));
}

/// Return whether a path names credentials or other private local controls.
fn is_sensitive_local_path(path: &str, file_name: &str) -> bool {
    let cargo_credential = path == ".cargo/credentials"
        || path == ".cargo/credentials.toml"
        || path.ends_with("/.cargo/credentials")
        || path.ends_with("/.cargo/credentials.toml");
    let lower_file_name = file_name.to_ascii_lowercase();
    return cargo_credential
        || path == ".npmrc"
        || matches!(
            file_name,
            ".env" | ".envrc" | ".git-credentials" | ".netrc" | ".pypirc" | "id_ed25519" | "id_rsa"
        )
        || (file_name.starts_with(".env.") && file_name != ".env.example")
        || path_has_extension(file_name, "tfstate")
        || path_has_extension(file_name, "tfvars")
        || lower_file_name.contains(".tfstate.")
        || lower_file_name.ends_with(".tfvars.json");
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
