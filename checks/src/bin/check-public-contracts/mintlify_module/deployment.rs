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
const _: [usize; 0x000b] = [
    size_of_val(&check_exact_deployment),
    size_of_val(&deployment_matches),
    size_of_val(&deployment_path),
    size_of_val(&docs_commit_path),
    size_of_val(&fetch_deployments),
    size_of_val(&fetch_docs_revision),
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

/// Public `GitHub` commit fields required to resolve the deployed docs ancestor.
#[derive(Deserialize)]
struct Commit {
    /// Full Git object identifier.
    sha: String,
}

/// Public `GitHub` deployment fields required for exact revision verification.
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

/// Require a successful Mintlify deployment for the exact configured Git revision.
///
/// # Errors
///
/// Returns an error when the expected revision has no successful public docs deployment.
pub(super) fn check_exact_deployment(context: &FetchContext) -> CheckResult {
    let Some(commit_revision) = context.commit_revision() else {
        return Ok(());
    };
    let docs_revision = check_try!(fetch_docs_revision(context, commit_revision));
    let deployments = check_try!(fetch_deployments(context, docs_revision.as_str()));
    for deployment in deployments {
        if !deployment_matches(&deployment, docs_revision.as_str()) {
            continue;
        }
        if check_try!(has_successful_status(context, deployment.identifier)) {
            return Ok(());
        }
    }
    return Err(format!(
        "Mintlify has no successful {MINTLIFY_ENVIRONMENT} deployment for docs revision {docs_revision}"
    ));
}

/// Return whether one deployment has exact Mintlify provenance and revision data.
fn deployment_matches(deployment: &Deployment, revision: &str) -> bool {
    return deployment.creator.login == MINTLIFY_BOT_LOGIN
        && deployment.environment == MINTLIFY_ENVIRONMENT
        && deployment.reference == "main"
        && deployment.sha == revision
        && deployment.task == "deploy";
}

/// Build the exact public deployment query for one full readiness attempt.
fn deployment_path(revision: &str, attempt: i64) -> String {
    return format!(
        "/repos/tovuk/tovuk/deployments?sha={revision}&per_page=100&readiness_attempt={attempt}"
    );
}

/// Build the exact query for the latest docs-changing ancestor of one commit.
fn docs_commit_path(revision: &str, attempt: i64) -> String {
    return format!(
        "/repos/tovuk/tovuk/commits?sha={revision}&path=docs&per_page=1&readiness_attempt={attempt}"
    );
}

/// Fetch and parse public deployments for one immutable revision.
///
/// # Errors
///
/// Returns an error when the public deployment API cannot be fetched or decoded.
fn fetch_deployments(context: &FetchContext, revision: &str) -> CheckResult<Vec<Deployment>> {
    let path = deployment_path(revision, context.readiness_attempt());
    let source = check_try!(fetch_github(context, path.as_str()));
    return from_str(source.as_str())
        .map_err(|error| format!("parse public GitHub deployments: {error}"));
}

/// Resolve the latest docs-changing ancestor reachable from the current commit.
///
/// # Errors
///
/// Returns an error when no docs commit exists or the public commit API response is invalid.
fn fetch_docs_revision(context: &FetchContext, revision: &str) -> CheckResult<String> {
    let path = docs_commit_path(revision, context.readiness_attempt());
    let source = check_try!(fetch_github(context, path.as_str()));
    let commits: Vec<Commit> = check_try!(
        from_str(source.as_str()).map_err(|error| format!("parse public GitHub commits: {error}"))
    );
    let docs_revision = check_try!(
        commits
            .into_iter()
            .next()
            .map(|commit| return commit.sha)
            .ok_or_else(|| format!("revision {revision} has no docs-changing ancestor"))
    );
    check_try!(validate_revision(docs_revision.as_str()));
    return Ok(docs_revision);
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
    use super::{Actor, Deployment, DeploymentStatus, deployment_matches, status_is_successful};

    /// Verify exact Mintlify deployment provenance is accepted and a foreign actor is rejected.
    ///
    /// # Panics
    ///
    /// Panics when deployment provenance validation accepts or rejects the wrong fixture.
    #[test]
    fn validates_exact_deployment_provenance() {
        let revision = "6c3159be79131fc71faa678d8e09a0ad31191615";
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
        assert!(deployment_matches(&trusted, revision));

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
        assert!(!deployment_matches(&untrusted, revision));
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
