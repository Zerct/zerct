use super::{config::TovukConfig, errors::Result};
use std::path::Path;

mod loader;
mod process;
mod server;

use loader::preview_config;
use process::{preview_runtime, run_shell, spawn_preview_backend};
use server::serve_static;

pub(crate) fn preview_project(project_dir: &Path, port: u16) -> Result<()> {
    let config = preview_config(project_dir)?;
    preview_validated_project(project_dir, &config, port)
}

fn preview_validated_project(project_dir: &Path, config: &TovukConfig, port: u16) -> Result<()> {
    if config.kind.is_fullstack() {
        return preview_fullstack(project_dir, config, port);
    }
    run_shell(
        &config.build.command,
        project_dir,
        "Build failed before preview.",
    )?;
    if config.kind.is_static_frontend() {
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
    let mut backend = spawn_preview_backend(
        config.backend.command.as_deref().unwrap_or_default(),
        &backend_dir,
        backend_port,
    )?;
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
