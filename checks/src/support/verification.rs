//! Tests for shared check support.

use std::{
    env::join_paths,
    ffi::OsString,
    fs::{create_dir_all, metadata as filesystem_metadata, remove_dir_all, write},
    path::{Path, PathBuf},
    process,
};

use super::{
    CHECKS_MANIFEST, CheckResult, command, display_path, find_command, git_tracked_files,
    reject_secret_signatures, repo_root, run_status, tool_path,
};

/// Paths created for the command-precedence regression test.
#[derive(Debug)]
struct CandidateFixture {
    /// Expected fallback candidate path.
    fallback: PathBuf,
    /// Synthetic PATH containing the candidates.
    path: OsString,
    /// Expected preferred candidate path.
    preferred: PathBuf,
    /// Fixture root removed after the test.
    root: PathBuf,
}

/// Create two command candidates whose name and directory priorities conflict.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created.
fn candidate_fixture(label: &str) -> CheckResult<CandidateFixture> {
    let root = PathBuf::from("target")
        .join("support-tests")
        .join(format!("candidate-order-{}-{label}", process::id()));
    if check_try!(
        root.try_exists()
            .map_err(|error| return format!("inspect {}: {error}", root.display()))
    ) {
        check_try!(
            remove_dir_all(root.as_path())
                .map_err(|error| return format!("clear {}: {error}", root.display()))
        );
    }
    let fallback_directory = root.join("fallback-first");
    let preferred_directory = root.join("preferred-later");
    let fallback = check_try!(write_candidate(fallback_directory.as_path(), "fallback"));
    let preferred = check_try!(write_candidate(preferred_directory.as_path(), "preferred"));
    let path = check_try!(
        join_paths([fallback_directory, preferred_directory])
            .map_err(|error| return format!("join fixture PATH: {error}"))
    );
    let fixture = CandidateFixture {
        fallback,
        path,
        preferred,
        root,
    };
    return Ok(fixture);
}

/// Verify preferred command names win even when their directory is later in PATH.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created or candidate ordering is wrong.
#[test]
fn command_discovery_prioritizes_candidate_order() -> CheckResult {
    let fixture = check_try!(candidate_fixture("preferred"));
    let discovery = find_command(fixture.path.as_os_str(), &["preferred", "fallback"]);
    let cleanup = remove_dir_all(fixture.root.as_path());
    check_try!(cleanup.map_err(|error| return format!("clear fixture: {error}")));
    let discovered = check_try!(discovery);
    if discovered != fixture.preferred {
        return Err(format!(
            "found {}, expected preferred candidate {}",
            discovered.display(),
            fixture.preferred.display()
        ));
    }
    return Ok(());
}

/// Verify command discovery falls back only after preferred names are exhausted.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created or fallback discovery is wrong.
#[test]
fn command_discovery_uses_fallback_after_preferred_names() -> CheckResult {
    let fixture = check_try!(candidate_fixture("fallback"));
    let discovery = find_command(fixture.path.as_os_str(), &["missing", "fallback"]);
    let cleanup = remove_dir_all(fixture.root.as_path());
    check_try!(cleanup.map_err(|error| return format!("clear fixture: {error}")));
    let discovered = check_try!(discovery);
    if discovered != fixture.fallback {
        return Err(format!(
            "found {}, expected fallback candidate {}",
            discovered.display(),
            fixture.fallback.display()
        ));
    }
    return Ok(());
}

/// Construct a source-safe synthetic token from a split provider prefix.
fn prefixed_token(prefix: [&str; 0x0002], body: &str) -> String {
    return format!("{}{body}", prefix.concat());
}

/// Verify repository discovery and tracked-file rendering.
///
/// # Errors
///
/// Returns an error when the helpers cannot inspect the current repository.
#[test]
fn repository_helpers_find_manifest() -> CheckResult {
    let repository = check_try!(repo_root());
    let path = tool_path();
    let git = check_try!(find_command(path.as_os_str(), &["git"]));
    let prepared_command = command(&repository, path.as_os_str(), "git");
    if prepared_command.get_current_dir() != Some(repository.as_path()) {
        return Err("shared command helper did not set its working directory".to_owned());
    }
    check_try!(run_status(
        &repository,
        path.as_os_str(),
        "git",
        &["--version"]
    ));
    let tracked_files = check_try!(git_tracked_files(&repository));
    if !tracked_files
        .iter()
        .any(|file| return file == CHECKS_MANIFEST)
    {
        return Err(format!("Git does not track {CHECKS_MANIFEST}"));
    }
    if display_path(Path::new(CHECKS_MANIFEST)) != CHECKS_MANIFEST {
        return Err("display_path changed a repository-relative path".to_owned());
    }
    if filesystem_metadata(git).is_err() {
        return Err("find_command returned an unreadable path".to_owned());
    }
    return Ok(());
}

/// Verify the same helpers remain stable across a second invocation.
///
/// # Errors
///
/// Returns an error when repeated helper use changes the observed repository.
#[test]
fn repository_helpers_verify_repeatability() -> CheckResult {
    let first_repository = check_try!(repo_root());
    let path = tool_path();
    let git = check_try!(find_command(path.as_os_str(), &["git"]));
    let prepared_command = command(&first_repository, path.as_os_str(), "git");
    if prepared_command.get_program() != "git" {
        return Err("shared command helper changed the program".to_owned());
    }
    check_try!(run_status(
        &first_repository,
        path.as_os_str(),
        "git",
        &["rev-parse", "--is-inside-work-tree"]
    ));
    let tracked_files = check_try!(git_tracked_files(&first_repository));
    if tracked_files.is_empty() {
        return Err("Git returned no tracked files".to_owned());
    }
    if display_path(Path::new(CHECKS_MANIFEST)).is_empty() {
        return Err("display_path returned an empty path".to_owned());
    }
    if filesystem_metadata(git).is_err() {
        return Err("find_command returned an unreadable path".to_owned());
    }
    return Ok(());
}

/// Require one non-credential fixture to remain accepted.
///
/// # Errors
///
/// Returns an error when any common context is misclassified as a credential.
fn require_credential_allowed(label: &str, text: &str) -> CheckResult {
    let contexts = [
        text.to_owned(),
        format!("TOKEN: {text}"),
        format!("{{\"token\":\"{text}\"}}"),
        format!("prefix_{text}"),
    ];
    for context in contexts {
        check_try!(reject_secret_signatures(label, context.as_bytes()));
    }
    return Ok(());
}

/// Require one synthetic credential to be rejected in bare, YAML, and JSON contexts.
///
/// # Errors
///
/// Returns an error when any common context fails to reject the credential.
fn require_credential_rejected(label: &str, credential: &str) -> CheckResult {
    let contexts = [
        credential.to_owned(),
        format!("TOKEN: {credential}"),
        format!("{{\"token\":\"{credential}\"}}"),
        format!("prefix_{credential}"),
        format!("prefix{credential}"),
        format!("{credential}_suffix"),
        format!("{credential}suffix"),
    ];
    for context in contexts {
        if reject_secret_signatures(label, context.as_bytes()).is_ok() {
            return Err(format!("shared secret scanner accepted {label}"));
        }
    }
    return Ok(());
}

/// Verify documentation examples and ambiguous hashes are not guessed as credentials.
///
/// # Errors
///
/// Returns an error when an ordinary public string is rejected.
#[test]
fn secret_signature_scanning_accepts_examples_and_hashes() -> CheckResult {
    let examples = [
        "public npm_package documentation without credentials".to_owned(),
        prefixed_token(["gh", "p_"], "examplecredential"),
        prefixed_token(["github_", "pat_"], "example_identifier"),
        prefixed_token(["npm", "_"], "package"),
        prefixed_token(["pypi", "-"], "AgEIexamplecredential"),
        prefixed_token(["rk_", "live_"], "examplecredential"),
        prefixed_token(["sk_", "live_"], "examplecredential"),
        prefixed_token(["xo", "xb-"], "example"),
        prefixed_token(["xo", "xp-"], "example"),
        "a1b2c3d4".repeat(0x0005),
        format!("sha256: {}", "a1b2c3d4".repeat(0x0008)),
    ];
    for example in examples {
        check_try!(require_credential_allowed(
            "ordinary fixture",
            example.as_str()
        ));
    }
    return Ok(());
}

/// Verify invalid token alphabets and incomplete shapes remain accepted.
///
/// # Errors
///
/// Returns an error when a non-credential identifier is rejected.
#[test]
fn secret_signature_scanning_accepts_invalid_shapes() -> CheckResult {
    let fine_short = format!("{}_{}a", "aB".repeat(0x000b), "cD3".repeat(0x0013));
    let fine_wrong_separator = format!("{}-{}aB", "aB".repeat(0x000b), "cD3".repeat(0x0013));
    let examples = [
        prefixed_token(["gh", "r_"], "aB3_".repeat(0x0012).as_str()),
        prefixed_token(["gh", "s_"], "aB3".repeat(0x000b).as_str()),
        prefixed_token(["gh", "p_"], "aB3_".repeat(0x0009).as_str()),
        prefixed_token(
            ["gh", "p_"],
            format!("{}{}{}", "aB".repeat(0x0009), "/", "cD".repeat(0x0009)).as_str(),
        ),
        prefixed_token(["github_", "pat_"], fine_short.as_str()),
        prefixed_token(["github_", "pat_"], fine_wrong_separator.as_str()),
        prefixed_token(
            ["npm", "_"],
            format!("{}{}{}", "aB".repeat(0x0009), "-", "cD".repeat(0x0009)).as_str(),
        ),
        prefixed_token(["pypi", "-"], "Ab-_".repeat(0x0015).as_str()),
        prefixed_token(
            ["pypi", "-"],
            format!("{}{}{}", "aB".repeat(0x0015), "/", "cD".repeat(0x0016)).as_str(),
        ),
        prefixed_token(
            ["rk_", "live_"],
            format!("{}{}{}", "aB".repeat(0x0006), "-", "cD".repeat(0x0006)).as_str(),
        ),
        prefixed_token(
            ["sk_", "live_"],
            format!("{}{}{}", "aB".repeat(0x0006), "-", "cD".repeat(0x0006)).as_str(),
        ),
        prefixed_token(["xo", "xb-"], "12345678-12345678-short"),
        prefixed_token(["xo", "xb-"], "12345678901_19874698323_secretvalue"),
        prefixed_token(["xo", "xp-"], "12345678-12345678-short"),
    ];
    for example in examples {
        check_try!(require_credential_allowed(
            "identifier fixture",
            example.as_str()
        ));
    }
    return Ok(());
}

/// Verify every current `GitHub` prefixed token family is recognized precisely.
///
/// # Errors
///
/// Returns an error when a synthetic `GitHub` credential is accepted.
#[test]
fn secret_signature_scanning_rejects_github_credentials() -> CheckResult {
    let classic_body = "aB3".repeat(0x000c);
    for prefix in [["gh", "o_"], ["gh", "p_"], ["gh", "s_"], ["gh", "u_"]] {
        let token = prefixed_token(prefix, classic_body.as_str());
        check_try!(require_credential_rejected(
            "GitHub fixture",
            token.as_str()
        ));
    }
    let refresh_body = format!("{}a", "aB3".repeat(0x0019));
    let refresh = prefixed_token(["gh", "r_"], refresh_body.as_str());
    check_try!(require_credential_rejected(
        "GitHub fixture",
        refresh.as_str()
    ));
    let fine_grained_body = format!("{}_{}aB", "aB".repeat(0x000b), "cD3".repeat(0x0013));
    let fine_grained = prefixed_token(["github_", "pat_"], fine_grained_body.as_str());
    check_try!(require_credential_rejected(
        "GitHub fixture",
        fine_grained.as_str()
    ));
    let jwt_body = format!(
        "123456_{}-{}.{}.{}",
        "aB3".repeat(0x0006),
        "zZ9",
        "cD4",
        "eF5"
    );
    let stateless = prefixed_token(["gh", "s_"], jwt_body.as_str());
    check_try!(require_credential_rejected(
        "GitHub stateless fixture",
        stateless.as_str()
    ));
    return Ok(());
}

/// Verify ASCII credentials are rejected even when surrounding bytes are invalid UTF-8.
///
/// # Errors
///
/// Returns an error when mixed binary input suppresses an embedded credential.
#[test]
fn secret_signature_scanning_rejects_mixed_invalid_utf8() -> CheckResult {
    let token = prefixed_token(["npm", "_"], "aB3".repeat(0x000c).as_str());
    let mut mixed_bytes = vec![0xff];
    mixed_bytes.extend_from_slice(b"binary-prefix:");
    mixed_bytes.extend_from_slice(token.as_bytes());
    mixed_bytes.push(0xfe);
    if reject_secret_signatures("mixed binary fixture", mixed_bytes.as_slice()).is_ok() {
        return Err("shared secret scanner accepted a credential in invalid UTF-8".to_owned());
    }
    return Ok(());
}

/// Verify all standard private-key headers, including PGP, are recognized.
///
/// # Errors
///
/// Returns an error when a synthetic private-key header is accepted.
#[test]
fn secret_signature_scanning_rejects_private_key_headers() -> CheckResult {
    let headers = [
        ["-----BEGIN DSA PRIVATE ", "KEY-----"].concat(),
        ["-----BEGIN EC PRIVATE ", "KEY-----"].concat(),
        ["-----BEGIN ENCRYPTED PRIVATE ", "KEY-----"].concat(),
        ["-----BEGIN OPENSSH PRIVATE ", "KEY-----"].concat(),
        ["-----BEGIN PGP PRIVATE ", "KEY BLOCK-----"].concat(),
        ["-----BEGIN PRIVATE ", "KEY-----"].concat(),
        ["-----BEGIN RSA PRIVATE ", "KEY-----"].concat(),
    ];
    for header in headers {
        check_try!(require_credential_rejected(
            "private-key fixture",
            header.as_str()
        ));
    }
    return Ok(());
}

/// Verify registry, Slack, and Stripe credentials require complete provider-specific shapes.
///
/// # Errors
///
/// Returns an error when a synthetic provider credential is accepted.
#[test]
fn secret_signature_scanning_rejects_registry_slack_and_stripe_credentials() -> CheckResult {
    let npm = prefixed_token(["npm", "_"], "aB3".repeat(0x000c).as_str());
    check_try!(require_credential_rejected("npm fixture", npm.as_str()));
    let pypi = prefixed_token(["pypi", "-"], "Ab0-_".repeat(0x0011).as_str());
    check_try!(require_credential_rejected("PyPI fixture", pypi.as_str()));
    let slack_body = format!("12345678901-19874698323-{}", "aB3".repeat(0x0008));
    for prefix in [["xo", "xb-"], ["xo", "xp-"]] {
        let slack = prefixed_token(prefix, slack_body.as_str());
        check_try!(require_credential_rejected("Slack fixture", slack.as_str()));
    }
    let legacy_slack_body = format!("123456789012-{}", "aB3".repeat(0x0005));
    let legacy_slack = prefixed_token(["xo", "xb-"], legacy_slack_body.as_str());
    check_try!(require_credential_rejected(
        "legacy Slack fixture",
        legacy_slack.as_str()
    ));
    for prefix in [["rk_", "live_"], ["sk_", "live_"]] {
        let stripe = prefixed_token(prefix, "aB3".repeat(0x0008).as_str());
        check_try!(require_credential_rejected(
            "Stripe key fixture",
            stripe.as_str()
        ));
    }
    return Ok(());
}

/// Create one regular command-candidate fixture file.
///
/// # Errors
///
/// Returns an error when the directory or file cannot be created.
fn write_candidate(directory: &Path, name: &str) -> CheckResult<PathBuf> {
    check_try!(
        create_dir_all(directory)
            .map_err(|error| return format!("create {}: {error}", directory.display()))
    );
    let candidate = directory.join(name);
    check_try!(
        write(candidate.as_path(), [])
            .map_err(|error| return format!("write {}: {error}", candidate.display()))
    );
    return Ok(candidate);
}
