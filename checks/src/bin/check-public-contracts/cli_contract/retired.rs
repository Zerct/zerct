use crate::{
    helpers::{CheckResult, reject_contains, reject_contains_any},
    retired_contracts::RETIRED_PUBLIC_COMMANDS,
};

use super::ContractSources;

pub(super) fn reject_retired_commands(sources: &ContractSources) -> CheckResult {
    for source in [
        sources.cargo_cli.as_str(),
        sources.root_readme.as_str(),
        sources.cargo_readme.as_str(),
        sources.npm_readme.as_str(),
        sources.python_readme.as_str(),
        sources.docs_index.as_str(),
        sources.docs_quickstart.as_str(),
        sources.docs_agents.as_str(),
        sources.docs_packages.as_str(),
        sources.docs_llms.as_str(),
        sources.docs_skill.as_str(),
        sources.packaged_skill.as_str(),
    ] {
        for command in RETIRED_PUBLIC_COMMANDS {
            reject_contains(
                source,
                command,
                format!("retired public command {command}").as_str(),
            )?;
        }
    }
    Ok(())
}

pub(super) fn reject_retired_public_copy(sources: &ContractSources) -> CheckResult {
    for source in sources.public_sources() {
        let lower = source.to_lowercase();
        reject_contains_any(
            lower.as_str(),
            &[
                ("tovuk.toml", "retired deploy-platform wording tovuk.toml"),
                ("full-stack", "retired deploy-platform wording full-stack"),
                (
                    "static frontend",
                    "retired deploy-platform wording static frontend",
                ),
                (
                    "deploy workflow",
                    "retired deploy-platform wording deploy workflow",
                ),
                (
                    "deploy failed",
                    "retired deploy-platform wording deploy failed",
                ),
                (
                    "service snapshot",
                    "retired deploy-platform wording service snapshot",
                ),
                ("build id", "retired deploy-platform wording build id"),
                ("build logs", "retired deploy-platform wording build logs"),
                ("usage caps", "retired deploy-platform wording usage caps"),
            ],
        )?;
    }
    Ok(())
}

pub(super) fn reject_retired_cli_internals(sources: &ContractSources) -> CheckResult {
    reject_contains_any(
        sources.cargo_cli.as_str(),
        &[
            ("mod project;", "retired CLI internal wording mod project;"),
            ("project::", "retired CLI internal wording project::"),
            ("build job", "retired CLI internal wording build job"),
            ("Tovuk user", "retired CLI internal wording Tovuk user"),
        ],
    )
}
