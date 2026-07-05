use crate::helpers::{
    CheckResult, reject_contains_any, require_contains, require_contains_all, require_equal,
};
use crate::helpers_public_copy::RETIRED_PUBLIC_ORG_SCOPE;

use super::ContractSources;

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
        require_contains_all(
            source,
            &[
                (
                    "brew tap tovuk/tovuk https://github.com/tovuk/tovuk",
                    "main-repo Homebrew tap command",
                ),
                ("brew install tovuk", "simple Homebrew install command"),
                ("public data only", "public-data boundary"),
            ],
        )?;
    }
    Ok(())
}

pub(super) fn require_package_metadata(sources: &ContractSources) -> CheckResult {
    require_contains_all(
        sources.homebrew_formula.as_str(),
        &[
            (
                r#"depends_on "rust" => :build"#,
                "Homebrew builds native Rust CLI",
            ),
            ("crates/tovuk", "Homebrew installs Rust crate path"),
        ],
    )?;
    require_contains(
        sources.npm_install.as_str(),
        "TOVUK_NATIVE_BINARY",
        "npm local native binary override",
    )?;
    require_contains(
        sources.python_cli.as_str(),
        "TOVUK_NATIVE_BINARY",
        "PyPI local native binary override",
    )?;
    require_equal(
        sources.npm_package.description.as_str(),
        "Use Tovuk scraper APIs from a native CLI.",
        "npm package description",
    )?;
    require_contains(
        sources.python_project.as_str(),
        r#"description = "Use Tovuk scraper APIs from a native CLI.""#,
        "PyPI package description",
    )?;
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
    Ok(())
}

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
        reject_contains_any(
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
        )?;
    }
    Ok(())
}
