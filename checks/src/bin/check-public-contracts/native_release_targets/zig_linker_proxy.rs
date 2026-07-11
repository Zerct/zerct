//! Strict Zig linker proxy release policy.

use crate::helpers::{CheckResult, read_text, read_text_corpus, require_snippets};

/// Require strict native-linker configuration for Linux ARM64 and Windows.
///
/// # Errors
///
/// Returns an error when exact filtering, response-file safety, pinned Zig
/// delegation, bundled Windows LLD selection, or regression coverage is absent.
pub(super) fn require_zig_linker_proxy_contract() -> CheckResult {
    for path in [".cargo/config.toml", "crates/tovuk/.cargo/config.toml"] {
        let config = check_try!(read_text(path));
        check_try!(require_snippets(
            config.as_str(),
            path,
            &[
                "[target.x86_64-pc-windows-msvc]",
                "linker = \"rust-lld\"",
                "linker-flavor=lld-link",
                "link-arg=/NOIMPLIB",
            ],
        ));
    }
    let source = check_try!(read_text_corpus(&[
        "checks/src/bin/zig-linker-proxy.rs",
        "checks/src/bin/zig_linker_proxy_tests/verification.rs",
    ]));
    return require_snippets(
        source.as_str(),
        "zig-linker-proxy.rs and tests",
        &[
            "const DEPRECATED_LINKER_OPTIMIZATION: &str = \"-Wl,-O1\";",
            "argument == OsStr::new(DEPRECATED_LINKER_OPTIMIZATION)",
            "logical_line == DEPRECATED_LINKER_OPTIMIZATION",
            "contains_deprecated_option",
            "response-file argument must be valid UTF-8",
            "create_new(true)",
            "REAL_ZIG_PATH_ENVIRONMENT",
            "Command::new(real_zig)",
            "prepared.cleanup()",
            "direct_exact_option_is_removed",
            "response_exact_lines_are_removed_from_a_copy",
            "response_ambiguous_option_is_rejected",
            "response_invalid_utf8_is_rejected",
        ],
    );
}
