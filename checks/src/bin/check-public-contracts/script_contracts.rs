use crate::helpers::{CheckResult, read_text, require_contains, require_contains_all};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 5] = [
    size_of_val(&check),
    size_of_val(&require_rust_native_check_commands),
    size_of_val(&require_shell_style_contract),
    size_of_val(&require_toml_style_contract),
    size_of_val(&require_vacuum_installer_contract),
];

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check() -> CheckResult {
    check_try!(require_rust_native_check_commands());
    check_try!(require_vacuum_installer_contract());
    check_try!(require_shell_style_contract());
    return require_toml_style_contract();
}

/// Contract implementation for `require_rust_native_check_commands`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_rust_native_check_commands() -> CheckResult {
    let check_all = check_try!(read_text("checks/src/bin/check-all.rs"));
    for (snippet, label) in [
        (
            "&[\"docs\"]",
            "Rust check-all must run public contract docs through the Rust checker binary",
        ),
        (
            "(\"check-prose-style\", &[\"--self-test\"])",
            "Rust check-all must run prose self-test through the Rust checker binary",
        ),
        (
            "(\"check-github-actions\", &[])",
            "Rust check-all must run GitHub Actions policy through the Rust checker binary",
        ),
        (
            "(\"check-shell-style\", &[])",
            "Rust check-all must run shell style through the Rust checker binary",
        ),
        (
            "(\"check-toml-style\", &[])",
            "Rust check-all must run TOML style through the Rust checker binary",
        ),
        (
            "self.run(\"typos\", &[\"--config\", \".typos.toml\", \".\"])",
            "Rust check-all must call the Rust-native typos checker directly",
        ),
    ] {
        check_try!(require_contains(check_all.as_str(), snippet, label));
    }
    return Ok(());
}

/// Contract implementation for `require_shell_style_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_shell_style_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/check-shell-style.rs"));
    return require_contains_all(
        source.as_str(),
        &[
            (
                "if shell_sources.is_empty()",
                "public shell style check must accept a shell-free repository",
            ),
            (
                "\"shellcheck\",",
                "public shell style check must run ShellCheck with sourced-file analysis",
            ),
            (
                "&[\"-i\", \"2\", \"-ci\", \"-d\"]",
                "public shell style check must run shfmt",
            ),
        ],
    );
}

/// Contract implementation for `require_toml_style_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_toml_style_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/check-toml-style.rs"));
    return require_contains_all(
        source.as_str(),
        &[
            (
                "&[\"format\", \"--check\"]",
                "public TOML style check must run taplo format in check mode",
            ),
            (
                "&[\"lint\", \"--no-schema\"]",
                "public TOML style check must run taplo lint without schema downloads",
            ),
            (
                "matches!(name, \".git\" | \"node_modules\" | \"target\" | \"vendor\")",
                "public TOML style check must prune generated dependency directories",
            ),
        ],
    );
}

/// Contract implementation for `require_vacuum_installer_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_vacuum_installer_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/check-openapi/vacuum.rs"));
    return require_contains_all(
        source.as_str(),
        &[
            (
                "fn vacuum_asset_sha256",
                "Vacuum installer must pin asset checksums",
            ),
            (
                "Sha256::digest(archive_bytes)",
                "Vacuum installer must verify SHA-256 before extraction",
            ),
            (
                "checksum mismatch",
                "Vacuum installer must fail on checksum mismatch",
            ),
            (
                "Archive::new(GzDecoder::new(archive_bytes))",
                "Vacuum installer must extract the pinned tarball in Rust",
            ),
        ],
    );
}
