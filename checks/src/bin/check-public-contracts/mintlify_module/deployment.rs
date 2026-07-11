use crate::helpers::CheckResult;

use crate::mintlify_fetch::{FetchContext, RequestHeaders, fetch_text_from_base};

use serde::Deserialize;

use serde_json::from_str;

/// Public `GitHub` API origin used to verify the hosting-provider deployment record.
const GITHUB_API_BASE_URL: &str = "https://api.github.com";

/// Versioned public `GitHub` API headers.
const GITHUB_API_HEADERS: &RequestHeaders = &[
    ("Accept", "application/vnd.github+json"),
    ("X-GitHub-Api-Version", "2022-11-28"),
];

/// Exact public hosting environment created by the Mintlify `GitHub` App.
const MINTLIFY_ENVIRONMENT: &str = "staging - docs";

/// Exact public documentation origin recorded on a successful deployment.
const PUBLIC_DOCS_URL: &str = "https://docs.tovuk.com";

/// Compile-time references preserve the deployment verification boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&check_exact_deployment),
    size_of_val(&deployment_path),
    size_of_val(&fetch_deployments),
    size_of_val(&has_successful_status),
    size_of_val(&status_path),
];

/// Public `GitHub` deployment fields required for exact revision verification.
#[derive(Deserialize)]
struct Deployment {
    /// Deployment environment name.
    environment: String,
    /// Public deployment identifier.
    #[serde(rename = "id")]
    identifier: u64,
    /// Full deployed Git object identifier.
    sha: String,
}

/// Public `GitHub` deployment status fields required for readiness verification.
#[derive(Deserialize)]
struct DeploymentStatus {
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
    let Some(revision) = context.deployment_revision() else {
        return Ok(());
    };
    let deployments = check_try!(fetch_deployments(context, revision));
    for deployment in deployments {
        if deployment.sha != revision || deployment.environment != MINTLIFY_ENVIRONMENT {
            continue;
        }
        if check_try!(has_successful_status(context, deployment.identifier)) {
            return Ok(());
        }
    }
    return Err(format!(
        "Mintlify has no successful {MINTLIFY_ENVIRONMENT} deployment for revision {revision}"
    ));
}

/// Build the exact public deployment query for one full readiness attempt.
fn deployment_path(revision: &str, attempt: i64) -> String {
    return format!(
        "/repos/tovuk/tovuk/deployments?sha={revision}&per_page=100&readiness_attempt={attempt}"
    );
}

/// Fetch and parse public deployments for one immutable revision.
///
/// # Errors
///
/// Returns an error when the public deployment API cannot be fetched or decoded.
fn fetch_deployments(context: &FetchContext, revision: &str) -> CheckResult<Vec<Deployment>> {
    let path = deployment_path(revision, context.readiness_attempt());
    let source = check_try!(fetch_text_from_base(
        context,
        GITHUB_API_BASE_URL,
        path.as_str(),
        GITHUB_API_HEADERS,
    ));
    return from_str(source.as_str())
        .map_err(|error| format!("parse public GitHub deployments: {error}"));
}

/// Return whether one exact deployment has a successful public docs status.
///
/// # Errors
///
/// Returns an error when the public deployment status API cannot be fetched or decoded.
fn has_successful_status(context: &FetchContext, deployment_id: u64) -> CheckResult<bool> {
    let path = status_path(deployment_id, context.readiness_attempt());
    let source = check_try!(fetch_text_from_base(
        context,
        GITHUB_API_BASE_URL,
        path.as_str(),
        GITHUB_API_HEADERS,
    ));
    let statuses: Vec<DeploymentStatus> = check_try!(
        from_str(source.as_str())
            .map_err(|error| format!("parse public GitHub deployment statuses: {error}"))
    );
    return Ok(statuses.iter().any(|status| {
        return status.state == "success"
            && status.environment_url.as_deref() == Some(PUBLIC_DOCS_URL);
    }));
}

/// Build the exact public status query for one full readiness attempt.
fn status_path(deployment_id: u64, attempt: i64) -> String {
    return format!(
        "/repos/tovuk/tovuk/deployments/{deployment_id}/statuses?per_page=100&readiness_attempt={attempt}"
    );
}
