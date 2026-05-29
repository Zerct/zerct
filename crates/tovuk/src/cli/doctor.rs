use super::{
    config::{TovukConfig, parse_tovuk_toml, validate_config},
    constants::RUST_STRICT_CLIPPY_DENY_LINTS,
    deploy::discover_deploy_projects,
    errors::{
        AgentErrorPayload, CliError, CliFailure, Result, agent_error, internal_error, print_json,
    },
    frontend_checks::{
        backend_javascript_source_check, is_plain_static_frontend, static_frontend_checks,
    },
    project::walk_project_files,
    project_kind::ProjectKind,
};
use serde::Serialize;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DoctorCheck {
    pub(crate) name: String,
    pub(crate) ok: bool,
    pub(crate) message: String,
    pub(crate) agent_instruction: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DoctorReport {
    pub(crate) ok: bool,
    pub(crate) project: String,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProjectDoctorReport {
    pub(crate) relative: String,
    pub(crate) ok: bool,
    pub(crate) project: String,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WorkspaceDoctorReport {
    pub(crate) ok: bool,
    pub(crate) workspace: String,
    pub(crate) projects: Vec<ProjectDoctorReport>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum DoctorReportKind {
    Project(Box<DoctorReport>),
    Workspace(WorkspaceDoctorReport),
}

impl DoctorReportKind {
    pub(crate) fn ok(&self) -> bool {
        match self {
            Self::Project(report) => report.ok,
            Self::Workspace(report) => report.ok,
        }
    }

    pub(crate) fn checks(&self) -> Vec<DoctorCheck> {
        match self {
            Self::Project(report) => report.checks.clone(),
            Self::Workspace(report) => report
                .projects
                .iter()
                .flat_map(|project| project.checks.clone())
                .collect(),
        }
    }
}

pub(crate) fn doctor_project(project_dir: &Path, json_output: bool) -> Result<()> {
    let report = run_doctor_workspace(project_dir);
    if json_output {
        let value =
            serde_json::to_value(&report).map_err(|error| internal_error(error.to_string()))?;
        print_json(&value)?;
        if report.ok() {
            return Ok(());
        }
        return Err(CliError::new(CliFailure {
            payload: AgentErrorPayload {
                code: "doctor_failed".to_owned(),
                message: "Tovuk doctor failed.".to_owned(),
                agent_instruction: report
                    .checks()
                    .iter()
                    .find(|check| !check.ok)
                    .and_then(|check| check.agent_instruction.clone()),
                docs_url: None,
                checkout_url: None,
            },
            json: true,
            exit_code: 1,
        }));
    }

    print_doctor_report(&report);
    if !report.ok() {
        let instruction = report
            .checks()
            .iter()
            .find(|check| !check.ok)
            .and_then(|check| check.agent_instruction.clone())
            .unwrap_or_else(|| "Fix the failed checks and retry `tovuk doctor`.".to_owned());
        return Err(agent_error(
            "doctor_failed",
            "Tovuk doctor failed.",
            instruction,
            false,
        ));
    }
    Ok(())
}

pub(crate) fn run_doctor_workspace(project_dir: &Path) -> DoctorReportKind {
    if project_dir.join("tovuk.toml").exists() {
        return DoctorReportKind::Project(Box::new(run_doctor(project_dir)));
    }
    let projects = discover_deploy_projects(project_dir).unwrap_or_default();
    if projects.is_empty() {
        return DoctorReportKind::Project(Box::new(run_doctor(project_dir)));
    }
    let reports = projects
        .iter()
        .map(|project| {
            let report = run_doctor(&project.dir);
            ProjectDoctorReport {
                relative: project.relative.clone(),
                ok: report.ok,
                project: report.project,
                config: report.config,
                checks: report.checks,
            }
        })
        .collect::<Vec<_>>();
    DoctorReportKind::Workspace(WorkspaceDoctorReport {
        ok: reports.iter().all(|report| report.ok),
        workspace: project_dir.display().to_string(),
        projects: reports,
    })
}

pub(crate) fn run_doctor(project_dir: &Path) -> DoctorReport {
    let config_result = read_config(project_dir);
    let mut checks = vec![config_result.check];
    let kind = config_result
        .config
        .as_ref()
        .map_or(ProjectKind::RustBackend, |config| config.kind);

    if kind.is_fullstack() {
        if let Some(config) = config_result.config.as_ref() {
            checks.extend(fullstack_checks(project_dir, config, config_result.valid));
        }
        return doctor_report(project_dir, config_result.config, checks);
    }

    checks.extend(required_file_checks(project_dir, kind));
    if kind.is_static_frontend() {
        checks.extend(static_frontend_checks(project_dir, config_result.valid));
        checks.push(unsafe_check(project_dir));
    } else {
        checks.push(backend_javascript_source_check(project_dir, ""));
        checks.extend(rust_doctor_checks(project_dir, config_result.valid));
    }
    doctor_report(project_dir, config_result.config, checks)
}

pub(crate) struct ConfigResult {
    pub(crate) check: DoctorCheck,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) valid: bool,
}

pub(crate) fn read_config(project_dir: &Path) -> ConfigResult {
    let config_path = project_dir.join("tovuk.toml");
    if !config_path.exists() {
        return ConfigResult {
            check: doctor_check(
                "tovuk.toml",
                false,
                "valid",
                "missing",
                "Create and commit tovuk.toml, then retry.",
            ),
            config: None,
            valid: false,
        };
    }
    let source = match fs::read_to_string(&config_path) {
        Ok(source) => source,
        Err(error) => {
            return ConfigResult {
                check: DoctorCheck {
                    name: "tovuk.toml".to_owned(),
                    ok: false,
                    message: error.to_string(),
                    agent_instruction: Some(format!("Fix tovuk.toml: {error}.")),
                },
                config: None,
                valid: false,
            };
        }
    };
    match parse_tovuk_toml(&source, project_dir).and_then(|config| {
        validate_config(&config)?;
        Ok(config)
    }) {
        Ok(config) => ConfigResult {
            check: doctor_check("tovuk.toml", true, "valid", "missing", ""),
            config: Some(config),
            valid: true,
        },
        Err(message) => ConfigResult {
            check: DoctorCheck {
                name: "tovuk.toml".to_owned(),
                ok: false,
                message: message.clone(),
                agent_instruction: Some(format!("Fix tovuk.toml: {message}.")),
            },
            config: None,
            valid: false,
        },
    }
}

pub(crate) fn doctor_report(
    project_dir: &Path,
    config: Option<TovukConfig>,
    checks: Vec<DoctorCheck>,
) -> DoctorReport {
    DoctorReport {
        ok: checks.iter().all(|check| check.ok),
        project: project_dir.display().to_string(),
        config,
        checks,
    }
}

pub(crate) fn print_doctor_report(report: &DoctorReportKind) {
    match report {
        DoctorReportKind::Project(report) => print_checks(&report.checks),
        DoctorReportKind::Workspace(report) => {
            for project in &report.projects {
                println!("project {}", project.relative);
                print_checks(&project.checks);
            }
        }
    }
}

pub(crate) fn print_checks(checks: &[DoctorCheck]) {
    for check in checks {
        println!(
            "{} {}{}",
            if check.ok { "ok" } else { "fail" },
            check.name,
            if check.message.is_empty() {
                String::new()
            } else {
                format!(" - {}", check.message)
            }
        );
    }
}

pub(crate) fn doctor_check(
    name: &str,
    ok: bool,
    success: &str,
    failure: &str,
    instruction: &str,
) -> DoctorCheck {
    DoctorCheck {
        name: name.to_owned(),
        ok,
        message: if ok { success } else { failure }.to_owned(),
        agent_instruction: if ok {
            None
        } else {
            Some(instruction.to_owned())
        },
    }
}

pub(crate) fn required_file_checks(project_dir: &Path, kind: ProjectKind) -> Vec<DoctorCheck> {
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

pub(crate) fn required_files(project_dir: &Path, kind: ProjectKind) -> Vec<&'static str> {
    if kind.is_static_frontend() {
        if is_plain_static_frontend(project_dir) {
            vec!["index.html"]
        } else {
            vec!["package.json"]
        }
    } else {
        vec!["Cargo.toml", "Cargo.lock"]
    }
}

pub(crate) fn fullstack_checks(
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
        &["Cargo.toml", "Cargo.lock"],
    ));
    checks.push(backend_javascript_source_check(&backend_dir, &backend_root));
    checks.extend(rust_doctor_checks(&backend_dir, config_valid));
    checks.extend(required_files_at(
        &frontend_dir,
        &frontend_root,
        if is_plain_static_frontend(&frontend_dir) {
            &["index.html"][..]
        } else {
            &["package.json"][..]
        },
    ));
    checks.extend(static_frontend_checks(&frontend_dir, config_valid));
    checks
}

pub(crate) fn required_files_at(
    project_dir: &Path,
    label: &str,
    files: &[&str],
) -> Vec<DoctorCheck> {
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

pub(crate) fn rust_doctor_checks(project_dir: &Path, config_valid: bool) -> Vec<DoctorCheck> {
    let mut checks = vec![cargo_lints(project_dir), unsafe_check(project_dir)];
    if config_valid {
        checks.push(cargo_command_check(
            project_dir,
            "cargo fmt",
            &["fmt", "--all", "--check"],
            "Install rustfmt with Rust, then run `cargo fmt --all --check` before deploying.",
            "Run `cargo fmt --all`, then redeploy.",
        ));
        checks.push(cargo_command_check(
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
        ));
        checks.push(cargo_command_check(
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
        ));
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
        checks.push(cargo_command_check(
            project_dir,
            "cargo clippy",
            &clippy_args,
            "Install Rust clippy, then run Tovuk strict Clippy checks before deploying.",
            "Run the strict Tovuk Clippy command from tovuk.toml, fix every warning, panic/unwrap issue, and resource lint, then redeploy.",
        ));
    }
    checks
}

pub(crate) fn cargo_command_check(
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

pub(crate) fn unsafe_check(project_dir: &Path) -> DoctorCheck {
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

pub(crate) fn scan_unsafe(project_dir: &Path) -> Vec<String> {
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

pub(crate) fn is_rust_source(relative: &str) -> bool {
    Path::new(relative)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

pub(crate) fn cargo_lints(project_dir: &Path) -> DoctorCheck {
    let cargo_toml = project_dir.join("Cargo.toml");
    let source = match fs::read_to_string(&cargo_toml) {
        Ok(source) => source,
        Err(error) => {
            return DoctorCheck {
                name: "cargo lints".to_owned(),
                ok: false,
                message: error.to_string(),
                agent_instruction: Some(
                    "Create Cargo.toml with strict Rust lints, then retry.".to_owned(),
                ),
            };
        }
    };
    let required_clippy_lints = RUST_STRICT_CLIPPY_DENY_LINTS
        .iter()
        .map(|lint| lint.trim_start_matches("clippy::"))
        .collect::<Vec<_>>();
    let ok = cargo_lint_level(&source, "rust", "unsafe_code") == "forbid"
        && cargo_lint_level(&source, "rust", "warnings") == "deny"
        && required_clippy_lints
            .iter()
            .all(|lint| cargo_lint_level(&source, "clippy", lint) == "deny");
    DoctorCheck {
        name: "cargo lints".to_owned(),
        ok,
        message: if ok {
            "strict".to_owned()
        } else {
            "missing strict Rust or Clippy resource lints".to_owned()
        },
        agent_instruction: if ok {
            None
        } else {
            Some("Add `[lints.rust]` with `unsafe_code = \"forbid\"` and `warnings = \"deny\"`, plus `[lints.clippy]` deny entries for all, pedantic, panic/unwrap bans, and resource lints, then retry.".to_owned())
        },
    }
}

pub(crate) fn cargo_lint_level(source: &str, lint_group: &str, lint_name: &str) -> String {
    let mut section = String::new();
    for raw_line in source.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if let Some(next_section) = toml_section(line) {
            section = next_section;
            continue;
        }
        if section != format!("lints.{lint_group}")
            && section != format!("workspace.lints.{lint_group}")
        {
            continue;
        }
        if let Some(level) = lint_assignment_level(line, lint_name) {
            return level;
        }
    }
    String::new()
}

pub(crate) fn toml_section(line: &str) -> Option<String> {
    line.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::to_owned)
}

pub(crate) fn lint_assignment_level(line: &str, lint_name: &str) -> Option<String> {
    let (key, value) = line.split_once('=')?;
    if key.trim() != lint_name {
        return None;
    }
    let value = value.trim();
    if let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.split('"').next())
    {
        return Some(value.to_owned());
    }
    value
        .split("level")
        .nth(1)
        .and_then(|value| value.split('"').nth(1))
        .map(str::to_owned)
}
