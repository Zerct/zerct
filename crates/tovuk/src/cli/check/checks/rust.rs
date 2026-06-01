use super::super::{cargo_lints::cargo_lints, report::QualityCheck};
use crate::cli::{constants::RUST_STRICT_CLIPPY_DENY_LINTS, project::walk_project_files};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn rust_backend_checks(project_dir: &Path, config_valid: bool) -> Vec<QualityCheck> {
    let mut checks = vec![cargo_lints(project_dir), unsafe_check(project_dir)];
    if config_valid {
        checks.extend(rust_command_checks(project_dir));
    }
    checks
}

fn rust_command_checks(project_dir: &Path) -> Vec<QualityCheck> {
    vec![
        cargo_command_check(
            project_dir,
            "cargo fmt",
            &["fmt", "--all", "--check"],
            "Install rustfmt with Rust, then run `cargo fmt --all --check` before deploying.",
            "Run `cargo fmt --all`, then redeploy.",
        ),
        cargo_command_check(
            project_dir,
            "cargo check",
            &[
                "check",
                "--locked",
                "--release",
                "--all-targets",
                "--all-features",
                "--quiet",
            ],
            "Install Rust and Cargo, then run `cargo check --locked --release --all-targets --all-features` locally before deploying.",
            "Run `cargo check --locked --release --all-targets --all-features`, fix every compiler error and warning, then redeploy.",
        ),
        cargo_command_check(
            project_dir,
            "cargo test",
            &[
                "test",
                "--locked",
                "--release",
                "--all-targets",
                "--all-features",
                "--quiet",
            ],
            "Install Rust and Cargo, then run `cargo test --locked --release --all-targets --all-features` locally before deploying.",
            "Run `cargo test --locked --release --all-targets --all-features`, fix every failed test, then redeploy.",
        ),
        strict_clippy_check(project_dir),
    ]
}

fn strict_clippy_check(project_dir: &Path) -> QualityCheck {
    let mut clippy_args = vec![
        "clippy",
        "--locked",
        "--release",
        "--all-targets",
        "--all-features",
        "--quiet",
        "--",
        "-D",
        "warnings",
    ];
    for lint in RUST_STRICT_CLIPPY_DENY_LINTS {
        clippy_args.push("-D");
        clippy_args.push(lint);
    }
    cargo_command_check(
        project_dir,
        "cargo clippy",
        &clippy_args,
        "Install Rust clippy, then run Tovuk strict Clippy checks before deploying.",
        "Run the strict Tovuk Clippy command from tovuk.toml, fix every warning, panic/unwrap issue, and resource lint, then redeploy.",
    )
}

fn cargo_command_check(
    project_dir: &Path,
    name: &str,
    args: &[&str],
    missing: &str,
    failed: &str,
) -> QualityCheck {
    let result = Command::new("cargo")
        .args(args)
        .current_dir(project_dir)
        .env("CARGO_TERM_COLOR", "never")
        .stdin(Stdio::null())
        .output();
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            return QualityCheck {
                name: name.to_owned(),
                ok: false,
                message: error.to_string(),
                agent_instruction: Some(missing.to_owned()),
            };
        }
    };
    let message = if output.status.success() {
        "passed".to_owned()
    } else {
        first_output_line(&output.stderr, &output.stdout, name)
    };
    QualityCheck {
        name: name.to_owned(),
        ok: output.status.success(),
        message,
        agent_instruction: if output.status.success() {
            None
        } else {
            Some(failed.to_owned())
        },
    }
}

pub(crate) fn first_output_line(stderr: &[u8], stdout: &[u8], fallback: &str) -> String {
    let text = if stderr.is_empty() { stdout } else { stderr };
    let value = String::from_utf8_lossy(text).trim().to_owned();
    let value = if value.is_empty() {
        format!("{fallback} failed")
    } else {
        value
    };
    value.chars().take(240).collect()
}

pub(super) fn unsafe_check(project_dir: &Path) -> QualityCheck {
    let hits = scan_unsafe(project_dir);
    QualityCheck {
        name: "unsafe".to_owned(),
        ok: hits.is_empty(),
        message: if hits.is_empty() {
            "no direct unsafe found".to_owned()
        } else {
            hits.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        },
        agent_instruction: if hits.is_empty() {
            None
        } else {
            Some(
                "Remove direct unsafe usage from workspace Rust source before deploying."
                    .to_owned(),
            )
        },
    }
}

fn scan_unsafe(project_dir: &Path) -> Vec<String> {
    let mut hits = Vec::new();
    walk_project_files(project_dir, |file, relative| {
        if is_rust_source(relative)
            && fs::read_to_string(file).is_ok_and(|source| source.contains("unsafe"))
        {
            hits.push(relative.to_owned());
        }
    });
    hits
}

fn is_rust_source(relative: &str) -> bool {
    Path::new(relative)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}
