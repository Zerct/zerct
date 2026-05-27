"""Small dependency-free Zerct CLI for Python users."""

from __future__ import annotations

import argparse
import base64
import concurrent.futures
import io
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
import webbrowser
from dataclasses import dataclass
from typing import Any, Iterator

from . import __version__

DEFAULT_API_URL = "https://api.zerct.com"
ARCHIVE_LIMIT_BYTES = 48 * 1024 * 1024
SESSION_DIR = ".zerct"
SESSION_FILE = "session-token"
SESSION_SERVICE = "com.zerct.cli"
SESSION_ACCOUNT = "session-token"
SESSION_LABEL = "Zerct session"
DEFAULT_LOGIN_EXPIRES_SECONDS = 600
DEFAULT_LOGIN_INTERVAL_SECONDS = 5
DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS = 900
DEFAULT_RUST_CHECK_COMMAND = "cargo fmt --all --check && cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings"
DEFAULT_NPM_FRONTEND_CHECK_COMMAND = "npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint"
DEFAULT_BUN_FRONTEND_CHECK_COMMAND = "bun ci && bun run typecheck && bun run lint"
RUST_TEMPLATE_SOURCE = """use std::{
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
        "HTTP/1.1 {status}\\r\\ncontent-type: application/json\\r\\ncontent-length: {}\\r\\naccess-control-allow-origin: {origin}\\r\\naccess-control-allow-methods: GET, OPTIONS\\r\\naccess-control-allow-headers: content-type, authorization\\r\\nconnection: close\\r\\n\\r\\n{body}",
        body.len()
    )
}
"""
EXCLUDED_PARTS = {
    ".git",
    "target",
    "node_modules",
    ".zerct",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
}
EXCLUDED_NAMES = {
    ".npmrc",
    ".pypirc",
    ".netrc",
    "id_rsa",
    "id_ed25519",
    ".DS_Store",
}
EXCLUDED_SUFFIXES = (".pem", ".key", ".p12", ".pfx", ".sqlite", ".sqlite3", ".db", ".log")
WORKSPACE_EXCLUDED_PARTS = EXCLUDED_PARTS | {".cache", ".next", ".turbo", "build", "coverage", "dist", "vendor"}


@dataclass(frozen=True)
class AgentError(Exception):
    code: str
    message: str
    agent_instruction: str
    docs_url: str | None = None
    checkout_url: str | None = None
    already_reported: bool = False

    def payload(self) -> dict[str, Any]:
        return {
            "code": self.code,
            "message": self.message,
            "agent_instruction": self.agent_instruction,
            "docs_url": self.docs_url,
            "checkout_url": self.checkout_url,
        }


@dataclass(frozen=True)
class DeployProject:
    directory: pathlib.Path
    relative: str
    name: str
    kind: str


def main(argv: list[str] | None = None) -> None:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        run(args)
    except AgentError as error:
        print_error(error, args.json)
        raise SystemExit(1) from error


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="zerct",
        epilog=(
            "Agent contract: Rust backends keep Cargo.lock committed, pass rustfmt, listen on "
            '0.0.0.0:$PORT, and return HTTP 200 from health. Static frontends set kind = "static_frontend", '
            "keep TypeScript source, a package lockfile, and typecheck + lint scripts. Frontends call Rust "
            "backends for APIs, managed Postgres, and server-side logic. Run deploy from a repo "
            "root with nested zerct.toml files to deploy the workspace in one "
            "command. When a frontend calls a backend on another hostname, "
            "configure backend CORS or use a same-origin custom domain. Keep "
            "direct unsafe out of Rust source."
        ),
    )
    parser.add_argument("--version", action="version", version=__version__)
    parser.add_argument("--json", action="store_true", help="print JSON")
    parser.add_argument("--api", default=DEFAULT_API_URL, help="Zerct API URL")
    parser.add_argument("--token", default="", help="Zerct session token")

    subcommands = parser.add_subparsers(dest="command", required=True)
    init = subcommands.add_parser("init")
    init.add_argument("path", nargs="?", default=".")
    init.add_argument("--template", choices=["rust-api", "tanstack-static-frontend", "fullstack-rust-tanstack"], default="")

    install = subcommands.add_parser("install")
    install.add_argument("path", nargs="?", default=".")

    doctor = subcommands.add_parser("doctor")
    doctor.add_argument("path", nargs="?", default=".")

    preview = subcommands.add_parser("preview")
    preview.add_argument("path", nargs="?", default=".")
    preview.add_argument("--port", type=positive_seconds, default=0)

    login = subcommands.add_parser("login")
    login.add_argument("--token", default="", help="Zerct session token")

    deploy = subcommands.add_parser("deploy")
    deploy.add_argument("path", nargs="?", default=".")
    deploy.add_argument("--database", action="store_true")
    deploy.add_argument("--wait", action="store_true")
    deploy.add_argument("--wait-timeout", type=positive_seconds, default=DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS)

    shared_json_commands: list[argparse.ArgumentParser] = []
    for command in ("capabilities", "me", "usage", "apps"):
        shared = subcommands.add_parser(command)
        shared_json_commands.append(shared)

    activity = subcommands.add_parser("activity")
    activity.add_argument("--limit", default="")
    activity.add_argument("--cursor", default="")
    shared_json_commands.append(activity)

    overview = subcommands.add_parser("overview")
    overview.add_argument("--app", required=True)
    overview.add_argument("--limit", default="")
    overview.add_argument("--cursor", default="")
    shared_json_commands.append(overview)

    deploys = subcommands.add_parser("deploys")
    deploys.add_argument("--app", default="")
    deploys.add_argument("--limit", default="")
    deploys.add_argument("--cursor", default="")
    shared_json_commands.append(deploys)

    builds = subcommands.add_parser("builds")
    builds.add_argument("--app", default="")
    builds.add_argument("--limit", default="")
    builds.add_argument("--cursor", default="")
    shared_json_commands.append(builds)

    logs = subcommands.add_parser("logs")
    logs.add_argument("--app", default="")
    logs.add_argument("--build", default="")
    logs.add_argument("--deploy", default="")
    logs.add_argument("--limit", default="")
    logs.add_argument("--cursor", default="")

    shared_json_commands.extend([init, install, doctor, preview, login, deploy, logs])
    for command in ("status", "inspect", "db"):
        item = subcommands.add_parser(command)
        item.add_argument("--app", required=True)
        shared_json_commands.append(item)

    env = subcommands.add_parser("env")
    env.add_argument("action", choices=["list", "set", "delete"], nargs="?", default="list")
    env.add_argument("assignment", nargs="?")
    env.add_argument("--app", required=True)
    shared_json_commands.append(env)

    domains = subcommands.add_parser("domains")
    domains.add_argument("action", choices=["list", "add", "verify", "delete"], nargs="?", default="list")
    domains.add_argument("domain", nargs="?")
    domains.add_argument("--app", required=True)
    shared_json_commands.append(domains)

    billing = subcommands.add_parser("billing")
    billing.add_argument("action", choices=["portal"], nargs="?")
    shared_json_commands.append(billing)
    for command in shared_json_commands:
        add_command_common_options(command)
    return parser


def add_command_common_options(command: argparse.ArgumentParser) -> None:
    command.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    command.add_argument("--api", default=argparse.SUPPRESS)


def run(args: argparse.Namespace) -> None:
    args.api = args.api.rstrip("/")
    match args.command:
        case "init":
            init_project(pathlib.Path(args.path).resolve(), args.template)
        case "install":
            init_project(pathlib.Path(args.path).resolve(), "")
            doctor_project(pathlib.Path(args.path).resolve(), args.json)
        case "doctor":
            doctor_project(pathlib.Path(args.path).resolve(), args.json)
        case "preview":
            preview_project(pathlib.Path(args.path).resolve(), args.port)
        case "login":
            login(args)
        case "deploy":
            deploy(pathlib.Path(args.path).resolve(), args)
        case "capabilities":
            print_response(api_request(args, "GET", "/v1/capabilities", None, None), args.json)
        case "me":
            print_response(authenticated_get(args, "/v1/me"), args.json)
        case "usage":
            print_response(authenticated_get(args, "/v1/usage"), args.json)
        case "activity":
            print_response(authenticated_get(args, f"/v1/activity{page_query(args)}"), args.json)
        case "apps":
            print_response(authenticated_get(args, "/v1/apps"), args.json)
        case "overview":
            print_response(authenticated_get(args, f"/v1/apps/{urllib.parse.quote(args.app)}/overview{page_query(args)}"), args.json)
        case "deploys":
            deploys(args)
        case "builds":
            builds(args)
        case "logs":
            logs(args)
        case "status":
            print_response(app_get(args, "status"), args.json)
        case "inspect":
            print_response(app_get(args, "inspect"), args.json)
        case "db":
            print_response(app_get(args, "database"), args.json)
        case "env":
            env_command(args)
        case "domains":
            domains_command(args)
        case "billing":
            billing(args)
        case _:
            raise AgentError(
                "unknown_command",
                "Unknown Zerct command.",
                "Run `zerct --help` and retry with a supported command.",
            )


def positive_seconds(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a positive integer") from error
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return parsed


def init_project(project_dir: pathlib.Path, template: str = "") -> None:
    if template:
        project_dir.mkdir(parents=True, exist_ok=True)
        create_template(project_dir, template)
        return

    if not project_dir.is_dir():
        raise AgentError(
            "missing_project",
            "Project directory does not exist.",
            "Run Zerct from the root of a Rust project or pass the project path.",
        )

    config_path = project_dir / "zerct.toml"
    if config_path.exists():
        print("zerct.toml already exists")
        return

    kind = infer_project_kind(project_dir)
    config_path.write_text(frontend_config(project_dir) if kind == "static_frontend" else rust_backend_config(project_dir), encoding="utf-8")
    print(f"created {config_path}")
    print(f"detected {kind}")


def create_template(project_dir: pathlib.Path, template: str) -> None:
    if template == "rust-api":
        write_rust_template(project_dir, service_name_from_dir(project_dir))
    elif template == "tanstack-static-frontend":
        write_frontend_template(project_dir, service_name_from_dir(project_dir), "/api")
    elif template == "fullstack-rust-tanstack":
        write_rust_template(project_dir / "api", "api")
        write_frontend_template(project_dir / "web", "web", "http://localhost:3000")
    else:
        raise AgentError("invalid_template", "Zerct template is unknown.", "Use rust-api, tanstack-static-frontend, or fullstack-rust-tanstack.")
    print(f"created {template} template")


def write_rust_template(project_dir: pathlib.Path, name: str) -> None:
    (project_dir / "src").mkdir(parents=True, exist_ok=True)
    write_new_file(project_dir / "Cargo.toml", f"""[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
publish = false

[lints.rust]
unsafe_code = "forbid"
warnings = "deny"
""")
    write_new_file(project_dir / "Cargo.lock", f"""# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "{name}"
version = "0.1.0"
""")
    write_new_file(project_dir / "src" / "main.rs", RUST_TEMPLATE_SOURCE)
    write_new_file(project_dir / "zerct.toml", rust_backend_config(project_dir))


def write_frontend_template(project_dir: pathlib.Path, name: str, api_base_url: str) -> None:
    (project_dir / "src").mkdir(parents=True, exist_ok=True)
    write_new_file(project_dir / "package.json", f"""{{
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
    "@typescript/native-preview": "^7.0.0-dev.20251126.1",
    "@vitejs/plugin-react": "^5.1.1",
    "oxlint": "^1.30.0",
    "typescript": "^5.9.3",
    "vite": "^7.2.4"
  }}
}}
""")
    write_new_file(project_dir / "index.html", '<div id="root"></div><script type="module" src="/src/main.tsx"></script>\n')
    write_new_file(project_dir / "src" / "styles.css", "body{margin:0;font-family:system-ui,sans-serif}main{min-height:100svh;display:grid;place-items:center;padding:2rem}code{font-family:ui-monospace,monospace}\n")
    write_new_file(project_dir / "src" / "vite-env.d.ts", '/// <reference types="vite/client" />\n')
    write_new_file(project_dir / "src" / "main.tsx", frontend_template_source(api_base_url))
    write_new_file(project_dir / "tsconfig.json", '{"compilerOptions":{"strict":true,"jsx":"react-jsx","module":"ESNext","moduleResolution":"Bundler","target":"ES2022","noEmit":true,"skipLibCheck":true},"include":["src","vite.config.ts"]}\n')
    write_new_file(project_dir / "vite.config.ts", 'import react from "@vitejs/plugin-react";\nimport { defineConfig } from "vite";\n\nexport default defineConfig({ plugins: [react()] });\n')
    write_new_file(project_dir / "zerct.toml", frontend_config(project_dir))
    print("run package install in the frontend directory before doctor: bun install or npm install")


def write_new_file(path: pathlib.Path, source: str) -> None:
    if path.exists():
        raise AgentError("file_exists", f"Refusing to overwrite {path}.", "Move the existing file or choose an empty directory, then retry.")
    path.write_text(source, encoding="utf-8")


def rust_backend_config(project_dir: pathlib.Path) -> str:
    name = service_name_from_cargo(project_dir) or service_name_from_dir(project_dir)
    return f"""name = "{name}"

[build]
check = "{DEFAULT_RUST_CHECK_COMMAND}"
command = "cargo build --release"

[run]
command = "./target/release/{name}"
port = 3000
health = "/healthz"

[resources]
memory = "512mb"
cpu = "0.25"
idle_timeout_minutes = 15
"""


def frontend_config(project_dir: pathlib.Path) -> str:
    name = service_name_from_package(project_dir) or service_name_from_dir(project_dir)
    return f"""name = "{name}"
kind = "static_frontend"

[build]
check = "{frontend_check_command(project_dir)}"
command = "{frontend_build_command(project_dir)}"
output = "dist"
"""


def doctor_project(project_dir: pathlib.Path, json_output: bool) -> None:
    report = run_doctor_workspace(project_dir)
    if json_output:
        print(json.dumps(report, indent=2))
    else:
        if "projects" in report:
            for project in report["projects"]:
                print(f"project {project['relative']}")
                for check in project["checks"]:
                    state = "ok" if check["ok"] else "fail"
                    print(f"{state} {check['name']} - {check['message']}")
        else:
            for check in report["checks"]:
                state = "ok" if check["ok"] else "fail"
                print(f"{state} {check['name']} - {check['message']}")

    if not report["ok"]:
        checks = (
            [check for project in report["projects"] for check in project["checks"]]
            if "projects" in report
            else report["checks"]
        )
        failure = next(check for check in checks if not check["ok"])
        raise AgentError(
            "doctor_failed",
            "Zerct doctor failed.",
            failure["agent_instruction"],
            already_reported=json_output,
        )


def preview_project(project_dir: pathlib.Path, port: int) -> None:
    report = run_doctor_workspace(project_dir)
    if "projects" in report:
        raise AgentError("workspace_preview_unsupported", "Preview one project at a time.", "Run `zerct preview api` or `zerct preview web` from the workspace root.")
    if not report["ok"]:
        failure = next(check for check in report["checks"] if not check["ok"])
        raise AgentError("doctor_failed", "Zerct doctor failed.", failure["agent_instruction"])

    config = parse_config(project_dir / "zerct.toml")
    validate_config(config)
    run_shell(str(config["build"]["command"]), project_dir, "Build failed before preview.")
    if config["kind"] == "static_frontend":
        output = project_dir / str(config["build"]["output"])
        local_port = port or 4173
        print(f"preview http://127.0.0.1:{local_port}")
        subprocess.run([sys.executable, "-m", "http.server", str(local_port), "--bind", "127.0.0.1", "--directory", str(output)], check=False)
        return

    local_port = port or int(config["run"]["port"])
    print(f"preview http://127.0.0.1:{local_port}")
    env = {**os.environ, "PORT": str(local_port)}
    result = subprocess.run(str(config["run"]["command"]), cwd=project_dir, env=env, shell=True, check=False)
    if result.returncode != 0:
        raise AgentError("preview_failed", "Preview command exited with an error.", "Fix the local runtime command and retry `zerct preview`.")


def run_shell(command: str, project_dir: pathlib.Path, failure_message: str) -> None:
    print(command)
    result = subprocess.run(command, cwd=project_dir, shell=True, check=False)
    if result.returncode != 0:
        raise AgentError("command_failed", failure_message, "Fix the command output above, then retry.")


def run_doctor_workspace(project_dir: pathlib.Path) -> dict[str, Any]:
    if (project_dir / "zerct.toml").exists():
        return run_doctor(project_dir)

    projects = discover_deploy_projects(project_dir)
    if not projects:
        return run_doctor(project_dir)

    reports = []
    for project in projects:
        report = run_doctor(project.directory)
        report["relative"] = project.relative
        reports.append(report)
    return {
        "ok": all(report["ok"] for report in reports),
        "workspace": str(project_dir),
        "projects": reports,
    }


def run_doctor(project_dir: pathlib.Path) -> dict[str, Any]:
    checks: list[dict[str, Any]] = []
    config: dict[str, Any] | None = None
    config_valid = False
    config_path = project_dir / "zerct.toml"
    if config_path.exists():
        try:
            config = parse_config(config_path)
            validate_config(config)
            config_valid = True
            checks.append({"name": "zerct.toml", "ok": True, "message": "valid", "agent_instruction": ""})
        except (OSError, tomllib.TOMLDecodeError, AgentError) as error:
            instruction = (
                error.agent_instruction
                if isinstance(error, AgentError)
                else "Fix zerct.toml so it matches the Zerct deploy contract."
            )
            message = error.message if isinstance(error, AgentError) else str(error)
            checks.append(
                {
                    "name": "zerct.toml",
                    "ok": False,
                    "message": message,
                    "agent_instruction": instruction,
                }
            )
    else:
        checks.append(
            {
                "name": "zerct.toml",
                "ok": False,
                "message": "missing",
                "agent_instruction": "Create and commit zerct.toml, then retry.",
            }
        )

    kind = str(config.get("kind", "rust_backend")) if config else "rust_backend"
    required_files = ("package.json",) if kind == "static_frontend" else ("Cargo.toml", "Cargo.lock")
    for filename in required_files:
        exists = (project_dir / filename).exists()
        checks.append(
            {
                "name": filename,
                "ok": exists,
                "message": "found" if exists else "missing",
                "agent_instruction": f"Create and commit {filename}, then retry.",
            }
        )

    if kind == "static_frontend":
        has_lockfile = frontend_lockfile_exists(project_dir)
        checks.append(
            {
                "name": "frontend lockfile",
                "ok": has_lockfile,
                "message": "found" if has_lockfile else "missing",
                "agent_instruction": "Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.",
            }
        )
        checks.extend(frontend_source_checks(project_dir))
        checks.extend(frontend_script_checks(project_dir, config_valid))

    unsafe_hits = scan_unsafe(project_dir)
    checks.append(
        {
            "name": "unsafe",
            "ok": not unsafe_hits,
            "message": "no direct unsafe found" if not unsafe_hits else ", ".join(unsafe_hits[:5]),
            "agent_instruction": "Remove direct unsafe usage from workspace Rust source before deploying.",
        }
    )
    if kind == "rust_backend" and config_valid:
        checks.append(cargo_fmt(project_dir))
        checks.append(cargo_check(project_dir))
        checks.append(cargo_clippy(project_dir))

    return {
        "ok": all(check["ok"] for check in checks),
        "project": str(project_dir),
        "config": config,
        "checks": checks,
    }


def cargo_check(project_dir: pathlib.Path) -> dict[str, Any]:
    try:
        result = subprocess.run(
            ["cargo", "check", "--locked", "--quiet"],
            cwd=project_dir,
            env={**os.environ, "CARGO_TERM_COLOR": "never"},
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return {
            "name": "cargo check",
            "ok": False,
            "message": str(error),
            "agent_instruction": "Install Rust and Cargo, then run `cargo check --locked` locally before deploying.",
        }
    message = "passed" if result.returncode == 0 else (result.stderr or result.stdout or "cargo check failed").strip()[:240]
    return {
        "name": "cargo check",
        "ok": result.returncode == 0,
        "message": message,
        "agent_instruction": "Run `cargo check --locked`, fix every compiler error and warning, then redeploy.",
    }


def cargo_fmt(project_dir: pathlib.Path) -> dict[str, Any]:
    try:
        result = subprocess.run(
            ["cargo", "fmt", "--all", "--check"],
            cwd=project_dir,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return {
            "name": "cargo fmt",
            "ok": False,
            "message": str(error),
            "agent_instruction": "Install rustfmt with Rust, then run `cargo fmt --all --check` before deploying.",
        }
    message = "passed" if result.returncode == 0 else (result.stderr or result.stdout or "cargo fmt failed").strip()[:240]
    return {
        "name": "cargo fmt",
        "ok": result.returncode == 0,
        "message": message,
        "agent_instruction": "Run `cargo fmt --all`, then redeploy.",
    }


def cargo_clippy(project_dir: pathlib.Path) -> dict[str, Any]:
    try:
        result = subprocess.run(
            ["cargo", "clippy", "--locked", "--all-targets", "--all-features", "--quiet", "--", "-D", "warnings"],
            cwd=project_dir,
            env={**os.environ, "CARGO_TERM_COLOR": "never"},
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return {
            "name": "cargo clippy",
            "ok": False,
            "message": str(error),
            "agent_instruction": "Install Rust clippy, then run `cargo clippy --locked --all-targets --all-features -- -D warnings` before deploying.",
        }
    message = "passed" if result.returncode == 0 else (result.stderr or result.stdout or "cargo clippy failed").strip()[:240]
    return {
        "name": "cargo clippy",
        "ok": result.returncode == 0,
        "message": message,
        "agent_instruction": "Run `cargo clippy --locked --all-targets --all-features -- -D warnings`, fix every warning, then redeploy.",
    }


def parse_config(path: pathlib.Path) -> dict[str, Any]:
    config = tomllib.loads(path.read_text(encoding="utf-8"))
    config.setdefault("build", {})
    config.setdefault("run", {})
    config.setdefault("resources", {})
    config.setdefault("kind", "rust_backend")
    project_dir = path.parent
    config["build"].setdefault(
        "check",
        frontend_check_command(project_dir) if config["kind"] == "static_frontend" else DEFAULT_RUST_CHECK_COMMAND,
    )
    config["build"].setdefault(
        "command",
        frontend_build_command(project_dir) if config["kind"] == "static_frontend" else "cargo build --release",
    )
    if config["kind"] == "static_frontend":
        config["build"].setdefault("output", "dist")
    config["run"].setdefault("port", 3000)
    config["run"].setdefault("health", "/healthz")
    config["resources"].setdefault("memory", "512mb")
    config["resources"].setdefault("cpu", "0.25")
    config["resources"].setdefault("idle_timeout_minutes", 15)
    return config


def validate_config(config: dict[str, Any]) -> None:
    if not re.fullmatch(r"[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?", str(config.get("name", ""))):
        raise AgentError(
            "invalid_service_name",
            "Service name must be lowercase DNS-safe text.",
            "Set `name` in zerct.toml to lowercase letters, numbers, and hyphens only.",
        )
    if config.get("kind") not in ("rust_backend", "static_frontend"):
        raise AgentError(
            "invalid_project_kind",
            "Project kind must be rust_backend or static_frontend.",
            "Set kind in zerct.toml to rust_backend or static_frontend.",
        )
    if not isinstance(config["build"].get("command"), str) or not config["build"]["command"].strip():
        raise AgentError(
            "missing_command",
            "Build command is missing.",
            "Set [build].command in zerct.toml, then redeploy.",
        )
    if not isinstance(config["build"].get("check"), str) or not config["build"]["check"].strip():
        raise AgentError(
            "missing_command",
            "Check command is missing.",
            "Set [build].check in zerct.toml to a command that typechecks and lints before the release build.",
        )
    validate_check_command(str(config["kind"]), config["build"]["check"])
    if config["kind"] == "static_frontend":
        output = config["build"].get("output")
        if not isinstance(output, str) or not safe_relative_path(output):
            raise AgentError(
                "invalid_build_output",
                "Static frontend output must be a safe relative directory.",
                "Set [build].output to a relative directory like dist.",
            )
        return
    if config["build"].get("output"):
        raise AgentError(
            "invalid_build_output",
            "build.output is only valid for static frontend projects.",
            "Remove [build].output or set kind to static_frontend.",
        )
    if not config["run"].get("command"):
        raise AgentError(
            "missing_command",
            "A required command is missing.",
            "Set [run].command in zerct.toml to the release binary command.",
        )
    if not isinstance(config["run"].get("port"), int) or not 1 <= config["run"]["port"] <= 65535:
        raise AgentError(
            "invalid_port",
            "Port must be between 1 and 65535.",
            "Set [run].port in zerct.toml to the local HTTP port.",
        )
    health = config["run"].get("health")
    if not isinstance(health, str) or not health.startswith("/"):
        raise AgentError(
            "invalid_health_endpoint",
            "Health endpoint must be an absolute path.",
            "Set [run].health to a short absolute path such as `/healthz`.",
        )


def validate_check_command(kind: str, command: str) -> None:
    if kind == "static_frontend":
        validate_frontend_check_command(command)
        return
    required = ("cargo fmt --all --check", "cargo check --locked", "cargo clippy --locked", "--all-targets", "--all-features", "-D warnings")
    if all(fragment in command for fragment in required):
        return
    raise AgentError(
        "policy_rejected",
        "Check command is too weak for Zerct deploys.",
        "Set [build].check to include `cargo fmt --all --check`, `cargo check --locked`, and `cargo clippy --locked --all-targets --all-features -- -D warnings`, then redeploy.",
    )


def validate_frontend_check_command(command: str) -> None:
    if uses_javascript_linter(command):
        raise AgentError(
            "policy_rejected",
            "Check command uses a JavaScript-based linter.",
            "Use native frontend linting such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then redeploy.",
        )
    tokens = command_tokens(command)
    if has_frontend_install_command(tokens) and has_frontend_script_run(tokens, "typecheck") and has_frontend_script_run(tokens, "lint"):
        return
    raise AgentError(
        "policy_rejected",
        "Check command is too weak for Zerct deploys.",
        "Set [build].check to install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`, then redeploy.",
    )


def frontend_lockfile_exists(project_dir: pathlib.Path) -> bool:
    return any(
        (project_dir / filename).exists()
        for filename in ("package-lock.json", "npm-shrinkwrap.json", "pnpm-lock.yaml", "yarn.lock", "bun.lock", "bun.lockb")
    )


def frontend_package_manager(project_dir: pathlib.Path) -> str:
    if (project_dir / "bun.lock").exists() or (project_dir / "bun.lockb").exists():
        return "bun"
    return "npm"


def frontend_check_command(project_dir: pathlib.Path) -> str:
    if frontend_package_manager(project_dir) == "bun":
        return DEFAULT_BUN_FRONTEND_CHECK_COMMAND
    return DEFAULT_NPM_FRONTEND_CHECK_COMMAND


def frontend_build_command(project_dir: pathlib.Path) -> str:
    if frontend_package_manager(project_dir) == "bun":
        return "bun run build"
    return "npm run build"


def frontend_script_checks(project_dir: pathlib.Path, run_scripts: bool) -> list[dict[str, Any]]:
    manifest = read_package_json(project_dir)

    def has_script(name: str) -> bool:
        scripts = manifest.get("scripts") if isinstance(manifest, dict) else None
        return isinstance(scripts, dict) and isinstance(scripts.get(name), str) and bool(scripts[name].strip())

    checks = [
        {
            "name": f"package script {script}",
            "ok": has_script(script),
            "message": "found" if has_script(script) else "missing",
            "agent_instruction": f'Add a non-empty "{script}" script to package.json, then retry.',
        }
        for script in ("typecheck", "lint")
    ]
    lint_script = ""
    if isinstance(manifest, dict) and isinstance(manifest.get("scripts"), dict):
        raw_lint_script = manifest["scripts"].get("lint")
        lint_script = raw_lint_script if isinstance(raw_lint_script, str) else ""
    native_lint = not lint_script or not uses_javascript_linter(lint_script)
    checks.append(
        {
            "name": "native frontend lint",
            "ok": native_lint,
            "message": "accepted" if native_lint else "JavaScript linter found",
            "agent_instruction": "Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.",
        }
    )
    if run_scripts and all(check["ok"] for check in checks):
        checks.append(package_script_check(project_dir, "typecheck"))
        checks.append(package_script_check(project_dir, "lint"))
    return checks


def frontend_source_checks(project_dir: pathlib.Path) -> list[dict[str, Any]]:
    source = frontend_source_report(project_dir)
    return [
        {
            "name": "typescript source",
            "ok": bool(source["typescript"]),
            "message": ", ".join(source["typescript"][:3]) if source["typescript"] else "missing",
            "agent_instruction": "Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry.",
        },
        {
            "name": "javascript source",
            "ok": not source["javascript"],
            "message": "none found" if not source["javascript"] else ", ".join(source["javascript"][:5]),
            "agent_instruction": "Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.",
        },
    ]


def frontend_source_report(project_dir: pathlib.Path) -> dict[str, list[str]]:
    source: dict[str, list[str]] = {"typescript": [], "javascript": []}
    for _item, relative in iter_project_files(project_dir):
        if not is_frontend_source_path(relative):
            continue
        relative_text = relative.as_posix()
        if is_frontend_typescript_source(relative_text):
            source["typescript"].append(relative_text)
        elif is_frontend_javascript_source(relative_text):
            source["javascript"].append(relative_text)
    return source


def is_frontend_source_path(relative: pathlib.Path) -> bool:
    return bool(relative.parts) and relative.parts[0] in {"src", "app", "pages", "routes", "components"}


def is_frontend_typescript_source(relative: str) -> bool:
    return not relative.endswith(".d.ts") and relative.endswith((".ts", ".tsx"))


def is_frontend_javascript_source(relative: str) -> bool:
    return relative.endswith((".js", ".jsx", ".mjs", ".cjs"))


def read_package_json(project_dir: pathlib.Path) -> dict[str, Any] | None:
    try:
        raw = (project_dir / "package.json").read_text(encoding="utf-8")
        parsed = json.loads(raw)
    except (OSError, json.JSONDecodeError):
        return None
    return parsed if isinstance(parsed, dict) else None


def uses_javascript_linter(command: str) -> bool:
    tokens = command_tokens(command)
    for index, token in enumerate(tokens):
        command_name = command_basename(token)
        if command_name in {"eslint", "eslint_d", "standard", "xo"}:
            return True
        if command_name == "next" and index + 1 < len(tokens) and tokens[index + 1] == "lint":
            return True
    return False


def command_tokens(command: str) -> list[str]:
    return [
        token.strip("\"'")
        for token in re.split(r"[\s&|;()]+", command)
        if token.strip("\"'")
    ]


def command_basename(token: str) -> str:
    return token.rsplit("/", 1)[-1]


def has_frontend_install_command(tokens: list[str]) -> bool:
    return any(
        (command_basename(command), subcommand)
        in {
            ("npm", "ci"),
            ("bun", "ci"),
            ("bun", "install"),
            ("pnpm", "install"),
            ("yarn", "install"),
        }
        for command, subcommand in zip(tokens, tokens[1:], strict=False)
    )


def has_frontend_script_run(tokens: list[str], script: str) -> bool:
    managers = {"npm", "bun", "pnpm", "yarn"}
    for index, token in enumerate(tokens):
        if command_basename(token) not in managers:
            continue
        if index + 2 < len(tokens) and tokens[index + 1] == "run" and tokens[index + 2] == script:
            return True
        if (
            index + 3 < len(tokens)
            and tokens[index + 1] == "run"
            and tokens[index + 2].startswith("-")
            and tokens[index + 3] == script
        ):
            return True
    return False


def package_script_check(project_dir: pathlib.Path, script: str) -> dict[str, Any]:
    manager = frontend_package_manager(project_dir)
    command = [manager, "run", script] if manager == "bun" else [manager, "run", "--silent", script]
    try:
        result = subprocess.run(
            command,
            cwd=project_dir,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return {
            "name": f"{manager} run {script}",
            "ok": False,
            "message": str(error),
            "agent_instruction": f"Install {'Bun' if manager == 'bun' else 'Node.js and npm'}, then run `{manager} run {script}` before deploying.",
        }
    message = "passed" if result.returncode == 0 else (result.stderr or result.stdout or f"{manager} run {script} failed").strip()[:240]
    return {
        "name": f"{manager} run {script}",
        "ok": result.returncode == 0,
        "message": message,
        "agent_instruction": f"Run `{manager} run {script}`, fix every error, then redeploy.",
    }


def safe_relative_path(value: str) -> bool:
    return (
        bool(value)
        and not pathlib.PurePosixPath(value).is_absolute()
        and "\\" not in value
        and all(part and part not in (".", "..") for part in value.split("/"))
    )


def discover_deploy_projects(root_dir: pathlib.Path) -> list[DeployProject]:
    if not root_dir.is_dir():
        raise AgentError(
            "missing_project",
            "Project directory does not exist.",
            "Run Zerct from the root of a Rust project or pass the project path.",
        )
    if (root_dir / "zerct.toml").exists():
        return [deploy_project_info(root_dir, root_dir)]

    project_dirs: list[pathlib.Path] = []
    discover_project_dirs(root_dir, project_dirs)
    return sorted(
        (deploy_project_info(project_dir, root_dir) for project_dir in project_dirs),
        key=lambda project: (kind_order(project.kind), project.relative),
    )


def discover_project_dirs(directory: pathlib.Path, project_dirs: list[pathlib.Path]) -> None:
    for item in sorted(directory.iterdir(), key=lambda path: path.name):
        if item.is_symlink() or not item.is_dir() or item.name in WORKSPACE_EXCLUDED_PARTS:
            continue
        if (item / "zerct.toml").exists():
            project_dirs.append(item)
            continue
        discover_project_dirs(item, project_dirs)


def deploy_project_info(project_dir: pathlib.Path, root_dir: pathlib.Path) -> DeployProject:
    relative = project_dir.relative_to(root_dir).as_posix() if project_dir != root_dir else "."
    try:
        config = parse_config(project_dir / "zerct.toml")
        name = str(config.get("name", ""))
        kind = str(config.get("kind", "unknown"))
    except AgentError:
        name = ""
        kind = "unknown"
    return DeployProject(project_dir, relative, name, kind)


def kind_order(kind: str) -> int:
    if kind == "rust_backend":
        return 0
    if kind == "static_frontend":
        return 1
    return 2


def login(args: argparse.Namespace) -> None:
    token = args.token
    if token:
        write_session_token(token)
        print("saved Zerct session token")
        return

    login_and_store(args)


def deploy(project_dir: pathlib.Path, args: argparse.Namespace) -> None:
    projects = discover_deploy_projects(project_dir)
    if not projects:
        raise AgentError(
            "missing_project_contract",
            "No zerct.toml was found.",
            "Run `zerct init` in each app directory, or pass a project path.",
        )

    if len(projects) == 1:
        project = projects[0]
        if project.kind == "static_frontend" and args.database:
            raise AgentError(
                "invalid_database_target",
                "Static frontends cannot attach managed Postgres directly.",
                "Deploy a Rust backend with managed Postgres and call it from the frontend.",
            )
        token = read_or_login_token(project.directory, args)
        preflight_deploy_limits([project], args, token, args.database)
        response = deploy_project(project.directory, args, token, args.database)
        if args.wait:
            response["final_build"] = wait_for_build(args, token, response["build_job"]["id"])
        print_deploy_response(response, args)
        return

    token = read_or_login_token(project_dir, args)
    preflight_deploy_limits(projects, args, token, args.database)
    results: list[tuple[DeployProject, bool, dict[str, Any]]] = []
    if not args.json:
        print(f"deploying {len(projects)} projects")
    for project in projects:
        wants_database = args.database and project.kind == "rust_backend"
        if not args.json:
            print(f"checking {project.relative}")
        response = deploy_project(project.directory, args, token, wants_database)
        results.append((project, wants_database, response))
        if not args.json:
            print(f"{project.relative} queued {response['build_job']['id']}")
            print(f"{project.relative} url {response['app']['url']}")

    if args.wait:
        wait_for_workspace_builds(args, token, results)

    print_workspace_deploy_response(project_dir, results, args)


def preflight_deploy_limits(
    projects: list[DeployProject],
    args: argparse.Namespace,
    token: str,
    database_requested: bool,
) -> None:
    usage_response = api_request(args, "GET", "/v1/usage", token, None)
    apps_response = api_request(args, "GET", "/v1/apps", token, None)
    usage = usage_response.get("usage", {})
    limits = usage_response.get("limits", {})
    apps = apps_response.get("apps", [])
    existing = {
        str(app.get("name", "")): app
        for app in apps
        if isinstance(app, dict)
    }
    new_projects = 0
    new_databases = 0

    for project in projects:
        if not project.name or project.kind == "unknown":
            continue
        app = existing.get(project.name)
        if app is None:
            new_projects += 1
        if database_requested and project.kind == "rust_backend" and not (app or {}).get("databaseStorageMib"):
            new_databases += 1

    app_count = int(usage.get("appCount", 0))
    project_limit = int(limits.get("projects", 0))
    if new_projects > 0 and app_count + new_projects > project_limit:
        raise AgentError(
            "payment_required",
            f"Project limit reached: {app_count}/{project_limit} projects are already used.",
            "Redeploy an existing app by reusing its `name` in zerct.toml, or run `zerct billing` to open Stripe Checkout before creating another project.",
        )

    database_count = int(usage.get("databaseCount", 0))
    database_limit = int(limits.get("managedDatabases", 0))
    if new_databases > 0 and database_count + new_databases > database_limit:
        raise AgentError(
            "payment_required",
            f"Managed Postgres limit reached: {database_count}/{database_limit} databases are already used.",
            "Redeploy an app that already has managed Postgres, deploy without `--database`, or run `zerct billing` to open Stripe Checkout.",
        )


def deploy_project(project_dir: pathlib.Path, args: argparse.Namespace, token: str, wants_database: bool) -> dict[str, Any]:
    report = run_doctor(project_dir)
    if not report["ok"]:
        failure = next(check for check in report["checks"] if not check["ok"])
        raise AgentError("doctor_failed", "Zerct doctor failed.", failure["agent_instruction"])

    return api_request(
        args,
        "POST",
        "/v1/deploy",
        token,
        {
            "config": report["config"],
            "commit_sha": git_commit_sha(project_dir),
            "wants_database": wants_database,
            "source_archive_base64": archive_base64(project_dir),
        },
    )


def print_deploy_response(response: dict[str, Any], args: argparse.Namespace) -> None:
    if args.json:
        print(json.dumps(response, indent=2))
        return

    print(f"queued {response['build_job']['id']}")
    print(f"app {response['app']['id']}")
    print(f"url {response['app']['url']}")
    print(f"next zerct logs --app {response['app']['id']}")


def print_workspace_deploy_response(
    project_dir: pathlib.Path,
    results: list[tuple[DeployProject, bool, dict[str, Any]]],
    args: argparse.Namespace,
) -> None:
    if args.json:
        print(
            json.dumps(
                {
                    "workspace": str(project_dir),
                    "deploys": [
                        {
                            "path": project.relative,
                            "kind": project.kind,
                            "wants_database": wants_database,
                            "app": response["app"],
                            "build_job": response["build_job"],
                            "final_build": response.get("final_build"),
                        }
                        for project, wants_database, response in results
                    ],
                },
                indent=2,
            )
        )
        return

    if results:
        print(f"next zerct logs --app {results[0][2]['app']['id']}")


def wait_for_workspace_builds(
    args: argparse.Namespace,
    token: str,
    results: list[tuple[DeployProject, bool, dict[str, Any]]],
) -> None:
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(results)) as executor:
        pending = {
            executor.submit(wait_for_build, args, token, response["build_job"]["id"]): response
            for _project, _wants_database, response in results
        }
        for future in concurrent.futures.as_completed(pending):
            pending[future]["final_build"] = future.result()


def wait_for_build(args: argparse.Namespace, token: str, build_id: str) -> dict[str, Any]:
    deadline = time.time() + int(args.wait_timeout)
    last_status = ""
    while time.time() <= deadline:
        response = api_request(args, "GET", f"/v1/builds/{build_id}", token, None)
        build = response.get("build", {})
        status = build.get("status")
        if not status:
            raise AgentError(
                "build_status_unavailable",
                "Build status is unavailable.",
                f"Retry with `zerct logs --build {build_id}`.",
            )
        if status != last_status:
            progress(args, f"build {build_id} {status}")
            last_status = status
        if status in {"succeeded", "failed", "canceled"}:
            return build
        time.sleep(3)

    raise AgentError(
        "build_wait_timeout",
        f"Timed out waiting for build {build_id}.",
        f"Run `zerct logs --build {build_id}` to continue watching.",
    )


def logs(args: argparse.Namespace) -> None:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    page = page_query(args)
    if args.build:
        response = api_request(args, "GET", f"/v1/builds/{urllib.parse.quote(args.build)}/logs{page}", token, None)
    elif args.deploy:
        response = api_request(args, "GET", f"/v1/deploys/{urllib.parse.quote(args.deploy)}/logs{page}", token, None)
    elif args.app:
        response = api_request(args, "GET", f"/v1/apps/{urllib.parse.quote(args.app)}/logs{page}", token, None)
    else:
        raise AgentError(
            "missing_app",
            "App, deploy, or build id is required.",
            "Pass `--app <app>`, `--deploy <deploy_id>`, or `--build <build_id>`.",
        )
    if args.json:
        print(json.dumps(response, indent=2))
        return
    for line in response.get("lines", []):
        print(f"[{line['timestamp']}] {line['stream']}: {line['message']}")
    if response.get("has_more") and response.get("next_cursor"):
        target = f"--build {args.build}" if args.build else f"--deploy {args.deploy}" if args.deploy else f"--app {args.app}"
        print(f"next zerct logs {target} --cursor {response['next_cursor']}")


def app_get(args: argparse.Namespace, route: str) -> dict[str, Any]:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    return api_request(args, "GET", f"/v1/apps/{urllib.parse.quote(args.app)}/{route}", token, None)


def authenticated_get(args: argparse.Namespace, route: str) -> dict[str, Any]:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    return api_request(args, "GET", route, token, None)


def page_query(args: argparse.Namespace) -> str:
    params: dict[str, str] = {}
    if getattr(args, "limit", ""):
        params["limit"] = str(args.limit)
    if getattr(args, "cursor", ""):
        params["cursor"] = str(args.cursor)
    encoded = urllib.parse.urlencode(params)
    return f"?{encoded}" if encoded else ""


def deploys(args: argparse.Namespace) -> None:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    route = (
        f"/v1/apps/{urllib.parse.quote(args.app)}/deploys{page_query(args)}"
        if args.app
        else f"/v1/deploys{page_query(args)}"
    )
    print_response(api_request(args, "GET", route, token, None), args.json)


def builds(args: argparse.Namespace) -> None:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    route = (
        f"/v1/apps/{urllib.parse.quote(args.app)}/builds{page_query(args)}"
        if args.app
        else f"/v1/builds{page_query(args)}"
    )
    print_response(api_request(args, "GET", route, token, None), args.json)


def env_command(args: argparse.Namespace) -> None:
    if args.action == "list":
        print_response(app_get(args, "env"), args.json)
        return

    token = read_or_login_token(pathlib.Path.cwd(), args)
    if args.action == "delete":
        if not args.assignment:
            raise AgentError(
                "invalid_env",
                "Environment variable name is required.",
                "Use `zerct env delete --app <app> KEY`.",
            )
        response = api_request(
            args,
            "DELETE",
            f"/v1/apps/{urllib.parse.quote(args.app)}/env/{urllib.parse.quote(args.assignment)}",
            token,
            None,
        )
        print_response(response, args.json)
        return

    assignment = args.assignment or ""
    name, separator, value = assignment.partition("=")
    if not separator:
        raise AgentError(
            "invalid_env",
            "Environment assignment must be KEY=value.",
            "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
        )
    response = api_request(args, "PUT", f"/v1/apps/{urllib.parse.quote(args.app)}/env", token, {"name": name, "value": value})
    print_response(response, args.json)


def domains_command(args: argparse.Namespace) -> None:
    if args.action == "list":
        print_response(app_get(args, "domains"), args.json)
        return

    if not args.domain:
        raise AgentError(
            "missing_domain",
            "Domain is required.",
            "Use `zerct domains add --app <app> api.example.com`.",
        )

    token = read_or_login_token(pathlib.Path.cwd(), args)
    app = urllib.parse.quote(args.app)
    domain = urllib.parse.quote(args.domain)
    match args.action:
        case "add":
            response = api_request(args, "POST", f"/v1/apps/{app}/domains", token, {"domain": args.domain})
        case "verify":
            response = api_request(args, "POST", f"/v1/apps/{app}/domains/{domain}/verify", token, None)
        case "delete":
            response = api_request(args, "DELETE", f"/v1/apps/{app}/domains/{domain}", token, None)
        case _:
            raise AgentError(
                "unknown_command",
                "Unknown domains command.",
                "Use `domains list`, `domains add`, `domains verify`, or `domains delete`.",
            )
    print_response(response, args.json)


def billing(args: argparse.Namespace) -> None:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    if args.action == "portal":
        response = api_request(args, "POST", "/v1/billing/portal", token, None)
        if args.json:
            print(json.dumps(response, indent=2))
            return
        print(response["checkout"]["url"])
        webbrowser.open(response["checkout"]["url"])
        return

    response = api_request(
        args,
        "POST",
        "/v1/billing/checkout",
        token,
        {"target_plan": "pro", "reason": "Upgrade to Zerct Pro."},
    )
    if args.json:
        print(json.dumps(response, indent=2))
        return
    print(response["checkout"]["url"])
    webbrowser.open(response["checkout"]["url"])


def read_or_login_token(project_dir: pathlib.Path, args: argparse.Namespace) -> str:
    token = read_stored_token(project_dir, args)
    if token:
        return token

    return login_and_store(args)


def login_and_store(args: argparse.Namespace) -> str:
    start = api_request(args, "POST", "/v1/login/device", None, None)
    login_url = str(start.get("loginUrl") or start.get("login_url") or "")
    if not login_url:
        raise AgentError(
            "login_failed",
            "Zerct login did not return a browser URL.",
            "Retry `zerct login`. If it keeps failing, check Zerct status.",
        )
    webbrowser.open(login_url)
    progress(args, "opened browser login")
    progress(args, f"waiting for browser login code {start.get('userCode') or start.get('user_code', 'ZERCT')}")

    session = poll_login(args, start)
    token = str(session.get("token", "")).strip()
    if not token:
        raise AgentError(
            "login_failed",
            "Zerct login did not return a session token.",
            "Run `zerct login` again and complete the browser login.",
        )

    write_session_token(token)
    progress(args, f"logged in as {session.get('email', 'Zerct user')}")
    return token


def poll_login(args: argparse.Namespace, start: dict[str, Any]) -> dict[str, Any]:
    device_code = str(start.get("deviceCode") or start.get("device_code", "")).strip()
    if not device_code:
        raise AgentError(
            "login_failed",
            "Zerct login did not return a device code.",
            "Retry `zerct login`. If it keeps failing, check Zerct status.",
        )

    expires = int(start.get("expiresInSeconds") or start.get("expires_in_seconds") or DEFAULT_LOGIN_EXPIRES_SECONDS)
    interval = int(start.get("intervalSeconds") or start.get("interval_seconds") or DEFAULT_LOGIN_INTERVAL_SECONDS)
    deadline = time.monotonic() + expires

    while time.monotonic() < deadline:
        time.sleep(max(interval, DEFAULT_LOGIN_INTERVAL_SECONDS))
        response = api_request(args, "GET", f"/v1/login/device/{device_code}", None, None)
        status = response.get("status")
        if status == "complete":
            return response
        if status == "expired":
            raise AgentError(
                "login_expired",
                "Zerct login expired before it completed.",
                "Run `zerct login` again and finish the browser login in the newly opened tab.",
            )
        interval = int(response.get("intervalSeconds") or response.get("interval_seconds") or DEFAULT_LOGIN_INTERVAL_SECONDS)

    raise AgentError(
        "login_expired",
        "Zerct login expired before it completed.",
        "Run `zerct login` again and finish the browser login in the newly opened tab.",
    )


def api_request(
    args: argparse.Namespace,
    method: str,
    route: str,
    token: str | None,
    body: dict[str, Any] | None,
) -> dict[str, Any]:
    headers = {"Accept": "application/json", "User-Agent": f"zerct-python/{__version__}"}
    data = None
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if body is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(body).encode("utf-8")

    request = urllib.request.Request(f"{args.api}{route}", data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        payload = json.loads(error.read().decode("utf-8"))
        raise AgentError(
            payload.get("code", "api_error"),
            payload.get("message", f"Zerct API returned HTTP {error.code}."),
            payload.get("agent_instruction", "Retry the command. If it keeps failing, check Zerct status."),
            payload.get("docs_url"),
            payload.get("checkout_url"),
        ) from error
    except urllib.error.URLError as error:
        raise AgentError(
            "api_unavailable",
            "Zerct API is unavailable.",
            "Retry the command. If it keeps failing, check Zerct status before changing your project.",
        ) from error


def archive_base64(project_dir: pathlib.Path) -> str:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
        for item, relative in iter_project_files(project_dir):
            archive.add(item, arcname=relative)

    raw = buffer.getvalue()
    if len(raw) > ARCHIVE_LIMIT_BYTES:
        raise AgentError(
            "archive_too_large",
            "Source archive is too large.",
            "Remove build outputs, target directories, logs, and local caches before deploying.",
        )
    return base64.b64encode(raw).decode("ascii")


def should_exclude(relative: pathlib.Path) -> bool:
    parts = relative.parts
    return (
        bool(set(parts) & EXCLUDED_PARTS)
        or (".config", "gcloud") in zip(parts, parts[1:])
        or relative.name.startswith(".env")
        or relative.name.startswith("._")
        or relative.name in EXCLUDED_NAMES
        or relative.name.endswith(EXCLUDED_SUFFIXES)
    )


def iter_project_files(project_dir: pathlib.Path) -> Iterator[tuple[pathlib.Path, pathlib.Path]]:
    stack = [project_dir]
    while stack:
        directory = stack.pop()
        for item in directory.iterdir():
            relative = item.relative_to(project_dir)
            if should_exclude(relative) or item.is_symlink():
                continue
            if item.is_dir():
                stack.append(item)
            elif item.is_file():
                yield item, relative


def scan_unsafe(project_dir: pathlib.Path) -> list[str]:
    hits: list[str] = []
    for item, relative in iter_project_files(project_dir):
        if item.suffix != ".rs":
            continue
        if re.search(r"\bunsafe\b", item.read_text(encoding="utf-8", errors="ignore")):
            hits.append(str(relative))
    return hits


def read_stored_token(project_dir: pathlib.Path, args: argparse.Namespace) -> str:
    if args.token:
        return args.token
    if os.environ.get("ZERCT_TOKEN"):
        return os.environ["ZERCT_TOKEN"]

    keychain_token = read_keychain_token()
    if keychain_token:
        return keychain_token

    for candidate in (
        user_session_path(),
        project_dir / SESSION_DIR / SESSION_FILE,
        pathlib.Path.home() / SESSION_DIR / SESSION_FILE,
    ):
        if candidate.exists():
            return candidate.read_text(encoding="utf-8").strip()
    return ""


def write_session_token(token: str) -> None:
    clean_token = token.strip()
    if not clean_token:
        raise AgentError(
            "login_failed",
            "Zerct session token is empty.",
            "Run `zerct login` again and complete the browser login.",
        )
    if write_keychain_token(clean_token):
        return

    token_path = user_session_path()
    token_path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    token_path.write_text(clean_token + "\n", encoding="utf-8")
    token_path.chmod(0o600)


def user_session_path() -> pathlib.Path:
    if sys.platform == "win32" and os.environ.get("APPDATA"):
        return pathlib.Path(os.environ["APPDATA"]) / "Zerct" / SESSION_FILE
    config_home = pathlib.Path(os.environ.get("XDG_CONFIG_HOME", pathlib.Path.home() / ".config"))
    return config_home / "zerct" / SESSION_FILE


def read_keychain_token() -> str:
    if sys.platform == "darwin":
        result = subprocess.run(
            ["security", "find-generic-password", "-s", SESSION_SERVICE, "-a", SESSION_ACCOUNT, "-w"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        return result.stdout.strip() if result.returncode == 0 else ""

    if sys.platform.startswith("linux") and shutil.which("secret-tool"):
        result = subprocess.run(
            ["secret-tool", "lookup", "service", SESSION_SERVICE, "account", SESSION_ACCOUNT],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        return result.stdout.strip() if result.returncode == 0 else ""

    return ""


def write_keychain_token(token: str) -> bool:
    if sys.platform == "darwin":
        result = subprocess.run(
            [
                "security",
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
            ],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return result.returncode == 0

    if sys.platform.startswith("linux") and shutil.which("secret-tool"):
        result = subprocess.run(
            [
                "secret-tool",
                "store",
                "--label",
                SESSION_LABEL,
                "service",
                SESSION_SERVICE,
                "account",
                SESSION_ACCOUNT,
            ],
            input=token,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        return result.returncode == 0

    return False


def git_commit_sha(project_dir: pathlib.Path) -> str | None:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=project_dir,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    return result.stdout.strip() if result.returncode == 0 else None


def service_name_from_dir(project_dir: pathlib.Path) -> str:
    name = service_name_from_value(project_dir.name)
    return name or "api"


def service_name_from_value(value: str) -> str:
    return re.sub(r"[^a-z0-9-]+", "-", value.lower()).strip("-")[:48]


def service_name_from_cargo(project_dir: pathlib.Path) -> str:
    try:
        source = (project_dir / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return ""
    match = re.search(r'^\s*name\s*=\s*"([^"]+)"', source, flags=re.MULTILINE)
    return service_name_from_value(match.group(1)) if match else ""


def service_name_from_package(project_dir: pathlib.Path) -> str:
    manifest = read_package_json(project_dir)
    name = manifest.get("name") if isinstance(manifest, dict) else ""
    return service_name_from_value(name) if isinstance(name, str) else ""


def infer_project_kind(project_dir: pathlib.Path) -> str:
    if (project_dir / "Cargo.toml").exists():
        return "rust_backend"
    if (project_dir / "package.json").exists():
        return "static_frontend"
    return "rust_backend"


def frontend_template_source(api_base_url: str) -> str:
    return f"""import {{ createRootRoute, createRouter, RouterProvider }} from '@tanstack/react-router'
import {{ createRoot }} from 'react-dom/client'
import './styles.css'

const apiBaseUrl = import.meta.env.VITE_API_URL ?? '{api_base_url}'

function App() {{
  return (
    <main>
      <section>
        <h1>Zerct TanStack Frontend</h1>
        <p>Static runtime, dynamic Rust backend calls.</p>
        <code>{{apiBaseUrl}}</code>
      </section>
    </main>
  )
}}

const rootRoute = createRootRoute({{ component: App }})
const router = createRouter({{ routeTree: rootRoute }})

declare module '@tanstack/react-router' {{
  interface Register {{
    router: typeof router
  }}
}}

createRoot(document.getElementById('root')!).render(<RouterProvider router={{router}} />)
"""


def print_response(response: dict[str, Any], json_output: bool) -> None:
    print(json.dumps(response, indent=2 if json_output else 2))


def progress(args: argparse.Namespace, message: str) -> None:
    if args.json:
        print(message, file=sys.stderr)
        return
    print(message)


def print_error(error: AgentError, json_output: bool) -> None:
    if error.already_reported:
        return
    if json_output:
        print(json.dumps(error.payload(), indent=2), file=sys.stderr)
        return
    print(error.message, file=sys.stderr)
    print(f"agent_instruction: {error.agent_instruction}", file=sys.stderr)
    if error.docs_url:
        print(f"docs: {error.docs_url}", file=sys.stderr)
    if error.checkout_url:
        print(f"checkout: {error.checkout_url}", file=sys.stderr)
