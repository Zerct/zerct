use super::{
    api_commands::{api_request, payment_required_agent_error},
    args::CliOptions,
    auth::read_or_login_token,
    config::parse_tovuk_toml,
    constants::{
        ARCHIVE_EXCLUDES, ARCHIVE_LIMIT_BYTES, WALK_EXCLUDED_DIRS, WORKSPACE_EXCLUDED_DIRS,
    },
    doctor::run_doctor,
    errors::{Result, agent_error, print_json},
    project::{
        encode_component, ensure_directory, nested_string, number_field, path_relative, progress,
        string_field,
    },
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::{Compression, write::GzEncoder};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug)]
pub(crate) struct DeployProjectInfo {
    pub(crate) dir: PathBuf,
    pub(crate) relative: String,
    pub(crate) name: String,
    pub(crate) kind: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DeployPlanProject {
    pub(crate) project: DeployProjectInfo,
    pub(crate) wants_database: bool,
}

pub(crate) fn deploy(project_dir: &Path, cli: &CliOptions) -> Result<()> {
    let projects = discover_deploy_projects(project_dir)?;
    if projects.is_empty() {
        return Err(agent_error(
            "missing_project_contract",
            "No tovuk.toml was found.",
            "Run `tovuk init` in each app directory, or pass a project path.",
            cli.output.json,
        ));
    }
    let token = read_or_login_token(cli)?;
    let plan = create_deploy_plan(&projects, cli, &token)?;
    let mut results = deploy_projects(&plan, cli, &token)?;
    if cli.deployment.wait {
        wait_for_workspace_builds(cli, &token, &mut results)?;
    }
    if results.len() == 1 {
        print_deploy_result(&results[0].response, cli)
    } else {
        print_workspace_deploy_results(project_dir, &results, cli)
    }
}

pub(crate) struct WorkspaceDeployResult {
    pub(crate) project: DeployProjectInfo,
    pub(crate) wants_database: bool,
    pub(crate) response: Value,
    pub(crate) final_build: Option<Value>,
}

pub(crate) fn deploy_projects(
    plan: &[DeployPlanProject],
    cli: &CliOptions,
    token: &str,
) -> Result<Vec<WorkspaceDeployResult>> {
    let mut results = Vec::new();
    let workspace_deploy = plan.len() > 1;
    if workspace_deploy && !cli.output.json {
        println!("deploying {} projects", plan.len());
    }
    for item in plan {
        if workspace_deploy && !cli.output.json {
            println!("checking {}", item.project.relative);
        }
        let response = deploy_project(&item.project.dir, cli, token, item.wants_database)?;
        if workspace_deploy && !cli.output.json {
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

pub(crate) fn deploy_project(
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
            cli.output.json,
        ));
    }
    let body = json!({
        "config": report.config,
        "commit_sha": git_commit_sha(project_dir),
        "wants_database": wants_database,
        "source_archive_base64": create_archive_base64(project_dir, cli.output.json)?,
    });
    api_request(cli, Method::POST, "/v1/deploy", Some(token), Some(body))
}

pub(crate) fn print_deploy_result(response: &Value, cli: &CliOptions) -> Result<()> {
    if cli.output.json {
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

pub(crate) fn print_workspace_deploy_results(
    project_dir: &Path,
    results: &[WorkspaceDeployResult],
    cli: &CliOptions,
) -> Result<()> {
    if cli.output.json {
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

pub(crate) fn wait_for_workspace_builds(
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

pub(crate) fn wait_for_build(cli: &CliOptions, token: &str, build_id: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(cli.deployment.wait_timeout_seconds);
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
                cli.output.json,
            ));
        }
        if status != last_status {
            progress(
                cli,
                &format!("build {} {status}", string_field(&build, "id")),
            );
            last_status.clone_from(&status);
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
        cli.output.json,
    ))
}

pub(crate) fn create_deploy_plan(
    projects: &[DeployProjectInfo],
    cli: &CliOptions,
    token: &str,
) -> Result<Vec<DeployPlanProject>> {
    let plan = projects
        .iter()
        .map(|project| DeployPlanProject {
            project: project.clone(),
            wants_database: cli.deployment.database
                && (project.kind == "rust_backend" || project.kind == "fullstack"),
        })
        .collect::<Vec<_>>();
    reject_invalid_database_targets(&plan, cli)?;
    preflight_deploy_limits(&plan, cli, token)?;
    Ok(plan)
}

pub(crate) fn reject_invalid_database_targets(
    plan: &[DeployPlanProject],
    cli: &CliOptions,
) -> Result<()> {
    if cli.deployment.database
        && plan.len() == 1
        && plan
            .first()
            .is_some_and(|item| item.project.kind == "static_frontend")
    {
        return Err(agent_error(
            "invalid_database_target",
            "Static frontends cannot attach managed Postgres directly.",
            "Deploy a Rust backend with managed Postgres and call it from the frontend.",
            cli.output.json,
        ));
    }
    Ok(())
}

pub(crate) fn preflight_deploy_limits(
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

pub(crate) struct RequestedResources {
    pub(crate) projects: u64,
    pub(crate) databases: u64,
}

pub(crate) fn app_name_set(response: &Value) -> BTreeSet<String> {
    response
        .get("apps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|app| app.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

pub(crate) fn requested_new_resources(
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

pub(crate) fn discover_deploy_projects(root_dir: &Path) -> Result<Vec<DeployProjectInfo>> {
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

pub(crate) fn discover_project_dirs(dir: &Path, project_dirs: &mut Vec<PathBuf>) {
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

pub(crate) fn deploy_project_info(dir: &Path, root_dir: &Path) -> DeployProjectInfo {
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

pub(crate) fn kind_order(kind: &str) -> u8 {
    match kind {
        "rust_backend" => 0,
        "fullstack" => 1,
        "static_frontend" => 2,
        _ => 3,
    }
}

pub(crate) fn create_archive_base64(project_dir: &Path, json_output: bool) -> Result<String> {
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

pub(crate) fn is_archive_excluded_entry(project_dir: &Path, entry: &DirEntry) -> bool {
    if entry.path() == project_dir {
        return false;
    }
    let relative = match entry.path().strip_prefix(project_dir) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_error) => return true,
    };
    is_archive_excluded(&relative, entry.file_type().is_dir())
}

pub(crate) fn is_archive_excluded(relative: &str, is_dir: bool) -> bool {
    let basename = relative.rsplit('/').next().unwrap_or(relative);
    if is_dir && WALK_EXCLUDED_DIRS.contains(&basename) {
        return true;
    }
    ARCHIVE_EXCLUDES.iter().any(|pattern| match *pattern {
        "*.pem" => basename_has_extension(basename, "pem"),
        "*.key" => basename_has_extension(basename, "key"),
        "*.p12" => basename_has_extension(basename, "p12"),
        "*.pfx" => basename_has_extension(basename, "pfx"),
        "*.tfstate" => basename_has_extension(basename, "tfstate"),
        "*.tfstate.*" => basename.contains(".tfstate."),
        "*.sqlite" => basename_has_extension(basename, "sqlite"),
        "*.sqlite3" => basename_has_extension(basename, "sqlite3"),
        "*.db" => basename_has_extension(basename, "db"),
        "*.log" => basename_has_extension(basename, "log"),
        "._*" => basename.starts_with("._"),
        ".env.*" => basename.starts_with(".env."),
        pattern => relative == pattern || relative.starts_with(&format!("{pattern}/")),
    })
}

pub(crate) fn basename_has_extension(basename: &str, extension: &str) -> bool {
    Path::new(basename)
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

pub(crate) fn git_commit_sha(project_dir: &Path) -> Option<String> {
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
