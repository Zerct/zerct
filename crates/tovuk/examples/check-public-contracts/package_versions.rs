use crate::helpers::{
    CheckResult, extract_cargo_lock_package_version, extract_line_quoted_value,
    extract_rust_const_str, read_package_json, read_text, require_contains,
};

pub(crate) fn check() -> CheckResult {
    let npm_package = read_package_json("packages/tovuk/package.json")?;
    let pyproject = read_text("packages/tovuk-py/pyproject.toml")?;
    let py_init = read_text("packages/tovuk-py/src/tovuk/__init__.py")?;
    let python_cli = read_text("packages/tovuk-py/src/tovuk/cli.py")?;
    let cargo_toml = read_text("crates/tovuk/Cargo.toml")?;
    let cargo_lock = read_text("crates/tovuk/Cargo.lock")?;
    let cargo_cli_constants = read_text("crates/tovuk/src/cli/constants.rs")?;
    let formula = read_text("Formula/tovuk.rb")?;

    for (label, version) in [
        (
            "PyPI project",
            extract_line_quoted_value(pyproject.as_str(), "version = ", "PyPI project version")?,
        ),
        (
            "Python package",
            extract_line_quoted_value(
                py_init.as_str(),
                "__version__ = ",
                "Python package version",
            )?,
        ),
        (
            "Cargo.toml",
            extract_line_quoted_value(cargo_toml.as_str(), "version = ", "Cargo.toml version")?,
        ),
        (
            "Cargo.lock",
            extract_cargo_lock_package_version(cargo_lock.as_str(), "tovuk")?,
        ),
        (
            "Cargo CLI",
            extract_rust_const_str(cargo_cli_constants.as_str(), "VERSION", "Cargo CLI version")?,
        ),
    ] {
        if version != npm_package.version {
            return Err(format!(
                "{label} {version} does not match npm package {}",
                npm_package.version
            ));
        }
    }

    require_contains(
        python_cli.as_str(),
        "releases/download/v{__version__}",
        "Python native binary downloader release path",
    )?;
    require_contains(
        python_cli.as_str(),
        "tovuk-{__version__}-",
        "Python native binary downloader asset path",
    )?;
    require_contains(
        formula.as_str(),
        format!("tag: \"v{}\"", npm_package.version).as_str(),
        "Homebrew formula version tag",
    )?;

    println!("Checked package version consistency.");
    Ok(())
}
