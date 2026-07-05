use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::{
    helpers::{
        CheckResult, file_exists, map_keys, must_abs, read_package_json, read_text, require_equal,
        require_snippets, require_string_map_keys_exactly, require_string_slice_exactly,
    },
    types::PackageJson,
};

const PUBLIC_CONTRACTS_COMMAND: &str = "cargo run --locked --quiet --manifest-path ../../checks/Cargo.toml --bin check-public-contracts --";
const SYNC_NATIVE_TARGETS_COMMAND: &str = "cargo run --locked --quiet --manifest-path ../../checks/Cargo.toml --bin sync-native-release-targets --";

#[derive(Debug)]
struct NpmPackagePaths {
    package_dir: PathBuf,
    package_json: PathBuf,
    install: PathBuf,
    launcher: PathBuf,
}

pub(crate) fn check_cli_package_contract() -> CheckResult {
    let paths = NpmPackagePaths::new()?;
    let required_files = required_files();
    let required_scripts = required_package_scripts();
    let package_json = read_package_json(paths.package_json.as_path())?;
    require_manifest_policy(&package_json)?;
    require_published_files(&paths, &package_json, &required_files)?;
    require_package_scripts(&package_json, &required_scripts)?;
    require_install_source(paths.install.as_path())?;
    require_executable_bin(paths.launcher.as_path())?;
    println!("Checked npm native CLI package policy.");
    Ok(())
}

impl NpmPackagePaths {
    fn new() -> CheckResult<Self> {
        let repo_root = must_abs(".")?;
        let package_dir = Path::new(repo_root.as_str()).join("packages").join("tovuk");
        Ok(Self {
            package_json: package_dir.join("package.json"),
            install: package_dir.join("install.mjs"),
            launcher: package_dir.join("bin").join("tovuk.mjs"),
            package_dir,
        })
    }
}

fn required_files() -> Vec<String> {
    vec![
        "bin".to_owned(),
        "install.mjs".to_owned(),
        "native-release-targets.json".to_owned(),
        "README.md".to_owned(),
    ]
}

fn required_package_scripts() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "check".to_owned(),
            "npm run check:policy && npm run runtime && npm run pack:dry".to_owned(),
        ),
        (
            "check:policy".to_owned(),
            format!("{PUBLIC_CONTRACTS_COMMAND} npm-cli-package"),
        ),
        ("pack:dry".to_owned(), "npm pack --dry-run".to_owned()),
        ("postinstall".to_owned(), "node install.mjs".to_owned()),
        (
            "precheck".to_owned(),
            SYNC_NATIVE_TARGETS_COMMAND.to_owned(),
        ),
        ("prepack".to_owned(), SYNC_NATIVE_TARGETS_COMMAND.to_owned()),
        (
            "runtime".to_owned(),
            format!("{PUBLIC_CONTRACTS_COMMAND} npm-native-runtime"),
        ),
    ])
}

fn require_manifest_policy(package_json: &PackageJson) -> CheckResult {
    require_equal(package_json.name.as_str(), "tovuk", "package name")?;
    require_equal(package_json.package_type.as_str(), "module", "package type")?;
    require_equal(
        package_json.description.as_str(),
        "Use Tovuk scraper APIs from a native CLI.",
        "package description",
    )?;
    require_equal(
        package_json.homepage.as_str(),
        "https://tovuk.com",
        "package homepage",
    )?;
    require_equal(package_json.license.as_str(), "MIT", "package license")?;
    if package_json.private.is_some() {
        return Err("package private flag must be omitted".to_owned());
    }
    require_equal(
        package_json.engines.get("node").map_or("", String::as_str),
        ">=18.17",
        "Node engine",
    )?;
    require_equal(
        package_json
            .publish_config
            .get("access")
            .map_or("", String::as_str),
        "public",
        "publish access",
    )?;
    require_equal(
        package_json
            .repository
            .get("directory")
            .map_or("", String::as_str),
        "packages/tovuk",
        "repository directory",
    )?;
    require_equal(
        package_json.bin.get("tovuk").map_or("", String::as_str),
        "bin/tovuk.mjs",
        "tovuk bin path",
    )?;
    if !package_json.dependencies.is_empty() {
        return Err("runtime dependencies must be omitted".to_owned());
    }
    if !package_json.dev_dependencies.is_empty() {
        return Err("development dependencies must be omitted".to_owned());
    }
    Ok(())
}

fn require_published_files(
    paths: &NpmPackagePaths,
    package_json: &PackageJson,
    required_files: &[String],
) -> CheckResult {
    require_string_slice_exactly(&package_json.files, required_files, "published files")?;
    for file in required_files {
        if !file_exists(paths.package_dir.join(file)) {
            return Err(format!("published file entry does not exist: {file}"));
        }
    }
    Ok(())
}

fn require_package_scripts(
    package_json: &PackageJson,
    required_scripts: &BTreeMap<String, String>,
) -> CheckResult {
    require_string_map_keys_exactly(
        &package_json.scripts,
        &map_keys(required_scripts),
        "scripts",
    )?;
    for (script, command) in required_scripts {
        require_equal(
            package_json
                .scripts
                .get(script.as_str())
                .map_or("", String::as_str),
            command.as_str(),
            format!("{script} script").as_str(),
        )?;
    }
    Ok(())
}

fn require_install_source(install_path: &Path) -> CheckResult {
    let install_source = read_text(install_path)?;
    require_snippets(
        install_source.as_str(),
        "install.mjs",
        &[
            "https://github.com/tovuk/tovuk/releases/download",
            ".sha256",
            "nativeTargets",
            "target.asset_ext",
            "verifySha256",
            "linuxLibc",
            "requires glibc Linux",
            "TOVUK_NATIVE_BINARY",
            "nativeBinaryName",
        ],
    )
}

fn require_executable_bin(bin_path: &Path) -> CheckResult {
    let metadata =
        fs::metadata(bin_path).map_err(|error| format!("stat bin/tovuk.mjs: {error}"))?;
    if metadata.permissions().mode() & 0o111 == 0 {
        Err("bin/tovuk.mjs must stay executable".to_owned())
    } else {
        Ok(())
    }
}
