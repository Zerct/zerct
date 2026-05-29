use super::{
    config::ProjectKind,
    constants::{
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_RUST_CHECK_COMMAND, PROJECT_TEMPLATES,
    },
    doctor::doctor_project,
    errors::{Result, agent_error, internal_error},
    frontend_checks::{frontend_build_command, frontend_check_command, is_plain_static_frontend},
    project::{
        detect_fullstack_roots, ensure_directory, infer_project_kind, path_relative,
        service_name_from_cargo, service_name_from_dir, service_name_from_package,
    },
    template_sources::{
        frontend_package_json, frontend_source, frontend_ts_config, frontend_vite_env_source,
        rust_api_source, rust_template_cargo_lock, rust_template_cargo_toml,
    },
};
use std::{env, fs, path::Path};

pub(crate) fn init_project(project_dir: &Path, template: &str) -> Result<()> {
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
    let source = init_config(project_dir, kind)?;
    fs::write(&config_path, source).map_err(|error| internal_error(error.to_string()))?;
    println!(
        "created {}",
        path_relative(&config_path, &env::current_dir().unwrap_or_default())
    );
    println!("detected {}", kind.as_str());
    Ok(())
}

pub(crate) fn install_project(project_dir: &Path, template: &str) -> Result<()> {
    init_project(project_dir, template)?;
    doctor_project(project_dir, false)
}

pub(crate) fn create_template(project_dir: &Path, template: &str) -> Result<()> {
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
            write_rust_api_template(project_dir, &service_name_from_dir(project_dir), true)?;
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

pub(crate) fn write_fullstack_template(project_dir: &Path) -> Result<()> {
    write_rust_api_template(&project_dir.join("api"), "api", false)?;
    write_frontend_template(&project_dir.join("web"), "web", "/api", false)?;
    write_new_file(
        &project_dir.join("tovuk.toml"),
        &fullstack_config(project_dir, "api", "web", true),
    )
}

pub(crate) fn write_rust_api_template(
    project_dir: &Path,
    name: &str,
    include_config: bool,
) -> Result<()> {
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

pub(crate) fn write_frontend_template(
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

pub(crate) fn write_new_file(path: &Path, source: &str) -> Result<()> {
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

pub(crate) fn init_config(project_dir: &Path, kind: ProjectKind) -> Result<String> {
    match kind {
        ProjectKind::Fullstack => {
            if let Some((backend, frontend)) = detect_fullstack_roots(project_dir) {
                return Ok(fullstack_config(project_dir, &backend, &frontend, false));
            }
            Err(agent_error(
                "fullstack_roots_missing",
                "Could not find fullstack roots.",
                "Create api/Cargo.toml and web/package.json or web/index.html, then retry.",
                false,
            ))
        }
        ProjectKind::StaticFrontend => Ok(frontend_config(project_dir, false)),
        ProjectKind::RustBackend => Ok(rust_backend_config(project_dir)),
    }
}

pub(crate) fn rust_backend_config(project_dir: &Path) -> String {
    let name = service_name_from_cargo(project_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    format!(
        "name = \"{name}\"\n\n[build]\ncheck = \"{DEFAULT_RUST_CHECK_COMMAND}\"\ncommand = \"cargo build --release\"\n\n[run]\ncommand = \"./target/release/{name}\"\nport = 3000\nhealth = \"/healthz\"\n\n[resources]\nmemory = \"512mb\"\ncpu = \"0.25\"\nidle_timeout_minutes = 15\n"
    )
}

pub(crate) fn frontend_config(project_dir: &Path, prefer_bun: bool) -> String {
    let name = service_name_from_package(project_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    let settings = frontend_build_settings(project_dir, prefer_bun);
    format!(
        "name = \"{name}\"\nkind = \"static_frontend\"\n\n[build]\ncheck = \"{}\"\ncommand = \"{}\"\noutput = \"{}\"\n",
        settings.check, settings.build, settings.output
    )
}

pub(crate) fn fullstack_config(
    project_dir: &Path,
    backend: &str,
    frontend: &str,
    prefer_bun: bool,
) -> String {
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

pub(crate) struct FrontendBuildSettings {
    pub(crate) check: String,
    pub(crate) build: String,
    pub(crate) output: String,
}

pub(crate) fn frontend_build_settings(
    project_dir: &Path,
    prefer_bun: bool,
) -> FrontendBuildSettings {
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
