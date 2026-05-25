"""Small dependency-free Zerct CLI for Python users."""

from __future__ import annotations

import argparse
import base64
import io
import json
import os
import pathlib
import re
import subprocess
import sys
import tarfile
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
    for filename in ("Cargo.toml", "Cargo.lock", "zerct.toml"):
        exists = (project_dir / filename).exists()
        checks.append(
            {
                "name": filename,
                "ok": exists,
                "message": "found" if exists else "missing",
                "agent_instruction": f"Create and commit {filename}, then retry.",
            }
        )

    config: dict[str, Any] | None = None
    config_path = project_dir / "zerct.toml"
    if config_path.exists():
        try:
            config = parse_config(config_path)
            validate_config(config)
            checks.append({"name": "zerct.toml", "ok": True, "message": "valid", "agent_instruction": ""})
        except (OSError, tomllib.TOMLDecodeError, AgentError) as error:
            checks.append(
                {
                    "name": "zerct.toml",
                    "ok": False,
                    "message": str(error),
                    "agent_instruction": "Fix zerct.toml so it matches the Zerct deploy contract.",
                }
            )

    unsafe_hits = scan_unsafe(project_dir)
    checks.append(
        {
            "name": "unsafe",
            "ok": not unsafe_hits,
            "message": "no direct unsafe found" if not unsafe_hits else ", ".join(unsafe_hits[:5]),
            "agent_instruction": "Remove direct unsafe usage from workspace Rust source before deploying.",
        }
    )

    return {
        "ok": all(check["ok"] for check in checks),
        "project": str(project_dir),
        "config": config,
        "checks": checks,
    }


def parse_config(path: pathlib.Path) -> dict[str, Any]:
    config = tomllib.loads(path.read_text(encoding="utf-8"))
    config.setdefault("build", {})
    config.setdefault("run", {})
    config.setdefault("resources", {})
    config["build"].setdefault("command", "cargo build --release")
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


def login(args: argparse.Namespace) -> None:
    token = args.token
    if token:
        write_session_token(pathlib.Path.cwd(), token)
        print("saved Zerct session token to .zerct/session-token")
        return

    response = api_request(args, "POST", "/v1/login/device", None, None)
    webbrowser.open(response["login_url"])
    print(f"opened {response['login_url']}")
    print("After login, retry your deploy. If the CLI cannot finish automatically yet, set ZERCT_TOKEN or run `zerct login --token <token>`.")


def deploy(project_dir: pathlib.Path, args: argparse.Namespace) -> None:
    report = run_doctor(project_dir)
    if not report["ok"]:
        failure = next(check for check in report["checks"] if not check["ok"])
        raise AgentError("doctor_failed", "Zerct doctor failed.", failure["agent_instruction"])

    token = read_token(project_dir, args)
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
    token = read_token(pathlib.Path.cwd(), args)
    return api_request(args, "GET", f"/v1/apps/{args.app}/{route}", token, None)


def set_env(args: argparse.Namespace) -> None:
    name, separator, value = args.assignment.partition("=")
    if not separator:
        raise AgentError(
            "invalid_env",
            "Environment assignment must be KEY=value.",
            "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
        )
    token = read_token(pathlib.Path.cwd(), args)
    response = api_request(args, "PUT", f"/v1/apps/{args.app}/env", token, {"name": name, "value": value})
    print_response(response, args.json)


def billing(args: argparse.Namespace) -> None:
    token = read_token(pathlib.Path.cwd(), args)
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


def read_token(project_dir: pathlib.Path, args: argparse.Namespace) -> str:
    if args.token:
        return args.token
    if os.environ.get("ZERCT_TOKEN"):
        return os.environ["ZERCT_TOKEN"]
    for candidate in (
        project_dir / SESSION_DIR / SESSION_FILE,
        pathlib.Path.home() / SESSION_DIR / SESSION_FILE,
    ):
        if candidate.exists():
            return candidate.read_text(encoding="utf-8").strip()
    raise AgentError(
        "login_required",
        "Zerct login is required.",
        "Run `zerct login`, set `ZERCT_TOKEN`, or run `zerct login --token <token>`, then retry.",
    )


def write_session_token(project_dir: pathlib.Path, token: str) -> None:
    session_dir = project_dir / SESSION_DIR
    session_dir.mkdir(mode=0o700, exist_ok=True)
    token_path = session_dir / SESSION_FILE
    token_path.write_text(token.strip() + "\n", encoding="utf-8")
    token_path.chmod(0o600)


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
