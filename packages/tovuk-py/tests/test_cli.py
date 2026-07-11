"""Security and cache tests for the Python native launcher."""

from __future__ import annotations

import hashlib
import io
import os
import pathlib
import tempfile
import unittest

from tovuk import cli


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
            cli._require_trusted_download_url(value)

    def test_rejects_untrusted_download_urls(self) -> None:
        """Non-HTTPS, credentialed, ported, and unknown hosts are rejected."""
        for value in (
            "http://github.com/tovuk/tovuk/releases/download/v1/asset",
            "https://example.com/asset",
            "https://user:secret@github.com/asset",
            "https://github.com:8443/asset",
        ):
            with self.assertRaises(RuntimeError, msg=value):
                cli._require_trusted_download_url(value)

    def test_rejects_oversized_or_invalid_content_length(self) -> None:
        """Malformed or oversized response bodies fail closed."""
        for response in (
            _Response(b"small", "not-a-number"),
            _Response(b"small", "6"),
            _Response(b"too large"),
        ):
            with self.assertRaises(RuntimeError):
                cli._read_limited(
                    response,
                    response.headers.get("Content-Length"),
                    5,
                    "test payload",
                )

    def test_accepts_payload_within_bound(self) -> None:
        """A response exactly at the configured bound is accepted."""
        response = _Response(b"small", "5")
        self.assertEqual(
            cli._read_limited(response, response.headers.get("Content-Length"), 5, "test payload"),
            b"small",
        )


class CachePolicyTests(unittest.TestCase):
    """Verify that cached launchers require a matching checksum sidecar."""

    def test_cache_requires_matching_checksum(self) -> None:
        """Missing and stale checksums invalidate a cached executable."""
        with tempfile.TemporaryDirectory() as directory:
            binary = pathlib.Path(directory) / "tovuk"
            binary.write_bytes(b"trusted binary")
            self.assertFalse(cli._cached_binary_is_valid(binary))

            digest = hashlib.sha256(binary.read_bytes()).hexdigest()
            cli._checksum_path(binary).write_text(f"{digest}\n", encoding="utf-8")
            self.assertTrue(cli._cached_binary_is_valid(binary))

            binary.write_bytes(b"tampered binary")
            self.assertFalse(cli._cached_binary_is_valid(binary))

    @unittest.skipIf(os.name == "nt", "Windows symlink creation requires elevated privileges")
    def test_checksum_write_replaces_symlink_without_following_it(self) -> None:
        """Checksum publication must never overwrite a symlink target."""
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            binary = root / "tovuk"
            binary.write_bytes(b"trusted binary")
            victim = root / "victim"
            victim.write_text("unchanged\n", encoding="utf-8")
            checksum_path = cli._checksum_path(binary)
            checksum_path.symlink_to(victim)

            digest = hashlib.sha256(binary.read_bytes()).hexdigest()
            cli._write_checksum_sidecar(binary, digest)

            self.assertFalse(checksum_path.is_symlink())
            self.assertEqual(checksum_path.read_text(encoding="utf-8"), f"{digest}\n")
            self.assertEqual(victim.read_text(encoding="utf-8"), "unchanged\n")


if __name__ == "__main__":
    unittest.main()
