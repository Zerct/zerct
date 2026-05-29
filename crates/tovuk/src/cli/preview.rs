use super::{
    Client, Command, DoctorReportKind, Method, Path, PathBuf, Read, Result, StatusCode,
    TcpListener, TcpStream, TovukConfig, Write, agent_error, ensure_directory, fs, internal_error,
    parse_tovuk_toml, run_doctor_workspace, validate_config,
};

pub(crate) fn preview_project(project_dir: &Path, port: u16) -> Result<()> {
    let config = preview_config(project_dir)?;
    preview_validated_project(project_dir, &config, port)
}

pub(crate) fn preview_config(project_dir: &Path) -> Result<TovukConfig> {
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

pub(crate) fn preview_validated_project(
    project_dir: &Path,
    config: &TovukConfig,
    port: u16,
) -> Result<()> {
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

pub(crate) fn preview_fullstack(project_dir: &Path, config: &TovukConfig, port: u16) -> Result<()> {
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

pub(crate) fn preview_static(project_dir: &Path, output: &str, port: u16) -> Result<()> {
    serve_static(
        &project_dir.join(output),
        if port == 0 { 4173 } else { port },
        None,
    )
}

pub(crate) fn preview_runtime(project_dir: &Path, command: &str, port: u16) -> Result<()> {
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

pub(crate) fn run_shell(command: &str, project_dir: &Path, failure_message: &str) -> Result<()> {
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

pub(crate) fn shell_command(command: &str) -> Command {
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

pub(crate) fn serve_static(root: &Path, port: u16, api_proxy_port: Option<u16>) -> Result<()> {
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

pub(crate) fn handle_static_request(
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

pub(crate) fn proxy_to_backend(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    port: u16,
) -> Result<()> {
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

pub(crate) fn write_http_response(
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

pub(crate) fn static_target(root: &Path, pathname: &str) -> PathBuf {
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

pub(crate) fn content_type(file: &Path) -> &'static str {
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

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}
