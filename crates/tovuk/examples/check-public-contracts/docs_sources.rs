use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{
    helpers::{
        CheckResult, read_json, read_sorted_texts_recursive, read_text,
        reject_forbidden_public_copy_terms,
    },
    types::DocsJson,
};

#[derive(Debug)]
pub(crate) struct DocsSources {
    pub(crate) nav_pages: String,
    pub(crate) openapi: String,
    pub(crate) readme: String,
    pub(crate) pricing: String,
    pub(crate) scrapers: String,
    pub(crate) agents: String,
    pub(crate) packages: String,
    pub(crate) llms: String,
    pub(crate) skill: String,
    pub(crate) packaged_skill: String,
    pub(crate) status: String,
    pub(crate) public_copy: String,
}

impl DocsSources {
    pub(crate) fn load(pages: &[String]) -> CheckResult<Self> {
        let openapi = read_text("docs/openapi.json")?;
        let readme = read_text("README.md")?;
        let pricing = read_text("docs/pricing.mdx")?;
        let scrapers = read_text("docs/scrapers.mdx")?;
        let agents = read_text("docs/agents.mdx")?;
        let packages = read_text("docs/reference/packages.mdx")?;
        let llms = read_text("docs/llms.txt")?;
        let skill = read_text("docs/skill.md")?;
        let packaged_skill = read_text("skills/tovuk/SKILL.md")?;
        let docs_json_source = read_text("docs/docs.json")?;
        let all_mdx_docs = read_sorted_texts_recursive("docs", ".mdx")?.join("\n");
        let cargo_readme = read_text("crates/tovuk/README.md")?;
        let npm_readme = read_text("packages/tovuk/README.md")?;
        let npm_package = read_text("packages/tovuk/package.json")?;
        let python_readme = read_text("packages/tovuk-py/README.md")?;
        let python_project = read_text("packages/tovuk-py/pyproject.toml")?;
        let public_copy = [
            readme.as_str(),
            openapi.as_str(),
            docs_json_source.as_str(),
            llms.as_str(),
            skill.as_str(),
            all_mdx_docs.as_str(),
            cargo_readme.as_str(),
            npm_readme.as_str(),
            npm_package.as_str(),
            python_readme.as_str(),
            python_project.as_str(),
            packaged_skill.as_str(),
        ]
        .join("\n");
        reject_forbidden_public_copy_terms("public docs and package copy", public_copy.as_str())?;
        Ok(Self {
            nav_pages: pages.join("\n"),
            openapi,
            readme,
            pricing,
            scrapers,
            agents,
            packages,
            llms,
            skill,
            packaged_skill,
            status: read_text("docs/status.mdx")?,
            public_copy,
        })
    }
}

pub(crate) fn read_navigation_pages() -> CheckResult<Vec<String>> {
    let docs: DocsJson = read_json("docs/docs.json")?;
    let mut pages = Vec::new();
    for tab in docs.navigation.tabs {
        for group in tab.groups {
            for page in group.pages {
                let Value::String(page_name) = page else {
                    return Err("docs navigation page entries must be strings".to_owned());
                };
                pages.push(page_name);
            }
        }
    }
    Ok(pages)
}

pub(crate) fn openapi_config_path() -> CheckResult<PathBuf> {
    let docs: DocsJson = read_json("docs/docs.json")?;
    let openapi = docs.api.openapi.trim();
    if openapi.is_empty() {
        return Err("docs/docs.json must set api.openapi".to_owned());
    }
    Ok(Path::new("docs").join(openapi))
}
