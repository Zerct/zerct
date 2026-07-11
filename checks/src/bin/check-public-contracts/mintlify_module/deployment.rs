use crate::helpers::CheckResult;

use crate::mintlify_fetch::{FetchContext, RequestHeaders, fetch_text_from_base};

use crate::mintlify_fetch::request_cache::validate_revision;

use serde::Deserialize;

use serde_json::from_str;

use std::env;

/// Public `GitHub` API origin used to verify the hosting-provider deployment record.
const GITHUB_API_BASE_URL: &str = "https://api.github.com";

/// Versioned public `GitHub` API headers.
const GITHUB_API_HEADERS: &RequestHeaders<'static> = &[
    ("Accept", "application/vnd.github+json"),
    ("X-GitHub-Api-Version", "2022-11-28"),
];

/// Exact `GitHub` App actor that creates Mintlify deployments and statuses.
const MINTLIFY_BOT_LOGIN: &str = "mintlify[bot]";

/// Exact public hosting environment created by the Mintlify `GitHub` App.
const MINTLIFY_ENVIRONMENT: &str = "staging - docs";

/// Exact public documentation origin recorded on a successful deployment.
const PUBLIC_DOCS_URL: &str = "https://docs.tovuk.com";

/// Compile-time references preserve the deployment verification boundaries.
const _: [usize; 0x000d] = [
    size_of_val(&check_exact_deployment),
    size_of_val(&deployment_has_trusted_provenance),
    size_of_val(&deployment_matches_exact_revision),
    size_of_val(&deployment_path),
    size_of_val(&docs_contents_path),
    size_of_val(&docs_tree_sha),
    size_of_val(&fetch_deployments),
    size_of_val(&fetch_docs_tree_sha),
    size_of_val(&fetch_github),
    size_of_val(&github_authorization),
    size_of_val(&has_successful_status),
    size_of_val(&status_is_successful),
    size_of_val(&status_path),
];

/// Public `GitHub` actor fields required for deployment provenance verification.
#[derive(Deserialize)]
struct Actor {
    /// Stable actor login.
    login: String,
}

/// Public `GitHub` repository-content fields required to compare documentation trees.
#[derive(Deserialize)]
struct ContentEntry {
    /// `GitHub` content kind.
    #[serde(rename = "type")]
    kind: String,
    /// Repository-relative entry path.
    path: String,
    /// Stable Git object identifier for the entry.
    sha: String,
}

/// Public `GitHub` deployment fields required for provenance and content verification.
#[derive(Deserialize)]
struct Deployment {
    /// App actor that created the deployment.
    creator: Actor,
    /// Deployment environment name.
    environment: String,
    /// Public deployment identifier.
    #[serde(rename = "id")]
    identifier: u64,
    /// Branch reference recorded by the deployment.
    #[serde(rename = "ref")]
    reference: String,
    /// Full deployed Git object identifier.
    sha: String,
    /// Deployment task recorded by the hosting provider.
    task: String,
}

/// Public `GitHub` deployment status fields required for readiness verification.
#[derive(Deserialize)]
struct DeploymentStatus {
    /// App actor that created the status.
    creator: Actor,
    /// Public URL recorded for the deployment.
    environment_url: Option<String>,
    /// Deployment conclusion.
    state: String,
}

/// Require a trusted Mintlify deployment of the exact current documentation tree.
///
/// # Errors
///
/// Returns an error when neither the current revision nor its exact docs tree was deployed.
pub(super) fn check_exact_deployment(context: &FetchContext) -> CheckResult {
    let Some(commit_revision) = context.commit_revision() else {
        return Ok(());
    };
    let deployments = check_try!(fetch_deployments(context));
    let mut found_exact_deployment = false;
    for deployment in &deployments {
        if !deployment_matches_exact_revision(deployment, commit_revision) {
            continue;
        }
        found_exact_deployment = true;
        if check_try!(has_successful_status(context, deployment.identifier)) {
            return Ok(());
        }
    }
    if found_exact_deployment {
        return Err(format!(
            "Mintlify has no successful {MINTLIFY_ENVIRONMENT} deployment for revision {commit_revision}"
        ));
    }

    let current_docs_tree = check_try!(fetch_docs_tree_sha(context, commit_revision));
    for deployment in deployments {
        if !deployment_has_trusted_provenance(&deployment) {
            continue;
        }
        let deployed_docs_tree = check_try!(fetch_docs_tree_sha(context, deployment.sha.as_str()));
        if deployed_docs_tree == current_docs_tree
            && check_try!(has_successful_status(context, deployment.identifier))
        {
            return Ok(());
        }
    }
    return Err(format!(
        "Mintlify has no successful {MINTLIFY_ENVIRONMENT} deployment matching the docs tree at revision {commit_revision}"
    ));
}

/// Return whether one deployment has exact Mintlify provenance data.
fn deployment_has_trusted_provenance(deployment: &Deployment) -> bool {
    return deployment.creator.login == MINTLIFY_BOT_LOGIN
        && deployment.environment == MINTLIFY_ENVIRONMENT
        && deployment.reference == "main"
        && deployment.task == "deploy";
}

/// Return whether one trusted deployment is bound to the configured workflow revision.
fn deployment_matches_exact_revision(deployment: &Deployment, revision: &str) -> bool {
    return deployment_has_trusted_provenance(deployment) && deployment.sha == revision;
}

/// Build the trusted-environment deployment query for one full readiness attempt.
fn deployment_path(attempt: i64) -> String {
    return format!(
        "/repos/tovuk/tovuk/deployments?environment=staging%20-%20docs&ref=main&per_page=100&readiness_attempt={attempt}"
    );
}

/// Build the root-content query used to resolve one immutable documentation tree.
fn docs_contents_path(revision: &str, attempt: i64) -> String {
    return format!("/repos/tovuk/tovuk/contents?ref={revision}&readiness_attempt={attempt}");
}

/// Extract and validate the immutable `docs` directory tree identifier.
///
/// # Errors
///
/// Returns an error when the repository root has no canonical `docs` directory entry.
fn docs_tree_sha(entries: Vec<ContentEntry>) -> CheckResult<String> {
    let docs_entry = check_try!(
        entries
            .into_iter()
            .find(|entry| return entry.path == "docs" && entry.kind == "dir")
            .ok_or_else(|| return "repository contents have no docs directory".to_owned())
    );
    check_try!(validate_revision(docs_entry.sha.as_str()));
    return Ok(docs_entry.sha);
}

/// Fetch and parse recent public documentation deployments.
///
/// # Errors
///
/// Returns an error when the public deployment API cannot be fetched or decoded.
fn fetch_deployments(context: &FetchContext) -> CheckResult<Vec<Deployment>> {
    let path = deployment_path(context.readiness_attempt());
    let source = check_try!(fetch_github(context, path.as_str()));
    return from_str(source.as_str())
        .map_err(|error| format!("parse public GitHub deployments: {error}"));
}

/// Resolve the immutable `docs` directory tree for one commit.
///
/// # Errors
///
/// Returns an error when the public contents response is invalid or lacks `docs`.
fn fetch_docs_tree_sha(context: &FetchContext, revision: &str) -> CheckResult<String> {
    check_try!(validate_revision(revision));
    let path = docs_contents_path(revision, context.readiness_attempt());
    let source = check_try!(fetch_github(context, path.as_str()));
    let entries: Vec<ContentEntry> = check_try!(
        from_str(source.as_str())
            .map_err(|error| format!("parse public GitHub repository contents: {error}"))
    );
    return docs_tree_sha(entries);
}

/// Fetch one public `GitHub` API response with optional workflow authentication.
///
/// # Errors
///
/// Returns an error when the token is invalid or the bounded API request fails.
fn fetch_github(context: &FetchContext, path: &str) -> CheckResult<String> {
    let authorization = check_try!(github_authorization());
    let mut headers = GITHUB_API_HEADERS.to_vec();
    if let Some(value) = authorization.as_deref() {
        headers.push(("Authorization", value));
    }
    return fetch_text_from_base(context, GITHUB_API_BASE_URL, path, headers.as_slice());
}

/// Build an optional bearer header from the ephemeral workflow token.
///
/// # Errors
///
/// Returns an error when the configured token is empty, non-ASCII, or not valid UTF-8.
fn github_authorization() -> CheckResult<Option<String>> {
    let token = match env::var("GITHUB_TOKEN") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(format!("read GITHUB_TOKEN: {error}")),
    };
    let valid = !token.is_empty()
        && token.len() <= 0x1000
        && token.bytes().all(|byte| return byte.is_ascii_graphic());
    if !valid {
        return Err("GITHUB_TOKEN is not a valid bearer credential".to_owned());
    }
    return Ok(Some(format!("Bearer {token}")));
}

/// Return whether one exact deployment has a successful public docs status.
///
/// # Errors
///
/// Returns an error when the public deployment status API cannot be fetched or decoded.
fn has_successful_status(context: &FetchContext, deployment_id: u64) -> CheckResult<bool> {
    let path = status_path(deployment_id, context.readiness_attempt());
    let source = check_try!(fetch_github(context, path.as_str()));
    let statuses: Vec<DeploymentStatus> = check_try!(
        from_str(source.as_str())
            .map_err(|error| format!("parse public GitHub deployment statuses: {error}"))
    );
    return Ok(statuses.iter().any(|status| {
        return status_is_successful(status);
    }));
}

/// Return whether one deployment status has exact successful Mintlify provenance.
fn status_is_successful(status: &DeploymentStatus) -> bool {
    return status.creator.login == MINTLIFY_BOT_LOGIN
        && status.environment_url.as_deref() == Some(PUBLIC_DOCS_URL)
        && status.state == "success";
}

/// Build the exact public status query for one full readiness attempt.
fn status_path(deployment_id: u64, attempt: i64) -> String {
    return format!(
        "/repos/tovuk/tovuk/deployments/{deployment_id}/statuses?per_page=100&readiness_attempt={attempt}"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        Actor, ContentEntry, Deployment, DeploymentStatus, deployment_has_trusted_provenance,
        deployment_matches_exact_revision, docs_tree_sha, status_is_successful,
    };

    /// Verify repository content parsing selects only the canonical documentation directory.
    ///
    /// # Panics
    ///
    /// Panics when an unrelated entry is accepted or the canonical tree is rejected.
    #[test]
    fn resolves_the_canonical_docs_tree() {
        let docs_sha = "9933bb5a18bf82fc4295a2c7d6573483f9453f71";
        let entries = vec![
            ContentEntry {
                kind: "dir".to_owned(),
                path: "documentation".to_owned(),
                sha: "dd9230884a248a32cbfb275646c039686c9a4f8e".to_owned(),
            },
            ContentEntry {
                kind: "dir".to_owned(),
                path: "docs".to_owned(),
                sha: docs_sha.to_owned(),
            },
        ];
        assert_eq!(docs_tree_sha(entries), Ok(docs_sha.to_owned()));
    }

    /// Verify exact Mintlify deployment provenance is accepted and a foreign actor is rejected.
    ///
    /// # Panics
    ///
    /// Panics when deployment provenance validation accepts or rejects the wrong fixture.
    #[test]
    fn validates_exact_deployment_provenance() {
        let revision = "dd9230884a248a32cbfb275646c039686c9a4f8e";
        let trusted = Deployment {
            creator: Actor {
                login: "mintlify[bot]".to_owned(),
            },
            environment: "staging - docs".to_owned(),
            identifier: 0x0142_5f43,
            reference: "main".to_owned(),
            sha: revision.to_owned(),
            task: "deploy".to_owned(),
        };
        assert!(deployment_has_trusted_provenance(&trusted));
        assert!(deployment_matches_exact_revision(&trusted, revision));

        let intermediate_revision = "9933bb5a18bf82fc4295a2c7d6573483f9453f71";
        assert!(!deployment_matches_exact_revision(
            &trusted,
            intermediate_revision
        ));

        let untrusted = Deployment {
            creator: Actor {
                login: "untrusted[bot]".to_owned(),
            },
            environment: "staging - docs".to_owned(),
            identifier: 0x0142_5f44,
            reference: "main".to_owned(),
            sha: revision.to_owned(),
            task: "deploy".to_owned(),
        };
        assert!(!deployment_has_trusted_provenance(&untrusted));
    }

    /// Verify successful status validation requires the Mintlify actor and public docs URL.
    ///
    /// # Panics
    ///
    /// Panics when status provenance validation accepts or rejects the wrong fixture.
    #[test]
    fn validates_exact_status_provenance() {
        let trusted = DeploymentStatus {
            creator: Actor {
                login: "mintlify[bot]".to_owned(),
            },
            environment_url: Some("https://docs.tovuk.com".to_owned()),
            state: "success".to_owned(),
        };
        assert!(status_is_successful(&trusted));

        let wrong_url = DeploymentStatus {
            creator: Actor {
                login: "mintlify[bot]".to_owned(),
            },
            environment_url: Some("https://example.com".to_owned()),
            state: "success".to_owned(),
        };
        assert!(!status_is_successful(&wrong_url));
    }
}
