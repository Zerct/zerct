#[path = "release_policy/crates.rs"]
mod crates;
#[path = "release_policy/native.rs"]
mod native;
#[path = "release_policy/wrappers.rs"]
mod wrappers;

use super::github_actions_policy::{Workflow, require_contains};

type ReleaseWorkflowCheck = fn(&Workflow, &mut Vec<String>);

struct ReleaseWorkflowPolicy {
    suffix: &'static str,
    checks: &'static [ReleaseWorkflowCheck],
}

const NATIVE_BINARY_RELEASE_CHECKS: &[ReleaseWorkflowCheck] =
    &[native::check_publish_workflow, require_package_versions];
const CRATE_RELEASE_CHECKS: &[ReleaseWorkflowCheck] =
    &[require_package_versions, crates::check_publish_workflow];
const WRAPPER_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    require_package_versions,
    wrappers::check_publish_workflow,
    wrappers::check_native_release_assets,
];

const RELEASE_WORKFLOW_POLICIES: &[ReleaseWorkflowPolicy] = &[
    ReleaseWorkflowPolicy {
        suffix: "publish-native-binaries.yml",
        checks: NATIVE_BINARY_RELEASE_CHECKS,
    },
    ReleaseWorkflowPolicy {
        suffix: "publish-crates.yml",
        checks: CRATE_RELEASE_CHECKS,
    },
    ReleaseWorkflowPolicy {
        suffix: "publish-npm.yml",
        checks: WRAPPER_RELEASE_CHECKS,
    },
    ReleaseWorkflowPolicy {
        suffix: "publish-pypi.yml",
        checks: WRAPPER_RELEASE_CHECKS,
    },
];

pub(super) fn check_public_package_release_order(workflow: &Workflow, findings: &mut Vec<String>) {
    let Some(policy) = RELEASE_WORKFLOW_POLICIES
        .iter()
        .find(|policy| workflow.path.ends_with(policy.suffix))
    else {
        return;
    };

    for check in policy.checks {
        check(workflow, findings);
    }
}

fn require_package_versions(workflow: &Workflow, findings: &mut Vec<String>) {
    require_contains(
        workflow.contents.as_str(),
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- package-versions",
        format!(
            "{}: publish workflows must verify all public package versions before publishing",
            workflow.path.display()
        )
        .as_str(),
        findings,
    );
}
