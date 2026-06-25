use std::path::Path;

pub(crate) fn is_public_text_scan_path(path: &str) -> bool {
    is_checked_text_path(path)
        && (path == "AGENTS.md"
            || path == "README.md"
            || path.starts_with(".github/")
            || path.starts_with("crates/")
            || path.starts_with("docs/")
            || path.starts_with("Formula/")
            || path.starts_with("packages/")
            || path.starts_with("scripts/")
            || path.starts_with("skills/"))
}

pub(crate) fn is_go_toolchain_scan_path(path: &str) -> bool {
    is_checked_text_path(path)
        && (path == "AGENTS.md"
            || path.starts_with(".github/")
            || path.starts_with("docs/")
            || path.starts_with("packages/")
            || path.starts_with("scripts/")
            || path.starts_with("skills/"))
}

pub(crate) fn is_guarded_source_path(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(std::ffi::OsStr::to_str),
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
    )
}

pub(crate) fn is_forbidden_tracked_path(path: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    matches!(file_name, "terraform.tfvars" | ".terraform.tfvars" | ".env")
        || file_name.ends_with(".auto.tfvars")
        || file_name.ends_with(".auto.tfvars.json")
        || path_has_extension(file_name, "tgz")
        || path_has_extension(file_name, "key")
        || path_has_extension(file_name, "pem")
        || path_has_extension(file_name, "secret")
        || (file_name.starts_with(".env.") && file_name != ".env.example")
        || path
            .split('/')
            .any(|component| component == "terraform.tfstate" || component.contains(".tfstate."))
}

pub(crate) fn path_has_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|actual| actual.eq_ignore_ascii_case(extension))
}

fn is_checked_text_path(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(std::ffi::OsStr::to_str),
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
    )
}
