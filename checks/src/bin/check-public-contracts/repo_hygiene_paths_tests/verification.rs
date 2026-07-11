use super::is_forbidden_tracked_path;

/// Verify generated machine-local files are forbidden from version control.
///
/// # Panics
///
/// Panics when a generated machine-local fixture is accepted.
#[test]
fn rejects_forced_generated_local_files() {
    for path in [".DS_Store", "docs/.DS_Store", "debug.log"] {
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
