use serde::Deserialize;

use crate::helpers::{CheckResult, read_json, read_text};

#[derive(Debug, Deserialize)]
struct NativeReleaseTargets {
    targets: Vec<NativeTarget>,
}

#[derive(Debug, Deserialize)]
struct NativeTarget {
    asset_ext: String,
    binary: String,
    #[serde(default)]
    libc: Option<String>,
    node: NodeTarget,
    python: Vec<PythonTarget>,
    runner: String,
    triple: String,
}

#[derive(Debug, Deserialize)]
struct NodeTarget {
    arch: String,
    platform: String,
}

#[derive(Debug, Deserialize)]
struct PythonTarget {
    machine: String,
    system: String,
}

pub(crate) fn check() -> CheckResult {
    let manifest = read_manifest()?;
    require_manifest_shape(&manifest)?;
    require_sync_script_contract()?;
    require_workflow_contract(&manifest)?;
    require_release_gate_contract()?;
    require_npm_installer_contract()?;
    require_python_installer_contract()?;
    println!("Checked native release target contracts.");
    Ok(())
}

fn read_manifest() -> CheckResult<NativeReleaseTargets> {
    read_json("native-release-targets.json")
}

fn require_manifest_shape(manifest: &NativeReleaseTargets) -> CheckResult {
    let mut triples = std::collections::BTreeSet::new();
    let mut node_aliases = std::collections::BTreeSet::new();
    let mut python_aliases = std::collections::BTreeSet::new();
    for target in &manifest.targets {
        if !triples.insert(target.triple.clone()) {
            return Err(format!("duplicate native release target {}", target.triple));
        }
        let node_alias = format!("{}/{}", target.node.platform, target.node.arch);
        if !node_aliases.insert(node_alias.clone()) {
            return Err(format!("duplicate npm native target alias {node_alias}"));
        }
        for alias in &target.python {
            let python_alias = format!("{}/{}", alias.system, alias.machine);
            if !python_aliases.insert(python_alias.clone()) {
                return Err(format!("duplicate PyPI native target alias {python_alias}"));
            }
        }
        require_target_shape(target)?;
    }
    Ok(())
}

fn require_target_shape(target: &NativeTarget) -> CheckResult {
    if target.triple.contains("windows") {
        if target.asset_ext != ".exe" || target.binary != "tovuk.exe" {
            return Err(format!(
                "Windows native target {} must publish tovuk.exe with .exe asset extension",
                target.triple
            ));
        }
    } else if !target.asset_ext.is_empty() || target.binary != "tovuk" {
        return Err(format!(
            "non-Windows native target {} must publish tovuk without an asset extension",
            target.triple
        ));
    }

    if target.triple.contains("unknown-linux-gnu") {
        if target.libc.as_deref() != Some("glibc") {
            return Err(format!(
                "GNU Linux native target {} must explicitly require glibc",
                target.triple
            ));
        }
    } else if target.libc.is_some() {
        return Err(format!(
            "non-Linux native target {} must not declare a libc",
            target.triple
        ));
    }

    let expected_runner = if target.triple.contains("unknown-linux") {
        "ubuntu-24.04"
    } else if target.triple.contains("apple-darwin") {
        "macos-15"
    } else if target.triple.contains("windows-msvc") {
        "windows-2025"
    } else {
        return Err(format!("unknown native target family {}", target.triple));
    };
    if target.runner != expected_runner {
        return Err(format!(
            "native target {} must run on {expected_runner}, got {}",
            target.triple, target.runner
        ));
    }
    Ok(())
}

fn require_workflow_contract(manifest: &NativeReleaseTargets) -> CheckResult {
    let source = read_text(".github/workflows/publish-native-binaries.yml")?;
    for snippet in [
        "- \"native-release-targets.json\"",
        "native-targets:",
        "fromJSON(needs.native-targets.outputs.matrix)",
        "\"asset_ext\": target[\"asset_ext\"]",
        "\"binary\": target[\"binary\"]",
        "\"runner\": target[\"runner\"]",
        "\"target\": target[\"triple\"]",
        "matrix.asset_ext",
    ] {
        if !source.contains(snippet) {
            return Err(format!("publish-native-binaries.yml missing {snippet}"));
        }
    }
    if manifest
        .targets
        .iter()
        .any(|target| target.triple == "aarch64-unknown-linux-gnu")
        && !source.contains("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER")
    {
        return Err("publish-native-binaries.yml must configure the Linux ARM64 linker".to_owned());
    }
    Ok(())
}

fn require_release_gate_contract() -> CheckResult {
    let source = read_text("scripts/check-native-release-assets.sh")?;
    for snippet in [
        "native-release-targets.json",
        "target['asset_ext']",
        "verify_asset_checksums",
        "gh release download",
        "hashlib.sha256",
        "checksum mismatch",
    ] {
        if !source.contains(snippet) {
            return Err(format!("check-native-release-assets.sh missing {snippet}"));
        }
    }
    if source.contains("endswith(\".exe\")") {
        return Err("native release asset names must use explicit asset_ext metadata".to_owned());
    }
    Ok(())
}

fn require_npm_installer_contract() -> CheckResult {
    let source = read_text("packages/tovuk/install.mjs")?;
    require_generated_manifest_matches_root("packages/tovuk/native-release-targets.json")?;
    require_contains_json_file("packages/tovuk/package.json", "native-release-targets.json")?;
    for snippet in [
        "native-release-targets.json",
        "target.asset_ext",
        "target.triple",
        "requires glibc Linux",
        "linuxLibc()",
        "nativeBinaryName()",
        "TOVUK_NATIVE_BINARY",
    ] {
        if !source.contains(snippet) {
            return Err(format!("install.mjs missing {snippet}"));
        }
    }
    let override_index = source
        .find("if (process.env.TOVUK_NATIVE_BINARY)")
        .ok_or_else(|| "install.mjs must honor TOVUK_NATIVE_BINARY".to_owned())?;
    let target_index = source
        .find("const target = nativeTarget()")
        .ok_or_else(|| "install.mjs must resolve nativeTarget inside release install".to_owned())?;
    if target_index < override_index {
        return Err(
            "install.mjs must not resolve the release target before local binary overrides"
                .to_owned(),
        );
    }
    Ok(())
}

fn require_python_installer_contract() -> CheckResult {
    let source = read_text("packages/tovuk-py/src/tovuk/cli.py")?;
    require_generated_manifest_matches_root(
        "packages/tovuk-py/src/tovuk/native_release_targets.json",
    )?;
    for snippet in [
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
    ] {
        if !source.contains(snippet) {
            return Err(format!("cli.py missing {snippet}"));
        }
    }
    if source.contains("/ target_triple / \"tovuk\"")
        || source.contains("/ target_triple / 'tovuk'")
        || source.contains("with_name(\"bin\") / \"tovuk\"")
        || source.contains("with_name(\"bin\") / 'tovuk'")
    {
        return Err(
            "cli.py must use manifest binary names instead of hard-coded tovuk paths".to_owned(),
        );
    }
    Ok(())
}

fn require_sync_script_contract() -> CheckResult {
    let source = read_text("scripts/sync-native-release-targets.sh")?;
    for snippet in [
        "source_manifest=\"native-release-targets.json\"",
        "generated_manifests=(",
        "\"packages/tovuk/native-release-targets.json\"",
        "\"packages/tovuk-py/src/tovuk/native_release_targets.json\"",
        "cmp -s \"$source_manifest\" \"$generated_manifest\"",
        "cp \"$source_manifest\" \"$generated_manifest\"",
    ] {
        if !source.contains(snippet) {
            return Err(format!("sync-native-release-targets.sh missing {snippet}"));
        }
    }
    Ok(())
}

fn require_generated_manifest_matches_root(path: &str) -> CheckResult {
    let root = read_text("native-release-targets.json")?;
    let packaged = read_text(path)?;
    if packaged != root {
        return Err(format!(
            "{path} must match native-release-targets.json exactly"
        ));
    }
    Ok(())
}

fn require_contains_json_file(path: &str, file_name: &str) -> CheckResult {
    let source = read_text(path)?;
    if source.contains(file_name) {
        return Ok(());
    }
    Err(format!(
        "{path} must include {file_name} in published files"
    ))
}
