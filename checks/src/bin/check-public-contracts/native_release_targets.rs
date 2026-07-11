/// Standalone release-tool checksum workflow policy.
#[path = "native_release_targets/quality_tools.rs"]
mod quality_tools;

use alloc::collections::BTreeSet;

use crate::helpers::{
    CheckResult, OutputChannel, read_json, read_text, read_text_corpus, require_contains,
    require_snippets, write_line,
};

use quality_tools::require_quality_tool_checksum_contract;

use serde::Deserialize;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0010] = [
    size_of_val(&require_manifest_unique_value),
    size_of_val(&read_manifest),
    size_of_val(&reject_legacy_workflow_contract),
    size_of_val(&require_contains_json_file),
    size_of_val(&require_manifest_shape),
    size_of_val(&require_npm_installer_contract),
    size_of_val(&require_python_installer_contract),
    size_of_val(&require_quality_tool_checksum_contract),
    size_of_val(&require_release_tool_contract),
    size_of_val(&require_release_gate_contract),
    size_of_val(&require_sync_binary_contract),
    size_of_val(&require_target_shape),
    size_of_val(&require_target_shape_asset),
    size_of_val(&require_target_shape_libc),
    size_of_val(&require_target_shape_runner),
    size_of_val(&require_workflow_contract),
];

#[derive(Debug, Deserialize)]
/// Contract representation for `NativeReleaseTargets`.
pub(super) struct NativeReleaseTargets {
    /// Contract data stored in `targets`.
    targets: Vec<NativeTarget>,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `NativeTarget`.
pub(super) struct NativeTarget {
    /// Contract data stored in `asset_ext`.
    asset_ext: String,
    /// Contract data stored in `binary`.
    binary: String,
    #[serde(default)]
    /// Contract data stored in `libc`.
    libc: Option<String>,
    /// Contract data stored in `node`.
    node: NodeTarget,
    /// Contract data stored in `python`.
    python: Vec<PythonTarget>,
    /// Contract data stored in `runner`.
    runner: String,
    /// Contract data stored in `triple`.
    triple: String,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `NodeTarget`.
struct NodeTarget {
    /// Contract data stored in `arch`.
    arch: String,
    /// Contract data stored in `platform`.
    platform: String,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `PythonTarget`.
struct PythonTarget {
    /// Contract data stored in `machine`.
    machine: String,
    /// Contract data stored in `system`.
    system: String,
}

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check() -> CheckResult {
    let manifest = check_try!(read_manifest());
    check_try!(require_manifest_shape(&manifest));
    check_try!(require_sync_binary_contract());
    check_try!(require_workflow_contract());
    check_try!(require_release_gate_contract());
    check_try!(require_npm_installer_contract());
    check_try!(require_python_installer_contract());
    check_try!(require_quality_tool_checksum_contract());
    check_try!(write_line(
        OutputChannel::Regular,
        "Checked native release target contracts.",
    ));
    return Ok(());
}

/// Contract implementation for `read_manifest`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn read_manifest() -> CheckResult<NativeReleaseTargets> {
    return read_json("native-release-targets.json");
}

/// Contract implementation for `require_workflow_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn reject_legacy_workflow_contract(source: &str) -> CheckResult {
    for forbidden in [
        "def build_strategy",
        "github.actor ==",
        "node - <<'NODE'",
        "runner_arch",
        "runner_os",
        "self-hosted",
        "status.env",
        "steps.release.outputs",
    ] {
        if source.contains(forbidden) {
            return Err(format!(
                "publish-native-binaries.yml contains retired private or inline release logic {forbidden:?}"
            ));
        }
    }
    return Ok(());
}

/// Contract implementation for `require_contains_json_file`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_contains_json_file(path: &str, file_name: &str) -> CheckResult {
    let source = check_try!(read_text(path));
    return require_contains(
        source.as_str(),
        file_name,
        format!("{path} must include {file_name} in published files").as_str(),
    );
}

/// Contract implementation for `require_generated_manifest_matches_root`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn require_generated_manifest_matches_root(path: &str) -> CheckResult {
    let root = check_try!(read_text("native-release-targets.json"));
    let packaged = check_try!(read_text(path));
    if packaged != root {
        return Err(format!(
            "{path} must match native-release-targets.json exactly"
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_manifest_shape`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_manifest_shape(manifest: &NativeReleaseTargets) -> CheckResult {
    let mut triples = BTreeSet::new();
    let mut node_aliases = BTreeSet::new();
    let mut python_aliases = BTreeSet::new();
    for target in &manifest.targets {
        check_try!(require_manifest_unique_value(
            &mut triples,
            target.triple.as_str(),
            "native release target",
        ));
        let node_alias = format!("{}/{}", target.node.platform, target.node.arch);
        check_try!(require_manifest_unique_value(
            &mut node_aliases,
            node_alias.as_str(),
            "npm native target alias",
        ));
        for alias in &target.python {
            let python_alias = format!("{}/{}", alias.system, alias.machine);
            check_try!(require_manifest_unique_value(
                &mut python_aliases,
                python_alias.as_str(),
                "PyPI native target alias",
            ));
        }
        check_try!(require_target_shape(target));
    }
    return Ok(());
}

/// Insert one release identifier and reject a duplicate.
///
/// # Errors
///
/// Returns an error when the identifier is already present.
fn require_manifest_unique_value(
    values: &mut BTreeSet<String>,
    value: &str,
    label: &str,
) -> CheckResult {
    if values.insert(value.to_owned()) {
        return Ok(());
    }
    return Err(format!("duplicate {label} {value}"));
}

/// Contract implementation for `require_npm_installer_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_npm_installer_contract() -> CheckResult {
    let source = check_try!(read_text("packages/tovuk/install.mjs"));
    check_try!(require_generated_manifest_matches_root(
        "packages/tovuk/native-release-targets.json"
    ));
    check_try!(require_contains_json_file(
        "packages/tovuk/package.json",
        "native-release-targets.json"
    ));
    check_try!(require_snippets(
        source.as_str(),
        "install.mjs",
        &[
            "native-release-targets.json",
            "target.asset_ext",
            "target.triple",
            "requires glibc Linux",
            "linuxLibc()",
            "nativeBinaryName()",
            "TOVUK_NATIVE_BINARY",
        ],
    ));
    let override_index = check_try!(
        source
            .find("process.env.TOVUK_NATIVE_BINARY")
            .ok_or_else(|| return "install.mjs must honor TOVUK_NATIVE_BINARY".to_owned())
    );
    let target_index = check_try!(source.find("const target = nativeTarget()").ok_or_else(|| {
        return "install.mjs must resolve nativeTarget inside release install".to_owned();
    }));
    if target_index < override_index {
        return Err(
            "install.mjs must not resolve the release target before local binary overrides"
                .to_owned(),
        );
    }
    return Ok(());
}

/// Contract implementation for `require_python_installer_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_python_installer_contract() -> CheckResult {
    let source = check_try!(read_text("packages/tovuk-py/src/tovuk/cli.py"));
    check_try!(require_generated_manifest_matches_root(
        "packages/tovuk-py/src/tovuk/native_release_targets.json",
    ));
    check_try!(require_snippets(
        source.as_str(),
        "cli.py",
        &[
            "native_release_targets.json",
            "target[\"asset_ext\"]",
            "target[\"binary\"]",
            "target[\"triple\"]",
            "requires glibc Linux",
            "_linux_libc()",
            "TOVUK_NATIVE_BINARY",
            "binary_name = str(target[\"binary\"])",
            "pathlib.Path(__file__).with_name(\"bin\") / binary_name",
            "/ target_triple / binary_name",
        ],
    ));
    if source.contains("/ target_triple / \"tovuk\"")
        || source.contains("/ target_triple / 'tovuk'")
        || source.contains("with_name(\"bin\") / \"tovuk\"")
        || source.contains("with_name(\"bin\") / 'tovuk'")
    {
        return Err(
            "cli.py must use manifest binary names instead of hard-coded tovuk paths".to_owned(),
        );
    }
    return Ok(());
}

/// Contract implementation for `require_release_gate_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_release_gate_contract() -> CheckResult {
    let source = check_try!(read_text_corpus(&[
        "checks/src/bin/check-native-release-assets.rs",
        "checks/src/bin/check-native-release-assets/checksum.rs",
        "checks/src/bin/check-native-release-assets/release.rs",
    ]));
    check_try!(require_snippets(
        source.as_str(),
        "check-native-release-assets.rs",
        &[
            "native-release-targets.json",
            "target.asset_ext",
            "verify_asset_checksums",
            "gh release download",
            "isDraft",
            "isPrerelease",
            "Sha256::new()",
            "checksum mismatch",
            "unexpected_assets",
        ],
    ));
    if source.contains("endswith(\".exe\")") {
        return Err("native release asset names must use explicit asset_ext metadata".to_owned());
    }
    return Ok(());
}

/// Require the platform-neutral Rust release utility contract.
///
/// # Errors
///
/// Returns an error when matrix or checksum behavior is absent.
fn require_release_tool_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/native-release-tool.rs"));
    return require_snippets(
        source.as_str(),
        "native-release-tool.rs",
        &[
            "AARCH64_LINUX_GNU_TARGET",
            "AARCH64_LINUX_GNU_ZIG_SUFFIX",
            "asset_name",
            "cargo-zigbuild",
            "release_tag",
            "verify-sha256",
            "write-sha256",
        ],
    );
}

/// Contract implementation for `require_sync_binary_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_sync_binary_contract() -> CheckResult {
    let source = check_try!(read_text("checks/src/bin/sync-native-release-targets.rs"));
    return require_snippets(
        source.as_str(),
        "sync-native-release-targets.rs",
        &[
            "const SOURCE_MANIFEST: &str = \"native-release-targets.json\";",
            "\"packages/tovuk/native-release-targets.json\"",
            "\"packages/tovuk-py/src/tovuk/native_release_targets.json\"",
            "return write(generated_path.as_path(), source_bytes)",
            "is stale; run {SYNC_COMMAND}",
        ],
    );
}

/// Contract implementation for `require_target_shape`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_target_shape(target: &NativeTarget) -> CheckResult {
    check_try!(require_target_shape_asset(target));
    check_try!(require_target_shape_libc(target));
    let expected = check_try!(require_target_shape_runner(target.triple.as_str()));
    if target.runner != expected {
        return Err(format!(
            "native target {} must run on {expected}, got {}",
            target.triple, target.runner
        ));
    }
    return Ok(());
}

/// Require platform-appropriate executable names and extensions.
///
/// # Errors
///
/// Returns an error when the executable name or extension is invalid.
fn require_target_shape_asset(target: &NativeTarget) -> CheckResult {
    match target.triple.contains("windows") {
        true if target.asset_ext != ".exe" || target.binary != "tovuk.exe" => {
            return Err(format!(
                "Windows native target {} must publish tovuk.exe with .exe asset extension",
                target.triple
            ));
        }
        false if !target.asset_ext.is_empty() || target.binary != "tovuk" => {
            return Err(format!(
                "non-Windows native target {} must publish tovuk without an asset extension",
                target.triple
            ));
        }
        _valid => return Ok(()),
    }
}

/// Require an explicit libc contract only for GNU Linux assets.
///
/// # Errors
///
/// Returns an error when a target has an invalid libc declaration.
fn require_target_shape_libc(target: &NativeTarget) -> CheckResult {
    match target.triple.contains("unknown-linux-gnu") {
        true if target.libc.as_deref() != Some("glibc") => {
            return Err(format!(
                "GNU Linux native target {} must explicitly require glibc",
                target.triple
            ));
        }
        false if target.libc.is_some() => {
            return Err(format!(
                "non-Linux native target {} must not declare a libc",
                target.triple
            ));
        }
        _valid => return Ok(()),
    }
}

/// Return the required GitHub-hosted runner for one Rust target.
///
/// # Errors
///
/// Returns an error when the target family is unknown.
fn require_target_shape_runner(triple: &str) -> CheckResult<&'static str> {
    let runner = if triple.contains("unknown-linux") {
        "ubuntu-24.04"
    } else if triple == "x86_64-apple-darwin" {
        "macos-15-intel"
    } else if triple.contains("apple-darwin") {
        "macos-15"
    } else if triple.contains("windows-msvc") {
        "windows-2025"
    } else {
        return Err(format!("unknown native target family {triple}"));
    };
    return Ok(runner);
}

/// Require GitHub-hosted matrix builds and centralized release uploads.
///
/// # Errors
///
/// Returns an error when the native publishing workflow violates the contract.
pub(super) fn require_workflow_contract() -> CheckResult {
    let source = check_try!(read_text(".github/workflows/publish-native-binaries.yml"));
    check_try!(require_snippets(
        source.as_str(),
        "publish-native-binaries.yml",
        &[
            "- \"native-release-targets.json\"",
            "native-targets:",
            "fromJSON(needs.native-targets.outputs.matrix)",
            "--bin native-release-tool -- matrix native-release-targets.json crates/tovuk/Cargo.toml",
            "matrix.asset_name",
            "--bin native-release-tool -- tag crates/tovuk/Cargo.toml",
            "runs-on: ${{ matrix.runner }}",
            "needs: [native-targets, release-gate, build]",
            "actions/upload-artifact@",
            "merge-multiple: true",
            "--bin native-release-tool -- prepare-release native-artifact native-release-targets.json crates/tovuk/Cargo.toml",
            "--bin native-release-tool -- verify-sha256",
            "--bin check-native-release-assets -- \"${RELEASE_TAG#v}\" 0 --allow-draft",
            "upload+=(\"native-artifact/$asset_name.sha256\")",
            "--draft=false --latest",
            "gh_version=\"2.96.0\"",
            "gh_${gh_version}_linux_amd64.tar.gz",
            "--repo \"$GITHUB_REPOSITORY\"",
            "cmake_version=\"4.3.3\"",
            "ninja_version=\"1.13.2\"",
            "zig_version=\"0.16.0\"",
            "zig-x86_64-linux-$zig_version.tar.xz",
            "cargo install --locked --version 0.23.0 cargo-zigbuild",
            "cargo zigbuild --locked --release",
        ],
    ));
    check_try!(reject_legacy_workflow_contract(source.as_str()));
    return require_release_tool_contract();
}
