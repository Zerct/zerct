/// Public contract checks for package.
#[path = "cli_contract_module/package.rs"]
pub mod package;

/// Public contract checks for retired.
#[path = "cli_contract_module/retired.rs"]
pub mod retired;

use core::{iter::once, ops::Deref};

use crate::{
    helpers::{
        CheckResult, OutputChannel, read_package_json, read_sorted_texts_recursive, read_text,
        read_text_corpus, require_contains, write_line,
    },
    support_contract,
    types::PackageJson,
};

use package::{reject_retired_packaging, require_install_guides, require_package_metadata};

use retired::{reject_retired_cli_internals, reject_retired_commands, reject_retired_public_copy};

/// Ordered paths loaded into `ContractTextSources`.
const CONTRACT_TEXT_PATHS: &[&str; 0x0010] = &[
    "crates/tovuk/README.md",
    "docs/agents.mdx",
    "docs/index.mdx",
    "docs/llms.txt",
    "docs/openapi.json",
    "docs/reference/packages.mdx",
    "docs/quickstart.mdx",
    "docs/skill.md",
    "docs/support.mdx",
    "Formula/tovuk.rb",
    "packages/tovuk/README.md",
    "skills/tovuk/SKILL.md",
    "packages/tovuk-py/src/tovuk/cli.py",
    "packages/tovuk-py/pyproject.toml",
    "packages/tovuk-py/README.md",
    "README.md",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&ContractSources::load),
    size_of_val(&check),
    size_of_val(&build_cargo_cli_source),
    size_of_val(&build_npm_installer_source),
    size_of_val(&require_core_commands),
    size_of_val(&require_native_command_dispatch),
];

#[derive(Debug)]
/// Contract representation for `ContractSources`.
pub(super) struct ContractSources {
    /// Contract data stored in `cargo_cli`.
    cargo_cli: String,
    /// Contract data stored in `npm_install`.
    npm_install: String,
    /// Contract data stored in `npm_package`.
    npm_package: PackageJson,
    /// Text-backed public contract sources.
    texts: ContractTextSources,
}

impl ContractSources {
    /// Contract implementation for `load`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    pub(super) fn load() -> CheckResult<Self> {
        let text_values = check_try!(
            CONTRACT_TEXT_PATHS
                .iter()
                .map(|path| return read_text(*path))
                .collect::<CheckResult<Vec<_>>>()
        );
        return Ok(Self {
            cargo_cli: check_try!(build_cargo_cli_source()),
            npm_install: check_try!(build_npm_installer_source()),
            npm_package: check_try!(read_package_json("packages/tovuk/package.json")),
            texts: check_try!(ContractTextSources::try_from(text_values)),
        });
    }

    /// Contract implementation for `public_sources`.
    const fn public_sources(&self) -> [&str; 12] {
        return [
            self.texts.root_readme.as_str(),
            self.texts.cargo_readme.as_str(),
            self.texts.npm_readme.as_str(),
            self.texts.python_readme.as_str(),
            self.texts.homebrew_formula.as_str(),
            self.texts.docs_index.as_str(),
            self.texts.docs_quickstart.as_str(),
            self.texts.docs_agents.as_str(),
            self.texts.docs_packages.as_str(),
            self.texts.docs_llms.as_str(),
            self.texts.docs_skill.as_str(),
            self.texts.packaged_skill.as_str(),
        ];
    }

    /// Contract implementation for `support_api_doc_sources`.
    pub(super) const fn support_api_doc_sources(&self) -> [&str; 10] {
        return [
            self.texts.root_readme.as_str(),
            self.texts.cargo_readme.as_str(),
            self.texts.npm_readme.as_str(),
            self.texts.python_readme.as_str(),
            self.texts.docs_agents.as_str(),
            self.texts.docs_packages.as_str(),
            self.texts.docs_support.as_str(),
            self.texts.docs_llms.as_str(),
            self.texts.docs_skill.as_str(),
            self.texts.packaged_skill.as_str(),
        ];
    }

    /// Contract implementation for `support_command_sources`.
    pub(super) const fn support_command_sources(&self) -> [&str; 0x0007] {
        return [
            self.texts.root_readme.as_str(),
            self.texts.docs_agents.as_str(),
            self.texts.docs_packages.as_str(),
            self.texts.docs_support.as_str(),
            self.texts.docs_llms.as_str(),
            self.texts.packaged_skill.as_str(),
            self.cargo_cli.as_str(),
        ];
    }

    /// Contract implementation for `support_openapi_source`.
    pub(super) const fn support_openapi_source(&self) -> &str {
        return self.texts.docs_openapi.as_str();
    }

    /// Contract implementation for `support_public_sources`.
    pub(super) const fn support_public_sources(&self) -> [&str; 12] {
        return [
            self.texts.root_readme.as_str(),
            self.texts.cargo_readme.as_str(),
            self.texts.npm_readme.as_str(),
            self.texts.python_readme.as_str(),
            self.texts.docs_agents.as_str(),
            self.texts.docs_packages.as_str(),
            self.texts.docs_support.as_str(),
            self.texts.docs_llms.as_str(),
            self.texts.docs_skill.as_str(),
            self.texts.docs_openapi.as_str(),
            self.texts.packaged_skill.as_str(),
            self.cargo_cli.as_str(),
        ];
    }
}

impl Deref for ContractSources {
    type Target = ContractTextSources;

    fn deref(&self) -> &Self::Target {
        return &self.texts;
    }
}

#[derive(Debug)]
/// Text-backed sources loaded in `CONTRACT_TEXT_PATHS` order.
pub(super) struct ContractTextSources {
    /// Contract data stored in `cargo_readme`.
    cargo_readme: String,
    /// Contract data stored in `docs_agents`.
    docs_agents: String,
    /// Contract data stored in `docs_index`.
    docs_index: String,
    /// Contract data stored in `docs_llms`.
    docs_llms: String,
    /// Contract data stored in `docs_openapi`.
    docs_openapi: String,
    /// Contract data stored in `docs_packages`.
    docs_packages: String,
    /// Contract data stored in `docs_quickstart`.
    docs_quickstart: String,
    /// Contract data stored in `docs_skill`.
    docs_skill: String,
    /// Contract data stored in `docs_support`.
    docs_support: String,
    /// Contract data stored in `homebrew_formula`.
    homebrew_formula: String,
    /// Contract data stored in `npm_readme`.
    npm_readme: String,
    /// Contract data stored in `packaged_skill`.
    packaged_skill: String,
    /// Contract data stored in `python_cli`.
    python_cli: String,
    /// Contract data stored in `python_project`.
    python_project: String,
    /// Contract data stored in `python_readme`.
    python_readme: String,
    /// Contract data stored in `root_readme`.
    root_readme: String,
}

impl From<[String; 0x0010]> for ContractTextSources {
    fn from(value: [String; 0x0010]) -> Self {
        let [
            cargo_readme,
            docs_agents,
            docs_index,
            docs_llms,
            docs_openapi,
            docs_packages,
            docs_quickstart,
            docs_skill,
            docs_support,
            homebrew_formula,
            npm_readme,
            packaged_skill,
            python_cli,
            python_project,
            python_readme,
            root_readme,
        ] = value;
        return Self {
            cargo_readme,
            docs_agents,
            docs_index,
            docs_llms,
            docs_openapi,
            docs_packages,
            docs_quickstart,
            docs_skill,
            docs_support,
            homebrew_formula,
            npm_readme,
            packaged_skill,
            python_cli,
            python_project,
            python_readme,
            root_readme,
        };
    }
}

impl TryFrom<Vec<String>> for ContractTextSources {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        return <Vec<String> as TryInto<[String; 0x0010]>>::try_into(value)
            .map(Self::from)
            .map_err(|unexpected_values| {
                return format!(
                    "public contract source count must be 16, got {}",
                    unexpected_values.len()
                );
            });
    }
}

/// Load the native CLI root and recursively split module sources.
///
/// # Errors
///
/// Returns an error when a native CLI source cannot be read.
fn build_cargo_cli_source() -> CheckResult<String> {
    let root = check_try!(read_text("crates/tovuk/src/main.rs"));
    let modules = check_try!(read_sorted_texts_recursive("crates/tovuk/src/cli", ".rs"));
    return Ok(once(root).chain(modules).collect::<Vec<_>>().join("\n"));
}

/// Load both npm installer modules as one policy corpus.
///
/// # Errors
///
/// Returns an error when an npm installer source cannot be read.
fn build_npm_installer_source() -> CheckResult<String> {
    return read_text_corpus(&[
        "packages/tovuk/install.mjs",
        "packages/tovuk/install-policy.mjs",
    ]);
}

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check() -> CheckResult {
    let sources = check_try!(ContractSources::load());
    check_try!(require_native_command_dispatch(&sources));
    check_try!(require_core_commands(&sources));
    check_try!(support_contract::check(&sources));
    check_try!(require_install_guides(&sources));
    check_try!(require_package_metadata(&sources));
    check_try!(reject_retired_packaging(&sources));
    check_try!(reject_retired_cli_internals(&sources));
    check_try!(reject_retired_commands(&sources));
    check_try!(reject_retired_public_copy(&sources));
    check_try!(write_line(
        OutputChannel::Regular,
        "Checked scraper-only native CLI command and package contract.",
    ));
    return Ok(());
}

/// Contract implementation for `require_core_commands`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_core_commands(sources: &ContractSources) -> CheckResult {
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
    for source in once(sources.cargo_cli.as_str()).chain(sources.public_sources().iter().copied()) {
        for snippet in core_commands {
            check_try!(require_contains(
                source,
                snippet,
                format!("scraper-only public command {snippet}").as_str(),
            ));
        }
    }
    return Ok(());
}

/// Contract implementation for `require_native_command_dispatch`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_native_command_dispatch(sources: &ContractSources) -> CheckResult {
    for command in [
        "login", "account", "api-key", "pricing", "scraper", "request", "usage", "billing",
        "support",
    ] {
        check_try!(require_contains(
            sources.cargo_cli.as_str(),
            format!("{command:?}").as_str(),
            format!("native command {command}").as_str(),
        ));
    }
    return Ok(());
}
