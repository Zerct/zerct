"""Security and cache tests for the Python native launcher."""

from __future__ import annotations

import hashlib
import io
import os
import pathlib
import tempfile
import unittest
from typing import TYPE_CHECKING

from tovuk import cli

if TYPE_CHECKING:
    from collections.abc import Callable


def require(*, condition: bool, label: str) -> None:
    """Require one test invariant without Python optimization semantics."""
    if not condition:
        raise AssertionError(label)


def require_runtime_error(operation: Callable[[], object], label: str) -> None:
    """Require one launcher policy operation to fail closed."""
    try:
        operation()
    except RuntimeError:
        return
    message = f"expected RuntimeError: {label}"
    raise AssertionError(message)


class _Response(io.BytesIO):
    """Minimal bounded-read response used by launcher tests."""

    def __init__(self, contents: bytes, content_length: str | None = None) -> None:
        super().__init__(contents)
        self.headers: dict[str, str] = {}
        if content_length is not None:
            self.headers["Content-Length"] = content_length


class DownloadPolicyTests(unittest.TestCase):
    """Verify that native downloads remain bounded and host-restricted."""

    def test_accepts_expected_release_hosts(self) -> None:
        """Known GitHub release hosts remain accepted."""
        for value in (
            "https://github.com/tovuk/tovuk/releases/download/v1/asset",
            "https://objects.githubusercontent.com/asset",
            "https://release-assets.githubusercontent.com/asset",
        ):
            cli.require_trusted_download_url(value)

    def test_rejects_untrusted_download_urls(self) -> None:
        """Non-HTTPS, credentialed, ported, and unknown hosts are rejected."""
        for value in (
            "http://github.com/tovuk/tovuk/releases/download/v1/asset",
            "https://example.com/asset",
            "https://user:secret@github.com/asset",
            "https://github.com:8443/asset",
        ):
            require_runtime_error(
                lambda value=value: cli.require_trusted_download_url(value),
                value,
            )

    def test_rejects_oversized_or_invalid_content_length(self) -> None:
        """Malformed or oversized response bodies fail closed."""
        for response in (
            _Response(b"small", "not-a-number"),
            _Response(b"small", "6"),
            _Response(b"too large"),
        ):
            require_runtime_error(
                lambda response=response: cli.read_limited(
                    response,
                    response.headers.get("Content-Length"),
                    5,
                    "test payload",
                ),
                "invalid or oversized response",
            )

    def test_accepts_payload_within_bound(self) -> None:
        """A response exactly at the configured bound is accepted."""
        response = _Response(b"small", "5")
        require(
            condition=cli.read_limited(
                response,
                response.headers.get("Content-Length"),
                5,
                "test payload",
            )
            == b"small",
            label="bounded payload must be unchanged",
        )


class CachePolicyTests(unittest.TestCase):
    """Verify that cached launchers require a matching checksum sidecar."""

    def test_cache_requires_matching_checksum(self) -> None:
        """Missing and stale checksums invalidate a cached executable."""
        with tempfile.TemporaryDirectory() as directory:
            binary = pathlib.Path(directory) / "tovuk"
            binary.write_bytes(b"trusted binary")
            require(
                condition=not cli.cached_binary_is_valid(binary),
                label="missing checksum must invalidate cache",
            )

            digest = hashlib.sha256(binary.read_bytes()).hexdigest()
            cli.checksum_path(binary).write_text(f"{digest}\n", encoding="utf-8")
            require(
                condition=cli.cached_binary_is_valid(binary),
                label="matching checksum must validate cache",
            )

            binary.write_bytes(b"tampered binary")
            require(
                condition=not cli.cached_binary_is_valid(binary),
                label="tampering must invalidate cache",
            )

    @unittest.skipIf(os.name == "nt", "Windows symlink creation requires elevated privileges")
    def test_checksum_write_replaces_symlink_without_following_it(self) -> None:
        """Checksum publication must never overwrite a symlink target."""
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            binary = root / "tovuk"
            binary.write_bytes(b"trusted binary")
            victim = root / "victim"
            victim.write_text("unchanged\n", encoding="utf-8")
            sidecar = cli.checksum_path(binary)
            sidecar.symlink_to(victim)

            digest = hashlib.sha256(binary.read_bytes()).hexdigest()
            cli.write_checksum_sidecar(binary, digest)

            require(
                condition=not sidecar.is_symlink(),
                label="checksum sidecar must replace the symlink",
            )
            require(
                condition=sidecar.read_text(encoding="utf-8") == f"{digest}\n",
                label="checksum sidecar must contain the digest",
            )
            require(
                condition=victim.read_text(encoding="utf-8") == "unchanged\n",
                label="checksum publication must not alter the symlink target",
            )


if __name__ == "__main__":
    unittest.main()
