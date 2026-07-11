use crate::{
    helpers::{
        CheckResult, read_json, read_sorted_texts_recursive, read_text, read_text_corpus,
        reject_forbidden_public_copy_terms,
    },
    types::DocsJson,
};

use serde_json::Value;

use std::path::{Path, PathBuf};

/// Non-MDX public copy included in leakage and positioning scans.
const PUBLIC_COPY_PATHS: &[&str] = &[
    "README.md",
    "docs/openapi.json",
    "docs/docs.json",
    "docs/llms.txt",
    "docs/skill.md",
    "crates/tovuk/README.md",
    "packages/tovuk/README.md",
    "packages/tovuk/package.json",
    "packages/tovuk-py/README.md",
    "packages/tovuk-py/pyproject.toml",
    "skills/tovuk/SKILL.md",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0003] = [
    size_of_val(&DocsSources::load),
    size_of_val(&openapi_config_path),
    size_of_val(&read_navigation_pages),
];

#[derive(Debug)]
/// Contract representation for `DocsSources`.
pub(super) struct DocsSources {
    /// Contract data stored in `agents`.
    pub agents: String,
    /// Contract data stored in `llms`.
    pub llms: String,
    /// Contract data stored in `nav_pages`.
    pub nav_pages: String,
    /// Contract data stored in `openapi`.
    pub openapi: String,
    /// Contract data stored in `packaged_skill`.
    pub packaged_skill: String,
    /// Contract data stored in `packages`.
    pub packages: String,
    /// Contract data stored in `pricing`.
    pub pricing: String,
    /// Contract data stored in `public_copy`.
    pub public_copy: String,
    /// Contract data stored in `readme`.
    pub readme: String,
    /// Contract data stored in `scrapers`.
    pub scrapers: String,
    /// Contract data stored in `skill`.
    pub skill: String,
    /// Contract data stored in `status`.
    pub status: String,
    /// Contract data stored in `support`.
    pub support: String,
}

impl DocsSources {
    /// Contract implementation for `load`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    pub(super) fn load(pages: &[String]) -> CheckResult<Self> {
        let public_copy = [
            check_try!(read_text_corpus(PUBLIC_COPY_PATHS)),
            check_try!(read_sorted_texts_recursive("docs", ".mdx")).join("\n"),
        ]
        .join("\n");
        check_try!(reject_forbidden_public_copy_terms(
            "public docs and package copy",
            public_copy.as_str()
        ));
        return Ok(Self {
            agents: check_try!(read_text("docs/agents.mdx")),
            llms: check_try!(read_text("docs/llms.txt")),
            nav_pages: pages.join("\n"),
            openapi: check_try!(read_text("docs/openapi.json")),
            packaged_skill: check_try!(read_text("skills/tovuk/SKILL.md")),
            packages: check_try!(read_text("docs/reference/packages.mdx")),
            pricing: check_try!(read_text("docs/pricing.mdx")),
            public_copy,
            readme: check_try!(read_text("README.md")),
            scrapers: check_try!(read_text("docs/scrapers.mdx")),
            skill: check_try!(read_text("docs/skill.md")),
            status: check_try!(read_text("docs/status.mdx")),
            support: check_try!(read_text("docs/support.mdx")),
        });
    }
}

/// Contract implementation for `openapi_config_path`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn openapi_config_path() -> CheckResult<PathBuf> {
    let docs: DocsJson = check_try!(read_json("docs/docs.json"));
    let openapi = docs.api.openapi.trim();
    if openapi.is_empty() {
        return Err("docs/docs.json must set api.openapi".to_owned());
    }
    return Ok(Path::new("docs").join(openapi));
}

/// Contract implementation for `read_navigation_pages`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn read_navigation_pages() -> CheckResult<Vec<String>> {
    let docs: DocsJson = check_try!(read_json("docs/docs.json"));
    let mut pages = Vec::new();
    let navigation_pages = docs
        .navigation
        .tabs
        .into_iter()
        .flat_map(|tab| return tab.groups)
        .flat_map(|group| return group.pages);
    for page in navigation_pages {
        let Value::String(page_name) = page else {
            return Err("docs navigation page entries must be strings".to_owned());
        };
        pages.push(page_name);
    }
    return Ok(pages);
}
