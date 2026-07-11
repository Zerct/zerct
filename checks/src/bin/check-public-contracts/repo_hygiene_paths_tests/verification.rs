use crate::repo_hygiene_text::line_contains_private_repository_marker;

use super::is_forbidden_tracked_path;

/// Verify public source and package-control paths remain eligible for tracking.
///
/// # Panics
///
/// Panics when a legitimate source path is rejected.
#[test]
fn allows_public_source_paths() {
    for path in [
        ".env.example",
        "crates/tovuk/src/build/module.rs",
        "crates/tovuk/src/dist/schema.rs",
        "docs/sdks/rust.mdx",
        "packages/tovuk/.npmrc",
        "sdks/rust/src/lib.rs",
    ] {
        assert!(!is_forbidden_tracked_path(path), "{path}");
    }
}

/// Verify ordinary public URLs and prose do not look like private paths.
///
/// # Panics
///
/// Panics when harmless public copy is rejected.
#[test]
fn allows_public_urls_and_prose() {
    for source in [
        "https://api.github.com/users/example",
        "developer home users",
        "the engine supports apps and crates",
    ] {
        assert!(!line_contains_private_repository_marker(source), "{source}");
    }
}

/// Verify generated machine-local files are forbidden from version control.
///
/// # Panics
///
/// Panics when a generated machine-local fixture is accepted.
#[test]
fn rejects_forced_generated_local_files() {
    for path in [
        ".DS_Store",
        "archive.zip",
        "ARCHIVE.ZIP",
        "bin/tool.exe",
        "docs/.DS_Store",
        "debug.log",
        "DEBUG.LOG",
        "package.crate",
        "package.whl",
        "package.zip",
        "packages/tovuk/dist/tovuk",
        "target/libexample.dylib",
        "target/object.o",
        "vendor/example/src/lib.rs",
    ] {
        assert!(is_forbidden_tracked_path(path), "{path}");
    }
}

/// Verify alternate local agent guidance files are forbidden from version control.
///
/// # Panics
///
/// Panics when a local guidance fixture is accepted.
#[test]
fn rejects_forced_local_agent_guidance() {
    for path in [
        "AGENTS.override.md",
        "docs/AGENTS.override.md",
        "CLAUDE.md",
        "GEMINI.md",
        "PUBLISHING.md",
        "npm-support-request.md",
    ] {
        assert!(is_forbidden_tracked_path(path), "{path}");
    }
}

/// Verify ignored credentials and local state cannot be force-added.
///
/// # Panics
///
/// Panics when a sensitive or generated path is accepted.
#[test]
fn rejects_forced_sensitive_and_local_paths() {
    for path in [
        ".aws/credentials",
        ".cargo/credentials.toml",
        ".envrc",
        ".git-credentials",
        ".npmrc",
        ".pypirc",
        ".ssh/config",
        ".terraform/providers.lock",
        ".vscode/settings.json",
        "crates/tovuk/.cargo/credentials",
        "example.tfstate.backup",
        "example.tfvars.json",
        "packages/tovuk/node_modules/tovuk/index.js",
    ] {
        assert!(is_forbidden_tracked_path(path), "{path}");
    }
}

/// Verify developer-local and private-engine paths are rejected.
///
/// # Panics
///
/// Panics when a private path fixture is accepted.
#[test]
fn rejects_private_repository_paths() {
    let unix_user = ["", "Users", "alice", "Developer", "Tovuk", "engine", "apps"].join("/");
    let unix_home = ["", "home", "alice", "tovuk", "engine", "crates"].join("/");
    let windows = ["C:", "Users", "alice", "Developer", "Tovuk", "engine"].join("\\");
    let relative = ["tovuk", "engine", "crates"].join("/");
    for source in [unix_user, unix_home, windows, relative] {
        assert!(
            line_contains_private_repository_marker(source.as_str()),
            "{source}"
        );
    }
}
