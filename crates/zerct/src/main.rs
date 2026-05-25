//! Zerct command-line interface.

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

const VERSION: &str = "0.1.0";
const DEFAULT_API_URL: &str = "https://api.zerct.com";
const ARCHIVE_LIMIT_BYTES: usize = 48 * 1024 * 1024;
const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

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
    let cli = Cli::parse(env::args().skip(1).collect())?;
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
    fn parse(raw: Vec<String>) -> Result<Self, AgentError> {
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
                    api_url = required_value(&raw, index, "--api")?;
                    index += 2;
                }
                "--app" => {
                    app = Some(required_value(&raw, index, "--app")?);
                    index += 2;
                }
                "--token" => {
                    token = Some(required_value(&raw, index, "--token")?);
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
    build_command: String,
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
        format!(
            "{{\"name\":\"{}\",\"build\":{{\"command\":\"{}\"}},\"run\":{{\"command\":\"{}\",\"port\":{},\"health\":\"{}\"}},\"resources\":{{\"memory\":\"{}\",\"cpu\":\"{}\",\"idle_timeout_minutes\":{}}}}}",
            escape_json(&self.name),
            escape_json(&self.build_command),
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
    for filename in ["Cargo.toml", "Cargo.lock", "zerct.toml"] {
        let ok = project_dir.join(filename).exists();
        checks.push(Check {
            name: filename.to_owned(),
            ok,
            message: if ok { "found" } else { "missing" }.to_owned(),
            agent_instruction: format!("Create and commit {filename}, then retry."),
        });
    }

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
            }
            None
        }
    };

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

    DoctorReport {
        project: project_dir.to_path_buf(),
        config,
        checks,
    }
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
    let mut build_command = "cargo build --release".to_owned();
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
            ("", "name") => name = value.to_owned(),
            ("build", "command") => build_command = value.to_owned(),
            ("run", "command") => run_command = value.to_owned(),
            ("run", "port") => port = parse_u16(value, "invalid_port")?,
            ("run", "health") => health = value.to_owned(),
            ("resources", "memory") => memory = value.to_owned(),
            ("resources", "cpu") => cpu = value.to_owned(),
            ("resources", "idle_timeout_minutes") => {
                idle_timeout_minutes = parse_u16(value, "invalid_idle_timeout")?;
            }
            _unknown => {}
        }
    }

    let config = Config {
        name,
        build_command,
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

fn login(cli: &Cli) -> Result<(), AgentError> {
    if let Some(token) = &cli.token {
        write_session_token(&current_dir_or_dot(), token)?;
        println!("saved Zerct session token to .zerct/session-token");
        return Ok(());
    }

    let response = api_request(cli, "POST", "/v1/login/device", None, None)?;
    println!("{response}");
    open_login_url(&response);
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
    let token = read_token(&project_dir, cli)?;
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
    let token = read_token(&current_dir_or_dot(), cli)?;
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
    let token = read_token(&current_dir_or_dot(), cli)?;
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
    let token = read_token(&current_dir_or_dot(), cli)?;
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
    let output = Command::new("tar")
        .args([
            "--exclude=.git",
            "--exclude=target",
            "--exclude=node_modules",
            "--exclude=.zerct",
            "--exclude=.env",
            "--exclude=.env.*",
            "-czf",
            "-",
            "-C",
        ])
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

fn read_token(project_dir: &Path, cli: &Cli) -> Result<String, AgentError> {
    if let Some(token) = &cli.token {
        return Ok(token.to_owned());
    }
    if let Ok(token) = env::var("ZERCT_TOKEN") {
        return Ok(token);
    }
    for path in [
        project_dir.join(".zerct/session-token"),
        home_dir().join(".zerct/session-token"),
    ] {
        if path.exists() {
            return fs::read_to_string(path).map(|value| value.trim().to_owned()).map_err(|error| {
                AgentError::new(
                    "login_required",
                    format!("Could not read Zerct token: {error}"),
                    "Run `zerct login`, set `ZERCT_TOKEN`, or run `zerct login --token <token>`.",
                )
            });
        }
    }
    Err(AgentError::new(
        "login_required",
        "Zerct login is required.",
        "Run `zerct login`, set `ZERCT_TOKEN`, or run `zerct login --token <token>`, then retry.",
    ))
}

fn write_session_token(project_dir: &Path, token: &str) -> Result<(), AgentError> {
    let dir = project_dir.join(".zerct");
    fs::create_dir_all(&dir).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not create .zerct directory: {error}"),
            "Check directory permissions and retry.",
        )
    })?;
    fs::write(dir.join("session-token"), format!("{}\n", token.trim())).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not write Zerct token: {error}"),
            "Check directory permissions and retry.",
        )
    })
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
            if matches!(name, ".git" | "target" | "node_modules" | ".zerct") {
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

fn open_login_url(response: &str) {
    if let Some(url_start) = response.find("https://") {
        let tail = &response[url_start..];
        let end = tail.find('"').unwrap_or(tail.len());
        let url = &tail[..end];
        let _status = if cfg!(target_os = "macos") {
            Command::new("open").arg(url).status()
        } else if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/c", "start", "", url]).status()
        } else {
            Command::new("xdg-open").arg(url).status()
        };
    }
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
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(current_dir_or_dot)
}
