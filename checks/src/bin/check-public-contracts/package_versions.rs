use crate::helpers::{
    CheckResult, OutputChannel, extract_cargo_lock_package_version, extract_line_quoted_value,
    read_package_json, read_text, require_contains, require_contains_all, write_line,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&check),
    size_of_val(&print_canonical_version),
    size_of_val(&synchronized_version),
    size_of_val(&synchronized_version_require_packages),
    size_of_val(&synchronized_version_require_runtime),
    size_of_val(&synchronized_version_require_tag),
];

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check() -> CheckResult {
    drop(check_try!(synchronized_version()));
    return write_line(
        OutputChannel::Regular,
        "Checked package version consistency.",
    );
}

/// Print the canonical version after enforcing cross-package synchronization.
///
/// # Errors
///
/// Returns an error when package versions drift or output cannot be written.
pub(super) fn print_canonical_version() -> CheckResult {
    let version = check_try!(synchronized_version());
    return write_line(OutputChannel::Regular, version.as_str());
}

/// Validate every public package version and return the canonical Cargo value.
///
/// # Errors
///
/// Returns an error when package metadata cannot be read or versions differ.
fn synchronized_version() -> CheckResult<String> {
    let cargo_toml = check_try!(read_text("crates/tovuk/Cargo.toml"));
    let canonical = check_try!(extract_line_quoted_value(
        cargo_toml.as_str(),
        "version = ",
        "Cargo.toml version",
    ));
    let npm_package = check_try!(read_package_json("packages/tovuk/package.json"));
    check_try!(synchronized_version_require_packages(
        canonical.as_str(),
        npm_package.version.as_str(),
    ));
    check_try!(synchronized_version_require_runtime());
    check_try!(synchronized_version_require_tag(canonical.as_str()));
    return Ok(canonical);
}

/// Require every duplicated registry version to match Cargo metadata.
///
/// # Errors
///
/// Returns an error when a registry package version differs.
fn synchronized_version_require_packages(canonical: &str, npm_version: &str) -> CheckResult {
    let pyproject = check_try!(read_text("packages/tovuk-py/pyproject.toml"));
    let py_init = check_try!(read_text("packages/tovuk-py/src/tovuk/__init__.py"));
    let cargo_lock = check_try!(read_text("crates/tovuk/Cargo.lock"));
    for (label, version) in [
        ("npm package", npm_version.to_owned()),
        (
            "PyPI project",
            check_try!(extract_line_quoted_value(
                pyproject.as_str(),
                "version = ",
                "PyPI project version"
            )),
        ),
        (
            "Python package",
            check_try!(extract_line_quoted_value(
                py_init.as_str(),
                "__version__ = ",
                "Python package version",
            )),
        ),
        (
            "Cargo.lock",
            check_try!(extract_cargo_lock_package_version(
                cargo_lock.as_str(),
                "tovuk"
            )),
        ),
    ] {
        if version != canonical {
            return Err(format!(
                "{label} {version} does not match Cargo {canonical}"
            ));
        }
    }
    return Ok(());
}

/// Require launchers to derive release paths from synchronized metadata.
///
/// # Errors
///
/// Returns an error when a launcher duplicates or ignores canonical metadata.
fn synchronized_version_require_runtime() -> CheckResult {
    let python_cli = check_try!(read_text("packages/tovuk-py/src/tovuk/cli.py"));
    let cargo_constants = check_try!(read_text("crates/tovuk/src/cli/constants.rs"));
    check_try!(require_contains_all(
        python_cli.as_str(),
        &[
            (
                "releases/download/v{__version__}",
                "Python native binary downloader release path",
            ),
            (
                "tovuk-{__version__}-",
                "Python native binary downloader asset path",
            ),
            (".sha256", "Python native binary checksum asset path"),
            (
                "hashlib.sha256",
                "Python native binary checksum verification",
            ),
        ],
    ));
    return require_contains(
        cargo_constants.as_str(),
        "const VERSION: &str = env!(\"CARGO_PKG_VERSION\");",
        "Cargo CLI version must derive from CARGO_PKG_VERSION",
    );
}

/// Require the Homebrew tag to match the canonical Cargo version.
///
/// # Errors
///
/// Returns an error when the Homebrew tag differs.
fn synchronized_version_require_tag(canonical: &str) -> CheckResult {
    let formula = check_try!(read_text("Formula/tovuk.rb"));
    return require_contains(
        formula.as_str(),
        format!("tag: \"v{canonical}\"").as_str(),
        "Homebrew formula version tag",
    );
}
