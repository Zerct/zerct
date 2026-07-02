"""Python package entrypoint for the native Tovuk binary."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import platform
import stat
import sys
from typing import List, Optional
import urllib.error
import urllib.request

from . import __version__

REPOSITORY = "https://github.com/tovuk/tovuk"
NATIVE_TARGETS = {
    ("darwin", "aarch64"): "aarch64-apple-darwin",
    ("darwin", "arm64"): "aarch64-apple-darwin",
    ("darwin", "amd64"): "x86_64-apple-darwin",
    ("darwin", "x86_64"): "x86_64-apple-darwin",
    ("linux", "aarch64"): "aarch64-unknown-linux-gnu",
    ("linux", "arm64"): "aarch64-unknown-linux-gnu",
    ("linux", "amd64"): "x86_64-unknown-linux-gnu",
    ("linux", "x86_64"): "x86_64-unknown-linux-gnu",
    ("windows", "amd64"): "x86_64-pc-windows-msvc",
    ("windows", "x86_64"): "x86_64-pc-windows-msvc",
}


def main(argv: Optional[List[str]] = None) -> None:
    args = list(sys.argv[1:] if argv is None else argv)
    if _wants_version(args):
        print(__version__)
        return

    try:
        binary = _native_binary()
    except RuntimeError as error:
        _print_agent_error(str(error), _wants_json(args))
        raise SystemExit(1) from error

    os.execv(str(binary), [str(binary), *args])


def _wants_version(args: List[str]) -> bool:
    return len(args) == 1 and args[0] in {"--version", "-v", "-V"}


def _wants_json(args: List[str]) -> bool:
    output = os.environ.get("TOVUK_OUTPUT", "").strip().lower()
    json_output = output == "json"
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == "--json":
            json_output = True
        elif arg == "--output" and index + 1 < len(args):
            json_output = args[index + 1].strip().lower() == "json"
            index += 1
        elif arg.startswith("--output="):
            json_output = arg.split("=", 1)[1].strip().lower() == "json"
        index += 1
    return json_output


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
    target = NATIVE_TARGETS.get((system, machine))
    if target is not None:
        return target
    raise RuntimeError(f"Unsupported Tovuk native target: {system}/{machine}")


def _download_release_binary(target: str, destination: pathlib.Path) -> None:
    suffix = ".exe" if target.endswith("windows-msvc") else ""
    asset = f"tovuk-{__version__}-{target}{suffix}"
    url = f"{REPOSITORY}/releases/download/v{__version__}/{asset}"
    checksum_url = f"{url}.sha256"
    try:
        expected_sha256 = _release_checksum(checksum_url, asset)
        with urllib.request.urlopen(url, timeout=30) as response:
            contents = response.read()
        actual_sha256 = hashlib.sha256(contents).hexdigest()
        if actual_sha256 != expected_sha256:
            raise RuntimeError(
                f"native binary checksum mismatch: expected {expected_sha256}, got {actual_sha256}"
            )
        destination.write_bytes(contents)
    except (OSError, urllib.error.URLError) as error:
        raise RuntimeError(f"Could not install native Tovuk binary from {url}: {error}") from error


def _release_checksum(url: str, asset: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        text = response.read(4096).decode("utf-8", errors="replace")
    line = next((item.strip() for item in text.splitlines() if item.strip()), "")
    if not line:
        raise RuntimeError(f"checksum file for {asset} is empty")
    parts = line.split()
    digest = parts[0].lower()
    if len(digest) != 64 or any(character not in "0123456789abcdef" for character in digest):
        raise RuntimeError(f"checksum file for {asset} does not contain a SHA-256 digest")
    if len(parts) > 1:
        listed_asset = pathlib.Path(" ".join(parts[1:]).lstrip("*")).name
        if listed_asset != asset:
            raise RuntimeError(f"checksum file names {listed_asset}, expected {asset}")
    return digest


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
