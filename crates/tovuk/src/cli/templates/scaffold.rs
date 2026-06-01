use super::{
    super::{
        constants::PROJECT_TEMPLATES,
        errors::{Result, agent_error, internal_error},
        project::service_name_from_dir,
        template_sources::{
            frontend_package_json, frontend_source, frontend_ts_config, frontend_vite_env_source,
            rust_api_source, rust_template_cargo_lock, rust_template_cargo_toml,
        },
    },
    config::{frontend_config, fullstack_config, rust_backend_config},
};
use std::{fs, path::Path};

pub(super) fn create_template(project_dir: &Path, template: &str) -> Result<()> {
    if !PROJECT_TEMPLATES.contains(&template) {
        return Err(agent_error(
            "invalid_template",
            "Tovuk template is unknown.",
            format!("Use one of: {}.", PROJECT_TEMPLATES.join(", ")),
            false,
        ));
    }
    match template {
        "rust-worker" => {
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
        "run package install in the frontend directory before check: bun install or npm install"
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
