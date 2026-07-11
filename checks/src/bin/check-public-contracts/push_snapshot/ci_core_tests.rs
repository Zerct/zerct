//! Pull-request immutable-core and narrow-path regressions.

use std::{
    fs::{create_dir_all, read, write},
    path::Path,
};

use crate::helpers::CheckResult;

use super::{
    PushFixture, commit_fixture, create_fixture, git_text, remove_fixture, run_git,
    synchronize_fixture_public_tree,
};

use super::ci_tests::{build_pull_merge, ensure_ci_rejected, safe_pull_environment};

use super::super::continuous_integration::check_environment_in;

/// Whether Git must override ignore policy for one intentional bad path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StageMode {
    /// Stage through normal tracked-path rules.
    Normal,
    /// Force-stage an intentionally ignored sensitive path.
    Sensitive,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&add_path),
    size_of_val(&reject_added_path),
    size_of_val(&reject_core_modification),
    size_of_val(&scan_pull),
];

/// Add and commit one candidate pull-request path.
///
/// # Errors
///
/// Returns an error when the path, index, or commit cannot be created.
fn add_path(fixture: &PushFixture, relative: &str, contents: &str, mode: StageMode) -> CheckResult {
    let path = fixture.repository.join(relative);
    if let Some(parent) = path
        .parent()
        .filter(|parent| return *parent != Path::new(""))
    {
        check_try!(create_dir_all(parent).map_err(|error| format!("create test path: {error}")));
    }
    check_try!(write(path, contents).map_err(|error| format!("write test path: {error}")));
    let arguments = match mode {
        StageMode::Normal => vec!["add", "--", relative],
        StageMode::Sensitive => vec!["add", "--force", "--", relative],
    };
    check_try!(run_git(fixture.repository.as_path(), arguments.as_slice()));
    check_try!(synchronize_fixture_public_tree(
        fixture.repository.as_path()
    ));
    return commit_fixture(fixture.repository.as_path(), "adversarial pull path");
}

/// Add one path and require the pull history scanner to reject it.
///
/// # Errors
///
/// Returns an error when fixture setup fails or the candidate is accepted.
fn reject_added_path(
    fixture_label: &str,
    relative: &str,
    contents: &str,
    mode: StageMode,
) -> CheckResult {
    let fixture = check_try!(create_fixture(fixture_label));
    check_try!(add_path(&fixture, relative, contents, mode));
    check_try!(ensure_ci_rejected(
        &scan_pull(&fixture),
        format!("adversarial pull path {relative}").as_str(),
    ));
    return remove_fixture(&fixture);
}

/// Modify one immutable core file and require pull history rejection.
///
/// # Errors
///
/// Returns an error when fixture setup fails or the modification is accepted.
fn reject_core_modification(fixture_label: &str, relative: &str) -> CheckResult {
    let fixture = check_try!(create_fixture(fixture_label));
    let mut contents = check_try!(
        read(fixture.repository.join(relative))
            .map_err(|error| format!("read immutable core file: {error}"))
    );
    contents.extend_from_slice(b"\n");
    check_try!(
        write(fixture.repository.join(relative), contents)
            .map_err(|error| format!("write immutable core file: {error}"))
    );
    check_try!(run_git(
        fixture.repository.as_path(),
        &["add", "--", relative]
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "modify immutable core",
    ));
    check_try!(ensure_ci_rejected(
        &scan_pull(&fixture),
        format!("immutable core modification {relative}").as_str(),
    ));
    return remove_fixture(&fixture);
}

/// Build and scan the pull merge for the fixture's current head.
///
/// # Errors
///
/// Returns an error when merge construction or public-surface scanning fails.
fn scan_pull(fixture: &PushFixture) -> CheckResult {
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(
        fixture,
        head.as_str(),
        "core policy merge"
    ));
    let event = check_try!(safe_pull_environment(
        fixture,
        head.as_str(),
        merge.as_str()
    ));
    return check_environment_in(fixture.repository.as_path(), &event);
}

/// Verify an arbitrary new package namespace is rejected.
///
/// # Errors
///
/// Returns an error when setup fails or the package is accepted.
#[test]
fn verify_ci_snapshot_rejects_arbitrary_package() -> CheckResult {
    return reject_added_path(
        "ci-rogue-package",
        "packages/rogue/index.mjs",
        "export const value = true;\n",
        StageMode::Normal,
    );
}

/// Verify crate-local Cargo execution configuration remains immutable.
///
/// # Errors
///
/// Returns an error when setup fails or the configuration change is accepted.
#[test]
fn verify_ci_snapshot_rejects_core_cargo_config_change() -> CheckResult {
    return reject_core_modification("ci-core-cargo-config", "crates/tovuk/.cargo/config.toml");
}

/// Verify deleting a history-enforcement hook is rejected.
///
/// # Errors
///
/// Returns an error when setup fails or core deletion is accepted.
#[test]
fn verify_ci_snapshot_rejects_core_deletion() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-core-deletion"));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["rm", "--quiet", "--", ".githooks/pre-push"]
    ));
    check_try!(synchronize_fixture_public_tree(
        fixture.repository.as_path()
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "delete core hook"
    ));
    check_try!(ensure_ci_rejected(
        &scan_pull(&fixture),
        "immutable core deletion",
    ));
    return remove_fixture(&fixture);
}

/// Verify the dependency-feature fingerprint remains immutable.
///
/// # Errors
///
/// Returns an error when setup fails or the dependency-policy change is accepted.
#[test]
fn verify_ci_snapshot_rejects_core_dependency_policy_change() -> CheckResult {
    return reject_core_modification(
        "ci-core-dependency-policy",
        "dependency-feature-policy.json",
    );
}

/// Verify the public crate manifest remains immutable at this boundary.
///
/// # Errors
///
/// Returns an error when setup fails or the manifest change is accepted.
#[test]
fn verify_ci_snapshot_rejects_core_manifest_change() -> CheckResult {
    return reject_core_modification("ci-core-manifest", "crates/tovuk/Cargo.toml");
}

/// Verify modifying the pinned audit workflow is rejected.
///
/// # Errors
///
/// Returns an error when setup fails or core modification is accepted.
#[test]
fn verify_ci_snapshot_rejects_core_modification() -> CheckResult {
    return reject_core_modification(
        "ci-core-modification",
        ".github/workflows/trusted-history.yml",
    );
}

/// Verify renaming the pinned audit workflow is rejected.
///
/// # Errors
///
/// Returns an error when setup fails or core rename is accepted.
#[test]
fn verify_ci_snapshot_rejects_core_rename() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-core-rename"));
    check_try!(run_git(
        fixture.repository.as_path(),
        &[
            "mv",
            ".github/workflows/trusted-history.yml",
            "docs/moved-history.md",
        ],
    ));
    check_try!(synchronize_fixture_public_tree(
        fixture.repository.as_path()
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "rename core workflow"
    ));
    check_try!(ensure_ci_rejected(
        &scan_pull(&fixture),
        "immutable core rename",
    ));
    return remove_fixture(&fixture);
}

/// Verify new scripts cannot hide in the documentation namespace.
///
/// # Errors
///
/// Returns an error when setup fails or the script is accepted.
#[test]
fn verify_ci_snapshot_rejects_docs_script() -> CheckResult {
    return reject_added_path(
        "ci-docs-script",
        "docs/install.sh",
        "exit 0\n",
        StageMode::Normal,
    );
}

/// Verify private engine source cannot enter the public crate namespace.
///
/// # Errors
///
/// Returns an error when setup fails or the engine path is accepted.
#[test]
fn verify_ci_snapshot_rejects_engine_path() -> CheckResult {
    return reject_added_path(
        "ci-engine-path",
        "crates/engine/src/lib.rs",
        "pub fn private_engine() {}\n",
        StageMode::Normal,
    );
}

/// Verify arbitrary formula additions are rejected.
///
/// # Errors
///
/// Returns an error when setup fails or the formula is accepted.
#[test]
fn verify_ci_snapshot_rejects_formula_addition() -> CheckResult {
    return reject_added_path(
        "ci-formula-addition",
        "Formula/rogue.rb",
        "class Rogue; end\n",
        StageMode::Normal,
    );
}

/// Verify Go cannot be added inside the approved Rust crate namespace.
///
/// # Errors
///
/// Returns an error when setup fails or Go source is accepted.
#[test]
fn verify_ci_snapshot_rejects_go_source() -> CheckResult {
    return reject_added_path(
        "ci-go-source",
        "crates/tovuk/src/rogue.go",
        "package public\n",
        StageMode::Normal,
    );
}

/// Verify a new workflow cannot expand the privileged automation surface.
///
/// # Errors
///
/// Returns an error when setup fails or the workflow is accepted.
#[test]
fn verify_ci_snapshot_rejects_rogue_workflow() -> CheckResult {
    return reject_added_path(
        "ci-rogue-workflow",
        ".github/workflows/rogue.yml",
        "on: push\njobs: {}\n",
        StageMode::Normal,
    );
}

/// Verify uppercase sensitive environment files are rejected.
///
/// # Errors
///
/// Returns an error when setup fails or the sensitive path is accepted.
#[test]
fn verify_ci_snapshot_rejects_uppercase_env() -> CheckResult {
    return reject_added_path(
        "ci-uppercase-env",
        "docs/.ENV",
        "PUBLIC_PLACEHOLDER=true\n",
        StageMode::Sensitive,
    );
}

/// Verify uppercase npm credential files are rejected.
///
/// # Errors
///
/// Returns an error when setup fails or the sensitive path is accepted.
#[test]
fn verify_ci_snapshot_rejects_uppercase_npmrc() -> CheckResult {
    return reject_added_path(
        "ci-uppercase-npmrc",
        "docs/.NPMRC",
        "registry=https://registry.npmjs.org/\n",
        StageMode::Sensitive,
    );
}

/// Verify guarded source additions cannot exceed the 500-line ceiling.
///
/// # Errors
///
/// Returns an error when setup fails or the oversized source is accepted.
#[test]
fn verify_ci_snapshot_rejects_z_oversized_source() -> CheckResult {
    let contents = "public line\n".repeat(0x01f5);
    return reject_added_path(
        "ci-oversized-source",
        "docs/oversized.MDX",
        contents.as_str(),
        StageMode::Normal,
    );
}
