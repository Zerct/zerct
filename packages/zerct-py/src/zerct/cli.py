"""Small dependency-free Zerct CLI for Python users."""

from __future__ import annotations

import argparse
import base64
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
import urllib.request
import webbrowser
from dataclasses import dataclass
from typing import Any

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
DEFAULT_RUST_CHECK_COMMAND = "cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings"
DEFAULT_FRONTEND_CHECK_COMMAND = "npm run typecheck && npm run lint"
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


@dataclass(frozen=True)
class AgentError(Exception):
    code: str
    message: str
    agent_instruction: str
    docs_url: str | None = None
    checkout_url: str | None = None

    def payload(self) -> dict[str, Any]:
        return {
            "code": self.code,
            "message": self.message,
            "agent_instruction": self.agent_instruction,
            "docs_url": self.docs_url,
            "checkout_url": self.checkout_url,
        }


def main(argv: list[str] | None = None) -> None:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        run(args)
    except AgentError as error:
        print_error(error, args.json)
        raise SystemExit(1) from error


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="zerct")
    parser.add_argument("--version", action="version", version=__version__)
    parser.add_argument("--json", action="store_true", help="print JSON")
    parser.add_argument("--api", default=DEFAULT_API_URL, help="Zerct API URL")
    parser.add_argument("--token", default="", help="Zerct session token")

    subcommands = parser.add_subparsers(dest="command", required=True)
    init = subcommands.add_parser("init")
    init.add_argument("path", nargs="?", default=".")

    install = subcommands.add_parser("install")
    install.add_argument("path", nargs="?", default=".")

    doctor = subcommands.add_parser("doctor")
    doctor.add_argument("path", nargs="?", default=".")

    login = subcommands.add_parser("login")
    login.add_argument("--token", default="", help="Zerct session token")

    deploy = subcommands.add_parser("deploy")
    deploy.add_argument("path", nargs="?", default=".")
    deploy.add_argument("--database", action="store_true")

    for command in ("logs", "status", "inspect", "db"):
        item = subcommands.add_parser(command)
        item.add_argument("--app", required=True)

    env = subcommands.add_parser("env")
    env.add_argument("action", choices=["set"])
    env.add_argument("assignment")
    env.add_argument("--app", required=True)

    subcommands.add_parser("billing")
    return parser


def run(args: argparse.Namespace) -> None:
    args.api = args.api.rstrip("/")
    match args.command:
        case "init":
            init_project(pathlib.Path(args.path).resolve())
        case "install":
            init_project(pathlib.Path(args.path).resolve())
            doctor_project(pathlib.Path(args.path).resolve(), args.json)
        case "doctor":
            doctor_project(pathlib.Path(args.path).resolve(), args.json)
        case "login":
            login(args)
        case "deploy":
            deploy(pathlib.Path(args.path).resolve(), args)
        case "logs":
            logs(args)
        case "status":
            print_response(app_get(args, "status"), args.json)
        case "inspect":
            print_response(app_get(args, "inspect"), args.json)
        case "db":
            print_response(app_get(args, "database"), args.json)
        case "env":
            set_env(args)
        case "billing":
            billing(args)
        case _:
            raise AgentError(
                "unknown_command",
                "Unknown Zerct command.",
                "Run `zerct --help` and retry with a supported command.",
            )


def init_project(project_dir: pathlib.Path) -> None:
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

    name = service_name_from_dir(project_dir)
    config_path.write_text(
        f"""name = "{name}"

[build]
command = "cargo build --release"

[run]
command = "./target/release/{name}"
port = 3000
health = "/healthz"

[resources]
memory = "512mb"
cpu = "0.25"
idle_timeout_minutes = 15
""",
        encoding="utf-8",
    )
    print(f"created {config_path}")


def doctor_project(project_dir: pathlib.Path, json_output: bool) -> None:
    report = run_doctor(project_dir)
    if json_output:
        print(json.dumps(report, indent=2))
    else:
        for check in report["checks"]:
            state = "ok" if check["ok"] else "fail"
            print(f"{state} {check['name']} - {check['message']}")

    if not report["ok"]:
        failure = next(check for check in report["checks"] if not check["ok"])
        raise AgentError(
            "doctor_failed",
            "Zerct doctor failed.",
            failure["agent_instruction"],
        )


def run_doctor(project_dir: pathlib.Path) -> dict[str, Any]:
    checks: list[dict[str, Any]] = []
    config: dict[str, Any] | None = None
    config_path = project_dir / "zerct.toml"
    if config_path.exists():
        try:
            config = parse_config(config_path)
            validate_config(config)
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
        checks.extend(frontend_script_checks(project_dir))

    unsafe_hits = scan_unsafe(project_dir)
    checks.append(
        {
            "name": "unsafe",
            "ok": not unsafe_hits,
            "message": "no direct unsafe found" if not unsafe_hits else ", ".join(unsafe_hits[:5]),
            "agent_instruction": "Remove direct unsafe usage from workspace Rust source before deploying.",
        }
    )
    if kind == "rust_backend":
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
    config["build"].setdefault(
        "check",
        DEFAULT_FRONTEND_CHECK_COMMAND if config["kind"] == "static_frontend" else DEFAULT_RUST_CHECK_COMMAND,
    )
    config["build"].setdefault(
        "command",
        "npm ci && npm run build" if config["kind"] == "static_frontend" else "cargo build --release",
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
    required = (
        ("typecheck", "lint")
        if kind == "static_frontend"
        else ("cargo check --locked", "cargo clippy --locked", "--all-targets", "--all-features", "-D warnings")
    )
    if all(fragment in command for fragment in required):
        return
    if kind == "static_frontend":
        raise AgentError(
            "policy_rejected",
            "Check command is too weak for Zerct deploys.",
            "Set [build].check to run both frontend typechecking and linting, for example `npm run typecheck && npm run lint`, then redeploy.",
        )
    raise AgentError(
        "policy_rejected",
        "Check command is too weak for Zerct deploys.",
        "Set [build].check to include `cargo check --locked` and `cargo clippy --locked --all-targets --all-features -- -D warnings`, then redeploy.",
    )


def frontend_lockfile_exists(project_dir: pathlib.Path) -> bool:
    return any(
        (project_dir / filename).exists()
        for filename in ("package-lock.json", "npm-shrinkwrap.json", "pnpm-lock.yaml", "yarn.lock", "bun.lock", "bun.lockb")
    )


def frontend_script_checks(project_dir: pathlib.Path) -> list[dict[str, Any]]:
    manifest = read_package_json(project_dir)

    def has_script(name: str) -> bool:
        scripts = manifest.get("scripts") if isinstance(manifest, dict) else None
        return isinstance(scripts, dict) and isinstance(scripts.get(name), str) and bool(scripts[name].strip())

    checks = [
        {
            "name": f"npm script {script}",
            "ok": has_script(script),
            "message": "found" if has_script(script) else "missing",
            "agent_instruction": f'Add a non-empty "{script}" script to package.json, then retry.',
        }
        for script in ("typecheck", "lint")
    ]
    if all(check["ok"] for check in checks):
        checks.append(npm_script_check(project_dir, "typecheck"))
        checks.append(npm_script_check(project_dir, "lint"))
    return checks


def read_package_json(project_dir: pathlib.Path) -> dict[str, Any] | None:
    try:
        raw = (project_dir / "package.json").read_text(encoding="utf-8")
        parsed = json.loads(raw)
    except (OSError, json.JSONDecodeError):
        return None
    return parsed if isinstance(parsed, dict) else None


def npm_script_check(project_dir: pathlib.Path, script: str) -> dict[str, Any]:
    try:
        result = subprocess.run(
            ["npm", "run", "--silent", script],
            cwd=project_dir,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return {
            "name": f"npm run {script}",
            "ok": False,
            "message": str(error),
            "agent_instruction": f"Install Node.js and npm, then run `npm run {script}` before deploying.",
        }
    message = "passed" if result.returncode == 0 else (result.stderr or result.stdout or f"npm run {script} failed").strip()[:240]
    return {
        "name": f"npm run {script}",
        "ok": result.returncode == 0,
        "message": message,
        "agent_instruction": f"Run `npm run {script}`, fix every error, then redeploy.",
    }


def safe_relative_path(value: str) -> bool:
    return (
        bool(value)
        and not pathlib.PurePosixPath(value).is_absolute()
        and "\\" not in value
        and all(part and part not in (".", "..") for part in value.split("/"))
    )


def login(args: argparse.Namespace) -> None:
    token = args.token
    if token:
        write_session_token(token)
        print("saved Zerct session token")
        return

    login_and_store(args)


def deploy(project_dir: pathlib.Path, args: argparse.Namespace) -> None:
    report = run_doctor(project_dir)
    if not report["ok"]:
        failure = next(check for check in report["checks"] if not check["ok"])
        raise AgentError("doctor_failed", "Zerct doctor failed.", failure["agent_instruction"])
    if report["config"] and report["config"].get("kind") == "static_frontend" and args.database:
        raise AgentError(
            "invalid_database_target",
            "Static frontends cannot attach managed Postgres directly.",
            "Deploy a Rust backend with managed Postgres and call it from the frontend.",
        )

    token = read_or_login_token(project_dir, args)
    response = api_request(
        args,
        "POST",
        "/v1/deployments",
        token,
        {
            "config": report["config"],
            "commit_sha": git_commit_sha(project_dir),
            "wants_database": args.database,
            "source_archive_base64": archive_base64(project_dir),
        },
    )
    if args.json:
        print(json.dumps(response, indent=2))
        return

    print(f"queued {response['build_job']['id']}")
    print(f"app {response['app']['id']}")
    print(f"url {response['app']['url']}")
    print(f"next zerct logs --app {response['app']['id']}")


def logs(args: argparse.Namespace) -> None:
    response = app_get(args, "logs")
    if args.json:
        print(json.dumps(response, indent=2))
        return
    for line in response.get("lines", []):
        print(f"[{line['timestamp']}] {line['stream']}: {line['message']}")


def app_get(args: argparse.Namespace, route: str) -> dict[str, Any]:
    token = read_or_login_token(pathlib.Path.cwd(), args)
    return api_request(args, "GET", f"/v1/apps/{args.app}/{route}", token, None)


def set_env(args: argparse.Namespace) -> None:
    name, separator, value = args.assignment.partition("=")
    if not separator:
        raise AgentError(
            "invalid_env",
            "Environment assignment must be KEY=value.",
            "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
        )
    token = read_or_login_token(pathlib.Path.cwd(), args)
    response = api_request(args, "PUT", f"/v1/apps/{args.app}/env", token, {"name": name, "value": value})
    print_response(response, args.json)


def billing(args: argparse.Namespace) -> None:
    token = read_or_login_token(pathlib.Path.cwd(), args)
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
    headers = {"Accept": "application/json"}
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
        for item in project_dir.rglob("*"):
            relative = item.relative_to(project_dir)
            if should_exclude(relative):
                continue
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


def scan_unsafe(project_dir: pathlib.Path) -> list[str]:
    hits: list[str] = []
    for item in project_dir.rglob("*.rs"):
        relative = item.relative_to(project_dir)
        if should_exclude(relative):
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
    name = re.sub(r"[^a-z0-9-]+", "-", project_dir.name.lower()).strip("-")
    return name or "api"


def print_response(response: dict[str, Any], json_output: bool) -> None:
    print(json.dumps(response, indent=2 if json_output else 2))


def progress(args: argparse.Namespace, message: str) -> None:
    if args.json:
        print(message, file=sys.stderr)
        return
    print(message)


def print_error(error: AgentError, json_output: bool) -> None:
    if json_output:
        print(json.dumps(error.payload(), indent=2), file=sys.stderr)
        return
    print(error.message, file=sys.stderr)
    print(f"agent_instruction: {error.agent_instruction}", file=sys.stderr)
    if error.docs_url:
        print(f"docs: {error.docs_url}", file=sys.stderr)
    if error.checkout_url:
        print(f"checkout: {error.checkout_url}", file=sys.stderr)
