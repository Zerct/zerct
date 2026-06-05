use super::{
    args::CliOptions,
    config::{TovukConfig, parse_tovuk_toml, validate_config},
    errors::{Result, agent_error, print_json},
    frontend_checks::{frontend_package_manager, is_next_static_frontend},
    project_kind::ProjectKind,
};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

const DEFAULT_FRONTEND_PORT: u16 = 5173;
const LOCAL_HOST: &str = "127.0.0.1";

#[derive(Clone, Debug, Serialize)]
struct DevPlan {
    frontend_url: Option<String>,
    kind: &'static str,
    next_actions: Vec<String>,
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

pub(crate) fn dev(project_dir: &Path, cli: &CliOptions) -> Result<()> {
    let config = read_dev_config(project_dir, cli)?;
    let plan = create_dev_plan(project_dir, &config);
    if cli.output.json {
        return print_json(&json!({
            "ok": true,
            "mode": "plan",
            "dev": plan,
            "agent_instruction": "Run `tovuk dev --output text` to start these local processes. JSON mode returns the plan only so child process logs do not corrupt machine-readable output."
        }));
    }
    print_dev_plan(&plan);
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

fn create_dev_plan(project_dir: &Path, config: &TovukConfig) -> DevPlan {
    let mut processes = Vec::new();
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
        let worker_port = config.backend.port.unwrap_or(config.run.port);
        let command = local_worker_command(&worker_root, config);
        worker_url = Some(format!("http://{LOCAL_HOST}:{worker_port}"));
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
        let command = local_frontend_command(&frontend_root);
        let mut env = BTreeMap::new();
        if let Some(url) = worker_url.as_deref() {
            let key = if is_next_static_frontend(&frontend_root) {
                "NEXT_PUBLIC_API_URL"
            } else {
                "VITE_API_URL"
            };
            env.insert(key.to_owned(), format!("{url}/api"));
        }
        frontend_url = Some(format!("http://{LOCAL_HOST}:{DEFAULT_FRONTEND_PORT}"));
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
        processes,
        project: display_path(project_dir),
        worker_url,
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

fn local_frontend_command(frontend_root: &Path) -> String {
    if is_next_static_frontend(frontend_root) {
        return match frontend_package_manager(frontend_root) {
            "bun" => format!("bun run dev --hostname {LOCAL_HOST} --port {DEFAULT_FRONTEND_PORT}"),
            _ => format!("npm run dev -- --hostname {LOCAL_HOST} --port {DEFAULT_FRONTEND_PORT}"),
        };
    }
    match frontend_package_manager(frontend_root) {
        "bun" => {
            format!("bun run dev --host {LOCAL_HOST} --port {DEFAULT_FRONTEND_PORT} --strictPort")
        }
        _ => format!(
            "npm run dev -- --host {LOCAL_HOST} --port {DEFAULT_FRONTEND_PORT} --strictPort"
        ),
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
    use super::{create_dev_plan, read_dev_config};
    use crate::cli::args::CliOptions;
    use std::{env, fs, path::PathBuf, time::SystemTime};

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
        let plan = create_dev_plan(&project_dir, &config);

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
        let plan = create_dev_plan(&project_dir, &config);

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
