use crate::helpers::{CheckResult, ascii_term, reject_contains, require_contains, require_equal};

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
        require_contains(
            source,
            "brew tap tovuk/tovuk https://github.com/tovuk/tovuk",
            "main-repo Homebrew tap command",
        )?;
        require_contains(
            source,
            "brew install tovuk",
            "simple Homebrew install command",
        )?;
        require_contains(source, "public data only", "public-data boundary")?;
    }
    Ok(())
}

pub(super) fn require_package_metadata(sources: &ContractSources) -> CheckResult {
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
    require_contains(
        sources.homebrew_formula.as_str(),
        r#"depends_on "rust" => :build"#,
        "Homebrew builds native Rust CLI",
    )?;
    require_contains(
        sources.homebrew_formula.as_str(),
        "crates/tovuk",
        "Homebrew installs Rust crate path",
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
        != "bin/tovuk"
    {
        return Err("npm package must expose bin/tovuk".to_owned());
    }
    if !sources.npm_package.dependencies.is_empty()
        || !sources.npm_package.dev_dependencies.is_empty()
    {
        return Err("npm package must not ship runtime JavaScript dependencies".to_owned());
    }
    Ok(())
}

pub(super) fn reject_retired_packaging(sources: &ContractSources) -> CheckResult {
    let retired_org_scope = ascii_term(&[64, 122, 101, 114, 99, 116]);
    for source in [
        sources.cargo_cli.as_str(),
        sources.npm_install.as_str(),
        sources.python_cli.as_str(),
        sources.cargo_readme.as_str(),
        sources.npm_readme.as_str(),
        sources.python_readme.as_str(),
        sources.homebrew_formula.as_str(),
    ] {
        reject_contains(source, "TOVUK_NPM_CLI", "retired npm delegation")?;
        reject_contains(source, "NPM_PACKAGE_VERSION", "retired npm package pin")?;
        reject_contains(source, "npx -y", "retired npx delegation")?;
        reject_contains(source, "tovuk/tap", "retired archived Homebrew tap")?;
        reject_contains(
            source,
            "brew install tovuk/tovuk/tovuk",
            "retired qualified Homebrew install",
        )?;
        reject_contains(source, "--app", "retired app flag")?;
        reject_contains(source, "/v1/apps", "retired apps API path")?;
        reject_contains(source, retired_org_scope.as_str(), "retired org scope")?;
    }
    Ok(())
}
