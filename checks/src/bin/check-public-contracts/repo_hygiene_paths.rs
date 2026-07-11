use alloc::collections::BTreeSet;

use crate::helpers::CheckResult;

use std::{ffi::OsStr, path::Path};

/// Reviewed files allowed directly at the public repository root.
const ALLOWED_ROOT_FILES: &[&str] = &[
    ".editorconfig",
    ".gitattributes",
    ".gitignore",
    ".oxlintrc.json",
    ".prettierrc.json",
    ".vacuum.yaml",
    "AGENTS.md",
    "LICENSE",
    "README.md",
    "SECURITY.md",
    "clippy.toml",
    "deny.toml",
    "dependency-feature-policy.json",
    "native-release-targets.json",
    "public-tree.json",
    "rust-toolchain.toml",
    "rustfmt.toml",
];

/// Reviewed top-level directories allowed on the public Git surface.
const ALLOWED_TOP_LEVEL_DIRECTORIES: &[&str] = &[
    ".cargo",
    ".githooks",
    ".github",
    "Formula",
    "checks",
    "crates",
    "docs",
    "packages",
    "skills",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000e] = [
    size_of_val(&has_forbidden_artifact_extension),
    size_of_val(&is_allowed_public_surface_path),
    size_of_val(&is_checked_text_path),
    size_of_val(&is_forbidden_directory_component),
    size_of_val(&is_forbidden_tracked_path),
    size_of_val(&is_guarded_source_path),
    size_of_val(&is_local_generated_file),
    size_of_val(&is_local_guidance_file),
    size_of_val(&is_portable_component),
    size_of_val(&is_public_repository_scan_path),
    size_of_val(&is_public_text_scan_path),
    size_of_val(&is_sensitive_local_path),
    size_of_val(&is_windows_reserved_name),
    size_of_val(&validate_portable_public_paths),
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

/// Return whether a tracked path belongs to the reviewed public repository surface.
pub(super) fn is_allowed_public_surface_path(path: &str) -> bool {
    let mut components = path.split('/');
    let Some(root) = components.next() else {
        return false;
    };
    if root.is_empty() {
        return false;
    }
    let nested = components.next().is_some();
    return if nested {
        ALLOWED_TOP_LEVEL_DIRECTORIES.contains(&root)
    } else {
        ALLOWED_ROOT_FILES.contains(&root)
    };
}

/// Contract implementation for `is_checked_text_path`.
fn is_checked_text_path(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    return matches!(
        extension.as_str(),
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
            | "yml"
    );
}

/// Return whether a path component belongs only to local or generated state.
fn is_forbidden_directory_component(component: &str) -> bool {
    let lower = component.to_ascii_lowercase();
    return matches!(
        lower.as_str(),
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
            | ".git"
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
            | "coverage"
            | "htmlcov"
            | "node_modules"
            | "pip-wheel-metadata"
            | "playwright-report"
            | "target"
            | "test-results"
            | "vendor"
    ) || lower.starts_with(".aider")
        || lower.ends_with(".egg-info");
}

/// Contract implementation for `is_forbidden_tracked_path`.
pub(super) fn is_forbidden_tracked_path(path: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    return path == "docs/README.md"
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
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    return matches!(
        extension.as_str(),
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
            | "yml"
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

/// Return whether one path component is portable across public Git hosts.
fn is_portable_component(component: &str) -> bool {
    let invalid_windows_byte = component.bytes().any(|byte| {
        return byte.is_ascii_control()
            || matches!(byte, b'<' | b'>' | b':' | b'"' | b'\\' | b'|' | b'?' | b'*');
    });
    let starts_or_ends_with_space = component.starts_with(' ') || component.ends_with(' ');
    return !component.is_empty()
        && component.is_ascii()
        && !matches!(component, "." | "..")
        && !component.ends_with('.')
        && !starts_or_ends_with_space
        && !invalid_windows_byte
        && !is_windows_reserved_name(component);
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
    let lower_path = path.to_ascii_lowercase();
    let cargo_credential = lower_path == ".cargo/credentials"
        || lower_path == ".cargo/credentials.toml"
        || lower_path.ends_with("/.cargo/credentials")
        || lower_path.ends_with("/.cargo/credentials.toml");
    let lower_file_name = file_name.to_ascii_lowercase();
    return cargo_credential
        || matches!(
            lower_file_name.as_str(),
            ".env"
                | ".envrc"
                | ".git-credentials"
                | ".netrc"
                | ".npmrc"
                | ".pypirc"
                | "id_ed25519"
                | "id_rsa"
        )
        || (lower_file_name.starts_with(".env.") && lower_file_name != ".env.example")
        || path_has_extension(file_name, "tfstate")
        || path_has_extension(file_name, "tfvars")
        || lower_file_name.contains(".tfstate.")
        || lower_file_name.ends_with(".tfvars.json");
}

/// Return whether one basename uses a reserved Windows device stem.
fn is_windows_reserved_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(stem.as_str(), "aux" | "con" | "nul" | "prn") {
        return true;
    }
    let Some((prefix, digit)) = stem.split_at_checked(0x0003) else {
        return false;
    };
    return matches!(prefix, "com" | "lpt")
        && digit.len() == 0x0001
        && matches!(digit.as_bytes().first(), Some(b'1'..=b'9'));
}

/// Contract implementation for `path_has_extension`.
pub(super) fn path_has_extension(path: &str, extension: &str) -> bool {
    return Path::new(path)
        .extension()
        .is_some_and(|actual| return actual.eq_ignore_ascii_case(extension));
}

/// Require canonical ASCII paths with no cross-platform case collisions.
///
/// # Errors
///
/// Returns an error for unsafe components or duplicate case-folded paths.
pub(super) fn validate_portable_public_paths(paths: &BTreeSet<String>) -> CheckResult {
    let mut folded_paths = BTreeSet::new();
    for path in paths {
        if path
            .split('/')
            .any(|component| return !is_portable_component(component))
        {
            return Err(format!(
                "tracked path is not portable across public platforms: {path:?}"
            ));
        }
        let folded = path.to_ascii_lowercase();
        if !folded_paths.insert(folded) {
            return Err(format!(
                "tracked paths collide under ASCII case folding at {path:?}"
            ));
        }
    }
    return Ok(());
}
#[cfg(test)]
#[path = "repo_hygiene_paths_tests/verification.rs"]
mod tests;
