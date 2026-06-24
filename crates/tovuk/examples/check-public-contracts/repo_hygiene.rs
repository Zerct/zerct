use std::{collections::BTreeSet, fs, path::Path, process::Command};

use crate::helpers::{CheckResult, read_text};

const MAX_AGENTS_CHAIN_BYTES: u64 = 32_768;
const MAX_SOURCE_FILE_LINES: usize = 500;

const REQUIRED_TRACKED_PATHS: &[&str] = &[
    ".github/workflows/ci.yml",
    ".github/workflows/docs-deploy.yml",
    ".github/workflows/docs-score.yml",
    ".github/workflows/docs-validate.yml",
    ".github/workflows/publish-crates.yml",
    ".github/workflows/publish-native-binaries.yml",
    ".github/workflows/publish-npm.yml",
    ".github/workflows/publish-pypi.yml",
    ".gitignore",
    ".github/actionlint.yaml",
    ".typos.toml",
    ".vacuum.yaml",
    "AGENTS.md",
    "README.md",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/examples/check-github-actions.rs",
    "crates/tovuk/examples/check-prose-style.rs",
    "crates/tovuk/examples/check-public-contracts/main.rs",
    "crates/tovuk/examples/check-public-contracts/repo_hygiene.rs",
    "crates/tovuk/src/main.rs",
    "docs/docs.json",
    "docs/openapi.json",
    "deny.toml",
    "Formula/tovuk.rb",
    "packages/tovuk/package.json",
    "packages/tovuk-py/pyproject.toml",
    "scripts/check-all.sh",
    "scripts/check-github-actions.sh",
    "scripts/check-openapi.sh",
    "scripts/check-prose-style.sh",
    "scripts/check-public-contracts.sh",
    "scripts/check-shell-style.sh",
    "scripts/check-toml-style.sh",
    "scripts/check-typos.sh",
    "skills/tovuk/SKILL.md",
];

const REQUIRED_IGNORED_PATHS: &[&str] = &[
    ".env",
    ".env.local",
    ".npmrc",
    ".pypirc",
    ".tovuk/example",
    "crates/tovuk/target/example",
    "docs/.mintlify/example",
    "node_modules/example",
    "packages/tovuk/dist/example",
    "packages/tovuk/node_modules/example",
];

pub(crate) fn check() -> CheckResult {
    let tracked_files = git_lines(&["ls-files"])?;
    let tracked_set = tracked_files.iter().cloned().collect::<BTreeSet<_>>();

    require_tracked_paths(&tracked_set)?;
    require_agents_chain_size()?;
    reject_retired_npx_guidance(&tracked_files)?;
    reject_tracked_go_files(&tracked_files)?;
    reject_go_toolchain_bootstrap(&tracked_files)?;
    reject_oversized_source_files(&tracked_files)?;
    reject_forbidden_tracked_files(&tracked_files)?;
    reject_untracked_files()?;
    require_ignored_paths()?;

    println!("Checked public repository hygiene.");
    Ok(())
}

fn require_tracked_paths(tracked_set: &BTreeSet<String>) -> CheckResult {
    let missing = REQUIRED_TRACKED_PATHS
        .iter()
        .copied()
        .filter(|path| !tracked_set.contains(*path))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "These required public repo files are not tracked:\n{}",
            missing.join("\n")
        ))
    }
}

fn require_agents_chain_size() -> CheckResult {
    let size = fs::metadata("AGENTS.md")
        .map_err(|error| format!("stat AGENTS.md: {error}"))?
        .len();
    if size <= MAX_AGENTS_CHAIN_BYTES {
        Ok(())
    } else {
        Err(format!(
            "AGENTS.md is {size} bytes, above Codex default project_doc_max_bytes {MAX_AGENTS_CHAIN_BYTES}"
        ))
    }
}

fn reject_retired_npx_guidance(tracked_files: &[String]) -> CheckResult {
    let mut matches = Vec::new();
    for path in tracked_files {
        if !is_public_text_scan_path(path) || !Path::new(path).is_file() {
            continue;
        }
        let source = read_text(path)?;
        for (index, line) in source.lines().enumerate() {
            if line_contains_retired_npm_runner_guidance(line) {
                matches.push(format!("{}:{}", path, index + 1));
            }
        }
    }
    if matches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Use native `tovuk` guidance instead of retired npm-runner guidance:\n{}",
            matches.join("\n")
        ))
    }
}

fn reject_tracked_go_files(tracked_files: &[String]) -> CheckResult {
    let go_files = tracked_files
        .iter()
        .filter(|path| path_has_extension(path, "go") && Path::new(path.as_str()).exists())
        .cloned()
        .collect::<Vec<_>>();
    if go_files.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Tracked Go source is not allowed in the public repo; use Rust-native checks:\n{}",
            go_files.join("\n")
        ))
    }
}

fn reject_go_toolchain_bootstrap(tracked_files: &[String]) -> CheckResult {
    let mut matches = Vec::new();
    for path in tracked_files {
        if !is_go_toolchain_scan_path(path) || !Path::new(path).is_file() {
            continue;
        }
        let source = read_text(path)?;
        for (index, line) in source.lines().enumerate() {
            if line_contains_forbidden_go_toolchain(line) {
                matches.push(format!("{}:{}", path, index + 1));
            }
        }
    }
    if matches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Public repo tooling must not bootstrap Go toolchains; use Rust-native or prebuilt native release tools:\n{}",
            matches.join("\n")
        ))
    }
}

fn reject_oversized_source_files(tracked_files: &[String]) -> CheckResult {
    let mut oversized = Vec::new();
    for path in tracked_files {
        if !Path::new(path).is_file() || !is_guarded_source_path(path) {
            continue;
        }
        let source = read_text(path)?;
        let line_count = source.lines().count();
        if line_count > MAX_SOURCE_FILE_LINES {
            oversized.push(format!("{path}:{line_count}"));
        }
    }
    if oversized.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Tracked public source files must stay at or below {MAX_SOURCE_FILE_LINES} lines; split these files first:\n{}",
            oversized.join("\n")
        ))
    }
}

fn reject_forbidden_tracked_files(tracked_files: &[String]) -> CheckResult {
    let forbidden = tracked_files
        .iter()
        .filter(|path| is_forbidden_tracked_path(path))
        .cloned()
        .collect::<Vec<_>>();
    if forbidden.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "These secret/generated files are tracked and must be removed from git:\n{}",
            forbidden.join("\n")
        ))
    }
}

fn reject_untracked_files() -> CheckResult {
    let untracked = git_lines(&["ls-files", "--others", "--exclude-standard"])?;
    if untracked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "These files are not tracked and not ignored:\n{}\nCommit them if they are source, or add a precise .gitignore rule if generated/secret.",
            untracked.join("\n")
        ))
    }
}

fn require_ignored_paths() -> CheckResult {
    for path in REQUIRED_IGNORED_PATHS {
        git_status_success(&["check-ignore", "-q", path])?
            .then_some(())
            .ok_or_else(|| format!("{path} must be ignored"))?;
    }
    Ok(())
}

fn is_public_text_scan_path(path: &str) -> bool {
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

fn is_go_toolchain_scan_path(path: &str) -> bool {
    is_checked_text_path(path)
        && (path == "AGENTS.md"
            || path.starts_with(".github/")
            || path.starts_with("docs/")
            || path.starts_with("packages/")
            || path.starts_with("scripts/")
            || path.starts_with("skills/"))
}

fn line_contains_retired_npm_runner_guidance(line: &str) -> bool {
    let words = line
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words
        .windows(2)
        .any(|pair| pair.first() == Some(&"npx") && pair.get(1) == Some(&"tovuk"))
}

fn line_contains_forbidden_go_toolchain(line: &str) -> bool {
    const FORBIDDEN_PATTERNS: &[&str] = &[
        "actions/setup-go",
        "go.dev/dl",
        "/opt/tovuk/go/bin",
        "/opt/tovuk/go-tools",
        "setup-go",
    ];

    if line_has_ascii_word_pair(line, "go", "install") {
        return true;
    }
    FORBIDDEN_PATTERNS
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn line_has_ascii_word_pair(line: &str, first: &str, second: &str) -> bool {
    let words = line
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words
        .windows(2)
        .any(|pair| pair.first() == Some(&first) && pair.get(1) == Some(&second))
}

fn is_guarded_source_path(path: &str) -> bool {
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

fn is_forbidden_tracked_path(path: &str) -> bool {
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

fn path_has_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|actual| actual.eq_ignore_ascii_case(extension))
}

fn git_lines(args: &[&str]) -> CheckResult<Vec<String>> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("run git {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        ));
    }
    Ok(String::from_utf8_lossy(output.stdout.as_slice())
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn git_status_success(args: &[&str]) -> CheckResult<bool> {
    Command::new("git")
        .args(args)
        .status()
        .map(|status| status.success())
        .map_err(|error| format!("run git {}: {error}", args.join(" ")))
}
