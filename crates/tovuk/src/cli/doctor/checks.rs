use super::{
    cargo_lints::cargo_lints,
    report::{DoctorCheck, doctor_check},
};
use crate::cli::{
    config::TovukConfig,
    constants::RUST_STRICT_CLIPPY_DENY_LINTS,
    frontend_checks::static_frontend_checks,
    project::walk_project_files,
    project_kind::ProjectKind,
    project_layout::{
        fullstack_backend_required_files, fullstack_frontend_required_files, required_files,
    },
    source_policy::backend_javascript_or_typescript_sources,
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn required_file_checks(project_dir: &Path, kind: ProjectKind) -> Vec<DoctorCheck> {
    required_files(project_dir, kind)
        .iter()
        .map(|file| {
            let ok = project_dir.join(file).exists();
            doctor_check(
                file,
                ok,
                "found",
                "missing",
                &format!("Create and commit {file}, then retry."),
            )
        })
        .collect()
}

pub(super) fn fullstack_checks(
    project_dir: &Path,
    config: &TovukConfig,
    config_valid: bool,
) -> Vec<DoctorCheck> {
    let backend_root = config.backend.root.clone().unwrap_or_default();
    let frontend_root = config.frontend.root.clone().unwrap_or_default();
    let backend_dir = project_dir.join(&backend_root);
    let frontend_dir = project_dir.join(&frontend_root);
    let mut checks = Vec::new();
    checks.extend(required_files_at(
        &backend_dir,
        &backend_root,
        fullstack_backend_required_files(),
    ));
    checks.push(backend_javascript_source_check(&backend_dir, &backend_root));
    checks.extend(rust_backend_checks(&backend_dir, config_valid));
    checks.extend(required_files_at(
        &frontend_dir,
        &frontend_root,
        fullstack_frontend_required_files(&frontend_dir),
    ));
    checks.extend(static_frontend_checks(&frontend_dir, config_valid));
    checks
}

pub(super) fn rust_doctor_checks(
    project_dir: &Path,
    kind: ProjectKind,
    config_valid: bool,
) -> Vec<DoctorCheck> {
    if kind.is_static_frontend() {
        let mut checks = static_frontend_checks(project_dir, config_valid);
        checks.push(unsafe_check(project_dir));
        return checks;
    }

    let mut checks = vec![backend_javascript_source_check(project_dir, "")];
    checks.extend(rust_backend_checks(project_dir, config_valid));
    checks
}

fn required_files_at(project_dir: &Path, label: &str, files: &[&str]) -> Vec<DoctorCheck> {
    files
        .iter()
        .map(|file| {
            let display = if label.is_empty() {
                (*file).to_owned()
            } else {
                format!("{label}/{file}")
            };
            doctor_check(
                &display,
                project_dir.join(file).exists(),
                "found",
                "missing",
                &format!("Create and commit {display}, then retry."),
            )
        })
        .collect()
}

fn rust_backend_checks(project_dir: &Path, config_valid: bool) -> Vec<DoctorCheck> {
    let mut checks = vec![cargo_lints(project_dir), unsafe_check(project_dir)];
    if config_valid {
        checks.extend(rust_command_checks(project_dir));
    }
    checks
}

fn rust_command_checks(project_dir: &Path) -> Vec<DoctorCheck> {
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

fn strict_clippy_check(project_dir: &Path) -> DoctorCheck {
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
) -> DoctorCheck {
    let result = Command::new("cargo")
        .args(args)
        .current_dir(project_dir)
        .env("CARGO_TERM_COLOR", "never")
        .stdin(Stdio::null())
        .output();
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            return DoctorCheck {
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
    DoctorCheck {
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

fn backend_javascript_source_check(project_dir: &Path, label: &str) -> DoctorCheck {
    let matches = backend_javascript_or_typescript_sources(project_dir, label);
    DoctorCheck {
        name: "rust backend js/ts server source".to_owned(),
        ok: matches.is_empty(),
        message: if matches.is_empty() {
            "none found".to_owned()
        } else {
            matches
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        },
        agent_instruction: if matches.is_empty() {
            None
        } else {
            Some("Move API routes, SSR handlers, middleware, and server logic to Rust. Keep JS/TS only in static frontend roots.".to_owned())
        },
    }
}

fn unsafe_check(project_dir: &Path) -> DoctorCheck {
    let hits = scan_unsafe(project_dir);
    DoctorCheck {
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
