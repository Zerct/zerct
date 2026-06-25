mod package;

use crate::{
    helpers::{
        CheckResult, read_package_json, read_sorted_texts_recursive, read_text, reject_contains,
        require_contains,
    },
    retired_contracts::RETIRED_PUBLIC_COMMANDS,
    support_contract,
    types::PackageJson,
};

use package::{reject_retired_packaging, require_install_guides, require_package_metadata};

#[derive(Debug)]
pub(crate) struct ContractSources {
    cargo_cli: String,
    root_readme: String,
    cargo_readme: String,
    npm_package: PackageJson,
    npm_install: String,
    npm_readme: String,
    python_cli: String,
    python_readme: String,
    python_project: String,
    homebrew_formula: String,
    docs_index: String,
    docs_quickstart: String,
    docs_agents: String,
    docs_packages: String,
    docs_llms: String,
    docs_skill: String,
    packaged_skill: String,
}

pub(crate) fn check() -> CheckResult {
    let sources = ContractSources::load()?;
    require_native_command_dispatch(&sources)?;
    require_core_commands(&sources)?;
    support_contract::check(&sources)?;
    require_install_guides(&sources)?;
    require_package_metadata(&sources)?;
    reject_retired_packaging(&sources)?;
    reject_retired_commands(&sources)?;
    reject_retired_public_copy(&sources)?;
    println!("Checked scraper-only native CLI command and package contract.");
    Ok(())
}

impl ContractSources {
    fn load() -> CheckResult<Self> {
        let mut cargo_cli_parts = vec![
            read_text("crates/tovuk/src/main.rs")?,
            read_text("crates/tovuk/src/cli.rs")?,
        ];
        cargo_cli_parts.extend(read_sorted_texts_recursive("crates/tovuk/src/cli", ".rs")?);
        Ok(Self {
            cargo_cli: cargo_cli_parts.join("\n"),
            root_readme: read_text("README.md")?,
            cargo_readme: read_text("crates/tovuk/README.md")?,
            npm_package: read_package_json("packages/tovuk/package.json")?,
            npm_install: read_text("packages/tovuk/install.mjs")?,
            npm_readme: read_text("packages/tovuk/README.md")?,
            python_cli: read_text("packages/tovuk-py/src/tovuk/cli.py")?,
            python_readme: read_text("packages/tovuk-py/README.md")?,
            python_project: read_text("packages/tovuk-py/pyproject.toml")?,
            homebrew_formula: read_text("Formula/tovuk.rb")?,
            docs_index: read_text("docs/index.mdx")?,
            docs_quickstart: read_text("docs/quickstart.mdx")?,
            docs_agents: read_text("docs/agents.mdx")?,
            docs_packages: read_text("docs/reference/packages.mdx")?,
            docs_llms: read_text("docs/llms.txt")?,
            docs_skill: read_text("docs/skill.md")?,
            packaged_skill: read_text("skills/tovuk/SKILL.md")?,
        })
    }

    fn public_sources(&self) -> [&str; 12] {
        [
            self.root_readme.as_str(),
            self.cargo_readme.as_str(),
            self.npm_readme.as_str(),
            self.python_readme.as_str(),
            self.homebrew_formula.as_str(),
            self.docs_index.as_str(),
            self.docs_quickstart.as_str(),
            self.docs_agents.as_str(),
            self.docs_packages.as_str(),
            self.docs_llms.as_str(),
            self.docs_skill.as_str(),
            self.packaged_skill.as_str(),
        ]
    }

    pub(crate) fn support_command_sources(&self) -> [&str; 6] {
        [
            self.root_readme.as_str(),
            self.docs_agents.as_str(),
            self.docs_packages.as_str(),
            self.docs_llms.as_str(),
            self.packaged_skill.as_str(),
            self.cargo_cli.as_str(),
        ]
    }

    pub(crate) fn support_api_doc_sources(&self) -> [&str; 9] {
        [
            self.root_readme.as_str(),
            self.cargo_readme.as_str(),
            self.npm_readme.as_str(),
            self.python_readme.as_str(),
            self.docs_agents.as_str(),
            self.docs_packages.as_str(),
            self.docs_llms.as_str(),
            self.docs_skill.as_str(),
            self.packaged_skill.as_str(),
        ]
    }
}

fn require_native_command_dispatch(sources: &ContractSources) -> CheckResult {
    for command in [
        "login", "account", "api-key", "pricing", "scraper", "request", "usage", "billing",
        "support",
    ] {
        require_contains(
            sources.cargo_cli.as_str(),
            format!("{command:?}").as_str(),
            format!("native command {command}").as_str(),
        )?;
    }
    Ok(())
}

fn require_core_commands(sources: &ContractSources) -> CheckResult {
    let core_commands = [
        "tovuk account show",
        "tovuk api-key create",
        "tovuk api-key list",
        "tovuk api-key revoke",
        "tovuk pricing",
        "tovuk scraper list",
        "tovuk scraper health",
        "tovuk scraper show",
        "tovuk request create",
        "tovuk request show",
        "tovuk request results",
        "tovuk usage",
        "tovuk billing checkout",
        "tovuk billing portal",
    ];
    for source in
        std::iter::once(sources.cargo_cli.as_str()).chain(sources.public_sources().iter().copied())
    {
        for snippet in core_commands {
            require_contains(
                source,
                snippet,
                format!("scraper-only public command {snippet}").as_str(),
            )?;
        }
    }
    Ok(())
}

fn reject_retired_commands(sources: &ContractSources) -> CheckResult {
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

fn reject_retired_public_copy(sources: &ContractSources) -> CheckResult {
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
