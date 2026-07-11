//! Public package release workflow policy.

#[path = "release_policy/crates.rs"]
mod crates;
#[path = "release_policy/native.rs"]
mod native;
#[path = "release_policy/wrappers.rs"]
mod wrappers;

use super::{
    CrateReleasePolicy as _, HostedActionsCheck, NativeReleasePolicy as _, ReleaseCheck,
    ReleasePolicy, Workflow, WrapperReleasePolicy as _, require_contains,
};

/// Ordered checks for the Crates.io release workflow.
const CRATE_RELEASE_CHECKS: &[ReleaseCheck] =
    &[ReleaseCheck::PackageVersions, ReleaseCheck::CratePublishing];

/// Ordered checks for the native binary release workflow.
const NATIVE_BINARY_RELEASE_CHECKS: &[ReleaseCheck] = &[
    ReleaseCheck::NativePublishing,
    ReleaseCheck::PackageVersions,
];

/// Release workflow policies selected by filename suffix.
const RELEASE_WORKFLOW_POLICIES: &[ReleaseWorkflowPolicy] = &[
    ReleaseWorkflowPolicy {
        checks: NATIVE_BINARY_RELEASE_CHECKS,
        suffix: "publish-native-binaries.yml",
    },
    ReleaseWorkflowPolicy {
        checks: CRATE_RELEASE_CHECKS,
        suffix: "publish-crates.yml",
    },
    ReleaseWorkflowPolicy {
        checks: WRAPPER_RELEASE_CHECKS,
        suffix: "publish-npm.yml",
    },
    ReleaseWorkflowPolicy {
        checks: WRAPPER_RELEASE_CHECKS,
        suffix: "publish-pypi.yml",
    },
];

/// Ordered checks for language wrapper release workflows.
const WRAPPER_RELEASE_CHECKS: &[ReleaseCheck] = &[
    ReleaseCheck::PackageVersions,
    ReleaseCheck::WrapperPublishing,
    ReleaseCheck::NativeReleaseAssets,
];

impl ReleasePolicy for HostedActionsCheck {
    fn check_public_package_release_order(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        let Some(policy) = RELEASE_WORKFLOW_POLICIES
            .iter()
            .find(|policy| return workflow.path.ends_with(policy.suffix))
        else {
            return;
        };

        for check in policy.checks.iter().copied() {
            self.execute_release_check(check, workflow, findings);
        }
    }

    fn execute_release_check(
        &self,
        check: ReleaseCheck,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    ) {
        match check {
            ReleaseCheck::CratePublishing => {
                self.check_crate_release_workflow(workflow, findings);
            }
            ReleaseCheck::NativePublishing => {
                self.check_native_release_workflow(workflow, findings);
            }
            ReleaseCheck::NativeReleaseAssets => {
                self.check_wrapper_release_assets(workflow, findings);
            }
            ReleaseCheck::PackageVersions => {
                self.require_package_versions(workflow, findings);
            }
            ReleaseCheck::WrapperPublishing => {
                self.check_wrapper_release_workflow(workflow, findings);
            }
        }
    }

    fn require_package_versions(&self, workflow: &Workflow, findings: &mut Vec<String>) {
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
}

/// Ordered policy assigned to one release workflow suffix.
#[derive(Clone, Copy, Debug)]
struct ReleaseWorkflowPolicy {
    /// Checks to execute in order.
    checks: &'static [ReleaseCheck],
    /// Workflow filename suffix selecting the policy.
    suffix: &'static str,
}
