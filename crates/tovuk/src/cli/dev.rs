use super::{
    args::CliOptions,
    config::{TovukConfig, parse_tovuk_toml, validate_config},
    dev_ports::{DevPortOwner, port_owner},
    errors::{Result, agent_error, print_json},
    frontend_checks::{frontend_package_manager, is_next_frontend},
    project_kind::ProjectKind,
};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

const DEFAULT_FRONTEND_PORT: u16 = 5173;
const LOCAL_HOST: &str = "127.0.0.1";

#[derive(Clone, Copy, Debug, Default)]
struct DevPorts {
    frontend: Option<u16>,
    worker: Option<u16>,
}

#[derive(Clone, Debug, Serialize)]
struct DevPlan {
    frontend_url: Option<String>,
    kind: &'static str,
    next_actions: Vec<String>,
    port_statuses: Vec<DevPortStatus>,
    processes: Vec<DevProcess>,
    project: String,
    worker_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DevProcess {
    command: String,
    cwd: String,
    env: BTreeMap<String, String>,
    name: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct DevPortStatus {
    agent_instruction: Option<String>,
    available: bool,
    host: &'static str,
    name: &'static str,
    owner: Option<DevPortOwner>,
    port: u16,
    url: String,
}

pub(crate) fn dev(project_dir: &Path, cli: &CliOptions) -> Result<()> {
    let config = read_dev_config(project_dir, cli)?;
    let ports = dev_ports(cli, &config)?;
    let plan = create_dev_plan(project_dir, &config, ports);
    if cli.output.json {
        return print_json(&json!({
            "ok": plan.ports_available(),
            "mode": "plan",
            "dev": plan,
            "agent_instruction": dev_agent_instruction(&plan)
        }));
    }
    print_dev_plan(&plan);
    flush_stdout();
    fail_on_port_conflicts(&plan, cli)?;
    run_dev_processes(&plan, cli)
}

fn read_dev_config(project_dir: &Path, cli: &CliOptions) -> Result<TovukConfig> {
    let source = fs::read_to_string(project_dir.join("tovuk.toml")).map_err(|error| {
        agent_error(
            "missing_project_contract",
            format!("Could not read tovuk.toml: {error}"),
            "Run `tovuk dev` from a Tovuk project root or pass the project path.",
            cli.output.json,
        )
    })?;
    let config = parse_tovuk_toml(&source, project_dir).and_then(|config| {
        validate_config(&config)?;
        Ok(config)
    });
    config.map_err(|message| {
        agent_error(
            "invalid_project_contract",
            format!("Invalid tovuk.toml: {message}"),
            "Fix tovuk.toml, then rerun `tovuk dev`.",
            cli.output.json,
        )
    })
}

fn create_dev_plan(project_dir: &Path, config: &TovukConfig, ports: DevPorts) -> DevPlan {
    let mut processes = Vec::new();
    let mut port_statuses = Vec::new();
    let mut worker_url = None;
    let mut frontend_url = None;

    if matches!(
        config.kind,
        ProjectKind::RustWorker | ProjectKind::Fullstack
    ) {
        let worker_root = config
            .backend
            .root
            .as_deref()
            .map_or_else(|| project_dir.to_path_buf(), |root| project_dir.join(root));
        let worker_port = ports
            .worker
            .unwrap_or_else(|| config.backend.port.unwrap_or(config.run.port));
        let command = local_worker_command(&worker_root, config);
        let url = format!("http://{LOCAL_HOST}:{worker_port}");
        worker_url = Some(url.clone());
        port_statuses.push(dev_port_status("worker", worker_port, &url));
        processes.push(DevProcess {
            command,
            cwd: display_path(&worker_root),
            env: BTreeMap::from([("PORT".to_owned(), worker_port.to_string())]),
            name: "worker",
        });
    }

    if matches!(
        config.kind,
        ProjectKind::StaticFrontend | ProjectKind::Fullstack
    ) {
        let frontend_root = config
            .frontend
            .root
            .as_deref()
            .map_or_else(|| project_dir.to_path_buf(), |root| project_dir.join(root));
        let frontend_port = ports.frontend.unwrap_or(DEFAULT_FRONTEND_PORT);
        let command = local_frontend_command(&frontend_root, frontend_port);
        let mut env = BTreeMap::new();
        if let Some(url) = worker_url.as_deref() {
            let key = if is_next_frontend(&frontend_root) {
                "NEXT_PUBLIC_API_URL"
            } else {
                "VITE_API_URL"
            };
            env.insert(key.to_owned(), format!("{url}/api"));
        }
        let url = format!("http://{LOCAL_HOST}:{frontend_port}");
        frontend_url = Some(url.clone());
        port_statuses.push(dev_port_status("frontend", frontend_port, &url));
        processes.push(DevProcess {
            command,
            cwd: display_path(&frontend_root),
            env,
            name: "frontend",
        });
    }

    DevPlan {
        frontend_url,
        kind: config.kind.as_str(),
        next_actions: vec![
            "Open frontend_url in a browser for local UX testing.".to_owned(),
            "Use Ctrl-C to stop all local dev processes.".to_owned(),
            "Run `tovuk check` before deploying.".to_owned(),
        ],
        port_statuses,
        processes,
        project: display_path(project_dir),
        worker_url,
    }
}

fn dev_ports(cli: &CliOptions, config: &TovukConfig) -> Result<DevPorts> {
    Ok(DevPorts {
        frontend: cli_dev_port(&cli.dev.frontend_port, "--frontend-port", cli)?
            .or(config.dev.frontend_port),
        worker: cli_dev_port(&cli.dev.worker_port, "--worker-port", cli)?
            .or(config.dev.worker_port),
    })
}

fn cli_dev_port(value: &str, flag: &str, cli: &CliOptions) -> Result<Option<u16>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    let parsed = value.parse::<u16>().map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{flag} must be a TCP port from 1 to 65535."),
            format!("Pass a valid local port, for example `{flag} 5174`."),
            cli.output.json,
        )
    })?;
    if parsed == 0 {
        return Err(agent_error(
            "invalid_argument",
            format!("{flag} must be a TCP port from 1 to 65535."),
            format!("Pass a valid local port, for example `{flag} 5174`."),
            cli.output.json,
        ));
    }

    Ok(Some(parsed))
}

fn dev_port_status(name: &'static str, port: u16, url: &str) -> DevPortStatus {
    let has_listener = TcpStream::connect((LOCAL_HOST, port)).is_ok();
    let available = !has_listener && TcpListener::bind((LOCAL_HOST, port)).is_ok();
    let owner = if available { None } else { port_owner(port) };
    DevPortStatus {
        agent_instruction: if available {
            None
        } else {
            Some(port_conflict_instruction(name, port, url, owner.as_ref()))
        },
        available,
        host: LOCAL_HOST,
        name,
        owner,
        port,
        url: url.to_owned(),
    }
}

fn dev_agent_instruction(plan: &DevPlan) -> &'static str {
    if plan.port_statuses.iter().any(|status| !status.available) {
        "One or more planned dev ports are already in use. Inspect dev.port_statuses, then rerun with --worker-port and/or --frontend-port set to available ports, or update [dev] ports in tovuk.toml before using `tovuk dev --output text`."
    } else {
        "Run `tovuk dev --output text` to start these local processes. JSON mode returns the plan only so child process logs do not corrupt machine-readable output."
    }
}

fn port_conflict_instruction(
    name: &'static str,
    port: u16,
    url: &str,
    owner: Option<&DevPortOwner>,
) -> String {
    let owner_text = owner.map_or_else(
        || "another local process".to_owned(),
        |owner| format!("pid {} ({})", owner.pid, owner.command),
    );
    let override_flag = match name {
        "worker" => "--worker-port",
        "frontend" => "--frontend-port",
        _ => "--worker-port or --frontend-port",
    };
    format!(
        "Port {port} is already in use by {owner_text}. Stop that process, rerun `tovuk dev {override_flag} <free_port>`, or set [dev].{name}_port to a free port before using `tovuk dev --output text`. Do not use {url} for verification until the planned Tovuk dev process owns it."
    )
}

fn fail_on_port_conflicts(plan: &DevPlan, cli: &CliOptions) -> Result<()> {
    if plan.ports_available() {
        return Ok(());
    }

    Err(agent_error(
        "dev_port_in_use",
        "One or more planned Tovuk dev ports are already in use.",
        "Inspect the printed port rows, then stop the owning process or rerun with --worker-port and/or --frontend-port set to free ports. Use `tovuk dev --json` for machine-readable owner details.",
        cli.output.json,
    ))
}

impl DevPlan {
    fn ports_available(&self) -> bool {
        self.port_statuses.iter().all(|status| status.available)
    }
}

fn local_worker_command(worker_root: &Path, config: &TovukConfig) -> String {
    if worker_root.join("Cargo.toml").exists() {
        return "cargo run --release".to_owned();
    }
    config
        .backend
        .command
        .clone()
        .or_else(|| config.run.command.clone())
        .unwrap_or_else(|| "cargo run --release".to_owned())
}

fn local_frontend_command(frontend_root: &Path, frontend_port: u16) -> String {
    if is_next_frontend(frontend_root) {
        return match frontend_package_manager(frontend_root) {
            "bun" => format!("bun run dev --hostname {LOCAL_HOST} --port {frontend_port}"),
            _ => format!("npm run dev -- --hostname {LOCAL_HOST} --port {frontend_port}"),
        };
    }
    match frontend_package_manager(frontend_root) {
        "bun" => format!("bun run dev --host {LOCAL_HOST} --port {frontend_port} --strictPort"),
        _ => format!("npm run dev -- --host {LOCAL_HOST} --port {frontend_port} --strictPort"),
    }
}

fn print_dev_plan(plan: &DevPlan) {
    println!("project {}", plan.project);
    if let Some(url) = plan.worker_url.as_deref() {
        println!("worker {url}");
    }
    if let Some(url) = plan.frontend_url.as_deref() {
        println!("frontend {url}");
    }
    for status in &plan.port_statuses {
        let availability = if status.available {
            "available"
        } else {
            "in-use"
        };
        let owner = status.owner.as_ref().map_or_else(String::new, |owner| {
            format!(" owner_pid={} owner_command={}", owner.pid, owner.command)
        });
        println!(
            "port {} {} {}{}",
            status.name, status.url, availability, owner
        );
    }
    for process in &plan.processes {
        let env = process
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        let prefix = if env.is_empty() {
            String::new()
        } else {
            format!("{env} ")
        };
        println!(
            "{}: (cd '{}' && {}{})",
            process.name, process.cwd, prefix, process.command
        );
    }
}

fn flush_stdout() {
    let _ignore = io::stdout().flush();
}

fn run_dev_processes(plan: &DevPlan, cli: &CliOptions) -> Result<()> {
    let mut children = Vec::new();
    for process in &plan.processes {
        children.push(start_process(process, cli)?);
    }
    wait_for_any_process(&mut children, cli)
}

fn start_process(process: &DevProcess, cli: &CliOptions) -> Result<RunningProcess> {
    let mut command = shell_command(&process.command);
    command
        .current_dir(PathBuf::from(&process.cwd))
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (key, value) in &process.env {
        command.env(key, value);
    }
    let child = command.spawn().map_err(|error| {
        agent_error(
            "dev_process_failed",
            format!("Could not start {} dev process: {error}", process.name),
            "Install the required local toolchain, check the printed command, then rerun `tovuk dev`.",
            cli.output.json,
        )
    })?;
    Ok(RunningProcess {
        child,
        name: process.name,
    })
}

fn wait_for_any_process(children: &mut [RunningProcess], cli: &CliOptions) -> Result<()> {
    loop {
        for index in 0..children.len() {
            if let Some(status) = children[index].child.try_wait().map_err(|error| {
                agent_error(
                    "dev_process_failed",
                    format!(
                        "Could not inspect {} dev process: {error}",
                        children[index].name
                    ),
                    "Stop local dev processes and rerun `tovuk dev`.",
                    cli.output.json,
                )
            })? {
                stop_other_processes(children, index);
                if status.success() {
                    return Ok(());
                }
                return Err(agent_error(
                    "dev_process_exited",
                    format!("{} dev process exited with {status}.", children[index].name),
                    "Read the local process output above, fix the failing command, then rerun `tovuk dev`.",
                    cli.output.json,
                ));
            }
        }
        thread::sleep(Duration::from_millis(300));
    }
}

fn stop_other_processes(children: &mut [RunningProcess], exiting_index: usize) {
    for (index, process) in children.iter_mut().enumerate() {
        if index != exiting_index {
            let _ignore = process.child.kill();
        }
    }
}

struct RunningProcess {
    child: Child,
    name: &'static str,
}

fn shell_command(source: &str) -> Command {
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", source]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", source]);
        command
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        DevPlan, DevPorts, LOCAL_HOST, cli_dev_port, create_dev_plan, dev_port_status, dev_ports,
        fail_on_port_conflicts, port_conflict_instruction, read_dev_config,
    };
    use crate::cli::args::CliOptions;
    use std::{env, fs, net::TcpListener, path::PathBuf, time::SystemTime};

    #[test]
    fn fullstack_plan_wires_worker_and_frontend() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project_dir("fullstack-plan")?;
        fs::create_dir_all(project_dir.join("api"))?;
        fs::create_dir_all(project_dir.join("web"))?;
        fs::write(
            project_dir.join("api/Cargo.toml"),
            "[package]\nname = \"api\"\n",
        )?;
        fs::write(
            project_dir.join("web/package.json"),
            "{\"scripts\":{\"dev\":\"vite\"}}",
        )?;
        fs::write(project_dir.join("web/bun.lock"), "")?;
        fs::write(
            project_dir.join("tovuk.toml"),
            r#"
name = "demo"
kind = "fullstack"

[capabilities]
static_frontend = true
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[worker]
root = "api"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
output = "dist"
"#,
        )?;

        let cli = CliOptions::default();
        let config = read_dev_config(&project_dir, &cli)?;
        let plan = create_dev_plan(&project_dir, &config, DevPorts::default());

        if plan.processes.len() != 2 {
            return Err(format!("unexpected process count: {}", plan.processes.len()).into());
        }
        if plan.worker_url.as_deref() != Some("http://127.0.0.1:3000") {
            return Err(format!("unexpected worker url: {:?}", plan.worker_url).into());
        }
        if plan.frontend_url.as_deref() != Some("http://127.0.0.1:5173") {
            return Err(format!("unexpected frontend url: {:?}", plan.frontend_url).into());
        }
        if plan.processes[0].command != "cargo run --release" {
            return Err(format!("unexpected worker command: {}", plan.processes[0].command).into());
        }
        if plan.processes[0].env.get("PORT").map(String::as_str) != Some("3000") {
            return Err(format!("unexpected worker env: {:?}", plan.processes[0].env).into());
        }
        if plan.processes[1]
            .env
            .get("VITE_API_URL")
            .map(String::as_str)
            != Some("http://127.0.0.1:3000/api")
        {
            return Err(format!("unexpected frontend env: {:?}", plan.processes[1].env).into());
        }
        if plan.processes[1].command != "bun run dev --host 127.0.0.1 --port 5173 --strictPort" {
            return Err(
                format!("unexpected frontend command: {}", plan.processes[1].command).into(),
            );
        }

        let _ignore = fs::remove_dir_all(project_dir);
        Ok(())
    }

    #[test]
    fn fullstack_plan_uses_next_dev_flags() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project_dir("next-fullstack-plan")?;
        fs::create_dir_all(project_dir.join("api"))?;
        fs::create_dir_all(project_dir.join("web"))?;
        fs::write(
            project_dir.join("api/Cargo.toml"),
            "[package]\nname = \"api\"\n",
        )?;
        fs::write(
            project_dir.join("web/package.json"),
            "{\"dependencies\":{\"next\":\"^16.2.7\"},\"scripts\":{\"dev\":\"next dev\"}}",
        )?;
        fs::write(project_dir.join("web/next.config.mjs"), "")?;
        fs::write(
            project_dir.join("tovuk.toml"),
            r#"
name = "demo"
kind = "fullstack"

[capabilities]
static_frontend = true
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[worker]
root = "api"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
output = "out"
"#,
        )?;

        let cli = CliOptions::default();
        let config = read_dev_config(&project_dir, &cli)?;
        let plan = create_dev_plan(&project_dir, &config, DevPorts::default());

        if plan.processes[1]
            .env
            .get("NEXT_PUBLIC_API_URL")
            .map(String::as_str)
            != Some("http://127.0.0.1:3000/api")
        {
            return Err(format!("unexpected frontend env: {:?}", plan.processes[1].env).into());
        }
        if plan.processes[1].command != "npm run dev -- --hostname 127.0.0.1 --port 5173" {
            return Err(
                format!("unexpected frontend command: {}", plan.processes[1].command).into(),
            );
        }

        let _ignore = fs::remove_dir_all(project_dir);
        Ok(())
    }

    #[test]
    fn fullstack_plan_honors_dev_port_overrides() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project_dir("fullstack-ports")?;
        fs::create_dir_all(project_dir.join("api"))?;
        fs::create_dir_all(project_dir.join("web"))?;
        fs::write(
            project_dir.join("api/Cargo.toml"),
            "[package]\nname = \"api\"\n",
        )?;
        fs::write(
            project_dir.join("web/package.json"),
            "{\"scripts\":{\"dev\":\"vite\"}}",
        )?;
        fs::write(
            project_dir.join("tovuk.toml"),
            r#"
name = "demo"
kind = "fullstack"

[capabilities]
static_frontend = true
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[worker]
root = "api"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
output = "dist"

[dev]
worker_port = 3002
frontend_port = 5175
"#,
        )?;

        let mut cli = CliOptions::default();
        cli.dev.frontend_port = "5174".to_owned();
        cli.dev.worker_port = "3001".to_owned();
        let config = read_dev_config(&project_dir, &cli)?;
        let ports = dev_ports(&cli, &config)?;
        let plan = create_dev_plan(&project_dir, &config, ports);

        if plan.worker_url.as_deref() != Some("http://127.0.0.1:3001") {
            return Err(format!("unexpected worker url: {:?}", plan.worker_url).into());
        }
        if plan.frontend_url.as_deref() != Some("http://127.0.0.1:5174") {
            return Err(format!("unexpected frontend url: {:?}", plan.frontend_url).into());
        }
        if plan.processes[0].env.get("PORT").map(String::as_str) != Some("3001") {
            return Err(format!("unexpected worker env: {:?}", plan.processes[0].env).into());
        }
        if plan.processes[1]
            .env
            .get("VITE_API_URL")
            .map(String::as_str)
            != Some("http://127.0.0.1:3001/api")
        {
            return Err(format!("unexpected frontend env: {:?}", plan.processes[1].env).into());
        }
        if !plan.processes[1].command.contains("--port 5174") {
            return Err(
                format!("unexpected frontend command: {}", plan.processes[1].command).into(),
            );
        }

        let _ignore = fs::remove_dir_all(project_dir);
        Ok(())
    }

    #[test]
    fn fullstack_plan_uses_dev_config_ports() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project_dir("fullstack-config-ports")?;
        fs::create_dir_all(project_dir.join("api"))?;
        fs::create_dir_all(project_dir.join("web"))?;
        fs::write(
            project_dir.join("api/Cargo.toml"),
            "[package]\nname = \"api\"\n",
        )?;
        fs::write(
            project_dir.join("web/package.json"),
            "{\"scripts\":{\"dev\":\"vite\"}}",
        )?;
        fs::write(
            project_dir.join("tovuk.toml"),
            r#"
name = "demo"
kind = "fullstack"

[capabilities]
static_frontend = true
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[worker]
root = "api"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
output = "dist"

[dev]
worker_port = 3001
frontend_port = 5174
"#,
        )?;

        let cli = CliOptions::default();
        let config = read_dev_config(&project_dir, &cli)?;
        let ports = dev_ports(&cli, &config)?;
        let plan = create_dev_plan(&project_dir, &config, ports);

        if plan.worker_url.as_deref() != Some("http://127.0.0.1:3001") {
            return Err(format!("unexpected worker url: {:?}", plan.worker_url).into());
        }
        if plan.frontend_url.as_deref() != Some("http://127.0.0.1:5174") {
            return Err(format!("unexpected frontend url: {:?}", plan.frontend_url).into());
        }
        if plan.processes[1]
            .env
            .get("VITE_API_URL")
            .map(String::as_str)
            != Some("http://127.0.0.1:3001/api")
        {
            return Err(format!("unexpected frontend env: {:?}", plan.processes[1].env).into());
        }
        if !plan.processes[1].command.contains("--port 5174") {
            return Err(
                format!("unexpected frontend command: {}", plan.processes[1].command).into(),
            );
        }

        let _ignore = fs::remove_dir_all(project_dir);
        Ok(())
    }

    #[test]
    fn dev_port_overrides_reject_invalid_ports() {
        let mut cli = CliOptions::default();
        cli.dev.frontend_port = "0".to_owned();

        let error = cli_dev_port(&cli.dev.frontend_port, "--frontend-port", &cli)
            .err()
            .map(|error| error.payload().code.clone());

        assert_eq!(error.as_deref(), Some("invalid_argument"));
    }

    #[test]
    fn port_status_reports_occupied_port() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind((LOCAL_HOST, 0))?;
        let port = listener.local_addr()?.port();
        let status = dev_port_status("frontend", port, &format!("http://{LOCAL_HOST}:{port}"));

        if status.available {
            return Err(format!("expected occupied port {port} to be unavailable").into());
        }
        if status.agent_instruction.is_none() {
            return Err("expected occupied port to include an agent instruction".into());
        }
        if !status
            .agent_instruction
            .as_deref()
            .unwrap_or_default()
            .contains("Do not use")
        {
            return Err(format!(
                "expected stale verification warning, got {:?}",
                status.agent_instruction
            )
            .into());
        }

        drop(listener);
        Ok(())
    }

    #[test]
    fn occupied_ports_stop_text_dev_before_process_start() -> Result<(), Box<dyn std::error::Error>>
    {
        let listener = TcpListener::bind((LOCAL_HOST, 0))?;
        let port = listener.local_addr()?.port();
        let status = dev_port_status("worker", port, &format!("http://{LOCAL_HOST}:{port}"));
        let plan = DevPlan {
            frontend_url: None,
            kind: "rust-worker",
            next_actions: Vec::new(),
            port_statuses: vec![status],
            processes: Vec::new(),
            project: "/tmp/demo".to_owned(),
            worker_url: Some(format!("http://{LOCAL_HOST}:{port}")),
        };
        let cli = CliOptions::default();

        let error = fail_on_port_conflicts(&plan, &cli)
            .err()
            .ok_or("expected occupied port to fail")?;
        if error.payload().code != "dev_port_in_use" {
            return Err(format!("unexpected error: {:?}", error.payload()).into());
        }

        drop(listener);
        Ok(())
    }

    #[test]
    fn port_conflict_instruction_names_owner() {
        let owner = super::DevPortOwner {
            command: "api".to_owned(),
            pid: 123,
        };
        let instruction =
            port_conflict_instruction("worker", 3000, "http://127.0.0.1:3000", Some(&owner));

        assert!(instruction.contains("pid 123 (api)"));
        assert!(instruction.contains("tovuk dev --worker-port <free_port>"));
        assert!(instruction.contains("[dev].worker_port"));
        assert!(instruction.contains("Do not use http://127.0.0.1:3000"));
    }

    fn temp_project_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let path = env::temp_dir().join(format!("tovuk-{name}-{nanos}"));
        let _ignore = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
