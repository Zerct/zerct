use alloc::collections::BTreeMap;

use crate::{
    helpers::{
        CheckResult, OutputChannel, file_exists, map_keys, must_abs, read_package_json, read_text,
        require_equal, require_results, require_snippets, require_string_map_keys_exactly,
        require_string_slice_exactly, write_line,
    },
    types::PackageJson,
};

use std::{
    ffi::OsStr,
    fs::{metadata as file_metadata, read_dir as read_directory},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
};

/// Contract value named `PUBLIC_CONTRACTS_COMMAND`.
const PUBLIC_CONTRACTS_COMMAND: &str = "cargo run --locked --quiet --manifest-path ../../checks/Cargo.toml --bin check-public-contracts --";

/// Contract value named `SYNC_NATIVE_TARGETS_COMMAND`.
const SYNC_NATIVE_TARGETS_COMMAND: &str = "cargo run --locked --quiet --manifest-path ../../checks/Cargo.toml --bin sync-native-release-targets -- --check";

/// Independent npm manifest policy facets.
trait ManifestPolicy {
    /// Reject runtime and development dependency declarations.
    ///
    /// # Errors
    ///
    /// Returns an error when the zero-dependency wrapper contract is violated.
    fn require_dependency_policy(&self) -> CheckResult;

    /// Require the public npm package identity fields.
    ///
    /// # Errors
    ///
    /// Returns an error when package identity metadata is missing or stale.
    fn require_identity_policy(&self) -> CheckResult;

    /// Require engine, publication, repository, and executable metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when required publication metadata is missing or stale.
    fn require_metadata_policy(&self) -> CheckResult;
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000a] = [
    size_of_val(&NpmPackagePaths::new),
    size_of_val(&check_cli_package_contract),
    size_of_val(&require_executable_bin),
    size_of_val(&require_install_source),
    size_of_val(&require_manifest_policy),
    size_of_val(&require_package_scripts),
    size_of_val(&require_published_files),
    size_of_val(&require_runtime_mjs_set),
    size_of_val(&required_files),
    size_of_val(&required_package_scripts),
];

#[derive(Debug)]
/// Contract representation for `NpmPackagePaths`.
pub(super) struct NpmPackagePaths {
    /// Contract data stored in `install`.
    install: PathBuf,
    /// Contract data stored in `install_policy`.
    install_policy: PathBuf,
    /// Contract data stored in `launcher`.
    launcher: PathBuf,
    /// Contract data stored in `package_dir`.
    package_dir: PathBuf,
    /// Contract data stored in `package_json`.
    package_json: PathBuf,
}

impl NpmPackagePaths {
    /// Contract implementation for `new`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    pub(super) fn new() -> CheckResult<Self> {
        let repo_root = check_try!(must_abs("."));
        let package_dir = Path::new(repo_root.as_str()).join("packages").join("tovuk");
        let install = package_dir.join("install.mjs");
        let install_policy = package_dir.join("install-policy.mjs");
        let launcher = package_dir.join("bin").join("tovuk.mjs");
        let package_json = package_dir.join("package.json");
        return Ok(Self {
            install,
            install_policy,
            launcher,
            package_dir,
            package_json,
        });
    }
}

impl ManifestPolicy for PackageJson {
    fn require_dependency_policy(&self) -> CheckResult {
        return [
            (&self.dependencies, "runtime"),
            (&self.dev_dependencies, "development"),
        ]
        .into_iter()
        .find(|candidate| return !candidate.0.is_empty())
        .map_or(Ok(()), |candidate| {
            return Err(format!("{} dependencies must be omitted", candidate.1));
        });
    }

    fn require_identity_policy(&self) -> CheckResult {
        check_try!(require_results(
            [
                (self.name.as_str(), "tovuk", "package name"),
                (self.package_type.as_str(), "module", "package type"),
                (
                    self.description.as_str(),
                    "Use Tovuk scraper APIs from a native CLI.",
                    "package description",
                ),
                (
                    self.homepage.as_str(),
                    "https://tovuk.com",
                    "package homepage"
                ),
                (self.license.as_str(), "MIT", "package license"),
            ]
            .map(|(actual, expected, label)| return require_equal(actual, expected, label))
        ));
        if self.private.is_some() {
            return Err("package private flag must be omitted".to_owned());
        }
        return Ok(());
    }

    fn require_metadata_policy(&self) -> CheckResult {
        return require_results(
            [
                (self.engines.get("node"), ">=22.22.3", "Node engine"),
                (
                    self.publish_config.get("access"),
                    "public",
                    "publish access",
                ),
                (
                    self.repository.get("directory"),
                    "packages/tovuk",
                    "repository directory",
                ),
                (self.bin.get("tovuk"), "bin/tovuk.mjs", "tovuk bin path"),
            ]
            .map(|(actual, expected, label)| {
                return require_equal(actual.map_or("", String::as_str), expected, label);
            }),
        );
    }
}

/// Contract implementation for `check_cli_package_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_cli_package_contract() -> CheckResult {
    let paths = check_try!(NpmPackagePaths::new());
    let required_files = required_files();
    let required_scripts = required_package_scripts();
    let package_json = check_try!(read_package_json(paths.package_json.as_path()));
    check_try!(require_manifest_policy(&package_json));
    check_try!(require_published_files(
        &paths,
        &package_json,
        &required_files
    ));
    check_try!(require_package_scripts(&package_json, &required_scripts));
    check_try!(require_install_source(
        paths.install.as_path(),
        paths.install_policy.as_path()
    ));
    check_try!(require_executable_bin(paths.launcher.as_path()));
    check_try!(require_runtime_mjs_set(&paths));
    check_try!(write_line(
        OutputChannel::Regular,
        "Checked npm native CLI package policy.",
    ));
    return Ok(());
}

/// Contract implementation for `require_executable_bin`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_executable_bin(bin_path: &Path) -> CheckResult {
    let metadata =
        check_try!(file_metadata(bin_path).map_err(|error| format!("stat bin/tovuk.mjs: {error}")));
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err("bin/tovuk.mjs must stay executable".to_owned());
    }
    return Ok(());
}

/// Contract implementation for `require_install_source`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_install_source(install_path: &Path, policy_path: &Path) -> CheckResult {
    let install_source = check_try!(read_text(install_path));
    let policy_source = check_try!(read_text(policy_path));
    let installer_source = format!("{install_source}\n{policy_source}");
    return require_snippets(
        installer_source.as_str(),
        "npm installer modules",
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
    );
}

/// Contract implementation for `require_manifest_policy`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_manifest_policy(package_json: &PackageJson) -> CheckResult {
    check_try!(package_json.require_dependency_policy());
    check_try!(package_json.require_identity_policy());
    return package_json.require_metadata_policy();
}

/// Contract implementation for `require_package_scripts`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_package_scripts(
    package_json: &PackageJson,
    required_scripts: &BTreeMap<String, String>,
) -> CheckResult {
    check_try!(require_string_map_keys_exactly(
        &package_json.scripts,
        &map_keys(required_scripts),
        "scripts",
    ));
    for (script, command) in required_scripts {
        check_try!(require_equal(
            package_json
                .scripts
                .get(script.as_str())
                .map_or("", String::as_str),
            command.as_str(),
            format!("{script} script").as_str(),
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_published_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_published_files(
    paths: &NpmPackagePaths,
    package_json: &PackageJson,
    required_files: &[String],
) -> CheckResult {
    check_try!(require_string_slice_exactly(
        &package_json.files,
        required_files,
        "published files"
    ));
    for file in required_files {
        if !file_exists(paths.package_dir.join(file)) {
            return Err(format!("published file entry does not exist: {file}"));
        }
    }
    return Ok(());
}

/// Require exactly the launcher plus installer modules as published runtime MJS sources.
///
/// # Errors
///
/// Returns an error when the published bin directory cannot be read or contains
/// another MJS source.
fn require_runtime_mjs_set(paths: &NpmPackagePaths) -> CheckResult {
    let bin_directory = paths.package_dir.join("bin");
    let entries = check_try!(
        read_directory(bin_directory.as_path())
            .map_err(|error| return format!("read {}: {error}", bin_directory.display()))
    );
    let mut runtime_sources = Vec::new();
    for entry_result in entries {
        let entry = check_try!(
            entry_result
                .map_err(|error| return format!("read {} entry: {error}", bin_directory.display()))
        );
        let path = entry.path();
        if path.extension() == Some(OsStr::new("mjs")) {
            runtime_sources.push(path);
        }
    }
    runtime_sources.sort();
    if runtime_sources == [paths.launcher.clone()]
        && paths.install.is_file()
        && paths.install_policy.is_file()
    {
        return Ok(());
    }
    return Err(
        "npm package must publish exactly install.mjs, install-policy.mjs, and bin/tovuk.mjs as runtime MJS sources"
            .to_owned(),
    );
}

/// Contract implementation for `required_files`.
pub(super) fn required_files() -> Vec<String> {
    return vec![
        "LICENSE".to_owned(),
        "bin".to_owned(),
        "install-policy.mjs".to_owned(),
        "install.mjs".to_owned(),
        "native-release-targets.json".to_owned(),
        "README.md".to_owned(),
    ];
}

/// Contract implementation for `required_package_scripts`.
pub(super) fn required_package_scripts() -> BTreeMap<String, String> {
    return BTreeMap::from([
        (
            "check".to_owned(),
            "npm run check:policy && npm run format:check && npm run lint && npm test && npm run runtime && npm run pack:dry".to_owned(),
        ),
        (
            "check:policy".to_owned(),
            format!("{PUBLIC_CONTRACTS_COMMAND} npm-cli-package"),
        ),
        (
            "format:check".to_owned(),
            "npx --yes prettier@3.9.5 --check install.mjs install-policy.mjs bin/tovuk.mjs tests/wrapper.test.mjs"
                .to_owned(),
        ),
        (
            "lint".to_owned(),
            "npx --yes oxlint@1.73.0 --config ../../.oxlintrc.json --deny-warnings --report-unused-disable-directives install.mjs install-policy.mjs bin/tovuk.mjs tests/wrapper.test.mjs"
                .to_owned(),
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
        (
            "test".to_owned(),
            "node --test tests/wrapper.test.mjs".to_owned(),
        ),
    ]);
}
