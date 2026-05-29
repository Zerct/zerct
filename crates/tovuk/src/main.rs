//! Native Tovuk command line interface.

#![allow(
    clippy::assigning_clones,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::format_push_string,
    clippy::large_enum_variant,
    clippy::map_unwrap_or,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::result_large_err,
    clippy::semicolon_if_nothing_returned,
    clippy::similar_names,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::useless_asref
)]

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::{Compression, write::GzEncoder};
use reqwest::{Method, StatusCode, blocking::Client};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeSet,
    env, fmt, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant},
};
use walkdir::{DirEntry, WalkDir};

const VERSION: &str = "0.1.51";
const DEFAULT_API_URL: &str = "https://api.tovuk.com";
const ARCHIVE_LIMIT_BYTES: usize = 48 * 1024 * 1024;
const DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS: u64 = 900;
const SESSION_DIR: &str = ".tovuk";
const SESSION_FILE: &str = "session-token";
const SESSION_SERVICE: &str = "com.tovuk.cli";
const SESSION_ACCOUNT: &str = "session-token";
const SESSION_LABEL: &str = "Tovuk session";
const DEFAULT_LOGIN_EXPIRES_SECONDS: u64 = 600;
const DEFAULT_LOGIN_INTERVAL_SECONDS: u64 = 5;
const BILLING_CHECKOUT_ROUTE: &str = "/v1/billing/checkout";

const RUST_STRICT_CLIPPY_DENY_LINTS: &[&str] = &[
    "clippy::all",
    "clippy::pedantic",
    "clippy::dbg_macro",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::panic",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::large_futures",
    "clippy::large_include_file",
    "clippy::large_stack_frames",
    "clippy::mem_forget",
    "clippy::rc_buffer",
    "clippy::rc_mutex",
    "clippy::redundant_clone",
    "clippy::clone_on_ref_ptr",
];

const DEFAULT_RUST_CHECK_COMMAND: &str = "cargo fmt --all --check && cargo check --locked --release --all-targets --all-features && cargo test --locked --release --all-targets --all-features && cargo clippy --locked --release --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -D clippy::large_futures -D clippy::large_include_file -D clippy::large_stack_frames -D clippy::mem_forget -D clippy::rc_buffer -D clippy::rc_mutex -D clippy::redundant_clone -D clippy::clone_on_ref_ptr";
const DEFAULT_NPM_FRONTEND_CHECK_COMMAND: &str =
    "npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint";
const DEFAULT_BUN_FRONTEND_CHECK_COMMAND: &str = "bun ci && bun run typecheck && bun run lint";

const PROJECT_KINDS: &[&str] = &["fullstack", "rust_backend", "static_frontend"];
const PROJECT_TEMPLATES: &[&str] = &[
    "rust-api",
    "tanstack-static-frontend",
    "fullstack-rust-tanstack",
];
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
const JAVASCRIPT_BACKEND_RUNTIMES: &[&str] = &[
    "astro",
    "bun",
    "deno",
    "next",
    "node",
    "npm",
    "npx",
    "pnpm",
    "svelte-kit",
    "tsx",
    "ts-node",
    "vite",
    "yarn",
];
const FRONTEND_SOURCE_ROOTS: &[&str] = &["src", "app", "pages", "routes", "components"];
const FRONTEND_JAVASCRIPT_EXTENSIONS: &[&str] = &[".js", ".jsx", ".mjs", ".cjs"];
const FRONTEND_PACKAGE_MANAGERS: &[&str] = &["npm", "bun", "pnpm", "yarn"];
const FRONTEND_INSTALL_COMMANDS: &[&str] = &[
    "npm ci",
    "bun ci",
    "bun install",
    "pnpm install",
    "yarn install",
];
const WALK_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tovuk",
    ".terraform",
    ".docker",
    ".gnupg",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".pulumi",
];
const WORKSPACE_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tovuk",
    ".terraform",
    ".docker",
    ".gnupg",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".pulumi",
    ".cache",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "vendor",
];
const ARCHIVE_EXCLUDES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tovuk",
    ".env",
    ".env.*",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".docker",
    ".gnupg",
    ".terraform",
    ".terraformrc",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".pulumi",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    ".config/gcloud",
    ".config/gh",
    ".config/hub",
    ".config/heroku",
    ".config/doctl",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.tfstate",
    "*.tfstate.*",
    "id_rsa",
    "id_ed25519",
    "*.sqlite",
    "*.sqlite3",
    "*.db",
    "*.log",
    "._*",
    ".DS_Store",
];

type Result<T> = std::result::Result<T, CliError>;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            error.print();
            ExitCode::from(error.exit_code)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = parse_args(env::args().skip(1).collect())?;
    if cli.help {
        println!("{}", help_text());
        return Ok(ExitCode::SUCCESS);
    }
    if cli.version {
        println!("{VERSION}");
        return Ok(ExitCode::SUCCESS);
    }

    match cli.command.as_str() {
        "init" => init_project(&project_path(cli.args.first())?, &cli.template),
        "install" => install_project(&project_path(cli.args.first())?, &cli.template),
        "doctor" => doctor_project(&project_path(cli.args.first())?, cli.json),
        "preview" => preview_project(&project_path(cli.args.first())?, cli.port),
        "login" => login(&cli),
        "deploy" => deploy(&project_path(cli.args.first())?, &cli),
        "capabilities" => capabilities(&cli),
        "me" => print_authenticated(&cli, "/v1/me"),
        "usage" => print_authenticated(&cli, "/v1/usage"),
        "activity" => print_paged_authenticated(&cli, "/v1/activity"),
        "apps" => print_authenticated(&cli, "/v1/apps"),
        "overview" => print_paged_authenticated(&cli, &app_route(&cli, "overview")?),
        "deploys" => deploys_command(&cli),
        "builds" => builds_command(&cli),
        "logs" => logs_command(&cli),
        "status" => app_get_command(&cli, "status"),
        "inspect" => app_get_command(&cli, "inspect"),
        "db" | "database" => app_get_command(&cli, "database"),
        "env" => env_command(&cli),
        "domains" => domains_command(&cli),
        "billing" => billing_command(&cli),
        "support" => support_command(&cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown Tovuk command.",
            "Run `tovuk --help` and retry with a supported command.",
            cli.json,
        )),
    }?;

    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Debug)]
struct CliOptions {
    command: String,
    args: Vec<String>,
    api_url: String,
    app: String,
    build: String,
    deploy: String,
    limit: String,
    cursor: String,
    failing_command: String,
    first_log_line: String,
    token: String,
    template: String,
    severity: String,
    port: u16,
    wait_timeout_seconds: u64,
    json: bool,
    database: bool,
    wait: bool,
    help: bool,
    version: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: "help".to_owned(),
            args: Vec::new(),
            api_url: DEFAULT_API_URL.to_owned(),
            app: String::new(),
            build: String::new(),
            deploy: String::new(),
            limit: String::new(),
            cursor: String::new(),
            failing_command: String::new(),
            first_log_line: String::new(),
            token: String::new(),
            template: String::new(),
            severity: String::new(),
            port: 0,
            wait_timeout_seconds: DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS,
            json: false,
            database: false,
            wait: false,
            help: false,
            version: false,
        }
    }
}

fn parse_args(argv: Vec<String>) -> Result<CliOptions> {
    let mut cli = CliOptions::default();
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < argv.len() {
        let arg = argv[index].clone();
        if arg == "--" {
            positional.extend(argv.iter().skip(index + 1).cloned());
            break;
        }
        if let Some((name, inline)) = parse_flag(&arg) {
            let consumed = apply_flag(&mut cli, name, inline, &argv, index)?;
            index += consumed;
        } else if arg.starts_with('-') {
            return Err(agent_error(
                "unknown_argument",
                format!("Unknown Tovuk option: {arg}."),
                "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
                cli.json,
            ));
        } else {
            positional.push(arg);
            index += 1;
        }
    }

    if let Some(command) = positional.first() {
        cli.command.clone_from(command);
        cli.args = positional.into_iter().skip(1).collect();
    }
    cli.api_url = cli.api_url.trim_end_matches('/').to_owned();
    Ok(cli)
}

fn parse_flag(arg: &str) -> Option<(&str, Option<String>)> {
    if !arg.starts_with('-') {
        return None;
    }
    if arg.starts_with("--") {
        if let Some(index) = arg.find('=') {
            if index > 2 {
                return Some((&arg[..index], Some(arg[index + 1..].to_owned())));
            }
        }
    }
    Some((arg, None))
}

fn apply_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    let json_output = cli.json;
    match name {
        "--help" | "-h" => set_boolean_flag(&inline, || cli.help = true, name, json_output),
        "--version" | "-v" | "-V" => {
            set_boolean_flag(&inline, || cli.version = true, name, json_output)
        }
        "--json" => set_boolean_flag(&inline, || cli.json = true, name, json_output),
        "--database" => set_boolean_flag(&inline, || cli.database = true, name, json_output),
        "--no-database" => set_boolean_flag(&inline, || cli.database = false, name, json_output),
        "--wait" => set_boolean_flag(&inline, || cli.wait = true, name, json_output),
        "--api" => {
            cli.api_url = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--app" => {
            cli.app = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--build" => {
            cli.build = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--deploy" => {
            cli.deploy = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--failing-command" => {
            cli.failing_command = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--first-log-line" => {
            cli.first_log_line = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--limit" => {
            cli.limit = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--cursor" => {
            cli.cursor = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--severity" => {
            cli.severity = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--token" => {
            cli.token = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--template" => {
            cli.template = flag_value(name, inline, argv, index, cli.json)?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--port" => {
            cli.port = parse_u16(
                &flag_value(name, inline, argv, index, cli.json)?,
                name,
                cli.json,
            )?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        "--wait-timeout" => {
            cli.wait_timeout_seconds = parse_u64(
                &flag_value(name, inline, argv, index, cli.json)?,
                name,
                cli.json,
            )?;
            Ok(if arg_has_inline_value(argv, index) {
                1
            } else {
                2
            })
        }
        _ => Err(agent_error(
            "unknown_argument",
            format!("Unknown Tovuk option: {name}."),
            "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
            cli.json,
        )),
    }
}

fn set_boolean_flag(
    inline: &Option<String>,
    mut set: impl FnMut(),
    name: &str,
    json_output: bool,
) -> Result<usize> {
    if inline.is_some() {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} does not accept a value."),
            format!("Use {name} without =value."),
            json_output,
        ));
    }
    set();
    Ok(1)
}

fn flag_value(
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<String> {
    let value = if let Some(value) = inline {
        value
    } else {
        argv.get(index + 1).cloned().ok_or_else(|| {
            agent_error(
                "missing_argument",
                format!("{name} requires a value."),
                format!("Pass a value after {name}."),
                json_output,
            )
        })?
    };
    if value.is_empty() || (!arg_has_inline_value(argv, index) && value.starts_with("--")) {
        return Err(agent_error(
            "missing_argument",
            format!("{name} requires a value."),
            format!("Pass a value after {name}."),
            json_output,
        ));
    }
    Ok(value)
}

fn arg_has_inline_value(argv: &[String], index: usize) -> bool {
    argv.get(index)
        .is_some_and(|arg| arg.starts_with("--") && arg.contains('='))
}

fn parse_u16(value: &str, name: &str, json_output: bool) -> Result<u16> {
    let parsed = value.parse::<u16>().map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as a positive integer."),
            json_output,
        )
    })?;
    if parsed == 0 {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as a positive integer."),
            json_output,
        ));
    }
    Ok(parsed)
}

fn parse_u64(value: &str, name: &str, json_output: bool) -> Result<u64> {
    let parsed = value.parse::<u64>().map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as seconds, for example {name} 900."),
            json_output,
        )
    })?;
    if parsed == 0 {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as seconds, for example {name} 900."),
            json_output,
        ));
    }
    Ok(parsed)
}

fn project_path(value: Option<&String>) -> Result<PathBuf> {
    let path = value.map_or_else(PathBuf::new, PathBuf::from);
    let path = if path.as_os_str().is_empty() {
        env::current_dir().map_err(|error| internal_error(error.to_string()))?
    } else if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|error| internal_error(error.to_string()))?
            .join(path)
    };
    Ok(path)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentErrorPayload {
    code: String,
    message: String,
    agent_instruction: Option<String>,
    docs_url: Option<String>,
    checkout_url: Option<String>,
}

#[derive(Debug)]
struct CliError {
    payload: AgentErrorPayload,
    json: bool,
    exit_code: u8,
}

impl CliError {
    fn print(&self) {
        print_agent_error(&self.payload, self.json);
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.payload.message)
    }
}

impl std::error::Error for CliError {}

fn agent_error(
    code: impl Into<String>,
    message: impl Into<String>,
    agent_instruction: impl Into<String>,
    json_output: bool,
) -> CliError {
    CliError {
        payload: AgentErrorPayload {
            code: code.into(),
            message: message.into(),
            agent_instruction: Some(agent_instruction.into()),
            docs_url: None,
            checkout_url: None,
        },
        json: json_output,
        exit_code: 1,
    }
}

fn internal_error(message: impl Into<String>) -> CliError {
    agent_error(
        "internal_error",
        message.into(),
        "Retry the command. If it keeps failing, create a Tovuk support ticket with command output.",
        false,
    )
}

fn print_agent_error(payload: &AgentErrorPayload, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(payload) {
            Ok(source) => eprintln!("{source}"),
            Err(error) => eprintln!("Tovuk command failed: {error}"),
        }
        return;
    }

    eprintln!("{}", payload.message);
    if let Some(instruction) = payload
        .agent_instruction
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        eprintln!("agent_instruction: {instruction}");
    }
    if let Some(docs_url) = payload
        .docs_url
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        eprintln!("docs: {docs_url}");
    }
    if let Some(checkout_url) = payload
        .checkout_url
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        eprintln!("checkout: {checkout_url}");
    }
}

fn print_json(value: &Value) -> Result<()> {
    let source =
        serde_json::to_string_pretty(value).map_err(|error| internal_error(error.to_string()))?;
    println!("{source}");
    Ok(())
}

fn capabilities(cli: &CliOptions) -> Result<()> {
    let response = api_request(cli, Method::GET, "/v1/capabilities", None, None)?;
    print_json(&response)
}

fn print_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, route, Some(&token), None)?;
    print_json(&response)
}

fn print_paged_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    print_authenticated(cli, &format!("{route}{}", page_query(cli)))
}

fn app_get_command(cli: &CliOptions, suffix: &str) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::GET,
        &app_route(cli, suffix)?,
        Some(&token),
        None,
    )?;
    print_json(&response)
}

fn deploys_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/deploys{}", page_query(cli))
    } else {
        format!(
            "/v1/apps/{}/deploys{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}

fn builds_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/builds{}", page_query(cli))
    } else {
        format!(
            "/v1/apps/{}/builds{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}

fn logs_command(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let (route, target) = if !cli.build.is_empty() {
        (
            format!(
                "/v1/builds/{}/logs{}",
                encode_component(&cli.build),
                page_query(cli)
            ),
            format!("--build {}", cli.build),
        )
    } else if !cli.deploy.is_empty() {
        (
            format!(
                "/v1/deploys/{}/logs{}",
                encode_component(&cli.deploy),
                page_query(cli)
            ),
            format!("--deploy {}", cli.deploy),
        )
    } else {
        let app = require_app(cli)?;
        (
            format!(
                "/v1/apps/{}/logs{}",
                encode_component(&app),
                page_query(cli)
            ),
            format!("--app {app}"),
        )
    };
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    if cli.json {
        return print_json(&response);
    }

    for line in response
        .get("lines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let timestamp = string_field(line, "timestamp");
        let stream = string_field(line, "stream");
        let message = string_field(line, "message");
        println!("[{timestamp}] {stream}: {message}");
    }
    if response
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let cursor = string_field(&response, "next_cursor");
        if !cursor.is_empty() {
            println!("next tovuk logs {target} --cursor {cursor}");
        }
    }
    Ok(())
}

fn env_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => app_get_command(cli, "env"),
        "set" => {
            let assignment = cli.args.get(1).cloned().unwrap_or_default();
            let separator = assignment.find('=').unwrap_or(0);
            if separator == 0 {
                return Err(agent_error(
                    "invalid_env",
                    "Environment assignment must be KEY=value.",
                    "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
                    cli.json,
                ));
            }
            let name = &assignment[..separator];
            let value = &assignment[separator + 1..];
            print_authenticated_mutation(
                cli,
                Method::PUT,
                &app_route(cli, "env")?,
                Some(json!({ "name": name, "value": value })),
            )
        }
        "delete" => {
            let name = command_arg(
                cli,
                "invalid_env",
                "Environment variable name is required.",
                "Use `tovuk env delete --app <app> KEY`.",
            )?;
            print_authenticated_mutation(
                cli,
                Method::DELETE,
                &app_route(cli, &format!("env/{}", encode_component(&name)))?,
                None,
            )
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown env command.",
            "Use `tovuk env list`, `env set`, or `env delete`.",
            cli.json,
        )),
    }
}

fn domains_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => app_get_command(cli, "domains"),
        "add" => {
            let domain = command_arg(
                cli,
                "missing_domain",
                "Domain is required.",
                "Use `tovuk domains add --app <app> api.example.com`.",
            )?;
            print_authenticated_mutation(
                cli,
                Method::POST,
                &app_route(cli, "domains")?,
                Some(json!({ "domain": domain })),
            )
        }
        "verify" => {
            let domain = command_arg(
                cli,
                "missing_domain",
                "Domain is required.",
                "Use `tovuk domains verify --app <app> api.example.com`.",
            )?;
            let app = require_app(cli)?;
            print_authenticated_mutation(
                cli,
                Method::POST,
                &format!(
                    "/v1/apps/{}/domains/{}/verify",
                    encode_component(&app),
                    encode_component(&domain)
                ),
                None,
            )
        }
        "delete" => {
            let domain = command_arg(
                cli,
                "missing_domain",
                "Domain is required.",
                "Use `tovuk domains delete --app <app> api.example.com`.",
            )?;
            let app = require_app(cli)?;
            print_authenticated_mutation(
                cli,
                Method::DELETE,
                &format!(
                    "/v1/apps/{}/domains/{}",
                    encode_component(&app),
                    encode_component(&domain)
                ),
                None,
            )
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown domains command.",
            "Use `domains list`, `domains add`, `domains verify`, or `domains delete`.",
            cli.json,
        )),
    }
}

fn billing_command(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let action = cli.args.first().map_or("checkout", String::as_str);
    let route = match action {
        "" | "checkout" => BILLING_CHECKOUT_ROUTE,
        "portal" => "/v1/billing/portal",
        _ => {
            return Err(agent_error(
                "unknown_billing_command",
                "Unknown billing command.",
                "Use `tovuk billing checkout --json` or `tovuk billing portal`.",
                cli.json,
            ));
        }
    };
    let reason = cli
        .args
        .iter()
        .skip(1)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let body = if route == BILLING_CHECKOUT_ROUTE {
        Some(json!({
            "target_plan": "pro",
            "reason": if reason.trim().is_empty() { "Upgrade to Tovuk Pro." } else { reason.trim() },
        }))
    } else {
        None
    };
    let response = api_request(cli, Method::POST, route, Some(&token), body)?;
    if cli.json {
        return print_json(&response);
    }
    let url = response
        .get("checkout")
        .and_then(|checkout| checkout.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    println!("{url}");
    if !url.is_empty() {
        open_url(url);
    }
    Ok(())
}

fn support_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_paged_authenticated(cli, "/v1/support/tickets"),
        "create" => support_create(cli),
        "resolve" => {
            let ticket_id = command_arg(
                cli,
                "invalid_support_ticket",
                "Support ticket id is required.",
                "Use `tovuk support resolve <ticket_id> --json` with an id from support list.",
            )?;
            print_authenticated_mutation(
                cli,
                Method::POST,
                &format!(
                    "/v1/support/tickets/{}/resolve",
                    encode_component(&ticket_id)
                ),
                None,
            )
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown support command.",
            "Use `tovuk support list --json` or `support create` with subject and details.",
            cli.json,
        )),
    }
}

fn support_create(cli: &CliOptions) -> Result<()> {
    let subject = cli.args.get(1).cloned().unwrap_or_default();
    let details = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if subject.is_empty() || details.trim().is_empty() {
        return Err(agent_error(
            "invalid_support_ticket",
            "Support ticket subject and details are required.",
            "Use `tovuk support create \"Short subject\" \"Command, app id, build id, deploy id, and first actionable log line\" --json`.",
            cli.json,
        ));
    }

    let token = read_or_login_token(cli)?;
    let mut body = Map::new();
    body.insert("subject".to_owned(), Value::String(subject));
    body.insert(
        "details".to_owned(),
        Value::String(details.trim().to_owned()),
    );
    body.insert(
        "severity".to_owned(),
        Value::String(if cli.severity.is_empty() {
            "normal".to_owned()
        } else {
            cli.severity.clone()
        }),
    );
    insert_optional(&mut body, "app_id", &cli.app);
    insert_optional(&mut body, "failing_command", &cli.failing_command);
    insert_optional(&mut body, "build_id", &cli.build);
    insert_optional(&mut body, "deploy_id", &cli.deploy);
    insert_optional(&mut body, "first_log_line", &cli.first_log_line);
    let response = api_request(
        cli,
        Method::POST,
        "/v1/support/tickets",
        Some(&token),
        Some(Value::Object(body)),
    )?;
    print_json(&response)
}

fn print_authenticated_mutation(
    cli: &CliOptions,
    method: Method,
    route: &str,
    body: Option<Value>,
) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, method, route, Some(&token), body)?;
    print_json(&response)
}

fn insert_optional(body: &mut Map<String, Value>, key: &str, value: &str) {
    if !value.is_empty() {
        body.insert(key.to_owned(), Value::String(value.to_owned()));
    }
}

fn command_arg(cli: &CliOptions, code: &str, message: &str, instruction: &str) -> Result<String> {
    cli.args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| agent_error(code, message, instruction, cli.json))
}

fn require_app(cli: &CliOptions) -> Result<String> {
    if cli.app.is_empty() {
        return Err(agent_error(
            "missing_app",
            "App is required.",
            "Pass `--app <app>` using either the app name from tovuk.toml or the app id printed by deploy.",
            cli.json,
        ));
    }
    Ok(cli.app.clone())
}

fn app_route(cli: &CliOptions, suffix: &str) -> Result<String> {
    Ok(format!(
        "/v1/apps/{}/{}",
        encode_component(&require_app(cli)?),
        suffix
    ))
}

fn page_query(cli: &CliOptions) -> String {
    let mut params = Vec::new();
    if !cli.limit.is_empty() {
        params.push(format!("limit={}", encode_component(&cli.limit)));
    }
    if !cli.cursor.is_empty() {
        params.push(format!("cursor={}", encode_component(&cli.cursor)));
    }
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

fn api_request(
    cli: &CliOptions,
    method: Method,
    route: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<Value> {
    let client = Client::builder()
        .user_agent(format!("tovuk-cli/{VERSION}"))
        .build()
        .map_err(|error| internal_error(error.to_string()))?;
    let url = format!("{}{}", cli.api_url, route);
    let mut request = client
        .request(method, url)
        .header("accept", "application/json");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    if let Some(body) = &body {
        request = request.json(body);
    }

    let response = request.send().map_err(|error| {
        agent_error(
            "api_unreachable",
            format!("Could not reach Tovuk API: {error}"),
            "Retry the command. If it keeps failing, check Tovuk status before changing your project.",
            cli.json,
        )
    })?;
    let status = response.status();
    let text = response.text().unwrap_or_default();
    let data = parse_json_text(&text);
    if status.is_success() {
        return Ok(data);
    }

    let mut payload = agent_payload_from_json(&data).unwrap_or_else(|| AgentErrorPayload {
        code: "api_error".to_owned(),
        message: format!("Tovuk API returned HTTP {}.", status.as_u16()),
        agent_instruction: Some(
            "Retry the command. If it keeps failing, check Tovuk status before changing your project."
                .to_owned(),
        ),
        docs_url: None,
        checkout_url: None,
    });
    enrich_agent_error_payload(cli, route, token, &mut payload);
    Err(CliError {
        payload,
        json: cli.json,
        exit_code: if status.is_server_error() { 2 } else { 1 },
    })
}

fn parse_json_text(text: &str) -> Value {
    if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(text).unwrap_or(Value::Null)
    }
}

fn agent_payload_from_json(value: &Value) -> Option<AgentErrorPayload> {
    let object = value.as_object()?;
    Some(AgentErrorPayload {
        code: object.get("code")?.as_str()?.to_owned(),
        message: object.get("message")?.as_str()?.to_owned(),
        agent_instruction: object
            .get("agent_instruction")
            .and_then(Value::as_str)
            .map(str::to_owned),
        docs_url: object
            .get("docs_url")
            .and_then(Value::as_str)
            .map(str::to_owned),
        checkout_url: object
            .get("checkout_url")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn enrich_agent_error_payload(
    cli: &CliOptions,
    route: &str,
    token: Option<&str>,
    payload: &mut AgentErrorPayload,
) {
    if payload.code != "payment_required"
        || payload
            .checkout_url
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        || token.is_none()
        || route == BILLING_CHECKOUT_ROUTE
    {
        return;
    }
    if let Some(url) = create_checkout_url(cli, token, &payload.message) {
        payload.checkout_url = Some(url);
    }
}

fn payment_required_agent_error(
    cli: &CliOptions,
    token: &str,
    message: impl Into<String>,
    instruction: impl Into<String>,
) -> CliError {
    let mut payload = AgentErrorPayload {
        code: "payment_required".to_owned(),
        message: message.into(),
        agent_instruction: Some(instruction.into()),
        docs_url: None,
        checkout_url: None,
    };
    enrich_agent_error_payload(cli, "local:preflight", Some(token), &mut payload);
    CliError {
        payload,
        json: cli.json,
        exit_code: 1,
    }
}

fn create_checkout_url(cli: &CliOptions, token: Option<&str>, reason: &str) -> Option<String> {
    let token = token?;
    let response = api_request(
        cli,
        Method::POST,
        BILLING_CHECKOUT_ROUTE,
        Some(token),
        Some(json!({
            "reason": if reason.is_empty() { "Plan limit reached." } else { reason },
            "target_plan": "pro",
        })),
    )
    .ok()?;
    response
        .get("checkout")
        .and_then(|checkout| checkout.get("url"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn read_or_login_token(cli: &CliOptions) -> Result<String> {
    let token = read_stored_token(cli);
    if !token.is_empty() {
        return Ok(token);
    }
    login_and_store(cli)
}

fn login(cli: &CliOptions) -> Result<()> {
    if !cli.token.trim().is_empty() {
        write_session_token(cli.token.trim())?;
        println!("saved Tovuk session token");
        return Ok(());
    }
    login_and_store(cli)?;
    Ok(())
}

fn login_and_store(cli: &CliOptions) -> Result<String> {
    let start = api_request(cli, Method::POST, "/v1/login/device", None, None)?;
    let login_url = string_alias(&start, &["loginUrl", "login_url"]);
    let user_code = string_alias(&start, &["userCode", "user_code"]);
    let device_code = string_alias(&start, &["deviceCode", "device_code"]);
    if login_url.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a browser URL.",
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.json,
        ));
    }
    open_url(&login_url);
    progress(cli, "opened browser login");
    progress(
        cli,
        &format!(
            "waiting for browser login code {}",
            if user_code.is_empty() {
                "TOVUK"
            } else {
                &user_code
            }
        ),
    );

    let session = poll_login(cli, &device_code, &start)?;
    let token = string_field(&session, "token");
    if token.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a session token.",
            "Run `tovuk login` again and complete the browser login.",
            cli.json,
        ));
    }
    write_session_token(&token)?;
    let email = string_field(&session, "email");
    progress(
        cli,
        &format!(
            "logged in as {}",
            if email.is_empty() {
                "Tovuk user"
            } else {
                &email
            }
        ),
    );
    Ok(token)
}

fn poll_login(cli: &CliOptions, device_code: &str, start: &Value) -> Result<Value> {
    if device_code.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a device code.",
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.json,
        ));
    }

    let expires_seconds = number_alias(start, &["expiresInSeconds", "expires_in_seconds"])
        .unwrap_or(DEFAULT_LOGIN_EXPIRES_SECONDS);
    let mut interval_seconds = number_alias(start, &["intervalSeconds", "interval_seconds"])
        .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS);
    let deadline = Instant::now() + Duration::from_secs(expires_seconds);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(interval_seconds));
        let response = api_request(
            cli,
            Method::GET,
            &format!("/v1/login/device/{}", encode_component(device_code)),
            None,
            None,
        )?;
        let status = string_field(&response, "status");
        if status == "complete" {
            return Ok(response);
        }
        if status == "expired" {
            return login_expired(cli);
        }
        interval_seconds = number_alias(&response, &["intervalSeconds", "interval_seconds"])
            .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS)
            .max(DEFAULT_LOGIN_INTERVAL_SECONDS);
    }
    login_expired(cli)
}

fn login_expired(cli: &CliOptions) -> Result<Value> {
    Err(agent_error(
        "login_expired",
        "Tovuk login expired before it completed.",
        "Run `tovuk login` again and finish the browser login in the newly opened tab.",
        cli.json,
    ))
}

fn read_stored_token(cli: &CliOptions) -> String {
    if !cli.token.trim().is_empty() {
        return cli.token.trim().to_owned();
    }
    if let Ok(token) = env::var("TOVUK_TOKEN") {
        if !token.trim().is_empty() {
            return token.trim().to_owned();
        }
    }
    let keychain = read_keychain_token();
    if !keychain.is_empty() {
        return keychain;
    }
    let user_token = read_token_file(&user_session_path());
    if !user_token.is_empty() {
        return user_token;
    }
    read_token_file(&home_dir().join(SESSION_DIR).join(SESSION_FILE))
}

fn write_session_token(token: &str) -> Result<()> {
    let clean_token = token.trim();
    if clean_token.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk session token is empty.",
            "Run `tovuk login` again and complete the browser login.",
            false,
        ));
    }
    if write_keychain_token(clean_token) {
        return Ok(());
    }
    write_token_file(&user_session_path(), clean_token)
}

fn read_token_file(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|source| source.trim().to_owned())
        .unwrap_or_default()
}

fn write_token_file(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| internal_error(error.to_string()))?;
        set_private_dir(parent);
    }
    fs::write(path, format!("{token}\n")).map_err(|error| internal_error(error.to_string()))?;
    set_private_file(path);
    Ok(())
}

fn user_session_path() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata).join("Tovuk").join(SESSION_FILE);
        }
    }
    env::var_os("XDG_CONFIG_HOME").map_or_else(
        || home_dir().join(".config").join("tovuk").join(SESSION_FILE),
        |path| PathBuf::from(path).join("tovuk").join(SESSION_FILE),
    )
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_keychain_token() -> String {
    if cfg!(target_os = "macos") {
        let result = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                SESSION_SERVICE,
                "-a",
                SESSION_ACCOUNT,
                "-w",
            ])
            .stderr(Stdio::null())
            .output();
        return result
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
    }

    if cfg!(target_os = "linux") && has_command("secret-tool") {
        let result = Command::new("secret-tool")
            .args([
                "lookup",
                "service",
                SESSION_SERVICE,
                "account",
                SESSION_ACCOUNT,
            ])
            .stderr(Stdio::null())
            .output();
        return result
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
    }

    String::new()
}

fn write_keychain_token(token: &str) -> bool {
    if cfg!(target_os = "macos") {
        return Command::new("security")
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
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
    }

    if cfg!(target_os = "linux") && has_command("secret-tool") {
        let mut child = match Command::new("secret-tool")
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_error) => return false,
        };
        if let Some(mut stdin) = child.stdin.take() {
            if stdin.write_all(token.as_bytes()).is_err() {
                return false;
            }
        }
        return child.wait().is_ok_and(|status| status.success());
    }

    false
}

#[cfg(unix)]
fn set_private_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ignore = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) {}

#[cfg(unix)]
fn set_private_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ignore = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) {}

#[derive(Clone, Debug, Serialize)]
struct TovukConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    kind: String,
    build: BuildConfig,
    run: RunConfig,
    frontend: FrontendConfig,
    backend: BackendConfig,
    resources: ResourceConfig,
}

#[derive(Clone, Debug, Serialize)]
struct BuildConfig {
    command: String,
    check: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RunConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    port: u16,
    health: String,
}

#[derive(Clone, Debug, Default, Serialize)]
struct FrontendConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct BackendConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ResourceConfig {
    memory: String,
    cpu: String,
    idle_timeout_minutes: u16,
}

fn parse_tovuk_toml(source: &str, project_dir: &Path) -> std::result::Result<TovukConfig, String> {
    let table = source
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())?;
    reject_unknown_root_keys(&table)?;
    let kind = get_string(&table, "kind")?.unwrap_or_else(|| "rust_backend".to_owned());
    if !PROJECT_KINDS.contains(&kind.as_str()) {
        return Err("kind must be fullstack, rust_backend, or static_frontend".to_owned());
    }
    let build_table = get_section(&table, "build")?;
    let run_table = get_section(&table, "run")?;
    let frontend_table = get_section(&table, "frontend")?;
    let backend_table = get_section(&table, "backend")?;
    let resources_table = get_section(&table, "resources")?;

    reject_unknown_section_keys(&build_table, "build", &["command", "check", "output"])?;
    reject_unknown_section_keys(&run_table, "run", &["command", "port", "health"])?;
    reject_unknown_section_keys(
        &frontend_table,
        "frontend",
        &["root", "check", "build", "output"],
    )?;
    reject_unknown_section_keys(
        &backend_table,
        "backend",
        &["root", "check", "build", "command", "port", "health"],
    )?;
    reject_unknown_section_keys(
        &resources_table,
        "resources",
        &["memory", "cpu", "idle_timeout_minutes"],
    )?;

    let build = BuildConfig {
        check: get_section_string(&build_table, "check")?
            .unwrap_or_else(|| default_check_command(&kind, project_dir)),
        command: get_section_string(&build_table, "command")?
            .unwrap_or_else(|| default_build_command(&kind, project_dir)),
        output: if kind == "static_frontend" {
            Some(get_section_string(&build_table, "output")?.unwrap_or_else(|| "dist".to_owned()))
        } else {
            get_section_string(&build_table, "output")?
        },
    };

    let frontend = if kind == "fullstack" {
        let root = get_section_string(&frontend_table, "root")?;
        let frontend_dir = root
            .as_deref()
            .map_or_else(|| project_dir.to_path_buf(), |root| project_dir.join(root));
        FrontendConfig {
            root,
            check: Some(
                get_section_string(&frontend_table, "check")?
                    .unwrap_or_else(|| frontend_check_command(&frontend_dir)),
            ),
            build: Some(
                get_section_string(&frontend_table, "build")?
                    .unwrap_or_else(|| frontend_build_command(&frontend_dir)),
            ),
            output: Some(
                get_section_string(&frontend_table, "output")?.unwrap_or_else(|| "dist".to_owned()),
            ),
        }
    } else {
        FrontendConfig::default()
    };

    let backend = if kind == "fullstack" {
        BackendConfig {
            root: get_section_string(&backend_table, "root")?,
            check: Some(
                get_section_string(&backend_table, "check")?
                    .unwrap_or_else(|| DEFAULT_RUST_CHECK_COMMAND.to_owned()),
            ),
            build: Some(
                get_section_string(&backend_table, "build")?
                    .unwrap_or_else(|| "cargo build --release".to_owned()),
            ),
            command: get_section_string(&backend_table, "command")?,
            port: Some(get_section_u16(&backend_table, "port")?.unwrap_or(3000)),
            health: Some(
                get_section_string(&backend_table, "health")?
                    .unwrap_or_else(|| "/api/healthz".to_owned()),
            ),
        }
    } else {
        BackendConfig::default()
    };

    Ok(TovukConfig {
        name: get_string(&table, "name")?,
        kind,
        build,
        run: RunConfig {
            command: get_section_string(&run_table, "command")?,
            port: get_section_u16(&run_table, "port")?.unwrap_or(3000),
            health: get_section_string(&run_table, "health")?
                .unwrap_or_else(|| "/healthz".to_owned()),
        },
        frontend,
        backend,
        resources: ResourceConfig {
            memory: get_section_string(&resources_table, "memory")?
                .unwrap_or_else(|| "512mb".to_owned()),
            cpu: get_section_string(&resources_table, "cpu")?.unwrap_or_else(|| "0.25".to_owned()),
            idle_timeout_minutes: get_section_u16(&resources_table, "idle_timeout_minutes")?
                .unwrap_or(15),
        },
    })
}

fn reject_unknown_root_keys(
    table: &toml::map::Map<String, toml::Value>,
) -> std::result::Result<(), String> {
    let allowed = [
        "name",
        "kind",
        "build",
        "run",
        "frontend",
        "backend",
        "resources",
    ];
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported root key {key}"));
        }
    }
    Ok(())
}

fn reject_unknown_section_keys(
    table: &toml::map::Map<String, toml::Value>,
    section: &str,
    allowed: &[&str],
) -> std::result::Result<(), String> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported [{section}] key {key}"));
        }
    }
    Ok(())
}

fn get_section(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<toml::map::Map<String, toml::Value>, String> {
    match table.get(key) {
        None => Ok(toml::map::Map::new()),
        Some(value) => value
            .as_table()
            .cloned()
            .ok_or_else(|| format!("[{key}] must be a table")),
    }
}

fn get_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<String>, String> {
    table.get(key).map_or(Ok(None), |value| {
        value
            .as_str()
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| format!("{key} must be a string"))
    })
}

fn get_section_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<String>, String> {
    get_string(table, key)
}

fn get_section_u16(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<u16>, String> {
    table.get(key).map_or(Ok(None), |value| {
        let integer = value
            .as_integer()
            .ok_or_else(|| format!("{key} must be a number"))?;
        u16::try_from(integer)
            .ok()
            .map(Some)
            .ok_or_else(|| format!("{key} must be between 0 and 65535"))
    })
}

fn default_check_command(kind: &str, project_dir: &Path) -> String {
    if kind == "static_frontend" {
        frontend_check_command(project_dir)
    } else {
        DEFAULT_RUST_CHECK_COMMAND.to_owned()
    }
}

fn default_build_command(kind: &str, project_dir: &Path) -> String {
    if kind == "static_frontend" {
        frontend_build_command(project_dir)
    } else {
        "cargo build --release".to_owned()
    }
}

fn validate_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let name = config.name.as_deref().unwrap_or_default();
    if !is_dns_safe_name(name) {
        return Err("name must be lowercase DNS-safe text up to 48 characters".to_owned());
    }
    if !PROJECT_KINDS.contains(&config.kind.as_str()) {
        return Err("kind must be fullstack, rust_backend, or static_frontend".to_owned());
    }
    if config.kind == "fullstack" {
        validate_fullstack_config(config)?;
    } else {
        validate_build_config(config)?;
        if config.kind == "static_frontend" {
            validate_output(config.build.output.as_deref(), "[build].output")?;
        } else {
            validate_rust_backend_config(config)?;
        }
    }
    Ok(())
}

fn validate_build_config(config: &TovukConfig) -> std::result::Result<(), String> {
    if config.build.command.trim().is_empty() {
        return Err("[build].command is required".to_owned());
    }
    if config.build.check.trim().is_empty() {
        return Err("[build].check is required".to_owned());
    }
    validate_check_command(&config.kind, &config.build.check)?;
    if config.kind == "rust_backend" {
        validate_rust_build_command(&config.build.command)?;
    }
    Ok(())
}

fn validate_rust_backend_config(config: &TovukConfig) -> std::result::Result<(), String> {
    if config.build.output.is_some() {
        return Err("[build].output is only valid for static_frontend".to_owned());
    }
    validate_rust_run_command(config.run.command.as_deref())?;
    validate_port(config.run.port, "[run].port")?;
    validate_health(&config.run.health, "[run].health")?;
    validate_resource_config(config)
}

fn validate_fullstack_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let backend_root = validate_root(config.backend.root.as_deref(), "[backend].root")?;
    let frontend_root = validate_root(config.frontend.root.as_deref(), "[frontend].root")?;
    if backend_root == frontend_root {
        return Err("[backend].root and [frontend].root must be different directories".to_owned());
    }
    validate_rust_check_command(require_config_command(
        config.backend.check.as_deref(),
        "[backend].check",
    )?)?;
    validate_rust_build_command(require_config_command(
        config.backend.build.as_deref(),
        "[backend].build",
    )?)?;
    validate_rust_run_command(config.backend.command.as_deref())?;
    validate_port(config.backend.port.unwrap_or(0), "[backend].port")?;
    validate_health(
        config.backend.health.as_deref().unwrap_or_default(),
        "[backend].health",
    )?;
    validate_frontend_check_command(require_config_command(
        config.frontend.check.as_deref(),
        "[frontend].check",
    )?)?;
    require_config_command(config.frontend.build.as_deref(), "[frontend].build")?;
    validate_output(config.frontend.output.as_deref(), "[frontend].output")?;
    validate_resource_config(config)
}

fn require_config_command<'a>(
    value: Option<&'a str>,
    field: &str,
) -> std::result::Result<&'a str, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} is required"))
}

fn validate_root<'a>(value: Option<&'a str>, field: &str) -> std::result::Result<&'a str, String> {
    let value = value
        .ok_or_else(|| format!("{field} must be a safe relative directory such as api or web"))?;
    if is_safe_relative_path(value) {
        Ok(value)
    } else {
        Err(format!(
            "{field} must be a safe relative directory such as api or web"
        ))
    }
}

fn validate_port(value: u16, field: &str) -> std::result::Result<(), String> {
    if value == 0 {
        Err(format!("{field} must be between 1 and 65535"))
    } else {
        Ok(())
    }
}

fn validate_health(value: &str, field: &str) -> std::result::Result<(), String> {
    if value.starts_with('/') {
        Ok(())
    } else {
        Err(format!("{field} must be an absolute path"))
    }
}

fn validate_output(value: Option<&str>, field: &str) -> std::result::Result<(), String> {
    if value.is_some_and(is_safe_relative_directory) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a safe relative directory like dist or ."
        ))
    }
}

fn validate_resource_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let memory_mib = memory_to_mib(&config.resources.memory)?;
    if !(128..=2048).contains(&memory_mib) {
        return Err(
            "[resources].memory must be between 128mb and 2gb; use the smallest working value"
                .to_owned(),
        );
    }
    let cpu_millis = cpu_to_millis(&config.resources.cpu)?;
    if !(50..=2000).contains(&cpu_millis) {
        return Err(
            "[resources].cpu must be between 0.05 and 2; use the smallest working value".to_owned(),
        );
    }
    if !(1..=60).contains(&config.resources.idle_timeout_minutes) {
        return Err("[resources].idle_timeout_minutes must be between 1 and 60".to_owned());
    }
    Ok(())
}

fn validate_check_command(kind: &str, command: &str) -> std::result::Result<(), String> {
    if kind == "static_frontend" {
        validate_frontend_check_command(command)
    } else {
        validate_rust_check_command(command)
    }
}

fn validate_rust_check_command(command: &str) -> std::result::Result<(), String> {
    let required = [
        "cargo fmt --all --check",
        "cargo check --locked --release --all-targets --all-features",
        "cargo test --locked --release --all-targets --all-features",
        "cargo clippy --locked --release --all-targets --all-features",
        "-D warnings",
    ];
    let lint_ok = RUST_STRICT_CLIPPY_DENY_LINTS
        .iter()
        .all(|lint| command.contains(&format!("-D {lint}")));
    if required.iter().all(|fragment| command.contains(fragment)) && lint_ok {
        Ok(())
    } else {
        Err("[build].check must run rustfmt, locked release-mode cargo check, locked release-mode tests, and strict Clippy resource lints".to_owned())
    }
}

fn validate_rust_build_command(command: &str) -> std::result::Result<(), String> {
    if uses_javascript_backend_runtime(command) {
        return Err(
            "Rust backend build commands cannot invoke JavaScript or TypeScript runtimes; use cargo build --release"
                .to_owned(),
        );
    }
    let tokens = command_tokens(command);
    if tokens
        .iter()
        .any(|token| command_name_from_token(token) == "cargo")
        && tokens.iter().any(|token| token == "build")
        && tokens.iter().any(|token| token == "--release")
    {
        Ok(())
    } else {
        Err("Rust backend build commands must run cargo build --release".to_owned())
    }
}

fn validate_rust_run_command(command: Option<&str>) -> std::result::Result<(), String> {
    let value = command.unwrap_or_default();
    if uses_javascript_backend_runtime(value) {
        return Err(
            "Rust backend runtime commands cannot invoke JavaScript or TypeScript runtimes; run ./target/release/<binary> instead"
                .to_owned(),
        );
    }
    if command_tokens(value)
        .iter()
        .any(|token| token.contains("target/release/"))
    {
        Ok(())
    } else {
        Err("Rust backend runtime commands must start a binary under ./target/release/".to_owned())
    }
}

fn validate_frontend_check_command(command: &str) -> std::result::Result<(), String> {
    if is_noop_command(command) {
        return Ok(());
    }
    if uses_javascript_linter(command) {
        return Err("[build].check must not run JavaScript-based lint or format tooling; use oxlint, biome, or deno lint".to_owned());
    }
    let tokens = command_tokens(command);
    if has_frontend_install_command(&tokens)
        && has_frontend_script_run(&tokens, "typecheck")
        && has_frontend_script_run(&tokens, "lint")
    {
        Ok(())
    } else {
        Err("[build].check must install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`".to_owned())
    }
}

fn uses_javascript_backend_runtime(command: &str) -> bool {
    command_tokens(command)
        .iter()
        .any(|token| JAVASCRIPT_BACKEND_RUNTIMES.contains(&command_name_from_token(token).as_str()))
}

fn is_noop_command(command: &str) -> bool {
    let command = command.trim();
    command == ":" || command == "true"
}

fn memory_to_mib(value: &str) -> std::result::Result<u32, String> {
    let clean = value.trim().to_ascii_lowercase();
    let amount = clean
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let unit = clean[amount.len()..].trim();
    let amount = amount
        .parse::<u32>()
        .map_err(|_error| "[resources].memory must look like 256mb, 512mb, or 1gb".to_owned())?;
    match unit {
        "mb" | "mib" => Ok(amount),
        "gb" | "gib" => Ok(amount * 1024),
        _ => Err("[resources].memory must look like 256mb, 512mb, or 1gb".to_owned()),
    }
}

fn cpu_to_millis(value: &str) -> std::result::Result<u32, String> {
    let clean = value.trim();
    if clean.is_empty()
        || clean
            .chars()
            .any(|character| !character.is_ascii_digit() && character != '.')
        || clean.matches('.').count() > 1
    {
        return Err("[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned());
    }
    let parsed = clean
        .parse::<f64>()
        .map_err(|_error| "[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned())?;
    Ok((parsed * 1000.0).round() as u32)
}

#[derive(Clone, Debug, Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
    message: String,
    agent_instruction: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DoctorReport {
    ok: bool,
    project: String,
    config: Option<TovukConfig>,
    checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Serialize)]
struct ProjectDoctorReport {
    relative: String,
    ok: bool,
    project: String,
    config: Option<TovukConfig>,
    checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceDoctorReport {
    ok: bool,
    workspace: String,
    projects: Vec<ProjectDoctorReport>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum DoctorReportKind {
    Project(DoctorReport),
    Workspace(WorkspaceDoctorReport),
}

impl DoctorReportKind {
    fn ok(&self) -> bool {
        match self {
            Self::Project(report) => report.ok,
            Self::Workspace(report) => report.ok,
        }
    }

    fn checks(&self) -> Vec<DoctorCheck> {
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

fn doctor_project(project_dir: &Path, json_output: bool) -> Result<()> {
    let report = run_doctor_workspace(project_dir);
    if json_output {
        let value =
            serde_json::to_value(&report).map_err(|error| internal_error(error.to_string()))?;
        print_json(&value)?;
        if report.ok() {
            return Ok(());
        }
        return Err(CliError {
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
        });
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

fn run_doctor_workspace(project_dir: &Path) -> DoctorReportKind {
    if project_dir.join("tovuk.toml").exists() {
        return DoctorReportKind::Project(run_doctor(project_dir));
    }
    let projects = discover_deploy_projects(project_dir).unwrap_or_default();
    if projects.is_empty() {
        return DoctorReportKind::Project(run_doctor(project_dir));
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

fn run_doctor(project_dir: &Path) -> DoctorReport {
    let config_result = read_config(project_dir);
    let mut checks = vec![config_result.check];
    let kind = config_result
        .config
        .as_ref()
        .map_or("rust_backend", |config| config.kind.as_str())
        .to_owned();

    if kind == "fullstack" {
        if let Some(config) = config_result.config.as_ref() {
            checks.extend(fullstack_checks(project_dir, config, config_result.valid));
        }
        return doctor_report(project_dir, config_result.config, checks);
    }

    checks.extend(required_file_checks(project_dir, &kind));
    if kind == "static_frontend" {
        checks.extend(static_frontend_checks(project_dir, config_result.valid));
        checks.push(unsafe_check(project_dir));
    } else {
        checks.push(backend_javascript_source_check(project_dir, ""));
        checks.extend(rust_doctor_checks(project_dir, config_result.valid));
    }
    doctor_report(project_dir, config_result.config, checks)
}

struct ConfigResult {
    check: DoctorCheck,
    config: Option<TovukConfig>,
    valid: bool,
}

fn read_config(project_dir: &Path) -> ConfigResult {
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

fn doctor_report(
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

fn print_doctor_report(report: &DoctorReportKind) {
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

fn print_checks(checks: &[DoctorCheck]) {
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

fn doctor_check(
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

fn required_file_checks(project_dir: &Path, kind: &str) -> Vec<DoctorCheck> {
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

fn required_files(project_dir: &Path, kind: &str) -> Vec<&'static str> {
    if kind == "static_frontend" {
        if is_plain_static_frontend(project_dir) {
            vec!["index.html"]
        } else {
            vec!["package.json"]
        }
    } else {
        vec!["Cargo.toml", "Cargo.lock"]
    }
}

fn fullstack_checks(
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

fn rust_doctor_checks(project_dir: &Path, config_valid: bool) -> Vec<DoctorCheck> {
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

fn first_output_line(stderr: &[u8], stdout: &[u8], fallback: &str) -> String {
    let text = if stderr.is_empty() { stdout } else { stderr };
    let value = String::from_utf8_lossy(text).trim().to_owned();
    let value = if value.is_empty() {
        format!("{fallback} failed")
    } else {
        value
    };
    value.chars().take(240).collect()
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
        if relative.ends_with(".rs")
            && fs::read_to_string(file)
                .map(|source| source.contains("unsafe"))
                .unwrap_or(false)
        {
            hits.push(relative.to_owned());
        }
    });
    hits
}

fn cargo_lints(project_dir: &Path) -> DoctorCheck {
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

fn cargo_lint_level(source: &str, lint_group: &str, lint_name: &str) -> String {
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

fn toml_section(line: &str) -> Option<String> {
    line.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::to_owned)
}

fn lint_assignment_level(line: &str, lint_name: &str) -> Option<String> {
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

fn static_frontend_checks(project_dir: &Path, run_scripts: bool) -> Vec<DoctorCheck> {
    if is_plain_static_frontend(project_dir) {
        return Vec::new();
    }
    let mut checks = vec![frontend_lockfile_check(project_dir)];
    checks.extend(frontend_source_checks(project_dir));
    checks.extend(frontend_script_checks(project_dir, run_scripts));
    checks
}

fn frontend_lockfile_check(project_dir: &Path) -> DoctorCheck {
    doctor_check(
        "frontend lockfile",
        frontend_lockfile_exists(project_dir),
        "found",
        "missing",
        "Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.",
    )
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
    .any(|file| project_dir.join(file).exists())
}

fn is_plain_static_frontend(project_dir: &Path) -> bool {
    !project_dir.join("package.json").exists() && project_dir.join("index.html").exists()
}

fn frontend_package_manager(project_dir: &Path) -> &'static str {
    if project_dir.join("bun.lock").exists() || project_dir.join("bun.lockb").exists() {
        "bun"
    } else {
        "npm"
    }
}

fn frontend_check_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND.to_owned()
    } else {
        DEFAULT_NPM_FRONTEND_CHECK_COMMAND.to_owned()
    }
}

fn frontend_build_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        "bun run build".to_owned()
    } else {
        "npm run build".to_owned()
    }
}

fn frontend_script_checks(project_dir: &Path, run_scripts: bool) -> Vec<DoctorCheck> {
    let manifest = read_package_json(project_dir);
    let typecheck = package_script_value(manifest.as_ref(), "typecheck");
    let lint = package_script_value(manifest.as_ref(), "lint");
    let mut checks = vec![
        package_script_exists_check("typecheck", &typecheck),
        package_script_exists_check("lint", &lint),
        strict_typecheck_check(&typecheck),
        native_lint_check(manifest.as_ref()),
        native_quality_gate_check(manifest.as_ref()),
    ];
    if run_scripts && checks.iter().all(|check| check.ok) {
        checks.push(package_script_check(project_dir, "typecheck"));
        checks.push(package_script_check(project_dir, "lint"));
    }
    checks
}

fn package_script_exists_check(script: &str, command: &str) -> DoctorCheck {
    doctor_check(
        &format!("package script {script}"),
        !command.is_empty(),
        "found",
        "missing",
        &format!("Add a non-empty \"{script}\" script to package.json, then retry."),
    )
}

fn strict_typecheck_check(command: &str) -> DoctorCheck {
    doctor_check(
        "strict frontend typecheck",
        uses_strict_frontend_typechecker(command),
        "accepted",
        "native typecheck missing",
        "Set package.json `typecheck` to `oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json`, then retry.",
    )
}

fn native_lint_check(manifest: Option<&Value>) -> DoctorCheck {
    let ok = !package_script_tree_uses(
        manifest,
        "lint",
        uses_javascript_linter,
        &mut BTreeSet::new(),
    ) && package_script_tree_uses(
        manifest,
        "lint",
        uses_native_frontend_linter,
        &mut BTreeSet::new(),
    );
    doctor_check(
        "native frontend lint",
        ok,
        "accepted",
        "native linter missing",
        "Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.",
    )
}

fn native_quality_gate_check(manifest: Option<&Value>) -> DoctorCheck {
    let ok = package_script_tree_uses(
        manifest,
        "lint",
        uses_native_dead_code_checker,
        &mut BTreeSet::new(),
    ) && package_script_tree_uses(
        manifest,
        "lint",
        uses_native_duplicate_checker,
        &mut BTreeSet::new(),
    ) && package_script_tree_uses(
        manifest,
        "lint",
        uses_native_health_checker,
        &mut BTreeSet::new(),
    );
    doctor_check(
        "native frontend quality gates",
        ok,
        "accepted",
        "dead-code, duplicate-code, or health gate missing",
        "Add Fallow checks for `dead-code`, semantic `dupes`, and `health` to package.json `lint`, then retry.",
    )
}

fn frontend_source_checks(project_dir: &Path) -> Vec<DoctorCheck> {
    let report = frontend_source_report(project_dir);
    vec![
        DoctorCheck {
            name: "typescript source".to_owned(),
            ok: !report.typescript.is_empty(),
            message: if report.typescript.is_empty() {
                "missing".to_owned()
            } else {
                report
                    .typescript
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            agent_instruction: if report.typescript.is_empty() {
                Some("Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry.".to_owned())
            } else {
                None
            },
        },
        forbidden_source_check(
            "javascript source",
            &report.javascript,
            "Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.",
        ),
        forbidden_source_check(
            "frontend server routes",
            &report.server_routes,
            "Move API routes, SSR handlers, middleware, and server logic to the Rust backend; static frontend source may only contain browser code.",
        ),
    ]
}

fn forbidden_source_check(name: &str, files: &[String], instruction: &str) -> DoctorCheck {
    DoctorCheck {
        name: name.to_owned(),
        ok: files.is_empty(),
        message: if files.is_empty() {
            "none found".to_owned()
        } else {
            files.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        },
        agent_instruction: if files.is_empty() {
            None
        } else {
            Some(instruction.to_owned())
        },
    }
}

#[derive(Default)]
struct FrontendSourceReport {
    typescript: Vec<String>,
    javascript: Vec<String>,
    server_routes: Vec<String>,
}

fn frontend_source_report(project_dir: &Path) -> FrontendSourceReport {
    let mut report = FrontendSourceReport::default();
    walk_project_files(project_dir, |_file, relative| {
        if is_frontend_server_route(relative) {
            report.server_routes.push(relative.to_owned());
        }
        if !is_frontend_source_path(relative) {
            return;
        }
        if is_frontend_typescript_source(relative) {
            report.typescript.push(relative.to_owned());
        } else if is_frontend_javascript_source(relative) {
            report.javascript.push(relative.to_owned());
        }
    });
    report
}

fn is_frontend_source_path(relative: &str) -> bool {
    relative
        .split('/')
        .next()
        .is_some_and(|root| FRONTEND_SOURCE_ROOTS.contains(&root))
}

fn is_frontend_typescript_source(relative: &str) -> bool {
    !relative.ends_with(".d.ts") && (relative.ends_with(".ts") || relative.ends_with(".tsx"))
}

fn is_frontend_javascript_source(relative: &str) -> bool {
    FRONTEND_JAVASCRIPT_EXTENSIONS
        .iter()
        .any(|extension| relative.ends_with(extension))
}

fn is_frontend_server_route(relative: &str) -> bool {
    if !is_frontend_typescript_source(relative) && !is_frontend_javascript_source(relative) {
        return false;
    }
    let parts = relative
        .to_ascii_lowercase()
        .split('/')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let file = parts.last().map_or("", String::as_str);
    file.starts_with("+server.")
        || file.starts_with("middleware.")
        || path_starts_with(&parts, &["pages", "api"])
        || path_starts_with(&parts, &["src", "pages", "api"])
        || (file.starts_with("route.")
            && (path_starts_with(&parts, &["app", "api"])
                || path_starts_with(&parts, &["src", "app", "api"])))
}

fn path_starts_with(path_parts: &[String], prefix: &[&str]) -> bool {
    path_parts.len() >= prefix.len()
        && prefix
            .iter()
            .enumerate()
            .all(|(index, part)| path_parts[index] == *part)
}

fn package_script_value(manifest: Option<&Value>, script: &str) -> String {
    manifest
        .and_then(|manifest| manifest.get("scripts"))
        .and_then(|scripts| scripts.get(script))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn uses_javascript_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        JAVASCRIPT_LINTERS.contains(&command_name.as_str())
            || (command_name == "next"
                && tokens.get(index + 1).is_some_and(|value| value == "lint"))
    })
}

fn uses_strict_frontend_typechecker(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        (command_name == "oxlint"
            && tokens.iter().any(|value| value == "--type-aware")
            && tokens.iter().any(|value| value == "--type-check"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|value| value == "check"))
    })
}

fn uses_native_frontend_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        command_name == "oxlint"
            || (command_name == "biome"
                && tokens
                    .get(index + 1)
                    .is_some_and(|value| value == "check" || value == "lint"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|value| value == "lint"))
    })
}

fn uses_native_dead_code_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "dead-code")
}

fn uses_native_duplicate_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "dupes")
}

fn uses_native_health_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "health")
}

fn uses_fallow_subcommand(command: &str, subcommand: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        command_name_from_token(token) == "fallow"
            && tokens
                .get(index + 1)
                .is_some_and(|value| value == subcommand)
    })
}

fn package_script_tree_uses(
    manifest: Option<&Value>,
    script: &str,
    predicate: fn(&str) -> bool,
    seen: &mut BTreeSet<String>,
) -> bool {
    if !seen.insert(script.to_owned()) {
        return false;
    }
    let command = package_script_value(manifest, script);
    if command.is_empty() {
        return false;
    }
    if predicate(&command) {
        return true;
    }
    referenced_package_scripts(&command)
        .iter()
        .any(|referenced| package_script_tree_uses(manifest, referenced, predicate, seen))
}

fn referenced_package_scripts(command: &str) -> Vec<String> {
    let tokens = command_tokens(command);
    let mut scripts = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !FRONTEND_PACKAGE_MANAGERS.contains(&command_name_from_token(token).as_str())
            || tokens.get(index + 1).is_none_or(|value| value != "run")
        {
            continue;
        }
        if let Some(script) = script_name_after_run(&tokens, index + 2) {
            scripts.push(script);
        }
    }
    scripts
}

fn script_name_after_run(tokens: &[String], start: usize) -> Option<String> {
    let mut index = start;
    while tokens
        .get(index)
        .is_some_and(|token| token.starts_with('-'))
    {
        index += 1;
    }
    tokens.get(index).cloned()
}

fn command_tokens(command: &str) -> Vec<String> {
    command
        .replace(['&', '|', ';', '(', ')'], " ")
        .split_whitespace()
        .map(|token| token.trim_matches(['"', '\'']).to_owned())
        .filter(|token| !token.is_empty())
        .collect()
}

fn command_name_from_token(token: &str) -> String {
    token.rsplit('/').next().unwrap_or_default().to_owned()
}

fn has_frontend_install_command(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        FRONTEND_INSTALL_COMMANDS.contains(
            &format!(
                "{} {}",
                command_name_from_token(token),
                tokens.get(index + 1).map_or("", String::as_str)
            )
            .as_str(),
        )
    })
}

fn has_frontend_script_run(tokens: &[String], script: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if !FRONTEND_PACKAGE_MANAGERS.contains(&command_name_from_token(token).as_str())
            || tokens.get(index + 1).is_none_or(|value| value != "run")
        {
            return false;
        }
        tokens.get(index + 2).is_some_and(|value| value == script)
            || (tokens
                .get(index + 2)
                .is_some_and(|value| value.starts_with('-'))
                && tokens.get(index + 3).is_some_and(|value| value == script))
    })
}

fn package_script_check(project_dir: &Path, script: &str) -> DoctorCheck {
    let manager = frontend_package_manager(project_dir);
    let args = if manager == "bun" {
        vec!["run", script]
    } else {
        vec!["run", "--silent", script]
    };
    let result = Command::new(manager)
        .args(args)
        .current_dir(project_dir)
        .stdin(Stdio::null())
        .output();
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            return DoctorCheck {
                name: format!("{manager} run {script}"),
                ok: false,
                message: error.to_string(),
                agent_instruction: Some(format!(
                    "Install {}, then run `{manager} run {script}` before deploying.",
                    if manager == "bun" {
                        "Bun"
                    } else {
                        "Node.js and npm"
                    }
                )),
            };
        }
    };
    DoctorCheck {
        name: format!("{manager} run {script}"),
        ok: output.status.success(),
        message: if output.status.success() {
            "passed".to_owned()
        } else {
            first_output_line(
                &output.stderr,
                &output.stdout,
                &format!("{manager} run {script}"),
            )
        },
        agent_instruction: if output.status.success() {
            None
        } else {
            Some(format!(
                "Run `{manager} run {script}`, fix every error, then redeploy."
            ))
        },
    }
}

fn backend_javascript_source_check(project_dir: &Path, label: &str) -> DoctorCheck {
    let mut matches = Vec::new();
    walk_project_files(project_dir, |_file, relative| {
        if is_backend_javascript_or_typescript_source(relative) {
            matches.push(if label.is_empty() {
                relative.to_owned()
            } else {
                format!("{label}/{relative}")
            });
        }
    });
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

fn is_backend_javascript_or_typescript_source(relative: &str) -> bool {
    if relative.ends_with(".d.ts") || !is_javascript_or_typescript_path(relative) {
        return false;
    }
    relative
        .split('/')
        .any(|component| ["api", "app", "pages", "routes", "server", "src"].contains(&component))
}

fn is_javascript_or_typescript_path(relative: &str) -> bool {
    [".cjs", ".js", ".jsx", ".mjs", ".ts", ".tsx"]
        .iter()
        .any(|extension| relative.ends_with(extension))
}

#[derive(Clone, Debug)]
struct DeployProjectInfo {
    dir: PathBuf,
    relative: String,
    name: String,
    kind: String,
}

#[derive(Clone, Debug)]
struct DeployPlanProject {
    project: DeployProjectInfo,
    wants_database: bool,
}

fn deploy(project_dir: &Path, cli: &CliOptions) -> Result<()> {
    let projects = discover_deploy_projects(project_dir)?;
    if projects.is_empty() {
        return Err(agent_error(
            "missing_project_contract",
            "No tovuk.toml was found.",
            "Run `tovuk init` in each app directory, or pass a project path.",
            cli.json,
        ));
    }
    let token = read_or_login_token(cli)?;
    let plan = create_deploy_plan(&projects, cli, &token)?;
    let mut results = deploy_projects(&plan, cli, &token)?;
    if cli.wait {
        wait_for_workspace_builds(cli, &token, &mut results)?;
    }
    if results.len() == 1 {
        print_deploy_result(&results[0].response, cli)
    } else {
        print_workspace_deploy_results(project_dir, &results, cli)
    }
}

struct WorkspaceDeployResult {
    project: DeployProjectInfo,
    wants_database: bool,
    response: Value,
    final_build: Option<Value>,
}

fn deploy_projects(
    plan: &[DeployPlanProject],
    cli: &CliOptions,
    token: &str,
) -> Result<Vec<WorkspaceDeployResult>> {
    let mut results = Vec::new();
    let workspace_deploy = plan.len() > 1;
    if workspace_deploy && !cli.json {
        println!("deploying {} projects", plan.len());
    }
    for item in plan {
        if workspace_deploy && !cli.json {
            println!("checking {}", item.project.relative);
        }
        let response = deploy_project(&item.project.dir, cli, token, item.wants_database)?;
        if workspace_deploy && !cli.json {
            println!(
                "{} queued {}",
                item.project.relative,
                nested_string(&response, &["build_job", "id"])
            );
            println!(
                "{} url {}",
                item.project.relative,
                nested_string(&response, &["app", "url"])
            );
        }
        results.push(WorkspaceDeployResult {
            project: item.project.clone(),
            wants_database: item.wants_database,
            response,
            final_build: None,
        });
    }
    Ok(results)
}

fn deploy_project(
    project_dir: &Path,
    cli: &CliOptions,
    token: &str,
    wants_database: bool,
) -> Result<Value> {
    let report = run_doctor(project_dir);
    if !report.ok {
        let instruction = report
            .checks
            .iter()
            .find(|check| !check.ok)
            .and_then(|check| check.agent_instruction.clone())
            .unwrap_or_else(|| "Fix the failed checks and retry.".to_owned());
        return Err(agent_error(
            "doctor_failed",
            "Tovuk doctor failed.",
            instruction,
            cli.json,
        ));
    }
    let body = json!({
        "config": report.config,
        "commit_sha": git_commit_sha(project_dir),
        "wants_database": wants_database,
        "source_archive_base64": create_archive_base64(project_dir, cli.json)?,
    });
    api_request(cli, Method::POST, "/v1/deploy", Some(token), Some(body))
}

fn print_deploy_result(response: &Value, cli: &CliOptions) -> Result<()> {
    if cli.json {
        return print_json(response);
    }
    println!("queued {}", nested_string(response, &["build_job", "id"]));
    println!("app {}", nested_string(response, &["app", "id"]));
    println!("url {}", nested_string(response, &["app", "url"]));
    println!(
        "next tovuk logs --app {}",
        nested_string(response, &["app", "id"])
    );
    Ok(())
}

fn print_workspace_deploy_results(
    project_dir: &Path,
    results: &[WorkspaceDeployResult],
    cli: &CliOptions,
) -> Result<()> {
    if cli.json {
        let deploys = results
            .iter()
            .map(|result| {
                json!({
                    "path": result.project.relative,
                    "kind": result.project.kind,
                    "wants_database": result.wants_database,
                    "app": result.response.get("app").cloned().unwrap_or(Value::Null),
                    "build_job": result.response.get("build_job").cloned().unwrap_or(Value::Null),
                    "final_build": result.final_build.clone().unwrap_or(Value::Null),
                })
            })
            .collect::<Vec<_>>();
        return print_json(
            &json!({ "workspace": project_dir.display().to_string(), "deploys": deploys }),
        );
    }
    if let Some(first) = results.first() {
        println!(
            "next tovuk logs --app {}",
            nested_string(&first.response, &["app", "id"])
        );
    }
    Ok(())
}

fn wait_for_workspace_builds(
    cli: &CliOptions,
    token: &str,
    results: &mut [WorkspaceDeployResult],
) -> Result<()> {
    for result in results {
        let build_id = nested_string(&result.response, &["build_job", "id"]);
        let final_build = wait_for_build(cli, token, &build_id)?;
        if let Some(object) = result.response.as_object_mut() {
            object.insert("final_build".to_owned(), final_build.clone());
        }
        result.final_build = Some(final_build);
    }
    Ok(())
}

fn wait_for_build(cli: &CliOptions, token: &str, build_id: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(cli.wait_timeout_seconds);
    let mut last_status = String::new();
    while Instant::now() <= deadline {
        let response = api_request(
            cli,
            Method::GET,
            &format!("/v1/builds/{}", encode_component(build_id)),
            Some(token),
            None,
        )?;
        let build = response.get("build").cloned().unwrap_or(Value::Null);
        let status = string_field(&build, "status");
        if status.is_empty() {
            return Err(agent_error(
                "build_status_unavailable",
                "Build status is unavailable.",
                format!("Retry with `tovuk logs --build {build_id}`."),
                cli.json,
            ));
        }
        if status != last_status {
            progress(
                cli,
                &format!("build {} {status}", string_field(&build, "id")),
            );
            last_status = status.clone();
        }
        if ["succeeded", "failed", "canceled"].contains(&status.as_str()) {
            return Ok(build);
        }
        thread::sleep(Duration::from_secs(3));
    }
    Err(agent_error(
        "build_wait_timeout",
        format!("Timed out waiting for build {build_id}."),
        format!("Run `tovuk logs --build {build_id}` to continue watching."),
        cli.json,
    ))
}

fn create_deploy_plan(
    projects: &[DeployProjectInfo],
    cli: &CliOptions,
    token: &str,
) -> Result<Vec<DeployPlanProject>> {
    let plan = projects
        .iter()
        .map(|project| DeployPlanProject {
            project: project.clone(),
            wants_database: cli.database
                && (project.kind == "rust_backend" || project.kind == "fullstack"),
        })
        .collect::<Vec<_>>();
    reject_invalid_database_targets(&plan, cli)?;
    preflight_deploy_limits(&plan, cli, token)?;
    Ok(plan)
}

fn reject_invalid_database_targets(plan: &[DeployPlanProject], cli: &CliOptions) -> Result<()> {
    if cli.database
        && plan.len() == 1
        && plan
            .first()
            .is_some_and(|item| item.project.kind == "static_frontend")
    {
        return Err(agent_error(
            "invalid_database_target",
            "Static frontends cannot attach managed Postgres directly.",
            "Deploy a Rust backend with managed Postgres and call it from the frontend.",
            cli.json,
        ));
    }
    Ok(())
}

fn preflight_deploy_limits(
    plan: &[DeployPlanProject],
    cli: &CliOptions,
    token: &str,
) -> Result<()> {
    let usage_response = api_request(cli, Method::GET, "/v1/usage", Some(token), None)?;
    let apps_response = api_request(cli, Method::GET, "/v1/apps", Some(token), None)?;
    let existing_apps = app_name_set(&apps_response);
    let requested = requested_new_resources(plan, &existing_apps);
    let usage = usage_response.get("usage").unwrap_or(&Value::Null);
    let limits = usage_response.get("limits").unwrap_or(&Value::Null);
    let used_projects = number_field(usage, "appCount");
    let project_limit = number_field(limits, "projects");
    let used_databases = number_field(usage, "databaseCount");
    let database_limit = number_field(limits, "managedDatabases");

    if requested.projects > 0 && used_projects + requested.projects > project_limit {
        return Err(payment_required_agent_error(
            cli,
            token,
            format!(
                "Project limit reached: {used_projects}/{project_limit} projects are already used."
            ),
            "Redeploy an existing app by reusing its `name` in tovuk.toml, or open the returned Stripe Checkout URL before creating another project.",
        ));
    }
    if requested.databases > 0 && used_databases + requested.databases > database_limit {
        return Err(payment_required_agent_error(
            cli,
            token,
            format!(
                "Managed Postgres limit reached: {used_databases}/{database_limit} databases are already used."
            ),
            "Redeploy an app that already has managed Postgres, deploy without `--database`, or open the returned Stripe Checkout URL.",
        ));
    }
    Ok(())
}

struct RequestedResources {
    projects: u64,
    databases: u64,
}

fn app_name_set(response: &Value) -> BTreeSet<String> {
    response
        .get("apps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|app| app.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn requested_new_resources(
    plan: &[DeployPlanProject],
    existing_apps: &BTreeSet<String>,
) -> RequestedResources {
    let mut projects = 0u64;
    let mut databases = 0u64;
    for target in plan {
        if target.project.name.is_empty() || target.project.kind == "unknown" {
            continue;
        }
        if !existing_apps.contains(&target.project.name) {
            projects += 1;
            if target.wants_database {
                databases += 1;
            }
        }
    }
    RequestedResources {
        projects,
        databases,
    }
}

fn discover_deploy_projects(root_dir: &Path) -> Result<Vec<DeployProjectInfo>> {
    ensure_directory(root_dir)?;
    if root_dir.join("tovuk.toml").exists() {
        return Ok(vec![deploy_project_info(root_dir, root_dir)]);
    }
    let mut project_dirs = Vec::new();
    discover_project_dirs(root_dir, &mut project_dirs);
    let mut projects = project_dirs
        .iter()
        .map(|dir| deploy_project_info(dir, root_dir))
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| {
        kind_order(&left.kind)
            .cmp(&kind_order(&right.kind))
            .then_with(|| left.relative.cmp(&right.relative))
    });
    Ok(projects)
}

fn discover_project_dirs(dir: &Path, project_dirs: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir()
            || entry
                .file_name()
                .to_str()
                .is_some_and(|name| WORKSPACE_EXCLUDED_DIRS.contains(&name))
        {
            continue;
        }
        if path.join("tovuk.toml").exists() {
            project_dirs.push(path);
        } else {
            discover_project_dirs(&path, project_dirs);
        }
    }
}

fn deploy_project_info(dir: &Path, root_dir: &Path) -> DeployProjectInfo {
    let relative = path_relative(dir, root_dir);
    let source = fs::read_to_string(dir.join("tovuk.toml")).unwrap_or_default();
    if let Ok(config) = parse_tovuk_toml(&source, dir) {
        return DeployProjectInfo {
            dir: dir.to_path_buf(),
            relative,
            name: config.name.unwrap_or_default(),
            kind: config.kind,
        };
    }
    DeployProjectInfo {
        dir: dir.to_path_buf(),
        relative,
        name: String::new(),
        kind: "unknown".to_owned(),
    }
}

fn kind_order(kind: &str) -> u8 {
    match kind {
        "rust_backend" => 0,
        "fullstack" => 1,
        "static_frontend" => 2,
        _ => 3,
    }
}

fn create_archive_base64(project_dir: &Path, json_output: bool) -> Result<String> {
    let mut archive = Vec::new();
    {
        let encoder = GzEncoder::new(&mut archive, Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for entry in WalkDir::new(project_dir)
            .into_iter()
            .filter_entry(|entry| !is_archive_excluded_entry(project_dir, entry))
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = match entry.path().strip_prefix(project_dir) {
                Ok(relative) => relative,
                Err(_error) => continue,
            };
            builder
                .append_path_with_name(entry.path(), Path::new(".").join(relative))
                .map_err(|error| {
                    agent_error(
                        "archive_failed",
                        "Could not create source archive.",
                        format!("Check project files and retry: {error}"),
                        json_output,
                    )
                })?;
        }
        let encoder = builder.into_inner().map_err(|error| {
            agent_error(
                "archive_failed",
                "Could not create source archive.",
                format!("Check project files and retry: {error}"),
                json_output,
            )
        })?;
        encoder.finish().map_err(|error| {
            agent_error(
                "archive_failed",
                "Could not create source archive.",
                format!("Check project files and retry: {error}"),
                json_output,
            )
        })?;
    }
    if archive.len() > ARCHIVE_LIMIT_BYTES {
        return Err(agent_error(
            "archive_too_large",
            "Source archive is too large.",
            "Remove build outputs, target directories, logs, and local caches before deploying.",
            json_output,
        ));
    }
    Ok(BASE64.encode(archive))
}

fn is_archive_excluded_entry(project_dir: &Path, entry: &DirEntry) -> bool {
    if entry.path() == project_dir {
        return false;
    }
    let relative = match entry.path().strip_prefix(project_dir) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_error) => return true,
    };
    is_archive_excluded(&relative, entry.file_type().is_dir())
}

fn is_archive_excluded(relative: &str, is_dir: bool) -> bool {
    let basename = relative.rsplit('/').next().unwrap_or(relative);
    if is_dir && WALK_EXCLUDED_DIRS.contains(&basename) {
        return true;
    }
    ARCHIVE_EXCLUDES.iter().any(|pattern| match *pattern {
        "*.pem" => basename.ends_with(".pem"),
        "*.key" => basename.ends_with(".key"),
        "*.p12" => basename.ends_with(".p12"),
        "*.pfx" => basename.ends_with(".pfx"),
        "*.tfstate" => basename.ends_with(".tfstate"),
        "*.tfstate.*" => basename.contains(".tfstate."),
        "*.sqlite" => basename.ends_with(".sqlite"),
        "*.sqlite3" => basename.ends_with(".sqlite3"),
        "*.db" => basename.ends_with(".db"),
        "*.log" => basename.ends_with(".log"),
        "._*" => basename.starts_with("._"),
        ".env.*" => basename.starts_with(".env."),
        pattern => relative == pattern || relative.starts_with(&format!("{pattern}/")),
    })
}

fn git_commit_sha(project_dir: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn init_project(project_dir: &Path, template: &str) -> Result<()> {
    if !template.is_empty() {
        fs::create_dir_all(project_dir).map_err(|error| internal_error(error.to_string()))?;
        create_template(project_dir, template)?;
        return Ok(());
    }
    ensure_directory(project_dir)?;
    let config_path = project_dir.join("tovuk.toml");
    if config_path.exists() {
        println!("tovuk.toml already exists");
        return Ok(());
    }
    let kind = infer_project_kind(project_dir);
    let source = init_config(project_dir, &kind)?;
    fs::write(&config_path, source).map_err(|error| internal_error(error.to_string()))?;
    println!(
        "created {}",
        path_relative(&config_path, &env::current_dir().unwrap_or_default())
    );
    println!("detected {kind}");
    Ok(())
}

fn install_project(project_dir: &Path, template: &str) -> Result<()> {
    init_project(project_dir, template)?;
    doctor_project(project_dir, false)
}

fn create_template(project_dir: &Path, template: &str) -> Result<()> {
    if !PROJECT_TEMPLATES.contains(&template) {
        return Err(agent_error(
            "invalid_template",
            "Tovuk template is unknown.",
            format!("Use one of: {}.", PROJECT_TEMPLATES.join(", ")),
            false,
        ));
    }
    match template {
        "rust-api" => {
            write_rust_api_template(project_dir, &service_name_from_dir(project_dir), true)?
        }
        "tanstack-static-frontend" => {
            write_frontend_template(
                project_dir,
                &service_name_from_dir(project_dir),
                "/api",
                true,
            )?;
        }
        "fullstack-rust-tanstack" => write_fullstack_template(project_dir)?,
        _ => {}
    }
    println!("created {template} template");
    Ok(())
}

fn write_fullstack_template(project_dir: &Path) -> Result<()> {
    write_rust_api_template(&project_dir.join("api"), "api", false)?;
    write_frontend_template(&project_dir.join("web"), "web", "/api", false)?;
    write_new_file(
        &project_dir.join("tovuk.toml"),
        &fullstack_config(project_dir, "api", "web", true),
    )
}

fn write_rust_api_template(project_dir: &Path, name: &str, include_config: bool) -> Result<()> {
    fs::create_dir_all(project_dir.join("src"))
        .map_err(|error| internal_error(error.to_string()))?;
    write_new_file(
        &project_dir.join("Cargo.toml"),
        &rust_template_cargo_toml(name),
    )?;
    write_new_file(
        &project_dir.join("Cargo.lock"),
        &rust_template_cargo_lock(name),
    )?;
    write_new_file(&project_dir.join("src/main.rs"), rust_api_source())?;
    if include_config {
        write_new_file(
            &project_dir.join("tovuk.toml"),
            &rust_backend_config(project_dir),
        )?;
    }
    Ok(())
}

fn write_frontend_template(
    project_dir: &Path,
    name: &str,
    api_base_url: &str,
    include_config: bool,
) -> Result<()> {
    fs::create_dir_all(project_dir.join("src"))
        .map_err(|error| internal_error(error.to_string()))?;
    write_new_file(
        &project_dir.join("package.json"),
        &frontend_package_json(name),
    )?;
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
        frontend_vite_env_source(),
    )?;
    write_new_file(
        &project_dir.join("src/main.tsx"),
        &frontend_source(api_base_url),
    )?;
    write_new_file(&project_dir.join("tsconfig.json"), &frontend_ts_config())?;
    write_new_file(
        &project_dir.join("vite.config.ts"),
        "import react from \"@vitejs/plugin-react\";\nimport { defineConfig } from \"vite\";\n\nexport default defineConfig({ plugins: [react()] });\n",
    )?;
    if include_config {
        write_new_file(
            &project_dir.join("tovuk.toml"),
            &frontend_config(project_dir, true),
        )?;
    }
    println!(
        "run package install in the frontend directory before doctor: bun install or npm install"
    );
    Ok(())
}

fn write_new_file(path: &Path, source: &str) -> Result<()> {
    if path.exists() {
        return Err(agent_error(
            "file_exists",
            format!("Refusing to overwrite {}.", path.display()),
            "Move the existing file or choose an empty directory, then retry.",
            false,
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| internal_error(error.to_string()))?;
    }
    fs::write(path, source).map_err(|error| internal_error(error.to_string()))
}

fn init_config(project_dir: &Path, kind: &str) -> Result<String> {
    if kind == "fullstack" {
        if let Some((backend, frontend)) = detect_fullstack_roots(project_dir) {
            return Ok(fullstack_config(project_dir, &backend, &frontend, false));
        }
        return Err(agent_error(
            "fullstack_roots_missing",
            "Could not find fullstack roots.",
            "Create api/Cargo.toml and web/package.json or web/index.html, then retry.",
            false,
        ));
    }
    if kind == "static_frontend" {
        Ok(frontend_config(project_dir, false))
    } else {
        Ok(rust_backend_config(project_dir))
    }
}

fn rust_backend_config(project_dir: &Path) -> String {
    let name = service_name_from_cargo(project_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    format!(
        "name = \"{name}\"\n\n[build]\ncheck = \"{DEFAULT_RUST_CHECK_COMMAND}\"\ncommand = \"cargo build --release\"\n\n[run]\ncommand = \"./target/release/{name}\"\nport = 3000\nhealth = \"/healthz\"\n\n[resources]\nmemory = \"512mb\"\ncpu = \"0.25\"\nidle_timeout_minutes = 15\n"
    )
}

fn frontend_config(project_dir: &Path, prefer_bun: bool) -> String {
    let name = service_name_from_package(project_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    let settings = frontend_build_settings(project_dir, prefer_bun);
    format!(
        "name = \"{name}\"\nkind = \"static_frontend\"\n\n[build]\ncheck = \"{}\"\ncommand = \"{}\"\noutput = \"{}\"\n",
        settings.check, settings.build, settings.output
    )
}

fn fullstack_config(project_dir: &Path, backend: &str, frontend: &str, prefer_bun: bool) -> String {
    let name = service_name_from_dir(project_dir);
    let backend_dir = project_dir.join(backend);
    let frontend_dir = project_dir.join(frontend);
    let backend_name = service_name_from_cargo(&backend_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(&backend_dir));
    let settings = frontend_build_settings(&frontend_dir, prefer_bun);
    format!(
        "name = \"{name}\"\nkind = \"fullstack\"\n\n[backend]\nroot = \"{backend}\"\ncheck = \"{DEFAULT_RUST_CHECK_COMMAND}\"\nbuild = \"cargo build --release\"\ncommand = \"./target/release/{backend_name}\"\nport = 3000\nhealth = \"/api/healthz\"\n\n[frontend]\nroot = \"{frontend}\"\ncheck = \"{}\"\nbuild = \"{}\"\noutput = \"{}\"\n\n[resources]\nmemory = \"512mb\"\ncpu = \"0.25\"\nidle_timeout_minutes = 15\n",
        settings.check, settings.build, settings.output
    )
}

struct FrontendBuildSettings {
    check: String,
    build: String,
    output: String,
}

fn frontend_build_settings(project_dir: &Path, prefer_bun: bool) -> FrontendBuildSettings {
    let output = if is_plain_static_frontend(project_dir) {
        "."
    } else {
        "dist"
    };
    let check = if prefer_bun && output != "." {
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND.to_owned()
    } else {
        frontend_check_command(project_dir)
    };
    let build = if prefer_bun && output != "." {
        "bun run build".to_owned()
    } else {
        frontend_build_command(project_dir)
    };
    FrontendBuildSettings {
        check,
        build,
        output: output.to_owned(),
    }
}

fn preview_project(project_dir: &Path, port: u16) -> Result<()> {
    let config = preview_config(project_dir)?;
    preview_validated_project(project_dir, &config, port)
}

fn preview_config(project_dir: &Path) -> Result<TovukConfig> {
    let report = run_doctor_workspace(project_dir);
    if matches!(report, DoctorReportKind::Workspace(_)) {
        return Err(agent_error(
            "workspace_preview_unsupported",
            "Preview one project at a time.",
            "Run `tovuk preview <project-dir>` for one discovered project, or use a fullstack root tovuk.toml.",
            false,
        ));
    }
    if !report.ok() {
        let instruction = report
            .checks()
            .iter()
            .find(|check| !check.ok)
            .and_then(|check| check.agent_instruction.clone())
            .unwrap_or_else(|| "Fix the failed checks and retry `tovuk preview`.".to_owned());
        return Err(agent_error(
            "doctor_failed",
            "Tovuk doctor failed.",
            instruction,
            false,
        ));
    }
    let source = fs::read_to_string(project_dir.join("tovuk.toml"))
        .map_err(|error| internal_error(error.to_string()))?;
    let config = parse_tovuk_toml(&source, project_dir).map_err(internal_error)?;
    validate_config(&config).map_err(internal_error)?;
    Ok(config)
}

fn preview_validated_project(project_dir: &Path, config: &TovukConfig, port: u16) -> Result<()> {
    if config.kind == "fullstack" {
        return preview_fullstack(project_dir, config, port);
    }
    run_shell(
        &config.build.command,
        project_dir,
        "Build failed before preview.",
    )?;
    if config.kind == "static_frontend" {
        return preview_static(
            project_dir,
            config.build.output.as_deref().unwrap_or("dist"),
            port,
        );
    }
    preview_runtime(
        project_dir,
        config.run.command.as_deref().unwrap_or_default(),
        if port == 0 { config.run.port } else { port },
    )
}

fn preview_fullstack(project_dir: &Path, config: &TovukConfig, port: u16) -> Result<()> {
    let backend_dir = project_dir.join(config.backend.root.as_deref().unwrap_or_default());
    let frontend_dir = project_dir.join(config.frontend.root.as_deref().unwrap_or_default());
    let backend_port = config.backend.port.unwrap_or(3000);
    run_shell(
        config.backend.build.as_deref().unwrap_or_default(),
        &backend_dir,
        "Backend build failed before preview.",
    )?;
    run_shell(
        config.frontend.build.as_deref().unwrap_or_default(),
        &frontend_dir,
        "Frontend build failed before preview.",
    )?;
    let mut backend = shell_command(config.backend.command.as_deref().unwrap_or_default())
        .current_dir(&backend_dir)
        .env("PORT", backend_port.to_string())
        .spawn()
        .map_err(|error| {
            agent_error(
                "preview_failed",
                "Backend preview command failed.",
                error.to_string(),
                false,
            )
        })?;
    let result = serve_static(
        &frontend_dir.join(config.frontend.output.as_deref().unwrap_or("dist")),
        if port == 0 { 4173 } else { port },
        Some(backend_port),
    );
    let _ignore = backend.kill();
    result
}

fn preview_static(project_dir: &Path, output: &str, port: u16) -> Result<()> {
    serve_static(
        &project_dir.join(output),
        if port == 0 { 4173 } else { port },
        None,
    )
}

fn preview_runtime(project_dir: &Path, command: &str, port: u16) -> Result<()> {
    println!("preview http://127.0.0.1:{port}");
    let status = shell_command(command)
        .current_dir(project_dir)
        .env("PORT", port.to_string())
        .status()
        .map_err(|error| {
            agent_error(
                "preview_failed",
                "Preview command failed.",
                error.to_string(),
                false,
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(agent_error(
            "preview_failed",
            "Preview command exited with an error.",
            "Fix the local runtime command and retry `tovuk preview`.",
            false,
        ))
    }
}

fn run_shell(command: &str, project_dir: &Path, failure_message: &str) -> Result<()> {
    println!("{command}");
    let status = shell_command(command)
        .current_dir(project_dir)
        .status()
        .map_err(|error| {
            agent_error("command_failed", failure_message, error.to_string(), false)
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(agent_error(
            "command_failed",
            failure_message,
            "Fix the command output above, then retry.",
            false,
        ))
    }
}

fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut process = Command::new("cmd");
        process.args(["/C", command]);
        process
    } else {
        let mut process = Command::new("sh");
        process.args(["-c", command]);
        process
    }
}

fn serve_static(root: &Path, port: u16, api_proxy_port: Option<u16>) -> Result<()> {
    ensure_directory(root)?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
        agent_error(
            "preview_failed",
            "Could not start preview server.",
            error.to_string(),
            false,
        )
    })?;
    println!("preview http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        let stream = stream.map_err(|error| internal_error(error.to_string()))?;
        handle_static_request(stream, root, port, api_proxy_port)?;
    }
    Ok(())
}

fn handle_static_request(
    mut stream: TcpStream,
    root: &Path,
    port: u16,
    api_proxy_port: Option<u16>,
) -> Result<()> {
    let mut buffer = [0_u8; 8192];
    let size = stream
        .read(&mut buffer)
        .map_err(|error| internal_error(error.to_string()))?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut request_line = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = request_line.next().unwrap_or("GET");
    let path = request_line.next().unwrap_or("/");
    let pathname = path.split('?').next().unwrap_or("/");
    if api_proxy_port.is_some_and(|_| pathname == "/api" || pathname.starts_with("/api/")) {
        return proxy_to_backend(&mut stream, method, path, api_proxy_port.unwrap_or(port));
    }
    let target = static_target(root, pathname);
    if target.as_os_str().is_empty() {
        write_http_response(
            &mut stream,
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            b"not found",
        )?;
        return Ok(());
    }
    let body = fs::read(&target).map_err(|error| internal_error(error.to_string()))?;
    write_http_response(&mut stream, StatusCode::OK, content_type(&target), &body)
}

fn proxy_to_backend(stream: &mut TcpStream, method: &str, path: &str, port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{port}{path}");
    let method = Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET);
    let response = Client::new().request(method, url).send();
    match response {
        Ok(response) => {
            let status = response.status();
            let body = response
                .bytes()
                .map_err(|error| internal_error(error.to_string()))?;
            write_http_response(stream, status, "application/octet-stream", &body)
        }
        Err(_error) => write_http_response(
            stream,
            StatusCode::BAD_GATEWAY,
            "text/plain; charset=utf-8",
            b"backend unavailable",
        ),
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: StatusCode,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {} {}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("OK"),
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .and_then(|()| stream.write_all(body))
        .map_err(|error| internal_error(error.to_string()))
}

fn static_target(root: &Path, pathname: &str) -> PathBuf {
    let safe_path = pathname.trim_start_matches('/');
    let candidate = normalize_path(&root.join(if safe_path.is_empty() {
        "index.html"
    } else {
        safe_path
    }));
    let root = normalize_path(root);
    if !candidate.starts_with(&root) {
        return PathBuf::new();
    }
    if candidate.is_file() {
        return candidate;
    }
    let index = root.join("index.html");
    if index.is_file() {
        index
    } else {
        PathBuf::new()
    }
}

fn content_type(file: &Path) -> &'static str {
    match file
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}

fn ensure_directory(dir: &Path) -> Result<()> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(agent_error(
            "missing_project",
            "Project directory does not exist.",
            "Run Tovuk from the root of a Rust project or pass the project path.",
            false,
        ))
    }
}

fn walk_project_files(mut project_dir: &Path, mut visit: impl FnMut(&Path, &str)) {
    project_dir = project_dir.as_ref();
    for entry in WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|entry| !is_walk_excluded_entry(entry))
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(relative) = entry.path().strip_prefix(project_dir) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            visit(entry.path(), &relative);
        }
    }
}

fn is_walk_excluded_entry(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| entry.depth() > 0 && WALK_EXCLUDED_DIRS.contains(&name))
}

fn has_command(command: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            let candidate = directory.join(command);
            if cfg!(windows) {
                candidate.is_file() || directory.join(format!("{command}.exe")).is_file()
            } else {
                candidate.is_file()
            }
        })
    })
}

fn is_safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn is_safe_relative_directory(value: &str) -> bool {
    value == "." || is_safe_relative_path(value)
}

fn read_package_json(project_dir: &Path) -> Option<Value> {
    let source = fs::read_to_string(project_dir.join("package.json")).ok()?;
    serde_json::from_str(&source).ok()
}

fn service_name_from_dir(project_dir: &Path) -> String {
    service_name_from_value(
        project_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
    )
    .unwrap_or_else(|| "api".to_owned())
}

fn service_name_from_cargo(project_dir: &Path) -> Option<String> {
    let source = fs::read_to_string(project_dir.join("Cargo.toml")).ok()?;
    let name = source.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with("name") {
            return None;
        }
        line.split('"').nth(1).map(str::to_owned)
    })?;
    service_name_from_value(&name)
}

fn service_name_from_package(project_dir: &Path) -> Option<String> {
    let manifest = read_package_json(project_dir)?;
    let name = manifest.get("name")?.as_str()?;
    service_name_from_value(name)
}

fn service_name_from_value(value: &str) -> Option<String> {
    let mut result = String::new();
    let mut last_dash = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
        if result.len() >= 48 {
            break;
        }
    }
    let result = result.trim_matches('-').to_owned();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn infer_project_kind(project_dir: &Path) -> String {
    if detect_fullstack_roots(project_dir).is_some() {
        "fullstack".to_owned()
    } else if project_dir.join("Cargo.toml").exists() {
        "rust_backend".to_owned()
    } else if project_dir.join("package.json").exists() || project_dir.join("index.html").exists() {
        "static_frontend".to_owned()
    } else {
        "rust_backend".to_owned()
    }
}

fn detect_fullstack_roots(project_dir: &Path) -> Option<(String, String)> {
    let backend = ["api", "backend", "server"]
        .iter()
        .find(|root| project_dir.join(root).join("Cargo.toml").exists())?;
    let frontend = ["web", "frontend", "app", "site"].iter().find(|root| {
        project_dir.join(root).join("package.json").exists()
            || project_dir.join(root).join("index.html").exists()
    })?;
    Some(((*backend).to_owned(), (*frontend).to_owned()))
}

fn is_dns_safe_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 48 {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn path_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map_or_else(
            || ".".to_owned(),
            |relative| relative.to_string_lossy().replace('\\', "/"),
        )
}

fn open_url(url: &str) {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    let _ignore = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn progress(cli: &CliOptions, message: &str) {
    if cli.json {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}

fn encode_component(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number_field(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_else(|| {
        value
            .get(key)
            .and_then(Value::as_i64)
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(0)
    })
}

fn string_alias(value: &Value, aliases: &[&str]) -> String {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

fn number_alias(value: &Value, aliases: &[&str]) -> Option<u64> {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_u64))
}

fn nested_string(value: &Value, path: &[&str]) -> String {
    let mut cursor = value;
    for part in path {
        cursor = cursor.get(part).unwrap_or(&Value::Null);
    }
    cursor.as_str().unwrap_or_default().to_owned()
}

fn rust_template_cargo_toml(name: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lints.rust]\nunsafe_code = \"forbid\"\nwarnings = \"deny\"\n\n[lints.clippy]\nall = {{ level = \"deny\", priority = -1 }}\npedantic = {{ level = \"deny\", priority = -1 }}\ndbg_macro = \"deny\"\ntodo = \"deny\"\nunimplemented = \"deny\"\npanic = \"deny\"\nunwrap_used = \"deny\"\nexpect_used = \"deny\"\nlarge_futures = \"deny\"\nlarge_include_file = \"deny\"\nlarge_stack_frames = \"deny\"\nmem_forget = \"deny\"\nrc_buffer = \"deny\"\nrc_mutex = \"deny\"\nredundant_clone = \"deny\"\nclone_on_ref_ptr = \"deny\"\n"
    )
}

fn rust_template_cargo_lock(name: &str) -> String {
    format!(
        "# This file is automatically @generated by Cargo.\nversion = 4\n\n[[package]]\nname = \"{name}\"\nversion = \"0.1.0\"\n"
    )
}

fn frontend_package_json(name: &str) -> String {
    serde_json::to_string_pretty(&json!({
        "name": name,
        "private": true,
        "type": "module",
        "scripts": {
            "typecheck": "oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json",
            "lint": "oxlint src vite.config.ts --deny-warnings && fallow dead-code --production --include-dupes --include-entry-exports --fail-on-issues && fallow dupes --production --mode semantic --threshold 1 --ignore-imports --fail-on-issues && fallow health --production --max-cyclomatic 10 --max-cognitive 15 --max-crap 20 --complexity",
            "build": "vite build",
            "preview": "vite preview --host 0.0.0.0"
        },
        "dependencies": {
            "@tanstack/react-router": "^1.170.8",
            "react": "^19.2.6",
            "react-dom": "^19.2.6"
        },
        "devDependencies": {
            "@types/node": "^25.9.1",
            "@types/react": "^19.2.15",
            "@types/react-dom": "^19.2.3",
            "@vitejs/plugin-react": "^6.0.2",
            "fallow": "^2.84.0",
            "oxlint": "^1.67.0",
            "oxlint-tsgolint": "^0.23.0",
            "vite": "^8.0.14"
        }
    }))
    .map(|source| format!("{source}\n"))
    .unwrap_or_else(|_error| "{}\n".to_owned())
}

fn frontend_ts_config() -> String {
    serde_json::to_string_pretty(&json!({
        "compilerOptions": {
            "allowUnreachableCode": false,
            "allowUnusedLabels": false,
            "alwaysStrict": true,
            "erasableSyntaxOnly": true,
            "exactOptionalPropertyTypes": true,
            "forceConsistentCasingInFileNames": true,
            "isolatedModules": true,
            "jsx": "react-jsx",
            "lib": ["ESNext", "DOM"],
            "module": "ESNext",
            "moduleDetection": "force",
            "moduleResolution": "Bundler",
            "noEmit": true,
            "noFallthroughCasesInSwitch": true,
            "noImplicitAny": true,
            "noImplicitOverride": true,
            "noImplicitReturns": true,
            "noImplicitThis": true,
            "noPropertyAccessFromIndexSignature": true,
            "noUncheckedIndexedAccess": true,
            "noUncheckedSideEffectImports": true,
            "noUnusedLocals": true,
            "noUnusedParameters": true,
            "skipLibCheck": false,
            "strict": true,
            "strictBindCallApply": true,
            "strictFunctionTypes": true,
            "strictNullChecks": true,
            "strictPropertyInitialization": true,
            "target": "ES2022",
            "types": ["vite/client", "node"],
            "useUnknownInCatchVariables": true,
            "verbatimModuleSyntax": true
        },
        "include": ["src", "vite.config.ts"]
    }))
    .map(|source| format!("{source}\n"))
    .unwrap_or_else(|_error| "{}\n".to_owned())
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

    let body = if path == "/healthz" || path == "/api/healthz" {
        r#"{"ok":true}"#
    } else {
        r#"{"message":"hello from tovuk","backend":"rust"}"#
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
    format!(
        "import {{ createRootRoute, createRouter, RouterProvider }} from '@tanstack/react-router'\nimport {{ createRoot }} from 'react-dom/client'\nimport './styles.css'\n\nconst apiBaseUrl = import.meta.env.VITE_API_URL ?? '{api_base_url}'\n\nfunction App() {{\n  return (\n    <main>\n      <section>\n        <h1>Tovuk TanStack Frontend</h1>\n        <p>Static runtime, dynamic Rust backend calls.</p>\n        <code>{{apiBaseUrl}}</code>\n      </section>\n    </main>\n  )\n}}\n\nconst rootRoute = createRootRoute({{ component: App }})\nconst router = createRouter({{ routeTree: rootRoute }})\n\ndeclare module '@tanstack/react-router' {{\n  interface Register {{\n    router: typeof router\n  }}\n}}\n\nconst rootElement = document.getElementById('root')\nif (rootElement === null) {{\n  throw new Error('missing root element')\n}}\n\ncreateRoot(rootElement).render(<RouterProvider router={{router}} />)\n"
    )
}

fn frontend_vite_env_source() -> &'static str {
    "/// <reference types=\"vite/client\" />\n\ninterface ViteTypeOptions {\n  strictImportMetaEnv: unknown\n}\n\ninterface ImportMetaEnv {\n  readonly VITE_API_URL?: string\n}\n"
}

fn help_text() -> String {
    format!(
        r#"Tovuk {VERSION}

Usage:
  tovuk init [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  tovuk install [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  tovuk doctor [path] [--json]
  tovuk preview [path] [--port <port>]
  tovuk login [--token <token>] [--api <url>]
  tovuk deploy [path] [--database] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
  tovuk capabilities [--api <url>] [--json]
  tovuk me [--api <url>] [--json]
  tovuk usage [--api <url>] [--json]
  tovuk activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk apps [--api <url>] [--json]
  tovuk overview --app <app> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk deploys [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk builds [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk logs --app <app> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk status --app <app> [--api <url>] [--json]
  tovuk inspect --app <app> [--api <url>] [--json]
  tovuk db --app <app> [--api <url>] [--json]
  tovuk env list --app <app> [--api <url>] [--json]
  tovuk env set --app <app> KEY=value [--api <url>] [--json]
  tovuk env delete --app <app> KEY [--api <url>] [--json]
  tovuk domains list --app <app> [--api <url>] [--json]
  tovuk domains add --app <app> <domain> [--api <url>] [--json]
  tovuk domains verify --app <app> <domain> [--api <url>] [--json]
  tovuk domains delete --app <app> <domain> [--api <url>] [--json]
  tovuk billing [checkout|portal] [reason] [--api <url>] [--json]
  tovuk support list [--limit <n>] [--api <url>] [--json]
  tovuk support create "Subject" "Details" [--app <app>] [--build <build_id>] [--deploy <deploy_id>] [--failing-command <command>] [--first-log-line <line>] [--severity low|normal|urgent] [--api <url>] [--json]
  tovuk support resolve <ticket_id> [--api <url>] [--json]

Agent contract:
  - Fullstack apps set kind = "fullstack", keep backend and frontend roots in one tovuk.toml, serve the frontend at /, and serve the Rust API under /api.
  - Rust backends keep Cargo.lock committed, pass rustfmt plus locked release-mode check/test/Clippy gates, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep TypeScript source, a package lockfile, stable native typecheck, native lint, and Fallow quality gates.
  - Plain static HTML/CSS/JS frontends may use kind = "static_frontend" with check = ":", command = ":", and output = ".".
  - JavaScript and TypeScript are frontend-only on Tovuk; backend build and runtime commands must be Cargo release builds and Rust release binaries.
  - Frontends call Rust backends for APIs, managed Postgres, and server-side logic.
  - Run deploy from a fullstack repo root with one tovuk.toml to build backend and frontend together.
  - When split frontend and backend apps use different hostnames, configure backend CORS or use a same-origin custom domain.
  - When a plan limit blocks work, run tovuk billing checkout --json and show the returned URL to the human.
  - Create support tickets only with command output, app id, build id, deploy id, and the first actionable log line.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - Keep direct unsafe out of Rust source.
  - Keep Rust backend resources small: 128mb-2gb memory, 0.05-2 CPU, and 1-60 minute idle timeout.
"#
    )
}
