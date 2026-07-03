use crate::{
    helpers::{CheckResult, reject_contains},
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
        for snippet in [
            "tovuk.toml",
            "full-stack",
            "static frontend",
            "deploy workflow",
            "deploy failed",
            "service snapshot",
            "build id",
            "build logs",
            "usage caps",
        ] {
            reject_contains(
                lower.as_str(),
                snippet,
                format!("retired deploy-platform wording {snippet}").as_str(),
            )?;
        }
    }
    Ok(())
}

pub(super) fn reject_retired_cli_internals(sources: &ContractSources) -> CheckResult {
    for snippet in ["mod project;", "project::", "build job", "Tovuk user"] {
        reject_contains(
            sources.cargo_cli.as_str(),
            snippet,
            format!("retired CLI internal wording {snippet}").as_str(),
        )?;
    }
    Ok(())
}
