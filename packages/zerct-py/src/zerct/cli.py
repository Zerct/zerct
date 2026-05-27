"""Thin Python entrypoint for the npm Zerct CLI source of truth."""

from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys

from . import __version__

NPM_PACKAGE = "@zerct/zerct"
NPM_PACKAGE_VERSION = "0.1.46"
NPM_PACKAGE_SPEC = f"{NPM_PACKAGE}@{NPM_PACKAGE_VERSION}"


def main(argv: list[str] | None = None) -> None:
    args = list(sys.argv[1:] if argv is None else argv)
    if _wants_version(args):
        print(__version__)
        return

    try:
        command = _delegate_command()
    except RuntimeError as error:
        _print_agent_error(str(error), "--json" in args)
        raise SystemExit(1) from error

    completed = subprocess.run([*command, *args], check=False)
    raise SystemExit(completed.returncode)


def _wants_version(args: list[str]) -> bool:
    return len(args) == 1 and args[0] in {"--version", "-v", "-V"}


def _delegate_command() -> list[str]:
    local_cli = os.environ.get("ZERCT_NPM_CLI", "").strip()
    if local_cli:
        path = pathlib.Path(local_cli).expanduser()
        if not path.is_file():
            raise RuntimeError(f"ZERCT_NPM_CLI does not point to a file: {path}")
        return [_local_tsx(path), str(path)]

    npx = shutil.which("npx")
    if npx:
        return [npx, "-y", NPM_PACKAGE_SPEC]

    raise RuntimeError("Node.js npm tooling is required for the PyPI Zerct CLI.")


def _required_executable(name: str) -> str:
    executable = shutil.which(name)
    if executable:
        return executable
    raise RuntimeError(f"{name} is required to run the local Zerct npm CLI.")


def _local_tsx(cli_path: pathlib.Path) -> str:
    package_root = cli_path.parent.parent
    local_bin = package_root / "node_modules" / ".bin" / ("tsx.cmd" if os.name == "nt" else "tsx")
    if local_bin.is_file():
        return str(local_bin)
    return _required_executable("tsx")


def _print_agent_error(message: str, json_output: bool) -> None:
    payload = {
        "code": "dependency_missing",
        "message": message,
        "agent_instruction": "Install Node.js 18+ with npx, or set ZERCT_NPM_CLI to packages/zerct/src/zerct.ts after running npm install in packages/zerct.",
        "docs_url": "https://docs.zerct.com/reference/packages",
        "checkout_url": None,
    }
    if json_output:
        print(json.dumps(payload, indent=2), file=sys.stderr)
        return

    print(payload["message"], file=sys.stderr)
    print(f"agent_instruction: {payload['agent_instruction']}", file=sys.stderr)
