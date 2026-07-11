"""Python package entrypoint for the native Tovuk binary."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import platform
import stat
import sys
import tempfile
import urllib.error
import urllib.parse
import urllib.request
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    import http.client

from . import __version__

REPOSITORY = "https://github.com/tovuk/tovuk"
MAX_BINARY_BYTES = 100 * 1024 * 1024
MAX_CHECKSUM_BYTES = 4096
MAX_CHECKSUM_SIDECAR_BYTES = 128
MAX_REDIRECTS = 5
SHA256_HEX_LENGTH = 64
HTTP_OK = 200
HTTP_REDIRECT_START = 300
HTTP_REDIRECT_END = 400
ALLOWED_DOWNLOAD_HOSTS = {
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
}
NATIVE_TARGETS = json.loads(
    pathlib.Path(__file__).with_name("native_release_targets.json").read_text(encoding="utf-8")
)["targets"]


class _ManualRedirectHandler(urllib.request.HTTPErrorProcessor):
    """Return redirect responses so the caller can validate every hop."""

    def http_response(
        self,
        request: urllib.request.Request,
        response: http.client.HTTPResponse,
    ) -> http.client.HTTPResponse:
        """Return an HTTP response without following or converting redirects."""
        _require_trusted_download_url(request.full_url)
        return response

    https_response = http_response


DOWNLOAD_OPENER = urllib.request.build_opener(_ManualRedirectHandler())


class _ReadableResponse(Protocol):
    def read(self, amount: int = -1, /) -> bytes:
        """Read at most ``amount`` response bytes."""
        ...


def main(argv: list[str] | None = None) -> None:
    """Launch the verified native Tovuk binary with the supplied arguments."""
    args = list(sys.argv[1:] if argv is None else argv)
    if _wants_version(args):
        sys.stdout.write(f"{__version__}\n")
        return

    try:
        binary = _native_binary()
    except RuntimeError as error:
        _print_agent_error(str(error), json_output=_wants_json(args))
        raise SystemExit(1) from error

    os.execv(str(binary), [str(binary), *args])


def _wants_version(args: list[str]) -> bool:
    return len(args) == 1 and args[0] in {"--version", "-v", "-V"}


def _wants_json(args: list[str]) -> bool:
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
        message = f"TOVUK_NATIVE_BINARY does not point to a file: {path}"
        raise RuntimeError(message)

    target = _native_target()
    target_triple = str(target["triple"])
    binary_name = str(target["binary"])

    packaged = pathlib.Path(__file__).with_name("bin") / binary_name
    if packaged.is_file() and not packaged.is_symlink():
        return packaged

    binary = _cache_dir() / __version__ / target_triple / binary_name
    if binary.is_file():
        if _cached_binary_is_valid(binary):
            return binary
        binary.unlink(missing_ok=True)
        _checksum_path(binary).unlink(missing_ok=True)

    binary.parent.mkdir(parents=True, exist_ok=True)
    _download_release_binary(target, binary)
    return binary


def _mark_executable(binary: pathlib.Path) -> None:
    if os.name != "nt":
        binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def _cache_dir() -> pathlib.Path:
    if os.name == "nt" and os.environ.get("LOCALAPPDATA"):
        return pathlib.Path(os.environ["LOCALAPPDATA"]) / "Tovuk" / "bin"
    root = pathlib.Path(os.environ.get("XDG_CACHE_HOME", pathlib.Path.home() / ".cache"))
    return root / "tovuk" / "bin"


def _native_target() -> dict[str, object]:
    system = platform.system().lower()
    machine = platform.machine().lower()
    for target in NATIVE_TARGETS:
        if not any(
            system == alias["system"] and machine == alias["machine"] for alias in target["python"]
        ):
            continue
        if target.get("libc") == "glibc" and _linux_libc() != "glibc":
            message = (
                f"Unsupported Tovuk native target: {system}/{machine} requires glibc Linux. "
                "Alpine/musl Linux is not supported by the published native binaries yet."
            )
            raise RuntimeError(message)
        return target
    message = f"Unsupported Tovuk native target: {system}/{machine}"
    raise RuntimeError(message)


def _linux_libc() -> str:
    libc_name, _ = platform.libc_ver()
    return libc_name.lower()


def _download_release_binary(target: dict[str, object], destination: pathlib.Path) -> None:
    target_triple = str(target["triple"])
    asset_ext = str(target["asset_ext"])
    asset = f"tovuk-{__version__}-{target_triple}{asset_ext}"
    url = f"{REPOSITORY}/releases/download/v{__version__}/{asset}"
    checksum_url = f"{url}.sha256"
    try:
        expected_sha256 = _release_checksum(checksum_url, asset)
        with _open_trusted_url(url) as response:
            contents = _read_limited(
                response,
                response.headers.get("Content-Length"),
                MAX_BINARY_BYTES,
                "native binary",
            )
        actual_sha256 = hashlib.sha256(contents).hexdigest()
        if actual_sha256 != expected_sha256:
            message = (
                f"native binary checksum mismatch: expected {expected_sha256}, got {actual_sha256}"
            )
            raise RuntimeError(message)
        _write_cached_binary(destination, contents, expected_sha256)
    except (OSError, urllib.error.URLError) as error:
        message = f"Could not install native Tovuk binary from {url}: {error}"
        raise RuntimeError(message) from error


def _cached_binary_is_valid(binary: pathlib.Path) -> bool:
    if binary.is_symlink() or not binary.is_file():
        return False
    binary_size = binary.stat().st_size
    if binary_size == 0 or binary_size > MAX_BINARY_BYTES:
        return False
    checksum_path = _checksum_path(binary)
    if checksum_path.is_symlink() or not checksum_path.is_file():
        return False
    if checksum_path.stat().st_size > MAX_CHECKSUM_SIDECAR_BYTES:
        return False
    expected_sha256 = checksum_path.read_text(encoding="utf-8").strip().lower()
    if len(expected_sha256) != SHA256_HEX_LENGTH or any(
        character not in "0123456789abcdef" for character in expected_sha256
    ):
        return False
    actual_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    return actual_sha256 == expected_sha256


def _write_cached_binary(destination: pathlib.Path, contents: bytes, expected_sha256: str) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    temp_path: pathlib.Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            "wb",
            dir=destination.parent,
            prefix=f".{destination.name}.",
            delete=False,
        ) as temp_file:
            temp_file.write(contents)
            temp_file.flush()
            os.fsync(temp_file.fileno())
            temp_path = pathlib.Path(temp_file.name)
        _mark_executable(temp_path)
        temp_path.replace(destination)
        _write_checksum_sidecar(destination, expected_sha256)
    finally:
        if temp_path is not None and temp_path.exists():
            temp_path.unlink()


def _write_checksum_sidecar(binary: pathlib.Path, expected_sha256: str) -> None:
    checksum_path = _checksum_path(binary)
    temp_path: pathlib.Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            "w",
            dir=checksum_path.parent,
            encoding="utf-8",
            prefix=f".{checksum_path.name}.",
            delete=False,
        ) as temp_file:
            temp_file.write(f"{expected_sha256}\n")
            temp_file.flush()
            os.fsync(temp_file.fileno())
            temp_path = pathlib.Path(temp_file.name)
        temp_path.replace(checksum_path)
    finally:
        if temp_path is not None and temp_path.exists():
            temp_path.unlink()


def _checksum_path(binary: pathlib.Path) -> pathlib.Path:
    return binary.with_name(f"{binary.name}.sha256")


def _release_checksum(url: str, asset: str) -> str:
    with _open_trusted_url(url) as response:
        text = _read_limited(
            response,
            response.headers.get("Content-Length"),
            MAX_CHECKSUM_BYTES,
            "checksum",
        ).decode("utf-8", errors="replace")
    line = next((item.strip() for item in text.splitlines() if item.strip()), "")
    if not line:
        message = f"checksum file for {asset} is empty"
        raise RuntimeError(message)
    parts = line.split()
    digest = parts[0].lower()
    if len(digest) != SHA256_HEX_LENGTH or any(
        character not in "0123456789abcdef" for character in digest
    ):
        message = f"checksum file for {asset} does not contain a SHA-256 digest"
        raise RuntimeError(message)
    if len(parts) > 1:
        listed_asset = pathlib.Path(" ".join(parts[1:]).lstrip("*")).name
        if listed_asset != asset:
            message = f"checksum file names {listed_asset}, expected {asset}"
            raise RuntimeError(message)
    return digest


def _open_trusted_url(url: str) -> http.client.HTTPResponse:
    current_url = url
    for _redirect_count in range(MAX_REDIRECTS + 1):
        _require_trusted_download_url(current_url)
        response = DOWNLOAD_OPENER.open(current_url, timeout=30)
        status = response.status
        if HTTP_REDIRECT_START <= status < HTTP_REDIRECT_END:
            location = response.headers.get("Location")
            response.close()
            if not location:
                message = f"download redirect from {current_url} has no Location header"
                raise RuntimeError(message)
            current_url = urllib.parse.urljoin(current_url, location)
            continue
        if status != HTTP_OK:
            response.close()
            message = f"download from {current_url} returned HTTP {status}"
            raise RuntimeError(message)
        _require_trusted_download_url(response.geturl())
        return response
    message = f"too many redirects while downloading {url}"
    raise RuntimeError(message)


def _require_trusted_download_url(value: str) -> None:
    parsed = urllib.parse.urlsplit(value)
    try:
        port = parsed.port
    except ValueError as error:
        message = "refusing download URL with an invalid port"
        raise RuntimeError(message) from error
    if (
        parsed.scheme != "https"
        or parsed.username is not None
        or parsed.password is not None
        or port is not None
        or parsed.hostname not in ALLOWED_DOWNLOAD_HOSTS
    ):
        message = f"refusing untrusted download URL: {parsed.scheme}://{parsed.netloc}"
        raise RuntimeError(message)


def _read_limited(
    response: _ReadableResponse,
    raw_length: str | None,
    maximum_bytes: int,
    label: str,
) -> bytes:
    if raw_length is not None:
        try:
            content_length = int(raw_length)
        except ValueError as error:
            message = f"{label} has an invalid Content-Length"
            raise RuntimeError(message) from error
        if content_length < 0 or content_length > maximum_bytes:
            message = f"{label} exceeds the {maximum_bytes}-byte download limit"
            raise RuntimeError(message)
    contents = response.read(maximum_bytes + 1)
    if len(contents) > maximum_bytes:
        message = f"{label} exceeds the {maximum_bytes}-byte download limit"
        raise RuntimeError(message)
    return contents


def _print_agent_error(message: str, *, json_output: bool) -> None:
    instruction = (
        "Install a supported Tovuk native binary from GitHub Releases, Homebrew, Cargo, npm, "
        "or rerun the PyPI command with network access."
    )
    payload = {
        "code": "native_binary_unavailable",
        "message": message,
        "agent_instruction": instruction,
        "docs_url": "https://docs.tovuk.com/reference/packages",
        "checkout_url": None,
    }
    if json_output:
        sys.stderr.write(f"{json.dumps(payload, indent=2)}\n")
        return

    sys.stderr.write(f"{payload['message']}\n")
    sys.stderr.write(f"agent_instruction: {payload['agent_instruction']}\n")
