use crate::helpers::{
    CheckResult, read_text, require_contains, require_contains_all, require_equal,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0008] = [
    size_of_val(&check),
    size_of_val(&require_ci_snapshot_contract),
    size_of_val(&require_pre_push_contract),
    size_of_val(&require_public_tree_sync_contract),
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
    check_try!(require_ci_snapshot_contract());
    check_try!(require_pre_push_contract());
    check_try!(require_public_tree_sync_contract());
    check_try!(require_rust_native_check_commands());
    check_try!(require_vacuum_installer_contract());
    check_try!(require_shell_style_contract());
    return require_toml_style_contract();
}

/// Require CI history scanning to remain a direct Rust-native command.
///
/// # Errors
///
/// Returns an error when command routing or workflow invocation is missing.
fn require_ci_snapshot_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/check-public-contracts/main.rs"));
    check_try!(require_contains(
        source.as_str(),
        "\"ci-snapshot\" => return run_ci_snapshot(&mut args),",
        "public contract routing must expose the argument-closed Rust-native CI snapshot gate",
    ));
    check_try!(require_contains(
        source.as_str(),
        "return Err(\"ci-snapshot accepts no arguments\".to_owned());",
        "the CI snapshot route must reject every positional argument",
    ));
    let workflow = check_try!(read_text(".github/workflows/trusted-history.yml"));
    return require_contains(
        workflow.as_str(),
        "run: '\"$RUNNER_TEMP/tovuk-trusted-history/debug/check-public-contracts\" ci-snapshot'",
        "ref history audit must invoke the workflow-built Rust binary without shell-derived object IDs",
    );
}

/// Require pre-push to scan exactly the refs Git supplies before the full gate.
///
/// # Errors
///
/// Returns an error when the hook bypasses the Rust pushed-object scanner.
fn require_pre_push_contract() -> CheckResult {
    let source = check_try!(read_text(".githooks/pre-push"));
    return require_equal(
        source.as_str(),
        "#!/bin/sh\nset -eu\n\nexport GIT_NO_REPLACE_OBJECTS=1\n\nrepository_root=\"$(git rev-parse --show-toplevel)\"\ncd \"$repository_root\"\n\npush_location=\"${2:?pre-push push location is required}\"\ncargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- push-snapshot \"$push_location\"\nexec cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --\n",
        "pre-push must pass Git's exact standard input to the Rust object scanner before the canonical full gate",
    );
}

/// Require data-only public-tree synchronization to remain Rust-native.
///
/// # Errors
///
/// Returns an error when routing, check mode, or write mode is absent.
fn require_public_tree_sync_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/check-public-contracts/main.rs"));
    return require_contains_all(
        source.as_str(),
        &[
            (
                "\"sync-public-tree-policy\" => return run_sync_public_tree_policy(&mut args)",
                "Rust public contracts must expose deterministic public-tree synchronization",
            ),
            (
                "\"--check\" => repo_hygiene_required::check_current_public_tree_policy()",
                "public-tree synchronization must support a read-only check mode",
            ),
            (
                "\"--write\" => repo_hygiene_required::synchronize_public_tree_policy()",
                "public-tree synchronization must write through reviewed Rust policy",
            ),
        ],
    );
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
            "self.run(\"typos\", &[\"--isolated\", \".\"])",
            "Rust check-all must call the Rust-native typos checker directly",
        ),
    ] {
        check_try!(require_contains(check_all.as_str(), snippet, label));
    }
    check_try!(require_contains(
        check_all.as_str(),
        "&[\"repo-hygiene\", \"head\"]",
        "Rust check-all must inspect bytes aligned with the pushed HEAD snapshot",
    ));
    let check_pre_commit = check_try!(read_text("checks/src/bin/check-pre-commit.rs"));
    return require_contains(
        check_pre_commit.as_str(),
        "(\"check-public-contracts\", &[\"repo-hygiene\", \"index\"])",
        "Rust pre-commit must inspect bytes aligned with the staged Git index",
    );
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
