//! Zerct command-line interface.

use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant},
};

const VERSION: &str = "0.1.14";
const DEFAULT_API_URL: &str = "https://api.zerct.com";
const ARCHIVE_LIMIT_BYTES: usize = 48 * 1024 * 1024;
const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const SESSION_SERVICE: &str = "com.zerct.cli";
const SESSION_ACCOUNT: &str = "session-token";
const SESSION_LABEL: &str = "Zerct session";
const DEFAULT_LOGIN_EXPIRES_SECONDS: u64 = 600;
const DEFAULT_LOGIN_INTERVAL_SECONDS: u64 = 5;
const DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS: u64 = 900;
const DEFAULT_RUST_CHECK_COMMAND: &str = "cargo fmt --all --check && cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings";
const DEFAULT_NPM_FRONTEND_CHECK_COMMAND: &str =
    "npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint";
const DEFAULT_BUN_FRONTEND_CHECK_COMMAND: &str = "bun ci && bun run typecheck && bun run lint";
const JAVASCRIPT_LINTERS: &[&str] = &[
    "eslint",
    "eslint_d",
    "jscs",
    "jshint",
    "prettier",
    "prettierd",
    "standard",
    "xo",
];
const FRONTEND_SOURCE_ROOTS: &[&str] = &["src", "app", "pages", "routes", "components"];
const FRONTEND_TYPESCRIPT_EXTENSIONS: &[&str] = &["ts", "tsx"];
const FRONTEND_JAVASCRIPT_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "cjs"];
const FRONTEND_PACKAGE_MANAGERS: &[&str] = &["npm", "bun", "pnpm", "yarn"];
const FRONTEND_INSTALL_COMMANDS: &[(&str, &str)] = &[
    ("npm", "ci"),
    ("bun", "ci"),
    ("bun", "install"),
    ("pnpm", "install"),
    ("yarn", "install"),
];
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
        "init" => init_project(&cli.project_path(), cli.template.as_deref()),
        "install" => {
            init_project(&cli.project_path(), cli.template.as_deref())?;
            doctor_project(&cli.project_path(), cli.json)
        }
        "doctor" => doctor_project(&cli.project_path(), cli.json),
        "preview" => preview_project(&cli),
        "login" => login(&cli),
        "deploy" => deploy(&cli),
        "capabilities" => public_get(&cli, "/v1/capabilities"),
        "me" => authenticated_get(&cli, "/v1/me"),
        "usage" => authenticated_get(&cli, "/v1/usage"),
        "activity" => authenticated_get(&cli, &format!("/v1/activity{}", page_query(&cli))),
        "apps" => authenticated_get(&cli, "/v1/apps"),
        "overview" => authenticated_get(
            &cli,
            &format!(
                "/v1/apps/{}/overview{}",
                url_encode(require_app(&cli)?),
                page_query(&cli)
            ),
        ),
        "deploys" => deploys(&cli),
        "builds" => builds(&cli),
        "logs" => logs(&cli),
        "status" => app_get(&cli, "status"),
        "inspect" => app_get(&cli, "inspect"),
        "db" | "database" => app_get(&cli, "database"),
        "env" => env_command(&cli),
        "domains" => domains_command(&cli),
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
  zerct init [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  zerct install [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  zerct doctor [path] [--json]
  zerct preview [path] [--port <port>]
  zerct login [--token <token>] [--api <url>]
  zerct deploy [path] [--database] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
  zerct capabilities [--api <url>] [--json]
  zerct me [--api <url>] [--json]
  zerct usage [--api <url>] [--json]
  zerct activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct apps [--api <url>] [--json]
  zerct overview --app <app> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct deploys [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct builds [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct logs --app <app> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct status --app <app> [--api <url>] [--json]
  zerct inspect --app <app> [--api <url>] [--json]
  zerct db --app <app> [--api <url>] [--json]
  zerct env list --app <app> [--api <url>] [--json]
  zerct env set --app <app> KEY=value [--api <url>] [--json]
  zerct env delete --app <app> KEY [--api <url>] [--json]
  zerct domains list --app <app> [--api <url>] [--json]
  zerct domains add --app <app> <domain> [--api <url>] [--json]
  zerct domains verify --app <app> <domain> [--api <url>] [--json]
  zerct domains delete --app <app> <domain> [--api <url>] [--json]
  zerct billing [portal] [--api <url>] [--json]"
    );
    println!(
        "
Agent contract:
  - Rust backends keep Cargo.lock committed, pass rustfmt, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = \"static_frontend\", keep TypeScript source, a package lockfile, and typecheck + lint scripts.
  - Frontends call Rust backends for APIs, managed Postgres, and server-side logic.
  - Run deploy from a repo root with nested zerct.toml files to deploy the whole workspace in one command.
  - When a frontend calls a backend on another hostname, configure backend CORS or use a same-origin custom domain.
  - Keep direct unsafe out of Rust source."
    );
}

#[derive(Debug)]
struct Cli {
    command: String,
    args: Vec<String>,
    api_url: String,
    app: Option<String>,
    build: Option<String>,
    deploy_id: Option<String>,
    limit: Option<String>,
    cursor: Option<String>,
    token: Option<String>,
    template: Option<String>,
    port: Option<u16>,
    database: bool,
    wait: bool,
    wait_timeout_seconds: u64,
    json: bool,
}

impl Cli {
    fn parse(raw: &[String]) -> Result<Self, AgentError> {
        let mut args = Vec::new();
        let mut api_url = DEFAULT_API_URL.to_owned();
        let mut app = None;
        let mut build = None;
        let mut deploy_id = None;
        let mut limit = None;
        let mut cursor = None;
        let mut token = None;
        let mut template = None;
        let mut port = None;
        let mut database = false;
        let mut wait = false;
        let mut wait_timeout_seconds = DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS;
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
                "--build" => {
                    build = Some(required_value(raw, index, "--build")?);
                    index += 2;
                }
                "--deploy" => {
                    deploy_id = Some(required_value(raw, index, "--deploy")?);
                    index += 2;
                }
                "--limit" => {
                    limit = Some(required_value(raw, index, "--limit")?);
                    index += 2;
                }
                "--cursor" => {
                    cursor = Some(required_value(raw, index, "--cursor")?);
                    index += 2;
                }
                "--token" => {
                    token = Some(required_value(raw, index, "--token")?);
                    index += 2;
                }
                "--template" => {
                    template = Some(required_value(raw, index, "--template")?);
                    index += 2;
                }
                "--port" => {
                    port = Some(parse_u16(
                        &required_value(raw, index, "--port")?,
                        "invalid_port",
                    )?);
                    index += 2;
                }
                "--database" => {
                    database = true;
                    index += 1;
                }
                "--wait" => {
                    wait = true;
                    index += 1;
                }
                "--wait-timeout" => {
                    wait_timeout_seconds = parse_u64(
                        &required_value(raw, index, "--wait-timeout")?,
                        "invalid_wait_timeout",
                    )?;
                    index += 2;
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
            build,
            deploy_id,
            limit,
            cursor,
            token,
            template,
            port,
            database,
            wait,
            wait_timeout_seconds,
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
    already_reported: bool,
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
            already_reported: false,
        }
    }

    fn already_reported(
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
            already_reported: true,
        }
    }

    fn print(&self) {
        if self.already_reported {
            return;
        }
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

#[derive(Debug)]
struct DeployProject {
    dir: PathBuf,
    relative: String,
    kind: String,
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

fn init_project(project_dir: &Path, template: Option<&str>) -> Result<(), AgentError> {
    if let Some(template) = template.filter(|value| !value.is_empty()) {
        fs::create_dir_all(project_dir).map_err(|error| {
            AgentError::new(
                "write_failed",
                format!("Could not create project directory: {error}"),
                "Check the path and retry `zerct init --template`.",
            )
        })?;
        create_template(project_dir, template)?;
        return Ok(());
    }

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

    let kind = infer_project_kind(project_dir);
    let source = if kind == "static_frontend" {
        frontend_config(project_dir)
    } else {
        rust_backend_config(project_dir)
    };
    fs::write(&config_path, source).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not write zerct.toml: {error}"),
            "Check project directory permissions and retry `zerct init`.",
        )
    })?;
    println!("created {}", config_path.display());
    println!("detected {kind}");
    Ok(())
}

fn create_template(project_dir: &Path, template: &str) -> Result<(), AgentError> {
    match template {
        "rust-api" => write_rust_template(project_dir, &service_name_from_dir(project_dir))?,
        "tanstack-static-frontend" => {
            write_frontend_template(project_dir, &service_name_from_dir(project_dir), "/api")?;
        }
        "fullstack-rust-tanstack" => {
            write_rust_template(&project_dir.join("api"), "api")?;
            write_frontend_template(&project_dir.join("web"), "web", "http://localhost:3000")?;
        }
        _other => {
            return Err(AgentError::new(
                "invalid_template",
                "Zerct template is unknown.",
                "Use rust-api, tanstack-static-frontend, or fullstack-rust-tanstack.",
            ));
        }
    }
    println!("created {template} template");
    Ok(())
}

fn write_rust_template(project_dir: &Path, name: &str) -> Result<(), AgentError> {
    fs::create_dir_all(project_dir.join("src")).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not create Rust template directory: {error}"),
            "Check the path and retry `zerct init --template rust-api`.",
        )
    })?;
    write_new_file(
        &project_dir.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lints.rust]\nunsafe_code = \"forbid\"\nwarnings = \"deny\"\n"
        ),
    )?;
    write_new_file(
        &project_dir.join("Cargo.lock"),
        &format!(
            "# This file is automatically @generated by Cargo.\nversion = 4\n\n[[package]]\nname = \"{name}\"\nversion = \"0.1.0\"\n"
        ),
    )?;
    write_new_file(&project_dir.join("src/main.rs"), rust_api_source())?;
    write_new_file(
        &project_dir.join("zerct.toml"),
        &rust_backend_config(project_dir),
    )
}

fn write_frontend_template(
    project_dir: &Path,
    name: &str,
    api_base_url: &str,
) -> Result<(), AgentError> {
    fs::create_dir_all(project_dir.join("src")).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not create frontend template directory: {error}"),
            "Check the path and retry `zerct init --template tanstack-static-frontend`.",
        )
    })?;
    let package_json = format!(
        r#"{{
  "name": "{name}",
  "private": true,
  "type": "module",
  "scripts": {{
    "typecheck": "tsgo --noEmit",
    "lint": "oxlint src vite.config.ts --deny-warnings",
    "build": "vite build",
    "preview": "vite preview --host 0.0.0.0"
  }},
  "dependencies": {{
    "react": "^19.2.1",
    "react-dom": "^19.2.1",
    "@tanstack/react-router": "^1.140.0"
  }},
  "devDependencies": {{
    "@types/react": "^19.2.7",
    "@types/react-dom": "^19.2.3",
    "@typescript/native-preview": "^7.0.0-dev.20260526.1",
    "@vitejs/plugin-react": "^5.1.1",
    "oxlint": "^1.30.0",
    "typescript": "^5.9.3",
    "vite": "^7.2.4"
  }}
}}
"#
    );
    write_new_file(&project_dir.join("package.json"), &package_json)?;
    write_new_file(
        &project_dir.join("index.html"),
        "<div id=\"root\"></div><script type=\"module\" src=\"/src/main.tsx\"></script>\n",
    )?;
    write_new_file(
        &project_dir.join("src/styles.css"),
        "body{margin:0;font-family:system-ui,sans-serif}main{min-height:100svh;display:grid;place-items:center;padding:2rem}code{font-family:ui-monospace,monospace}\n",
    )?;
    write_new_file(
        &project_dir.join("src/vite-env.d.ts"),
        "/// <reference types=\"vite/client\" />\n",
    )?;
    write_new_file(
        &project_dir.join("src/main.tsx"),
        &frontend_source(api_base_url),
    )?;
    write_new_file(
        &project_dir.join("tsconfig.json"),
        "{\"compilerOptions\":{\"strict\":true,\"jsx\":\"react-jsx\",\"module\":\"ESNext\",\"moduleResolution\":\"Bundler\",\"target\":\"ES2022\",\"noEmit\":true,\"skipLibCheck\":true},\"include\":[\"src\",\"vite.config.ts\"]}\n",
    )?;
    write_new_file(
        &project_dir.join("vite.config.ts"),
        "import react from \"@vitejs/plugin-react\";\nimport { defineConfig } from \"vite\";\n\nexport default defineConfig({ plugins: [react()] });\n",
    )?;
    write_new_file(
        &project_dir.join("zerct.toml"),
        &frontend_config(project_dir),
    )?;
    println!(
        "run package install in the frontend directory before doctor: bun install or npm install"
    );
    Ok(())
}

fn write_new_file(path: &Path, source: &str) -> Result<(), AgentError> {
    if path.exists() {
        return Err(AgentError::new(
            "file_exists",
            format!("Refusing to overwrite {}.", path.display()),
            "Move the existing file or choose an empty directory, then retry.",
        ));
    }
    fs::write(path, source).map_err(|error| {
        AgentError::new(
            "write_failed",
            format!("Could not write {}: {error}", path.display()),
            "Check directory permissions and retry.",
        )
    })
}

fn doctor_project(project_dir: &Path, json: bool) -> Result<(), AgentError> {
    if !project_dir.join("zerct.toml").exists() {
        let projects = discover_deploy_projects(project_dir)?;
        if !projects.is_empty() {
            return doctor_workspace(project_dir, &projects, json);
        }
    }

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
        if json {
            Err(AgentError::already_reported(
                "doctor_failed",
                "Zerct doctor failed.",
                check.agent_instruction.clone(),
            ))
        } else {
            Err(AgentError::new(
                "doctor_failed",
                "Zerct doctor failed.",
                check.agent_instruction.clone(),
            ))
        }
    } else {
        Ok(())
    }
}

fn preview_project(cli: &Cli) -> Result<(), AgentError> {
    let project_dir = cli.project_path();
    if !project_dir.join("zerct.toml").exists() {
        let projects = discover_deploy_projects(&project_dir)?;
        if !projects.is_empty() {
            return Err(AgentError::new(
                "workspace_preview_unsupported",
                "Preview one project at a time.",
                "Run `zerct preview api` or `zerct preview web` from the workspace root.",
            ));
        }
    }

    let report = doctor_report(&project_dir);
    if let Some(check) = report.checks.iter().find(|check| !check.ok) {
        return Err(AgentError::new(
            "doctor_failed",
            "Zerct doctor failed.",
            check.agent_instruction.clone(),
        ));
    }

    let config = parse_config(&project_dir.join("zerct.toml"))?;
    run_shell(
        &config.build_command,
        &project_dir,
        "Build failed before preview.",
    )?;
    if config.kind == "static_frontend" {
        let port = cli.port.unwrap_or(4173);
        let output = config.build_output_dir.unwrap_or_else(|| "dist".to_owned());
        println!("preview http://127.0.0.1:{port}");
        let status = Command::new("python3")
            .args([
                "-m",
                "http.server",
                &port.to_string(),
                "--bind",
                "127.0.0.1",
                "--directory",
                &output,
            ])
            .current_dir(&project_dir)
            .status()
            .map_err(|error| {
                AgentError::new(
                    "preview_failed",
                    format!("Could not start static preview: {error}"),
                    "Install Python 3 or run the frontend preview script directly.",
                )
            })?;
        return status_success(
            status,
            "preview_failed",
            "Static preview exited with an error.",
        );
    }

    let port = cli.port.unwrap_or(config.port);
    println!("preview http://127.0.0.1:{port}");
    let status = Command::new("sh")
        .arg("-c")
        .arg(&config.run_command)
        .current_dir(project_dir)
        .env("PORT", port.to_string())
        .status()
        .map_err(|error| {
            AgentError::new(
                "preview_failed",
                format!("Could not start runtime preview: {error}"),
                "Fix [run].command in zerct.toml and retry `zerct preview`.",
            )
        })?;
    status_success(
        status,
        "preview_failed",
        "Runtime preview exited with an error.",
    )
}

fn run_shell(command: &str, project_dir: &Path, message: &'static str) -> Result<(), AgentError> {
    println!("{command}");
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(project_dir)
        .status()
        .map_err(|error| {
            AgentError::new(
                "command_failed",
                format!("{message}: {error}"),
                "Fix the command output above, then retry.",
            )
        })?;
    status_success(status, "command_failed", message)
}

fn status_success(
    status: std::process::ExitStatus,
    code: &'static str,
    message: &'static str,
) -> Result<(), AgentError> {
    if status.success() {
        Ok(())
    } else {
        Err(AgentError::new(
            code,
            message,
            "Fix the command output above, then retry.",
        ))
    }
}

fn doctor_workspace(
    project_dir: &Path,
    projects: &[DeployProject],
    json: bool,
) -> Result<(), AgentError> {
    let reports = projects
        .iter()
        .map(|project| (project.relative.clone(), doctor_report(&project.dir)))
        .collect::<Vec<_>>();
    if json {
        println!("{}", doctor_workspace_json(project_dir, &reports));
    } else {
        for (relative, report) in &reports {
            println!("project {relative}");
            for check in &report.checks {
                let status = if check.ok { "ok" } else { "fail" };
                println!("{status} {} - {}", check.name, check.message);
            }
        }
    }

    if let Some(check) = reports
        .iter()
        .flat_map(|(_relative, report)| report.checks.iter())
        .find(|check| !check.ok)
    {
        if json {
            Err(AgentError::already_reported(
                "doctor_failed",
                "Zerct doctor failed.",
                check.agent_instruction.clone(),
            ))
        } else {
            Err(AgentError::new(
                "doctor_failed",
                "Zerct doctor failed.",
                check.agent_instruction.clone(),
            ))
        }
    } else {
        Ok(())
    }
}

fn doctor_workspace_json(project_dir: &Path, reports: &[(String, DoctorReport)]) -> String {
    let projects = reports
        .iter()
        .map(|(relative, report)| report.to_json_with_relative(relative))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"ok\":{},\"workspace\":\"{}\",\"projects\":[{}]}}",
        reports
            .iter()
            .all(|(_relative, report)| report.checks.iter().all(|check| check.ok)),
        escape_json(&project_dir.display().to_string()),
        projects
    )
}

#[derive(Debug)]
struct DoctorReport {
    project: PathBuf,
    config: Option<Config>,
    checks: Vec<Check>,
}

impl DoctorReport {
    fn to_json(&self) -> String {
        self.to_json_fields(None)
    }

    fn to_json_with_relative(&self, relative: &str) -> String {
        self.to_json_fields(Some(relative))
    }

    fn to_json_fields(&self, relative: Option<&str>) -> String {
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
        let relative_field = relative.map_or_else(String::new, |value| {
            format!("\"relative\":\"{}\",", escape_json(value))
        });
        format!(
            "{{{}\"ok\":{},\"project\":\"{}\",\"config\":{},\"checks\":[{}]}}",
            relative_field,
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
    let mut detected_kind = detect_project_kind(project_dir);
    let config = match parse_config(&project_dir.join("zerct.toml")) {
        Ok(config) => {
            detected_kind.clone_from(&config.kind);
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
        .map_or(detected_kind.as_str(), |config| config.kind.as_str());
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
        checks.extend(frontend_script_checks(project_dir, config.is_some()));
    } else {
        checks.push(cargo_lints(project_dir));
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
    if kind == "rust_backend" && config.is_some() {
        checks.push(cargo_fmt(project_dir));
        checks.push(cargo_check(project_dir));
        checks.push(cargo_clippy(project_dir));
    }

    DoctorReport {
        project: project_dir.to_path_buf(),
        config,
        checks,
    }
}

fn cargo_lints(project_dir: &Path) -> Check {
    let source = match fs::read_to_string(project_dir.join("Cargo.toml")) {
        Ok(source) => source,
        Err(error) => {
            return Check {
                name: "cargo lints".to_owned(),
                ok: false,
                message: error.to_string(),
                agent_instruction: "Create Cargo.toml with strict Rust lints, then retry."
                    .to_owned(),
            };
        }
    };
    let ok = cargo_lint_level(&source, "unsafe_code").as_deref() == Some("forbid")
        && cargo_lint_level(&source, "warnings").as_deref() == Some("deny");

    Check {
        name: "cargo lints".to_owned(),
        ok,
        message: if ok {
            "strict".to_owned()
        } else {
            "missing unsafe_code=forbid or warnings=deny".to_owned()
        },
        agent_instruction:
            "Add `[lints.rust]` with `unsafe_code = \"forbid\"` and `warnings = \"deny\"`, then retry."
                .to_owned(),
    }
}

fn cargo_lint_level(source: &str, lint_name: &str) -> Option<String> {
    let mut section = "";
    for raw_line in source.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if let Some(section_name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = section_name;
            continue;
        }
        if section != "lints.rust" && section != "workspace.lints.rust" {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == lint_name {
            return cargo_lint_value_level(value.trim());
        }
    }

    None
}

fn cargo_lint_value_level(value: &str) -> Option<String> {
    if let Some(level) = parse_json_string(value) {
        return Some(level);
    }

    let level_start = value.find("level")?;
    let after_level = &value[level_start + "level".len()..];
    let (_key, value) = after_level.split_once('=')?;
    parse_json_string(value.trim())
}

fn detect_project_kind(project_dir: &Path) -> String {
    let source = fs::read_to_string(project_dir.join("zerct.toml")).unwrap_or_default();
    source
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("kind")
                .and_then(|after_key| after_key.trim_start().strip_prefix('='))
                .map(str::trim)
                .map(|value| value.trim_matches('"').to_owned())
        })
        .filter(|kind| kind == "static_frontend" || kind == "rust_backend")
        .unwrap_or_else(|| "rust_backend".to_owned())
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

fn cargo_fmt(project_dir: &Path) -> Check {
    let output = Command::new("cargo")
        .args(["fmt", "--all", "--check"])
        .current_dir(project_dir)
        .output();

    match output {
        Ok(output) => Check {
            name: "cargo fmt".to_owned(),
            ok: output.status.success(),
            message: if output.status.success() {
                "passed".to_owned()
            } else {
                truncate_check_message(&String::from_utf8_lossy(&output.stderr))
            },
            agent_instruction: "Run `cargo fmt --all`, then redeploy.".to_owned(),
        },
        Err(error) => Check {
            name: "cargo fmt".to_owned(),
            ok: false,
            message: error.to_string(),
            agent_instruction:
                "Install rustfmt with Rust, then run `cargo fmt --all --check` before deploying."
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
    let project_dir = path.parent().unwrap_or_else(|| Path::new("."));

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
            frontend_build_command(project_dir).to_owned()
        } else {
            "cargo build --release".to_owned()
        };
    }
    if check_command.is_empty() {
        check_command = if kind == "static_frontend" {
            frontend_check_command(project_dir).to_owned()
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
    if config.build_output_dir.is_some() {
        return Err(AgentError::new(
            "invalid_build_output",
            "build.output is only valid for static frontend projects.",
            "Remove [build].output or set kind to static_frontend.",
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

fn validate_check_command(kind: &str, command: &str) -> Result<(), AgentError> {
    if kind == "static_frontend" {
        return validate_frontend_check_command(command);
    }

    validate_rust_check_command(command)
}

fn validate_rust_check_command(command: &str) -> Result<(), AgentError> {
    let required = [
        "cargo fmt --all --check",
        "cargo check --locked",
        "cargo clippy --locked",
        "--all-targets",
        "--all-features",
        "-D warnings",
    ];
    if required.iter().all(|fragment| command.contains(fragment)) {
        return Ok(());
    }

    Err(AgentError::new(
        "policy_rejected",
        "Check command is too weak for Zerct deploys.",
        "Set [build].check to include `cargo fmt --all --check`, `cargo check --locked`, and `cargo clippy --locked --all-targets --all-features -- -D warnings`, then redeploy.",
    ))
}

fn validate_frontend_check_command(command: &str) -> Result<(), AgentError> {
    if uses_javascript_linter(command) {
        return Err(AgentError::new(
            "policy_rejected",
            "Check command uses JavaScript-based lint or format tooling.",
            "Use native frontend checks such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then redeploy.",
        ));
    }

    let tokens = command_tokens(command);
    if has_frontend_install_command(&tokens)
        && has_frontend_script_run(&tokens, "typecheck")
        && has_frontend_script_run(&tokens, "lint")
    {
        return Ok(());
    }

    Err(AgentError::new(
        "policy_rejected",
        "Check command is too weak for Zerct deploys.",
        "Set [build].check to install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`, then redeploy.",
    ))
}

fn discover_deploy_projects(root_dir: &Path) -> Result<Vec<DeployProject>, AgentError> {
    if !root_dir.is_dir() {
        return Err(AgentError::new(
            "missing_project",
            "Project directory does not exist.",
            "Run Zerct from the root of a Rust project or pass the project path.",
        ));
    }
    if root_dir.join("zerct.toml").exists() {
        return Ok(vec![deploy_project_info(root_dir, root_dir)]);
    }

    let mut project_dirs = Vec::new();
    discover_project_dirs(root_dir, &mut project_dirs);
    let mut projects = project_dirs
        .iter()
        .map(|project_dir| deploy_project_info(project_dir, root_dir))
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| {
        kind_order(&left.kind)
            .cmp(&kind_order(&right.kind))
            .then_with(|| left.relative.cmp(&right.relative))
    });
    Ok(projects)
}

fn discover_project_dirs(dir: &Path, project_dirs: &mut Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    let mut entries = read_dir.flatten().collect::<Vec<_>>();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_symlink = entry
            .file_type()
            .is_ok_and(|file_type| file_type.is_symlink());
        if is_symlink || should_skip_workspace_dir(name) || !path.is_dir() {
            continue;
        }
        if path.join("zerct.toml").exists() {
            project_dirs.push(path);
        } else {
            discover_project_dirs(&path, project_dirs);
        }
    }
}

fn deploy_project_info(project_dir: &Path, root_dir: &Path) -> DeployProject {
    let relative = project_dir
        .strip_prefix(root_dir)
        .ok()
        .and_then(Path::to_str)
        .filter(|value| !value.is_empty())
        .map_or_else(|| ".".to_owned(), |value| value.replace('\\', "/"));
    let kind = parse_config(&project_dir.join("zerct.toml"))
        .map_or_else(|_error| "unknown".to_owned(), |config| config.kind);
    DeployProject {
        dir: project_dir.to_path_buf(),
        relative,
        kind,
    }
}

fn kind_order(kind: &str) -> u8 {
    match kind {
        "rust_backend" => 0,
        "static_frontend" => 1,
        _other => 2,
    }
}

fn should_skip_workspace_dir(name: &str) -> bool {
    matches!(
        name,
        ".cache"
            | ".git"
            | ".next"
            | ".turbo"
            | ".zerct"
            | "build"
            | "coverage"
            | "dist"
            | "node_modules"
            | "target"
            | "vendor"
    )
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
    let projects = discover_deploy_projects(&project_dir)?;
    if projects.is_empty() {
        return Err(AgentError::new(
            "missing_project_contract",
            "No zerct.toml was found.",
            "Run `zerct init` in each app directory, or pass a project path.",
        ));
    }

    if projects.len() == 1 {
        let Some(project) = projects.first() else {
            return Err(AgentError::new(
                "missing_project_contract",
                "No zerct.toml was found.",
                "Run `zerct init` in each app directory, or pass a project path.",
            ));
        };
        if project.kind == "static_frontend" && cli.database {
            return Err(AgentError::new(
                "invalid_database_target",
                "Static frontends cannot attach managed Postgres directly.",
                "Deploy a Rust backend with managed Postgres and call it from the frontend.",
            ));
        }
        let token = read_or_login_token(&project.dir, cli)?;
        let mut response = deploy_project(cli, &project.dir, &token, cli.database)?;
        if cli.wait {
            response = with_final_build(cli, &token, &response)?;
        }
        println!("{response}");
        return Ok(());
    }

    let token = read_or_login_token(&project_dir, cli)?;
    if !cli.json {
        println!("deploying {} projects", projects.len());
    }
    let mut responses = Vec::new();
    for project in projects {
        let wants_database = cli.database && project.kind == "rust_backend";
        if !cli.json {
            println!("checking {}", project.relative);
        }
        let response = deploy_project(cli, &project.dir, &token, wants_database)?;
        if !cli.json {
            println!("{} {response}", project.relative);
        }
        responses.push((project.relative, response));
    }
    if cli.wait {
        for (_relative, response) in &mut responses {
            *response = with_final_build(cli, &token, response)?;
        }
    }
    if cli.json {
        println!("[");
        for (index, (_relative, response)) in responses.iter().enumerate() {
            if index > 0 {
                println!(",");
            }
            print!("{response}");
        }
        println!("\n]");
    }
    Ok(())
}

fn deploy_project(
    cli: &Cli,
    project_dir: &Path,
    token: &str,
    wants_database: bool,
) -> Result<String, AgentError> {
    let report = doctor_report(project_dir);
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
        git_commit_sha(project_dir).map_or_else(
            || "null".to_owned(),
            |sha| format!("\"{}\"", escape_json(&sha))
        ),
        wants_database,
        archive_base64(project_dir)?
    );
    api_request(cli, "POST", "/v1/deploy", Some(token), Some(&body))
}

fn with_final_build(cli: &Cli, token: &str, response: &str) -> Result<String, AgentError> {
    let build_id = json_string_after(response, "\"build_job\"", "id").ok_or_else(|| {
        AgentError::new(
            "build_status_unavailable",
            "Deploy response did not include a build id.",
            "Run `zerct builds` and `zerct logs --build <build_id>` to inspect the deploy.",
        )
    })?;
    let final_build = wait_for_build(cli, token, &build_id)?;
    append_json_field(response, "final_build", &final_build)
}

fn wait_for_build(cli: &Cli, token: &str, build_id: &str) -> Result<String, AgentError> {
    let deadline = Instant::now() + Duration::from_secs(cli.wait_timeout_seconds);
    let mut last_status = String::new();
    while Instant::now() <= deadline {
        let response = api_request(
            cli,
            "GET",
            &format!("/v1/builds/{}", url_encode(build_id)),
            Some(token),
            None,
        )?;
        let status = json_string_after(&response, "\"build\"", "status")
            .or_else(|| json_string_field(&response, "status"))
            .ok_or_else(|| {
                AgentError::new(
                    "build_status_unavailable",
                    "Build status is unavailable.",
                    format!("Retry with `zerct logs --build {build_id}`."),
                )
            })?;
        if status != last_status {
            progress(cli, &format!("build {build_id} {status}"));
            last_status.clone_from(&status);
        }
        if matches!(status.as_str(), "succeeded" | "failed" | "canceled") {
            return Ok(response);
        }
        thread::sleep(Duration::from_secs(3));
    }

    Err(AgentError::new(
        "build_wait_timeout",
        format!("Timed out waiting for build {build_id}."),
        format!("Run `zerct logs --build {build_id}` to continue watching."),
    ))
}

fn require_app(cli: &Cli) -> Result<&str, AgentError> {
    cli.app.as_deref().ok_or_else(|| {
        AgentError::new(
            "missing_app",
            "App is required.",
            "Pass `--app <app>` using either the app name from zerct.toml or the app id printed by deploy.",
        )
    })
}

fn public_get(cli: &Cli, route: &str) -> Result<(), AgentError> {
    println!("{}", api_request(cli, "GET", route, None, None)?);
    Ok(())
}

fn authenticated_get(cli: &Cli, route: &str) -> Result<(), AgentError> {
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    println!("{}", api_request(cli, "GET", route, Some(&token), None)?);
    Ok(())
}

fn app_get(cli: &Cli, route: &str) -> Result<(), AgentError> {
    let app = require_app(cli)?;
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    let response = api_request(
        cli,
        "GET",
        &format!("/v1/apps/{}/{route}", url_encode(app)),
        Some(&token),
        None,
    )?;
    println!("{response}");
    Ok(())
}

fn deploys(cli: &Cli) -> Result<(), AgentError> {
    let route = if let Some(app) = &cli.app {
        format!("/v1/apps/{}/deploys{}", url_encode(app), page_query(cli))
    } else {
        format!("/v1/deploys{}", page_query(cli))
    };
    authenticated_get(cli, &route)
}

fn builds(cli: &Cli) -> Result<(), AgentError> {
    let route = if let Some(app) = &cli.app {
        format!("/v1/apps/{}/builds{}", url_encode(app), page_query(cli))
    } else {
        format!("/v1/builds{}", page_query(cli))
    };
    authenticated_get(cli, &route)
}

fn logs(cli: &Cli) -> Result<(), AgentError> {
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    let page = page_query(cli);
    let route = if let Some(build) = &cli.build {
        format!("/v1/builds/{}/logs{page}", url_encode(build))
    } else if let Some(deploy) = &cli.deploy_id {
        format!("/v1/deploys/{}/logs{page}", url_encode(deploy))
    } else {
        format!("/v1/apps/{}/logs{page}", url_encode(require_app(cli)?))
    };
    println!("{}", api_request(cli, "GET", &route, Some(&token), None)?);
    Ok(())
}

fn env_command(cli: &Cli) -> Result<(), AgentError> {
    let action = cli.args.first().map_or("list", String::as_str);
    if action == "list" {
        return app_get(cli, "env");
    }
    let app = require_app(cli)?;
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    if action == "delete" {
        let name = cli.args.get(1).ok_or_else(|| {
            AgentError::new(
                "invalid_env",
                "Environment variable name is required.",
                "Use `zerct env delete --app <app> KEY`.",
            )
        })?;
        let response = api_request(
            cli,
            "DELETE",
            &format!("/v1/apps/{}/env/{}", url_encode(app), url_encode(name)),
            Some(&token),
            None,
        )?;
        println!("{response}");
        return Ok(());
    }
    if action != "set" {
        return Err(AgentError::new(
            "unknown_command",
            "Unknown env command.",
            "Use `zerct env list`, `zerct env set`, or `zerct env delete`.",
        ));
    }

    let assignment = require_env_assignment(cli)?;
    let Some((name, value)) = assignment.split_once('=') else {
        return Err(invalid_env_error());
    };
    let body = format!(
        "{{\"name\":\"{}\",\"value\":\"{}\"}}",
        escape_json(name),
        escape_json(value)
    );
    let response = api_request(
        cli,
        "PUT",
        &format!("/v1/apps/{}/env", url_encode(app)),
        Some(&token),
        Some(&body),
    )?;
    println!("{response}");
    Ok(())
}

fn require_env_assignment(cli: &Cli) -> Result<&str, AgentError> {
    let assignment = cli.args.get(1).ok_or_else(invalid_env_error)?;
    if !assignment.contains('=') {
        return Err(invalid_env_error());
    }
    Ok(assignment)
}

fn invalid_env_error() -> AgentError {
    AgentError::new(
        "invalid_env",
        "Environment assignment must be KEY=value.",
        "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
    )
}

fn domains_command(cli: &Cli) -> Result<(), AgentError> {
    let action = cli.args.first().map_or("list", String::as_str);
    if action == "list" {
        return app_get(cli, "domains");
    }
    let domain = cli.args.get(1).ok_or_else(|| {
        AgentError::new(
            "missing_domain",
            "Domain is required.",
            "Use `zerct domains add --app <app> api.example.com`.",
        )
    })?;
    let app = require_app(cli)?;
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    let route = match action {
        "add" => format!("/v1/apps/{}/domains", url_encode(app)),
        "verify" => format!(
            "/v1/apps/{}/domains/{}/verify",
            url_encode(app),
            url_encode(domain)
        ),
        "delete" => format!(
            "/v1/apps/{}/domains/{}",
            url_encode(app),
            url_encode(domain)
        ),
        _unknown => {
            return Err(AgentError::new(
                "unknown_command",
                "Unknown domains command.",
                "Use `domains list`, `domains add`, `domains verify`, or `domains delete`.",
            ));
        }
    };
    let body = (action == "add").then(|| format!("{{\"domain\":\"{}\"}}", escape_json(domain)));
    let response = api_request(
        cli,
        domain_method(action),
        &route,
        Some(&token),
        body.as_deref(),
    )?;
    println!("{response}");
    Ok(())
}

fn domain_method(action: &str) -> &'static str {
    match action {
        "delete" => "DELETE",
        _other => "POST",
    }
}

fn billing(cli: &Cli) -> Result<(), AgentError> {
    let token = read_or_login_token(&current_dir_or_dot(), cli)?;
    if cli.args.first().map(String::as_str) == Some("portal") {
        let response = api_request(cli, "POST", "/v1/billing/portal", Some(&token), None)?;
        println!("{response}");
        return Ok(());
    }
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
        Some(std::path::Component::Normal(root)) => root
            .to_str()
            .is_some_and(|root| FRONTEND_SOURCE_ROOTS.contains(&root)),
        _ => false,
    }
}

fn is_frontend_typescript_source(relative: &Path) -> bool {
    !display_path(relative).ends_with(".d.ts")
        && path_has_extension(relative, FRONTEND_TYPESCRIPT_EXTENSIONS)
}

fn is_frontend_javascript_source(relative: &Path) -> bool {
    path_has_extension(relative, FRONTEND_JAVASCRIPT_EXTENSIONS)
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

#[derive(Clone, Copy)]
enum FrontendPackageManager {
    Bun,
    Npm,
}

impl FrontendPackageManager {
    fn from_project_dir(project_dir: &Path) -> Self {
        if project_dir.join("bun.lock").exists() || project_dir.join("bun.lockb").exists() {
            Self::Bun
        } else {
            Self::Npm
        }
    }

    const fn command(self) -> &'static str {
        match self {
            Self::Bun => "bun",
            Self::Npm => "npm",
        }
    }
}

fn frontend_check_command(project_dir: &Path) -> &'static str {
    match FrontendPackageManager::from_project_dir(project_dir) {
        FrontendPackageManager::Bun => DEFAULT_BUN_FRONTEND_CHECK_COMMAND,
        FrontendPackageManager::Npm => DEFAULT_NPM_FRONTEND_CHECK_COMMAND,
    }
}

fn frontend_build_command(project_dir: &Path) -> &'static str {
    match FrontendPackageManager::from_project_dir(project_dir) {
        FrontendPackageManager::Bun => "bun run build",
        FrontendPackageManager::Npm => "npm run build",
    }
}

fn frontend_script_checks(project_dir: &Path, run_scripts: bool) -> Vec<Check> {
    let manifest = fs::read_to_string(project_dir.join("package.json")).unwrap_or_default();
    let mut checks = ["typecheck", "lint"]
        .into_iter()
        .map(|script| {
            let ok = package_script_value(&manifest, script).is_some();
            Check {
                name: format!("package script {script}"),
                ok,
                message: if ok { "found" } else { "missing" }.to_owned(),
                agent_instruction: format!(
                    "Add a non-empty \"{script}\" script to package.json, then retry."
                ),
            }
        })
        .collect::<Vec<_>>();
    let lint_script = package_script_value(&manifest, "lint").unwrap_or_default();
    let typecheck_script = package_script_value(&manifest, "typecheck").unwrap_or_default();
    let strict_typecheck = uses_strict_frontend_typechecker(&typecheck_script);
    checks.push(Check {
        name: "strict frontend typecheck".to_owned(),
        ok: strict_typecheck,
        message: if strict_typecheck {
            "accepted".to_owned()
        } else {
            "tsgo --noEmit missing".to_owned()
        },
        agent_instruction: "Set package.json `typecheck` to `tsgo --noEmit`, install `@typescript/native-preview`, then retry.".to_owned(),
    });

    let native_lint =
        !uses_javascript_linter(&lint_script) && uses_native_frontend_linter(&lint_script);
    checks.push(Check {
        name: "native frontend lint".to_owned(),
        ok: native_lint,
        message: if native_lint {
            "accepted".to_owned()
        } else {
            "native linter missing".to_owned()
        },
        agent_instruction: "Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.".to_owned(),
    });

    if run_scripts && checks.iter().all(|check| check.ok) {
        checks.push(package_script_check(project_dir, "typecheck"));
        checks.push(package_script_check(project_dir, "lint"));
    }

    checks
}

fn package_script_value(manifest: &str, script: &str) -> Option<String> {
    let scripts_start = manifest.find("\"scripts\"")?;
    let scripts_source = &manifest[scripts_start..];
    let key = format!("\"{script}\"");
    let key_start = scripts_source.find(&key)?;
    let after_key = &scripts_source[key_start + key.len()..];
    let colon = after_key.find(':')?;
    parse_json_string(after_key[colon + 1..].trim_start())
}

fn parse_json_string(source: &str) -> Option<String> {
    let mut chars = source.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    for character in chars {
        if escaped {
            value.push(character);
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

fn uses_javascript_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name(token);
        JAVASCRIPT_LINTERS.contains(&command_name)
            || (command_name == "next" && tokens.get(index + 1).is_some_and(|next| *next == "lint"))
    })
}

fn uses_strict_frontend_typechecker(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name(token);
        (command_name == "tsgo" && tokens.contains(&"--noEmit"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|next| *next == "check"))
    })
}

fn uses_native_frontend_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name(token);
        command_name == "oxlint"
            || (command_name == "biome"
                && tokens
                    .get(index + 1)
                    .is_some_and(|next| matches!(*next, "check" | "lint")))
            || (command_name == "deno" && tokens.get(index + 1).is_some_and(|next| *next == "lint"))
    })
}

fn command_tokens(command: &str) -> Vec<&str> {
    command
        .split(|character: char| {
            matches!(
                character,
                ' ' | '\t' | '\n' | '\r' | '&' | '|' | ';' | '(' | ')'
            )
        })
        .filter_map(|token| {
            let trimmed = token.trim_matches(|character| character == '"' || character == '\'');
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .collect()
}

fn command_name(token: &str) -> &str {
    token.rsplit('/').next().unwrap_or(token)
}

fn has_frontend_install_command(tokens: &[&str]) -> bool {
    tokens.windows(2).any(|window| match window {
        [command, subcommand] => {
            FRONTEND_INSTALL_COMMANDS.contains(&(command_name(command), *subcommand))
        }
        _ => false,
    })
}

fn has_frontend_script_run(tokens: &[&str], script: &str) -> bool {
    tokens.windows(3).any(|window| match window {
        [command, "run", script_name] => {
            is_frontend_package_manager(command) && *script_name == script
        }
        _ => false,
    }) || tokens.windows(4).any(|window| match window {
        [command, "run", flag, script_name] => {
            is_frontend_package_manager(command) && flag.starts_with('-') && *script_name == script
        }
        _ => false,
    })
}

fn is_frontend_package_manager(command: &str) -> bool {
    FRONTEND_PACKAGE_MANAGERS.contains(&command_name(command))
}

fn package_script_check(project_dir: &Path, script: &str) -> Check {
    let manager = FrontendPackageManager::from_project_dir(project_dir);
    let mut command = Command::new(manager.command());
    match manager {
        FrontendPackageManager::Bun => {
            command.args(["run", script]);
        }
        FrontendPackageManager::Npm => {
            command.args(["run", "--silent", script]);
        }
    }
    let output = command.current_dir(project_dir).output();

    match output {
        Ok(output) => Check {
            name: format!("{} run {script}", manager.command()),
            ok: output.status.success(),
            message: if output.status.success() {
                "passed".to_owned()
            } else {
                truncate_check_message(&String::from_utf8_lossy(&output.stderr))
            },
            agent_instruction: format!(
                "Run `{} run {script}`, fix every error, then redeploy.",
                manager.command()
            ),
        },
        Err(error) => Check {
            name: format!("{} run {script}", manager.command()),
            ok: false,
            message: error.to_string(),
            agent_instruction: format!(
                "Install {}, then run `{} run {script}` before deploying.",
                match manager {
                    FrontendPackageManager::Bun => "Bun",
                    FrontendPackageManager::Npm => "Node.js and npm",
                },
                manager.command()
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

fn parse_u64(value: &str, code: &'static str) -> Result<u64, AgentError> {
    let parsed = value.parse::<u64>().map_err(|error| {
        AgentError::new(
            code,
            format!("Invalid numeric value: {error}"),
            "Use a positive integer value.",
        )
    })?;
    if parsed == 0 {
        return Err(AgentError::new(
            code,
            "Invalid numeric value: zero is not allowed.",
            "Use a positive integer value.",
        ));
    }
    Ok(parsed)
}

fn service_name_from_dir(project_dir: &Path) -> String {
    let value = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("api");
    service_name_from_value(value)
}

fn service_name_from_value(value: &str) -> String {
    let raw = value.to_ascii_lowercase();
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

fn service_name_from_cargo(project_dir: &Path) -> Option<String> {
    let source = fs::read_to_string(project_dir.join("Cargo.toml")).ok()?;
    source.lines().find_map(|line| {
        let trimmed = line.trim();
        let value = trimmed
            .strip_prefix("name")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .map(str::trim)?
            .trim_matches('"');
        Some(service_name_from_value(value))
    })
}

fn service_name_from_package(project_dir: &Path) -> Option<String> {
    let source = fs::read_to_string(project_dir.join("package.json")).ok()?;
    let key = source.find("\"name\"")?;
    let after_key = &source[key + "\"name\"".len()..];
    let colon = after_key.find(':')?;
    parse_json_string(after_key[colon + 1..].trim_start())
        .map(|name| service_name_from_value(&name))
}

fn infer_project_kind(project_dir: &Path) -> &'static str {
    if project_dir.join("Cargo.toml").exists() {
        "rust_backend"
    } else if project_dir.join("package.json").exists() {
        "static_frontend"
    } else {
        "rust_backend"
    }
}

fn rust_backend_config(project_dir: &Path) -> String {
    let name =
        service_name_from_cargo(project_dir).unwrap_or_else(|| service_name_from_dir(project_dir));
    format!(
        "name = \"{name}\"\n\n[build]\ncheck = \"{DEFAULT_RUST_CHECK_COMMAND}\"\ncommand = \"cargo build --release\"\n\n[run]\ncommand = \"./target/release/{name}\"\nport = 3000\nhealth = \"/healthz\"\n\n[resources]\nmemory = \"512mb\"\ncpu = \"0.25\"\nidle_timeout_minutes = 15\n"
    )
}

fn frontend_config(project_dir: &Path) -> String {
    let name = service_name_from_package(project_dir)
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    format!(
        "name = \"{name}\"\nkind = \"static_frontend\"\n\n[build]\ncheck = \"{}\"\ncommand = \"{}\"\noutput = \"dist\"\n",
        frontend_check_command(project_dir),
        frontend_build_command(project_dir)
    )
}

fn rust_api_source() -> &'static str {
    r##"use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_error| "3000".to_owned());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))?;

    for stream in listener.incoming() {
        handle(stream?)?;
    }

    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let size = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");
    let origin = request
        .lines()
        .find_map(|line| line.strip_prefix("Origin: "))
        .unwrap_or("*");
    let cors_origin = allowed_origin(origin);

    if method == "OPTIONS" {
        return write_response(&mut stream, "204 No Content", "", &cors_origin);
    }

    let body = if path == "/healthz" {
        r#"{"ok":true}"#
    } else {
        r#"{"message":"hello from zerct","backend":"rust"}"#
    };
    write_response(&mut stream, "200 OK", body, &cors_origin)
}

fn allowed_origin(request_origin: &str) -> String {
    let configured = std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_error| request_origin.to_owned());
    if configured == "*" || configured == request_origin {
        configured
    } else {
        "null".to_owned()
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    origin: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\naccess-control-allow-origin: {origin}\r\naccess-control-allow-methods: GET, OPTIONS\r\naccess-control-allow-headers: content-type, authorization\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}
"##
}

fn frontend_source(api_base_url: &str) -> String {
    r"import { createRootRoute, createRouter, RouterProvider } from '@tanstack/react-router'
import { createRoot } from 'react-dom/client'
import './styles.css'

const apiBaseUrl = import.meta.env.VITE_API_URL ?? '__API_BASE_URL__'

function App() {
  return (
    <main>
      <section>
        <h1>Zerct TanStack Frontend</h1>
        <p>Static runtime, dynamic Rust backend calls.</p>
        <code>{apiBaseUrl}</code>
      </section>
    </main>
  )
}

const rootRoute = createRootRoute({ component: App })
const router = createRouter({ routeTree: rootRoute })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

createRoot(document.getElementById('root')!).render(<RouterProvider router={router} />)
"
    .replace("__API_BASE_URL__", api_base_url)
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

fn page_query(cli: &Cli) -> String {
    let mut params = Vec::new();
    if let Some(limit) = &cli.limit {
        params.push(format!("limit={}", url_encode(limit)));
    }
    if let Some(cursor) = &cli.cursor {
        params.push(format!("cursor={}", url_encode(cursor)));
    }
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    encoded
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

fn json_string_after(source: &str, marker: &str, field: &str) -> Option<String> {
    let start = source.find(marker)?;
    json_string_field(source.get(start..)?, field)
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

fn append_json_field(source: &str, name: &str, raw_value: &str) -> Result<String, AgentError> {
    let body = source.trim_end().strip_suffix('}').ok_or_else(|| {
        AgentError::new(
            "api_unavailable",
            "Zerct API response was invalid.",
            "Retry the command. If it keeps failing, check Zerct status.",
        )
    })?;
    let mut output = String::with_capacity(source.len() + raw_value.len() + name.len() + 8);
    output.push_str(body);
    if !body.trim_end().ends_with('{') {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(raw_value);
    output.push('}');
    Ok(output)
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
