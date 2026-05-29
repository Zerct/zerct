"""Python package entrypoint for the native Tovuk binary."""

from __future__ import annotations

import json
import os
import pathlib
import platform
import stat
import sys
import urllib.error
import urllib.request

from . import __version__

REPOSITORY = "https://github.com/tovuk/tovuk"


def main(argv: list[str] | None = None) -> None:
    args = list(sys.argv[1:] if argv is None else argv)
    if _wants_version(args):
        print(__version__)
        return

    try:
        binary = _native_binary()
    except RuntimeError as error:
        _print_agent_error(str(error), "--json" in args)
        raise SystemExit(1) from error

    os.execv(str(binary), [str(binary), *args])


def _wants_version(args: list[str]) -> bool:
    return len(args) == 1 and args[0] in {"--version", "-v", "-V"}


def _native_binary() -> pathlib.Path:
    override = os.environ.get("TOVUK_NATIVE_BINARY", "").strip()
    if override:
        path = pathlib.Path(override).expanduser()
        if path.is_file():
            return path
        raise RuntimeError(f"TOVUK_NATIVE_BINARY does not point to a file: {path}")

    packaged = pathlib.Path(__file__).with_name("bin") / "tovuk"
    if packaged.is_file():
        return packaged

    target = _native_target()
    binary = _cache_dir() / __version__ / target / "tovuk"
    if binary.is_file():
        return binary

    binary.parent.mkdir(parents=True, exist_ok=True)
    _download_release_binary(target, binary)
    binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return binary


def _cache_dir() -> pathlib.Path:
    if os.name == "nt" and os.environ.get("LOCALAPPDATA"):
        return pathlib.Path(os.environ["LOCALAPPDATA"]) / "Tovuk" / "bin"
    root = pathlib.Path(os.environ.get("XDG_CACHE_HOME", pathlib.Path.home() / ".cache"))
    return root / "tovuk" / "bin"


def _native_target() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin" and machine in {"arm64", "aarch64"}:
        return "aarch64-apple-darwin"
    if system == "darwin" and machine in {"x86_64", "amd64"}:
        return "x86_64-apple-darwin"
    if system == "linux" and machine in {"arm64", "aarch64"}:
        return "aarch64-unknown-linux-gnu"
    if system == "linux" and machine in {"x86_64", "amd64"}:
        return "x86_64-unknown-linux-gnu"
    if system == "windows" and machine in {"x86_64", "amd64"}:
        return "x86_64-pc-windows-msvc"
    raise RuntimeError(f"Unsupported Tovuk native target: {system}/{machine}")


def _download_release_binary(target: str, destination: pathlib.Path) -> None:
    suffix = ".exe" if target.endswith("windows-msvc") else ""
    asset = f"tovuk-{__version__}-{target}{suffix}"
    url = f"{REPOSITORY}/releases/download/v{__version__}/{asset}"
    try:
        with urllib.request.urlopen(url, timeout=30) as response:
            destination.write_bytes(response.read())
    except (OSError, urllib.error.URLError) as error:
        raise RuntimeError(f"Could not install native Tovuk binary from {url}: {error}") from error


def _print_agent_error(message: str, json_output: bool) -> None:
    payload = {
        "code": "native_binary_unavailable",
        "message": message,
        "agent_instruction": "Install a supported Tovuk native binary from GitHub Releases, Homebrew, Cargo, npm, or rerun the PyPI command with network access.",
        "docs_url": "https://docs.tovuk.com/reference/packages",
        "checkout_url": None,
    }
    if json_output:
        print(json.dumps(payload, indent=2), file=sys.stderr)
        return

    print(payload["message"], file=sys.stderr)
    print(f"agent_instruction: {payload['agent_instruction']}", file=sys.stderr)
