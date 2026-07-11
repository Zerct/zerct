use crate::helpers::{CheckResult, read_text, require_snippets};

/// Require standalone quality-tool downloads to use the Rust checksum verifier.
///
/// # Errors
///
/// Returns an error when quality-tool checksum validation bypasses Rust policy.
pub(super) fn require_quality_tool_checksum_contract() -> CheckResult {
    let source = check_try!(read_text(".github/actions/setup-quality-tools/action.yml"));
    check_try!(require_snippets(
        source.as_str(),
        "setup-quality-tools/action.yml",
        &[
            "verify-sha256 \"$actionlint_archive\"",
            "verify-sha256 \"$shellcheck_archive\"",
            "verify-sha256 \"$shfmt_binary\"",
        ],
    ));
    for forbidden in ["sha256sum", "shasum -a 256"] {
        if source.contains(forbidden) {
            return Err(format!(
                "setup-quality-tools/action.yml must not use {forbidden}; use the Rust checksum verifier"
            ));
        }
    }
    return Ok(());
}
