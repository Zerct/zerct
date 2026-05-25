//! Zerct command-line interface.

use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant},
};

const VERSION: &str = "0.1.1";
const DEFAULT_API_URL: &str = "https://api.zerct.com";
const ARCHIVE_LIMIT_BYTES: usize = 48 * 1024 * 1024;
const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const SESSION_SERVICE: &str = "com.zerct.cli";
const SESSION_ACCOUNT: &str = "session-token";
const SESSION_LABEL: &str = "Zerct session";
const DEFAULT_LOGIN_EXPIRES_SECONDS: u64 = 600;
const DEFAULT_LOGIN_INTERVAL_SECONDS: u64 = 5;
const DEFAULT_RUST_CHECK_COMMAND: &str =
    "cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings";
const DEFAULT_FRONTEND_CHECK_COMMAND: &str = "npm run typecheck && npm run lint";
const ARCHIVE_EXCLUDES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".zerct",
    ".env",
    ".env.*",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".config/gcloud",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "id_rsa",
    "id_ed25519",
    "*.sqlite",
    "*.sqlite3",
    "*.db",
    "*.log",
    "._*",
    ".DS_Store",
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error.print();
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), AgentError> {
    let raw_args = env::args().skip(1).collect::<Vec<_>>();
    let cli = Cli::parse(&raw_args)?;
    match cli.command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" => {
            println!("{VERSION}");
            Ok(())
        }
        "init" => init_project(&cli.project_path()),
        "install" => {
            init_project(&cli.project_path())?;
            doctor_project(&cli.project_path(), cli.json)
        }
        "doctor" => doctor_project(&cli.project_path(), cli.json),
        "login" => login(&cli),
        "deploy" => deploy(&cli),
        "logs" => app_get(&cli, "logs"),
        "status" => app_get(&cli, "status"),
        "inspect" => app_get(&cli, "inspect"),
        "db" | "database" => app_get(&cli, "database"),
        "env" => env_command(&cli),
        "billing" => billing(&cli),
        _unknown => Err(AgentError::new(
            "unknown_command",
            "Unknown Zerct command.",
            "Run `zerct --help` and retry with a supported command.",
        )),
    }
}

fn print_help() {
    println!(
        "Zerct {VERSION}

Usage:
  zerct init [path]
  zerct install [path]
  zerct doctor [path] [--json]
  zerct login [--token <token>] [--api <url>]
  zerct deploy [path] [--database] [--api <url>] [--json]
  zerct logs --app <app_id> [--api <url>] [--json]
  zerct status --app <app_id> [--api <url>] [--json]
  zerct inspect --app <app_id> [--api <url>] [--json]
  zerct db --app <app_id> [--api <url>] [--json]
  zerct env set --app <app_id> KEY=value [--api <url>] [--json]
  zerct billing [--api <url>] [--json]"
    );
}

#[derive(Debug)]
struct Cli {
    command: String,
    args: Vec<String>,
    api_url: String,
    app: Option<String>,
    token: Option<String>,
    database: bool,
    json: bool,
}

impl Cli {
    fn parse(raw: &[String]) -> Result<Self, AgentError> {
        let mut args = Vec::new();
        let mut api_url = DEFAULT_API_URL.to_owned();
        let mut app = None;
        let mut token = None;
        let mut database = false;
        let mut json = false;
        let mut index = 0usize;

        while let Some(arg) = raw.get(index) {
            match arg.as_str() {
                "--api" => {
                    api_url = required_value(raw, index, "--api")?;
                    index += 2;
                }
                "--app" => {
                    app = Some(required_value(raw, index, "--app")?);
                    index += 2;
                }
                "--token" => {
                    token = Some(required_value(raw, index, "--token")?);
                    index += 2;
                }
                "--database" => {
                    database = true;
                    index += 1;
                }
                "--json" => {
                    json = true;
                    index += 1;
                }
                other => {
                    args.push(other.to_owned());
                    index += 1;
                }
            }
        }

        let command = args.first().cloned().unwrap_or_else(|| "help".to_owned());
        Ok(Self {
            command,
            args: args.into_iter().skip(1).collect(),
            api_url: api_url.trim_end_matches('/').to_owned(),
            app,
            token,
            database,
            json,
        })
    }

    fn project_path(&self) -> PathBuf {
        self.args
            .first()
            .map_or_else(current_dir_or_dot, |value| PathBuf::from(value).absolute())
    }
}

trait AbsolutePath {
    fn absolute(self) -> PathBuf;
}

impl AbsolutePath for PathBuf {
    fn absolute(self) -> PathBuf {
        if self.is_absolute() {
            self
        } else {
            current_dir_or_dot().join(self)
        }
    }
}

#[derive(Debug)]
struct AgentError {
    code: &'static str,
    message: String,
    agent_instruction: String,
    docs_url: Option<String>,
    checkout_url: Option<String>,
}

impl AgentError {
    fn new(
        code: &'static str,
        message: impl Into<String>,
        agent_instruction: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            agent_instruction: agent_instruction.into(),
            docs_url: None,
            checkout_url: None,
        }
    }

    fn print(&self) {
        eprintln!("code: {}", self.code);
        eprintln!("{}", self.message);
        eprintln!("agent_instruction: {}", self.agent_instruction);
        if let Some(docs_url) = &self.docs_url {
            eprintln!("docs: {docs_url}");
        }
        if let Some(checkout_url) = &self.checkout_url {
            eprintln!("checkout: {checkout_url}");
        }
    }
}

#[derive(Debug)]
struct Config {
    name: String,
    kind: String,
    check_command: String,
    build_command: String,
    build_output_dir: Option<String>,
    run_command: String,
    port: u16,
    health: String,
    memory: String,
    cpu: String,
    idle_timeout_minutes: u16,
}

fn required_value(raw: &[String], index: usize, name: &'static str) -> Result<String, AgentError> {
    let Some(value) = raw.get(index + 1) else {
        return Err(AgentError::new(
            "missing_argument",
            format!("{name} requires a value."),
            format!("Pass a value after {name}."),
        ));
    };
    if value.starts_with("--") {
        return Err(AgentError::new(
            "missing_argument",
            format!("{name} requires a value."),
            format!("Pass a value after {name}."),
        ));
    }
    Ok(value.to_owned())
}

fn init_project(project_dir: &Path) -> Result<(), AgentError> {
    if !project_dir.is_dir() {
        return Err(AgentError::new(
            "missing_project",
            "Project directory does not exist.",
            "Run Zerct from the root of a Rust project or pass the project path.",
        ));
    }

    let config_path = project_dir.join("zerct.toml");
    if config_path.exists() {
        println!("zerct.toml already exists");
        return Ok(());
    }

    let name = service_name_from_dir(project_dir);
    let source = format!(
        "name = \"{name}\"\n\n[build]\ncommand = \"cargo build --release\"\n\n[run]\ncommand = \"./target/release/{name}\"\nport = 3000\nhealth = \"/healthz\"\n\n[resources]\nmemory = \"512mb\"\ncpu = \"0.25\"\nidle_timeout_minutes = 15\n"
    );
    fs::write(&config_path, source).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not write zerct.toml: {error}"),
            "Check project directory permissions and retry `zerct init`.",
        )
    })?;
    println!("created {}", config_path.display());
    Ok(())
}

fn doctor_project(project_dir: &Path, json: bool) -> Result<(), AgentError> {
    let report = doctor_report(project_dir);
    if json {
        println!("{}", report.to_json());
    } else {
        for check in &report.checks {
            let status = if check.ok { "ok" } else { "fail" };
            println!("{status} {} - {}", check.name, check.message);
        }
    }

    if let Some(check) = report.checks.iter().find(|check| !check.ok) {
        Err(AgentError::new(
            "doctor_failed",
            "Zerct doctor failed.",
            check.agent_instruction.clone(),
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct DoctorReport {
    project: PathBuf,
    config: Option<Config>,
    checks: Vec<Check>,
}

impl DoctorReport {
    fn to_json(&self) -> String {
        let checks = self
            .checks
            .iter()
            .map(Check::to_json)
            .collect::<Vec<_>>()
            .join(",");
        let config = self
            .config
            .as_ref()
            .map_or_else(|| "null".to_owned(), Config::to_json);
        format!(
            "{{\"ok\":{},\"project\":\"{}\",\"config\":{},\"checks\":[{}]}}",
            self.checks.iter().all(|check| check.ok),
            escape_json(&self.project.display().to_string()),
            config,
            checks
        )
    }
}

#[derive(Debug)]
struct Check {
    name: String,
    ok: bool,
    message: String,
    agent_instruction: String,
}

impl Check {
    fn to_json(&self) -> String {
        format!(
            "{{\"name\":\"{}\",\"ok\":{},\"message\":\"{}\",\"agent_instruction\":\"{}\"}}",
            escape_json(&self.name),
            self.ok,
            escape_json(&self.message),
            escape_json(&self.agent_instruction)
        )
    }
}

impl Config {
    fn to_json(&self) -> String {
        let build = self.build_output_dir.as_ref().map_or_else(
            || {
                format!(
                    "{{\"check\":\"{}\",\"command\":\"{}\"}}",
                    escape_json(&self.check_command),
                    escape_json(&self.build_command)
                )
            },
            |output| {
                format!(
                    "{{\"check\":\"{}\",\"command\":\"{}\",\"output\":\"{}\"}}",
                    escape_json(&self.check_command),
                    escape_json(&self.build_command),
                    escape_json(output)
                )
            },
        );
        format!(
            "{{\"name\":\"{}\",\"kind\":\"{}\",\"build\":{},\"run\":{{\"command\":\"{}\",\"port\":{},\"health\":\"{}\"}},\"resources\":{{\"memory\":\"{}\",\"cpu\":\"{}\",\"idle_timeout_minutes\":{}}}}}",
            escape_json(&self.name),
            escape_json(&self.kind),
            build,
            escape_json(&self.run_command),
            self.port,
            escape_json(&self.health),
            escape_json(&self.memory),
            escape_json(&self.cpu),
            self.idle_timeout_minutes
        )
    }
}

fn doctor_report(project_dir: &Path) -> DoctorReport {
    let mut checks = Vec::new();
    let config = match parse_config(&project_dir.join("zerct.toml")) {
        Ok(config) => {
            checks.push(Check {
                name: "zerct.toml".to_owned(),
                ok: true,
                message: "valid".to_owned(),
                agent_instruction: String::new(),
            });
            Some(config)
        }
        Err(error) => {
            if project_dir.join("zerct.toml").exists() {
                checks.push(Check {
                    name: "zerct.toml".to_owned(),
                    ok: false,
                    message: error.message,
                    agent_instruction: error.agent_instruction,
                });
            } else {
                checks.push(Check {
                    name: "zerct.toml".to_owned(),
                    ok: false,
                    message: "missing".to_owned(),
                    agent_instruction: "Create and commit zerct.toml, then retry.".to_owned(),
                });
            }
            None
        }
    };

    let kind = config
        .as_ref()
        .map_or("rust_backend", |config| config.kind.as_str());
    let required_files: &[&str] = if kind == "static_frontend" {
        &["package.json"]
    } else {
        &["Cargo.toml", "Cargo.lock"]
    };
    for filename in required_files {
        let ok = project_dir.join(filename).exists();
        checks.push(Check {
            name: (*filename).to_owned(),
            ok,
            message: if ok { "found" } else { "missing" }.to_owned(),
            agent_instruction: format!("Create and commit {filename}, then retry."),
        });
    }
    if kind == "static_frontend" {
        let ok = frontend_lockfile_exists(project_dir);
        checks.push(Check {
            name: "frontend lockfile".to_owned(),
            ok,
            message: if ok { "found" } else { "missing" }.to_owned(),
            agent_instruction:
                "Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry."
                    .to_owned(),
        });
        checks.extend(frontend_source_checks(project_dir));
        checks.extend(frontend_script_checks(project_dir));
    }

    let unsafe_hits = scan_unsafe(project_dir);
    checks.push(Check {
        name: "unsafe".to_owned(),
        ok: unsafe_hits.is_empty(),
        message: if unsafe_hits.is_empty() {
            "no direct unsafe found".to_owned()
        } else {
            unsafe_hits
                .iter()
                .take(5)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        },
        agent_instruction:
            "Remove direct unsafe usage from workspace Rust source before deploying.".to_owned(),
    });
    if kind == "rust_backend" {
        checks.push(cargo_check(project_dir));
        checks.push(cargo_clippy(project_dir));
    }

    DoctorReport {
        project: project_dir.to_path_buf(),
        config,
        checks,
    }
}

fn cargo_check(project_dir: &Path) -> Check {
    let output = Command::new("cargo")
        .args(["check", "--locked", "--quiet"])
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(project_dir)
        .output();

    match output {
        Ok(output) => Check {
            name: "cargo check".to_owned(),
            ok: output.status.success(),
            message: if output.status.success() {
                "passed".to_owned()
            } else {
                truncate_check_message(&String::from_utf8_lossy(&output.stderr))
            },
            agent_instruction:
                "Run `cargo check --locked`, fix every compiler error and warning, then redeploy."
                    .to_owned(),
        },
        Err(error) => Check {
            name: "cargo check".to_owned(),
            ok: false,
            message: error.to_string(),
            agent_instruction:
                "Install Rust and Cargo, then run `cargo check --locked` locally before deploying."
                    .to_owned(),
        },
    }
}

fn cargo_clippy(project_dir: &Path) -> Check {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--locked",
            "--all-targets",
            "--all-features",
            "--quiet",
            "--",
            "-D",
            "warnings",
        ])
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(project_dir)
        .output();

    match output {
        Ok(output) => Check {
            name: "cargo clippy".to_owned(),
            ok: output.status.success(),
            message: if output.status.success() {
                "passed".to_owned()
            } else {
                truncate_check_message(&String::from_utf8_lossy(&output.stderr))
            },
            agent_instruction:
                "Run `cargo clippy --locked --all-targets --all-features -- -D warnings`, fix every warning, then redeploy."
                    .to_owned(),
        },
        Err(error) => Check {
            name: "cargo clippy".to_owned(),
            ok: false,
            message: error.to_string(),
            agent_instruction:
                "Install Rust clippy, then run `cargo clippy --locked --all-targets --all-features -- -D warnings` before deploying."
                    .to_owned(),
        },
    }
}

fn truncate_check_message(message: &str) -> String {
    message.chars().take(240).collect()
}

fn parse_config(path: &Path) -> Result<Config, AgentError> {
    let source = fs::read_to_string(path).map_err(|error| {
        AgentError::new(
            "missing_config",
            format!("Could not read zerct.toml: {error}"),
            "Create zerct.toml with `zerct init`, then retry.",
        )
    })?;
    let mut section = "";
    let mut name = String::new();
    let mut kind = "rust_backend".to_owned();
    let mut check_command = String::new();
    let mut build_command = String::new();
    let mut build_output_dir = None;
    let mut run_command = String::new();
    let mut port = 3_000u16;
    let mut health = "/healthz".to_owned();
    let mut memory = "512mb".to_owned();
    let mut cpu = "0.25".to_owned();
    let mut idle_timeout_minutes = 15u16;

    for line in source.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line {
            "[build]" => {
                section = "build";
                continue;
            }
            "[run]" => {
                section = "run";
                continue;
            }
            "[resources]" => {
                section = "resources";
                continue;
            }
            _other => {}
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(AgentError::new(
                "invalid_toml",
                format!("Invalid zerct.toml line: {line}"),
                "Fix zerct.toml so every setting uses `key = value`.",
            ));
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match (section, key) {
            ("", "name") => value.clone_into(&mut name),
            ("", "kind") => value.clone_into(&mut kind),
            ("build", "check") => value.clone_into(&mut check_command),
            ("build", "command") => value.clone_into(&mut build_command),
            ("build", "output") => build_output_dir = Some(value.to_owned()),
            ("run", "command") => value.clone_into(&mut run_command),
            ("run", "port") => port = parse_u16(value, "invalid_port")?,
            ("run", "health") => value.clone_into(&mut health),
            ("resources", "memory") => value.clone_into(&mut memory),
            ("resources", "cpu") => value.clone_into(&mut cpu),
            ("resources", "idle_timeout_minutes") => {
                idle_timeout_minutes = parse_u16(value, "invalid_idle_timeout")?;
            }
            _unknown => {}
        }
    }
    if build_command.is_empty() {
        build_command = if kind == "static_frontend" {
            "npm ci && npm run build".to_owned()
        } else {
            "cargo build --release".to_owned()
        };
    }
    if check_command.is_empty() {
        check_command = if kind == "static_frontend" {
            DEFAULT_FRONTEND_CHECK_COMMAND.to_owned()
        } else {
            DEFAULT_RUST_CHECK_COMMAND.to_owned()
        };
    }
    if kind == "static_frontend" && build_output_dir.is_none() {
        build_output_dir = Some("dist".to_owned());
    }

    let config = Config {
        name,
        kind,
        check_command,
        build_command,
        build_output_dir,
        run_command,
        port,
        health,
        memory,
        cpu,
        idle_timeout_minutes,
    };
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &Config) -> Result<(), AgentError> {
    if !valid_service_name(&config.name) {
        return Err(AgentError::new(
            "invalid_service_name",
            "Service name must be lowercase DNS-safe text.",
            "Set `name` in zerct.toml to lowercase letters, numbers, and hyphens only.",
        ));
    }
    if config.kind != "rust_backend" && config.kind != "static_frontend" {
        return Err(AgentError::new(
            "invalid_project_kind",
            "Project kind must be rust_backend or static_frontend.",
            "Set kind in zerct.toml to rust_backend or static_frontend.",
        ));
    }
    if config.build_command.trim().is_empty() {
        return Err(AgentError::new(
            "missing_command",
            "Build command is missing.",
            "Set [build].command in zerct.toml, then redeploy.",
        ));
    }
    if config.check_command.trim().is_empty() {
        return Err(AgentError::new(
            "missing_command",
            "Check command is missing.",
            "Set [build].check in zerct.toml to a command that typechecks and lints before the release build.",
        ));
    }
    validate_check_command(&config.kind, &config.check_command)?;
    if config.kind == "static_frontend" {
        let Some(output_dir) = &config.build_output_dir else {
            return Err(AgentError::new(
                "invalid_build_output",
                "Static frontend output must be a safe relative directory.",
                "Set [build].output to a relative directory like dist.",
            ));
        };
        if !valid_relative_path(output_dir) {
            return Err(AgentError::new(
                "invalid_build_output",
                "Static frontend output must be a safe relative directory.",
                "Set [build].output to a relative directory like dist.",
            ));
        }
        return Ok(());
    }
    if config.run_command.trim().is_empty() {
        return Err(AgentError::new(
            "missing_command",
            "A required command is missing.",
            "Set [run].command in zerct.toml to the release binary command.",
        ));
    }
    if !config.health.starts_with('/') {
        return Err(AgentError::new(
            "invalid_health_endpoint",
            "Health endpoint must be an absolute path.",
            "Set [run].health to a short absolute path such as `/healthz`.",
        ));
    }
    Ok(())
}

fn validate_check_command(kind: &str, command: &str) -> Result<(), AgentError> {
    let required = if kind == "static_frontend" {
        &["typecheck", "lint"][..]
    } else {
        &[
            "cargo check --locked",
            "cargo clippy --locked",
            "--all-targets",
            "--all-features",
            "-D warnings",
        ][..]
    };
    if required.iter().all(|fragment| command.contains(fragment)) {
        return Ok(());
    }
    Err(AgentError::new(
        "policy_rejected",
        "Check command is too weak for Zerct deploys.",
        if kind == "static_frontend" {
            "Set [build].check to run both frontend typechecking and linting, for example `npm run typecheck && npm run lint`, then redeploy."
        } else {
            "Set [build].check to include `cargo check --locked` and `cargo clippy --locked --all-targets --all-features -- -D warnings`, then redeploy."
        },
    ))
}

fn login(cli: &Cli) -> Result<(), AgentError> {
    if let Some(token) = &cli.token {
        write_session_token(token)?;
        println!("saved Zerct session token");
        return Ok(());
    }

    let _token = login_and_store(cli)?;
    Ok(())
}

fn deploy(cli: &Cli) -> Result<(), AgentError> {
    let project_dir = cli.project_path();
    let report = doctor_report(&project_dir);
    if let Some(check) = report.checks.iter().find(|check| !check.ok) {
        return Err(AgentError::new(
            "doctor_failed",
            "Zerct doctor failed.",
            check.agent_instruction.clone(),
        ));
    }
    let Some(config) = report.config else {
        return Err(AgentError::new(
            "missing_config",
            "zerct.toml is missing.",
            "Run `zerct init`, commit zerct.toml, then retry.",
        ));
    };
    if config.kind == "static_frontend" && cli.database {
        return Err(AgentError::new(
            "invalid_database_target",
            "Static frontends cannot attach managed Postgres directly.",
            "Deploy a Rust backend with managed Postgres and call it from the frontend.",
        ));
    }

    let body = format!(
        "{{\"config\":{},\"commit_sha\":{},\"wants_database\":{},\"source_archive_base64\":\"{}\"}}",
        config.to_json(),
        git_commit_sha(&project_dir).map_or_else(
            || "null".to_owned(),
            |sha| format!("\"{}\"", escape_json(&sha))
        ),
        cli.database,
        archive_base64(&project_dir)?
    );
    let token = read_or_login_token(&project_dir, cli)?;
    let response = api_request(cli, "POST", "/v1/deployments", Some(&token), Some(&body))?;
    println!("{response}");
    Ok(())
}

fn app_get(cli: &Cli, route: &str) -> Result<(), AgentError> {
    let app = cli.app.as_deref().ok_or_else(|| {
        AgentError::new(
            "missing_app",
            "App id is required.",
            "Pass `--app <app_id>`. Use the app id printed by `zerct deploy`.",
        )
    })?;
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    let response = api_request(
        cli,
        "GET",
        &format!("/v1/apps/{app}/{route}"),
        Some(&token),
        None,
    )?;
    println!("{response}");
    Ok(())
}

fn env_command(cli: &Cli) -> Result<(), AgentError> {
    if cli.args.first().map(String::as_str) != Some("set") {
        return Err(AgentError::new(
            "unknown_command",
            "Unknown env command.",
            "Use `zerct env set --app <app_id> KEY=value`.",
        ));
    }
    let assignment = cli.args.get(1).ok_or_else(|| {
        AgentError::new(
            "invalid_env",
            "Environment assignment must be KEY=value.",
            "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
        )
    })?;
    let Some((name, value)) = assignment.split_once('=') else {
        return Err(AgentError::new(
            "invalid_env",
            "Environment assignment must be KEY=value.",
            "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
        ));
    };
    let app = cli.app.as_deref().ok_or_else(|| {
        AgentError::new(
            "missing_app",
            "App id is required.",
            "Pass `--app <app_id>`.",
        )
    })?;
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    let body = format!(
        "{{\"name\":\"{}\",\"value\":\"{}\"}}",
        escape_json(name),
        escape_json(value)
    );
    let response = api_request(
        cli,
        "PUT",
        &format!("/v1/apps/{app}/env"),
        Some(&token),
        Some(&body),
    )?;
    println!("{response}");
    Ok(())
}

fn billing(cli: &Cli) -> Result<(), AgentError> {
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    let body = "{\"target_plan\":\"pro\",\"reason\":\"Upgrade to Zerct Pro.\"}";
    let response = api_request(
        cli,
        "POST",
        "/v1/billing/checkout",
        Some(&token),
        Some(body),
    )?;
    println!("{response}");
    Ok(())
}

fn api_request(
    cli: &Cli,
    method: &str,
    route: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<String, AgentError> {
    let mut command = Command::new("curl");
    command
        .arg("-sS")
        .arg("-X")
        .arg(method)
        .arg("-H")
        .arg("Accept: application/json")
        .arg("-w")
        .arg("\n%{http_code}")
        .arg(format!("{}{}", cli.api_url, route));
    if let Some(token) = token {
        command
            .arg("-H")
            .arg(format!("Authorization: Bearer {token}"));
    }
    if let Some(body) = body {
        command
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(body);
    }

    let output = command.output().map_err(|error| {
        AgentError::new(
            "api_unavailable",
            format!("Could not run curl: {error}"),
            "Install curl, then retry the Zerct command.",
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some((response, status)) = stdout.rsplit_once('\n') else {
        return Err(AgentError::new(
            "api_unavailable",
            "Zerct API response was invalid.",
            "Retry the command. If it keeps failing, check Zerct status.",
        ));
    };
    let status_code = status.trim().parse::<u16>().map_err(|error| {
        AgentError::new(
            "api_unavailable",
            format!("Zerct API status was invalid: {error}"),
            "Retry the command. If it keeps failing, check Zerct status.",
        )
    })?;
    if status_code >= 400 {
        return Err(AgentError::new(
            "api_error",
            response.to_owned(),
            "Read the Zerct API error payload, apply `agent_instruction`, then retry.",
        ));
    }
    Ok(response.to_owned())
}

fn archive_base64(project_dir: &Path) -> Result<String, AgentError> {
    let mut command = Command::new("tar");
    for pattern in ARCHIVE_EXCLUDES {
        command.arg(format!("--exclude={pattern}"));
    }
    let output = command
        .env("COPYFILE_DISABLE", "1")
        .args(["-czf", "-", "-C"])
        .arg(project_dir)
        .arg(".")
        .stdout(Stdio::piped())
        .output()
        .map_err(|error| {
            AgentError::new(
                "archive_failed",
                format!("Could not run tar: {error}"),
                "Install tar, remove local build outputs, then retry `zerct deploy`.",
            )
        })?;

    if !output.status.success() {
        return Err(AgentError::new(
            "archive_failed",
            String::from_utf8_lossy(&output.stderr),
            "Check project files and retry `zerct deploy`.",
        ));
    }
    if output.stdout.len() > ARCHIVE_LIMIT_BYTES {
        return Err(AgentError::new(
            "archive_too_large",
            "Source archive is too large.",
            "Remove build outputs, target directories, logs, and local caches before deploying.",
        ));
    }
    Ok(base64_encode(&output.stdout))
}

fn base64_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        let n = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        encoded.push(BASE64_TABLE[((n >> 18) & 63) as usize] as char);
        encoded.push(BASE64_TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(BASE64_TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(BASE64_TABLE[(n & 63) as usize] as char);
        } else {
            encoded.push('=');
        }
    }
    encoded
}

fn read_or_login_token(project_dir: &Path, cli: &Cli) -> Result<String, AgentError> {
    if let Some(token) = read_stored_token(project_dir, cli)? {
        return Ok(token);
    }

    login_and_store(cli)
}

fn login_and_store(cli: &Cli) -> Result<String, AgentError> {
    let start = api_request(cli, "POST", "/v1/login/device", None, None)?;
    let login_url = json_string_field(&start, "loginUrl")
        .or_else(|| json_string_field(&start, "login_url"))
        .ok_or_else(|| {
            AgentError::new(
                "login_failed",
                "Zerct login did not return a browser URL.",
                "Retry `zerct login`. If it keeps failing, check Zerct status.",
            )
        })?;
    open_url(&login_url);
    progress(cli, "opened browser login");
    let user_code = json_string_field(&start, "userCode")
        .or_else(|| json_string_field(&start, "user_code"))
        .unwrap_or_else(|| "ZERCT".to_owned());
    progress(cli, &format!("waiting for browser login code {user_code}"));

    let session = poll_login(cli, &start)?;
    let token = json_string_field(&session, "token").ok_or_else(|| {
        AgentError::new(
            "login_failed",
            "Zerct login did not return a session token.",
            "Run `zerct login` again and complete the browser login.",
        )
    })?;
    write_session_token(&token)?;
    let email = json_string_field(&session, "email").unwrap_or_else(|| "Zerct user".to_owned());
    progress(cli, &format!("logged in as {email}"));

    Ok(token)
}

fn poll_login(cli: &Cli, start: &str) -> Result<String, AgentError> {
    let device_code = json_string_field(start, "deviceCode")
        .or_else(|| json_string_field(start, "device_code"))
        .ok_or_else(|| {
            AgentError::new(
                "login_failed",
                "Zerct login did not return a device code.",
                "Retry `zerct login`. If it keeps failing, check Zerct status.",
            )
        })?;
    let expires = json_u64_field(start, "expiresInSeconds")
        .or_else(|| json_u64_field(start, "expires_in_seconds"))
        .unwrap_or(DEFAULT_LOGIN_EXPIRES_SECONDS);
    let mut interval = json_u64_field(start, "intervalSeconds")
        .or_else(|| json_u64_field(start, "interval_seconds"))
        .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS);
    let deadline = Instant::now() + Duration::from_secs(expires);

    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(
            interval.max(DEFAULT_LOGIN_INTERVAL_SECONDS),
        ));
        let response = api_request(
            cli,
            "GET",
            &format!("/v1/login/device/{device_code}"),
            None,
            None,
        )?;
        match json_string_field(&response, "status").as_deref() {
            Some("complete") => return Ok(response),
            Some("expired") => {
                return Err(login_expired_error());
            }
            _pending => {
                interval = json_u64_field(&response, "intervalSeconds")
                    .or_else(|| json_u64_field(&response, "interval_seconds"))
                    .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS);
            }
        }
    }

    Err(login_expired_error())
}

fn login_expired_error() -> AgentError {
    AgentError::new(
        "login_expired",
        "Zerct login expired before it completed.",
        "Run `zerct login` again and finish the browser login in the newly opened tab.",
    )
}

fn read_stored_token(project_dir: &Path, cli: &Cli) -> Result<Option<String>, AgentError> {
    if let Some(token) = &cli.token {
        return Ok(Some(token.to_owned()));
    }
    if let Ok(token) = env::var("ZERCT_TOKEN") {
        return Ok(Some(token));
    }
    if let Some(token) = read_keychain_token()? {
        return Ok(Some(token));
    }
    for path in [
        user_session_path(),
        project_dir.join(".zerct/session-token"),
        home_dir().join(".zerct/session-token"),
    ] {
        if path.exists() {
            return fs::read_to_string(path).map(|value| Some(value.trim().to_owned())).map_err(|error| {
                AgentError::new(
                    "login_required",
                    format!("Could not read Zerct token: {error}"),
                    "Run `zerct login`, set `ZERCT_TOKEN`, or run `zerct login --token <token>`.",
                )
            });
        }
    }
    Ok(None)
}

fn write_session_token(token: &str) -> Result<(), AgentError> {
    let clean_token = token.trim();
    if clean_token.is_empty() {
        return Err(AgentError::new(
            "login_failed",
            "Zerct session token is empty.",
            "Run `zerct login` again and complete the browser login.",
        ));
    }
    if write_keychain_token(clean_token)? {
        return Ok(());
    }

    write_session_token_file(&user_session_path(), clean_token)
}

fn write_session_token_file(token_path: &Path, token: &str) -> Result<(), AgentError> {
    let dir = token_path.parent().ok_or_else(|| {
        AgentError::new(
            "write_failed",
            "Could not determine Zerct token directory.",
            "Check home directory permissions and retry.",
        )
    })?;
    fs::create_dir_all(dir).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not create Zerct token directory: {error}"),
            "Check directory permissions and retry.",
        )
    })?;
    set_private_dir_permissions(dir)?;
    fs::write(token_path, format!("{token}\n")).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not write Zerct token: {error}"),
            "Check directory permissions and retry.",
        )
    })?;
    set_private_file_permissions(token_path)
}

fn read_keychain_token() -> Result<Option<String>, AgentError> {
    if cfg!(target_os = "macos") {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                SESSION_SERVICE,
                "-a",
                SESSION_ACCOUNT,
                "-w",
            ])
            .output()
            .map_err(|error| keychain_command_error(&error))?;
        return Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|value| !value.is_empty()));
    }

    if cfg!(target_os = "linux") && command_exists("secret-tool") {
        let output = Command::new("secret-tool")
            .args([
                "lookup",
                "service",
                SESSION_SERVICE,
                "account",
                SESSION_ACCOUNT,
            ])
            .output()
            .map_err(|error| keychain_command_error(&error))?;
        return Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|value| !value.is_empty()));
    }

    Ok(None)
}

fn write_keychain_token(token: &str) -> Result<bool, AgentError> {
    if cfg!(target_os = "macos") {
        let status = Command::new("security")
            .args([
                "add-generic-password",
                "-U",
                "-s",
                SESSION_SERVICE,
                "-a",
                SESSION_ACCOUNT,
                "-l",
                SESSION_LABEL,
                "-w",
                token,
            ])
            .status()
            .map_err(|error| keychain_command_error(&error))?;
        return Ok(status.success());
    }

    if cfg!(target_os = "linux") && command_exists("secret-tool") {
        let mut child = Command::new("secret-tool")
            .args([
                "store",
                "--label",
                SESSION_LABEL,
                "service",
                SESSION_SERVICE,
                "account",
                SESSION_ACCOUNT,
            ])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|error| keychain_command_error(&error))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(token.as_bytes())
                .map_err(|error| keychain_write_error(&error))?;
        }
        let status = child
            .wait()
            .map_err(|error| keychain_command_error(&error))?;
        return Ok(status.success());
    }

    Ok(false)
}

fn keychain_command_error(error: &std::io::Error) -> AgentError {
    AgentError::new(
        "credential_store_failed",
        format!("Could not access the Zerct credential store: {error}"),
        "Check OS credential-store access, or set `ZERCT_TOKEN` for this command.",
    )
}

fn keychain_write_error(error: &std::io::Error) -> AgentError {
    AgentError::new(
        "credential_store_failed",
        format!("Could not write the Zerct credential: {error}"),
        "Check OS credential-store access, or set `ZERCT_TOKEN` for this command.",
    )
}

fn command_exists(command: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|directory| directory.join(command).is_file())
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<(), AgentError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| permission_error(&error))?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).map_err(|error| permission_error(&error))
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> Result<(), AgentError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), AgentError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| permission_error(&error))?
        .permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|error| permission_error(&error))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), AgentError> {
    Ok(())
}

fn permission_error(error: &std::io::Error) -> AgentError {
    AgentError::new(
        "write_failed",
        format!("Could not restrict Zerct token permissions: {error}"),
        "Check directory permissions and retry.",
    )
}

fn scan_unsafe(project_dir: &Path) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    let mut stack = vec![project_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if should_skip_dir(name) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
                && file_contains_unsafe(&path)
            {
                hits.push(
                    path.strip_prefix(project_dir)
                        .map_or(path.clone(), Path::to_path_buf),
                );
            }
        }
    }
    hits
}

fn should_skip_dir(name: &str) -> bool {
    matches!(name, ".git" | "target" | "node_modules" | ".zerct")
}

fn file_contains_unsafe(path: &Path) -> bool {
    let mut source = String::new();
    fs::File::open(path)
        .and_then(|mut file| file.read_to_string(&mut source))
        .is_ok()
        && source
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|word| word == "unsafe")
}

fn valid_service_name(value: &str) -> bool {
    let len = value.len();
    len > 0
        && len <= 48
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        && value
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

fn frontend_lockfile_exists(project_dir: &Path) -> bool {
    [
        "package-lock.json",
        "npm-shrinkwrap.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        "bun.lockb",
    ]
    .iter()
    .any(|filename| project_dir.join(filename).exists())
}

fn frontend_source_checks(project_dir: &Path) -> Vec<Check> {
    let report = frontend_source_report(project_dir);
    vec![
        Check {
            name: "typescript source".to_owned(),
            ok: !report.typescript.is_empty(),
            message: if report.typescript.is_empty() {
                "missing".to_owned()
            } else {
                display_paths(&report.typescript, 3)
            },
            agent_instruction:
                "Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry."
                    .to_owned(),
        },
        Check {
            name: "javascript source".to_owned(),
            ok: report.javascript.is_empty(),
            message: if report.javascript.is_empty() {
                "none found".to_owned()
            } else {
                display_paths(&report.javascript, 5)
            },
            agent_instruction:
                "Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying."
                    .to_owned(),
        },
    ]
}

#[derive(Default)]
struct FrontendSourceReport {
    typescript: Vec<PathBuf>,
    javascript: Vec<PathBuf>,
}

fn frontend_source_report(project_dir: &Path) -> FrontendSourceReport {
    let mut report = FrontendSourceReport::default();
    let mut stack = vec![project_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if should_skip_dir(name) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(project_dir)
                .map_or(path.as_path(), |path| path);
            if !is_frontend_source_path(relative) {
                continue;
            }
            if is_frontend_typescript_source(relative) {
                report.typescript.push(relative.to_path_buf());
            } else if is_frontend_javascript_source(relative) {
                report.javascript.push(relative.to_path_buf());
            }
        }
    }
    report
}

fn is_frontend_source_path(relative: &Path) -> bool {
    match relative.components().next() {
        Some(std::path::Component::Normal(root)) => {
            matches!(
                root.to_str(),
                Some("src" | "app" | "pages" | "routes" | "components")
            )
        }
        _ => false,
    }
}

fn is_frontend_typescript_source(relative: &Path) -> bool {
    !display_path(relative).ends_with(".d.ts") && path_has_extension(relative, &["ts", "tsx"])
}

fn is_frontend_javascript_source(relative: &Path) -> bool {
    path_has_extension(relative, &["js", "jsx", "mjs", "cjs"])
}

fn path_has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| allowed.contains(&extension))
}

fn display_paths(paths: &[PathBuf], limit: usize) -> String {
    paths
        .iter()
        .take(limit)
        .map(|path| display_path(path))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn frontend_script_checks(project_dir: &Path) -> Vec<Check> {
    let manifest = fs::read_to_string(project_dir.join("package.json")).unwrap_or_default();
    let mut checks = ["typecheck", "lint"]
        .into_iter()
        .map(|script| {
            let ok = package_script_exists(&manifest, script);
            Check {
                name: format!("npm script {script}"),
                ok,
                message: if ok { "found" } else { "missing" }.to_owned(),
                agent_instruction: format!(
                    "Add a non-empty \"{script}\" script to package.json, then retry."
                ),
            }
        })
        .collect::<Vec<_>>();

    if checks.iter().all(|check| check.ok) {
        checks.push(npm_script_check(project_dir, "typecheck"));
        checks.push(npm_script_check(project_dir, "lint"));
    }

    checks
}

fn package_script_exists(manifest: &str, script: &str) -> bool {
    let needle = format!("\"{script}\"");
    manifest.contains("\"scripts\"") && manifest.contains(&needle)
}

fn npm_script_check(project_dir: &Path, script: &str) -> Check {
    let output = Command::new("npm")
        .args(["run", "--silent", script])
        .current_dir(project_dir)
        .output();

    match output {
        Ok(output) => Check {
            name: format!("npm run {script}"),
            ok: output.status.success(),
            message: if output.status.success() {
                "passed".to_owned()
            } else {
                truncate_check_message(&String::from_utf8_lossy(&output.stderr))
            },
            agent_instruction: format!("Run `npm run {script}`, fix every error, then redeploy."),
        },
        Err(error) => Check {
            name: format!("npm run {script}"),
            ok: false,
            message: error.to_string(),
            agent_instruction: format!(
                "Install Node.js and npm, then run `npm run {script}` before deploying."
            ),
        },
    }
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !value.contains('\\')
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn parse_u16(value: &str, code: &'static str) -> Result<u16, AgentError> {
    value.parse::<u16>().map_err(|error| {
        AgentError::new(
            code,
            format!("Invalid numeric value: {error}"),
            "Use a positive integer in zerct.toml.",
        )
    })
}

fn service_name_from_dir(project_dir: &Path) -> String {
    let raw = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("api")
        .to_ascii_lowercase();
    let mut output = String::new();
    let mut previous_dash = false;
    for character in raw.chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            output.push(character);
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    let trimmed = output.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "api".to_owned()
    } else {
        trimmed
    }
}

fn git_commit_sha(project_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        None
    }
}

fn open_url(url: &str) {
    let _status = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/c", "start", "", url]).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    };
}

fn progress(cli: &Cli, message: &str) {
    if cli.json {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}

fn json_string_field(source: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let start = source.find(&needle)? + needle.len();
    let tail = source.get(start..)?.trim_start();
    let raw = tail.strip_prefix('"')?;
    let mut value = String::new();
    let mut escaped = false;
    for character in raw.chars() {
        if escaped {
            value.push(match character {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(value);
        } else {
            value.push(character);
        }
    }

    None
}

fn json_u64_field(source: &str, field: &str) -> Option<u64> {
    let needle = format!("\"{field}\":");
    let start = source.find(&needle)? + needle.len();
    let digits: String = source
        .get(start..)?
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn escape_json(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output
}

fn current_dir_or_dot() -> PathBuf {
    env::current_dir().unwrap_or_else(|_error| PathBuf::from("."))
}

fn home_dir() -> PathBuf {
    env::var_os("HOME").map_or_else(current_dir_or_dot, PathBuf::from)
}

fn user_session_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Some(app_data) = env::var_os("APPDATA") {
            return PathBuf::from(app_data).join("Zerct").join("session-token");
        }
    }
    env::var_os("XDG_CONFIG_HOME").map_or_else(
        || home_dir().join(".config/zerct/session-token"),
        |config_home| PathBuf::from(config_home).join("zerct/session-token"),
    )
}
