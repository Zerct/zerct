//! Multi-era sanitized history rewrite regressions.

use std::fs::write;

use crate::helpers::CheckResult;

use super::{
    PUSH_LOCATION, PushFixture, check_input_in, commit_fixture, copy_public_tree, create_fixture,
    git_text, record, remove_fixture, require_rejected, run_git,
};

use super::ci_tests::construct_environment;

use super::super::continuous_integration::check_environment_in;

/// Legacy root content selected by one rewrite fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegacyContents {
    /// Canonical harmless public prose.
    Safe,
    /// Reconstructed provider credential signature.
    Secret,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&build_rewritten_history),
    size_of_val(&commit_legacy_root),
];

/// Create an unrelated legacy root followed by one current manifest-bound tip.
///
/// # Errors
///
/// Returns an error when branch, tree, commit, or ref construction fails.
fn build_rewritten_history(
    fixture: &PushFixture,
    legacy_contents: LegacyContents,
) -> CheckResult<String> {
    check_try!(commit_legacy_root(fixture, legacy_contents));
    check_try!(copy_public_tree(fixture.repository.as_path()));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["add", "--all", "--"]
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "current public snapshot",
    ));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["update-ref", "refs/heads/main", target.as_str()]
    ));
    return Ok(target);
}

/// Create one pre-manifest root commit for a rewrite fixture.
///
/// # Errors
///
/// Returns an error when the orphan branch or legacy root cannot be committed.
fn commit_legacy_root(fixture: &PushFixture, contents: LegacyContents) -> CheckResult {
    check_try!(run_git(
        fixture.repository.as_path(),
        &["switch", "--quiet", "--orphan", "sanitized-history"]
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["rm", "-r", "--quiet", "--ignore-unmatch", "--", "."]
    ));
    let legacy = match contents {
        LegacyContents::Safe => "Sanitized public legacy snapshot.\n".to_owned(),
        LegacyContents::Secret => {
            format!("{}{}\n", ["gh", "p_"].concat(), "aB3".repeat(0x000c))
        }
    };
    check_try!(
        write(fixture.repository.join("README.md"), legacy)
            .map_err(|error| return format!("write legacy rewrite fixture: {error}"))
    );
    check_try!(run_git(
        fixture.repository.as_path(),
        &["add", "--", "README.md"]
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "sanitized legacy root",
    ));
    return Ok(());
}

/// Verify trusted push CI accepts a fully scanned sanitized force rewrite.
///
/// # Errors
///
/// Returns an error when publication, event construction, or scanning fails.
#[test]
fn verify_ci_snapshot_accepts_sanitized_multi_era_rewrite() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-sanitized-rewrite"));
    let target = check_try!(build_rewritten_history(&fixture, LegacyContents::Safe));
    let refspec = format!("{target}:refs/heads/main");
    check_try!(run_git(
        fixture.repository.as_path(),
        &["push", "--force", "--no-verify", "origin", refspec.as_str(),]
    ));
    let environment = check_try!(construct_environment(&[
        "push",
        target.as_str(),
        target.as_str(),
        "refs/heads/main",
        "branch",
        target.as_str(),
        fixture.baseline.as_str(),
        "false",
        "false",
        "true",
        "",
        "",
        "",
        "",
    ]));
    check_try!(check_environment_in(
        fixture.repository.as_path(),
        &environment,
    ));
    return remove_fixture(&fixture);
}

/// Verify a complete sanitized rewrite may retain pre-manifest public commits.
///
/// # Errors
///
/// Returns an error when setup or strict rewrite scanning fails.
#[test]
fn verify_pre_push_accepts_sanitized_multi_era_rewrite() -> CheckResult {
    let fixture = check_try!(create_fixture("sanitized-rewrite"));
    let target = check_try!(build_rewritten_history(&fixture, LegacyContents::Safe));
    let input = record(
        "refs/heads/main",
        target.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    check_try!(check_input_in(
        fixture.repository.as_path(),
        PUSH_LOCATION,
        input.as_str(),
    ));
    return remove_fixture(&fixture);
}

/// Verify generic legacy policy still rejects removed secret history.
///
/// # Errors
///
/// Returns an error when setup fails or secret legacy bytes are accepted.
#[test]
fn verify_pre_push_rejects_secret_multi_era_rewrite() -> CheckResult {
    let fixture = check_try!(create_fixture("secret-rewrite"));
    let target = check_try!(build_rewritten_history(&fixture, LegacyContents::Secret));
    let input = record(
        "refs/heads/main",
        target.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str(),),
        "secret-bearing sanitized rewrite",
    ));
    return remove_fixture(&fixture);
}
