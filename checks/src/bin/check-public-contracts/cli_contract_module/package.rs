use crate::helpers::{
    CheckResult, reject_contains_any, require_contains, require_contains_all, require_equal,
};

use crate::helpers_public_copy::RETIRED_PUBLIC_ORG_SCOPE;

use super::ContractSources;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0007] = [
    size_of_val(&reject_retired_packaging),
    size_of_val(&require_package_policy_homebrew),
    size_of_val(&require_install_guides),
    size_of_val(&require_package_policy_local_overrides),
    size_of_val(&require_package_metadata),
    size_of_val(&require_package_policy_npm),
    size_of_val(&require_package_policy_python),
];

/// Contract implementation for `reject_retired_packaging`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_retired_packaging(sources: &ContractSources) -> CheckResult {
    for source in [
        sources.cargo_cli.as_str(),
        sources.npm_install.as_str(),
        sources.python_cli.as_str(),
        sources.cargo_readme.as_str(),
        sources.npm_readme.as_str(),
        sources.python_readme.as_str(),
        sources.homebrew_formula.as_str(),
    ] {
        check_try!(reject_contains_any(
            source,
            &[
                ("TOVUK_NPM_CLI", "retired npm delegation"),
                ("NPM_PACKAGE_VERSION", "retired npm package pin"),
                ("npx -y", "retired npx delegation"),
                ("tovuk/tap", "retired archived Homebrew tap"),
                (
                    "brew install tovuk/tovuk/tovuk",
                    "retired qualified Homebrew install",
                ),
                ("--app", "retired app flag"),
                ("/v1/apps", "retired apps API path"),
                (RETIRED_PUBLIC_ORG_SCOPE, "retired org scope"),
            ],
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_install_guides`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_install_guides(sources: &ContractSources) -> CheckResult {
    for source in [
        sources.root_readme.as_str(),
        sources.cargo_readme.as_str(),
        sources.npm_readme.as_str(),
        sources.python_readme.as_str(),
        sources.docs_index.as_str(),
        sources.docs_quickstart.as_str(),
        sources.docs_packages.as_str(),
        sources.docs_llms.as_str(),
    ] {
        check_try!(require_contains_all(
            source,
            &[
                (
                    "brew tap tovuk/tovuk https://github.com/tovuk/tovuk",
                    "main-repo Homebrew tap command",
                ),
                ("brew install tovuk", "simple Homebrew install command"),
                (
                    "cargo install --locked tovuk",
                    "locked Cargo install command"
                ),
                ("public data only", "public-data boundary"),
            ],
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_package_metadata`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_package_metadata(sources: &ContractSources) -> CheckResult {
    check_try!(require_package_policy_homebrew(sources));
    check_try!(require_package_policy_local_overrides(sources));
    check_try!(require_package_policy_npm(sources));
    return require_package_policy_python(sources);
}

/// Require the Homebrew formula to build the native Rust package.
///
/// # Errors
///
/// Returns an error when the Homebrew build contract is missing.
fn require_package_policy_homebrew(sources: &ContractSources) -> CheckResult {
    check_try!(require_contains_all(
        sources.homebrew_formula.as_str(),
        &[
            (
                r#"depends_on "rust" => :build"#,
                "Homebrew builds native Rust CLI",
            ),
            ("crates/tovuk", "Homebrew installs Rust crate path"),
        ],
    ));
    return Ok(());
}

/// Require both thin launchers to retain their local native override.
///
/// # Errors
///
/// Returns an error when a local native override is missing.
fn require_package_policy_local_overrides(sources: &ContractSources) -> CheckResult {
    check_try!(require_contains(
        sources.npm_install.as_str(),
        "TOVUK_NATIVE_BINARY",
        "npm local native binary override",
    ));
    check_try!(require_contains(
        sources.python_cli.as_str(),
        "TOVUK_NATIVE_BINARY",
        "PyPI local native binary override",
    ));
    return Ok(());
}

/// Require the zero-dependency npm package metadata contract.
///
/// # Errors
///
/// Returns an error when npm metadata violates the wrapper contract.
fn require_package_policy_npm(sources: &ContractSources) -> CheckResult {
    check_try!(require_equal(
        sources.npm_package.description.as_str(),
        "Use Tovuk scraper APIs from a native CLI.",
        "npm package description",
    ));
    if sources
        .npm_package
        .bin
        .get("tovuk")
        .map_or("", String::as_str)
        != "bin/tovuk.mjs"
    {
        return Err("npm package must expose bin/tovuk.mjs".to_owned());
    }
    if !sources.npm_package.dependencies.is_empty()
        || !sources.npm_package.dev_dependencies.is_empty()
    {
        return Err("npm package must not ship runtime JavaScript dependencies".to_owned());
    }
    return Ok(());
}

/// Require the pinned `PyPI` build and supported Python metadata.
///
/// # Errors
///
/// Returns an error when Python package metadata drifts.
fn require_package_policy_python(sources: &ContractSources) -> CheckResult {
    check_try!(require_contains(
        sources.python_project.as_str(),
        r#"description = "Use Tovuk scraper APIs from a native CLI.""#,
        "PyPI package description",
    ));
    check_try!(require_contains_all(
        sources.python_project.as_str(),
        &[
            (
                r#"requires = ["uv_build==0.11.28"]"#,
                "pinned PyPI build backend",
            ),
            (r#"requires-python = ">=3.11""#, "PyPI Python floor"),
            (
                r#""Programming Language :: Python :: 3.11""#,
                "Python 3.11 classifier",
            ),
            (
                r#""Programming Language :: Python :: 3.12""#,
                "Python 3.12 classifier",
            ),
            (
                r#""Programming Language :: Python :: 3.13""#,
                "Python 3.13 classifier",
            ),
            (
                r#""Programming Language :: Python :: 3.14""#,
                "Python 3.14 classifier",
            ),
        ],
    ));
    return Ok(());
}
